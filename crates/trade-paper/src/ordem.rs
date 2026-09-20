//! Montagem da ordem que vai para a Bybit.
//!
//! Três coisas que esta camada decide, e que o backtest nunca precisou
//! decidir porque não havia contraparte:
//!
//! 1. **Identificador de cliente.** Único por ordem. É o que faz um reenvio
//!    ser reconhecido como a mesma ordem em vez de virar uma segunda.
//! 2. **Stop junto da entrada.** A posição não existe sem stop aceito
//!    (`REQ-EXEC-011`), e por isso ele vai na mesma requisição — não numa
//!    segunda que pode falhar sozinha.
//! 3. **Stop a mercado no gatilho.** A banda de preço da Bybit para ordem
//!    limitada é mais estreita que o stop de 2% dos perfis: enviado como
//!    limitada, ele seria cancelado em silêncio (`REQ-EXEC-010`).

use rust_decimal::Decimal;
use trade_domain::{Money, Order, Qty, Side};

pub const CAMINHO_CRIAR: &str = "/v5/order/create";

/// Identificador que acompanha a ordem na corretora.
///
/// Formato `trade-{run}-{ordem}`: carrega a execução e a ordem, o que torna
/// possível reconciliar sem consultar o nosso registro. A Bybit aceita até 36
/// caracteres alfanuméricos com hífen e sublinhado.
pub fn order_link_id(run_id: &str, ordem: u64) -> String {
    let run = run_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(20)
        .collect::<String>()
        .to_ascii_lowercase();
    format!("trade-{run}-{ordem}")
}

/// O corpo JSON da criação de ordem.
///
/// Montado à mão e não por serialização de struct: a ordem dos campos é
/// irrelevante para a Bybit, mas o corpo assinado precisa ser **byte a byte**
/// o corpo enviado, e uma serialização que reordene campos entre a assinatura
/// e o envio produz recusa com mensagem genérica.
pub fn corpo_de_criacao(
    simbolo: &str,
    order: &Order,
    link_id: &str,
    stop: Option<Money>,
) -> String {
    let side = match order.side {
        Side::Buy => "Buy",
        Side::Sell => "Sell",
    };
    let mut c = format!(
        r#"{{"category":"spot","symbol":"{simbolo}","side":"{side}","orderType":"Market","qty":"{}","marketUnit":"baseCoin","orderLinkId":"{link_id}""#,
        order.qty
    );
    if let Some(gatilho) = stop {
        // slOrderType Market: no gatilho vira ordem a mercado, que a banda de
        // preço de ordem limitada não alcança.
        c.push_str(&format!(
            r#","stopLoss":"{gatilho}","slOrderType":"Market""#
        ));
    }
    c.push('}');
    c
}

/// Preço de gatilho do stop, a partir do preço de referência.
///
/// Só existe para compra: em spot comprado, o stop protege a posição detida.
/// Uma venda **é** a saída, e não leva stop próprio.
pub fn preco_do_stop(side: Side, referencia: Money, fracao: Decimal) -> Option<Money> {
    match side {
        Side::Buy if fracao > Decimal::ZERO => {
            Some((referencia * (Decimal::ONE - fracao)).round_dp(2))
        }
        _ => None,
    }
}

/// Quantidade efetivamente recebida, dada a taxa cobrada em moeda base.
pub fn recebido(qty: Qty, fee_base: Qty) -> Qty {
    qty - fee_base
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use rust_decimal::dec;
    use trade_domain::{OrderId, SignalId};

    fn ordem(side: Side, qty: Qty) -> Order {
        Order {
            id: OrderId(7),
            signal_ref: SignalId(1),
            side,
            qty,
            at: Utc.with_ymd_and_hms(2026, 9, 20, 12, 0, 0).unwrap(),
        }
    }

    #[test]
    fn o_identificador_carrega_execucao_e_ordem() {
        let id = order_link_id("01M30EXVCXVQ63XS0SSRSPDRJ8", 42);
        assert!(id.starts_with("trade-"));
        assert!(id.ends_with("-42"));
        assert!(
            id.len() <= 36,
            "a Bybit aceita até 36: {} tem {}",
            id,
            id.len()
        );
    }

    #[test]
    fn ordens_diferentes_tem_identificadores_diferentes() {
        // É o que impede um reenvio de virar ordem duplicada (FR-106).
        let a = order_link_id("run", 1);
        let b = order_link_id("run", 2);
        assert_ne!(a, b);
    }

    #[test]
    fn o_identificador_so_usa_caracteres_que_a_bybit_aceita() {
        let id = order_link_id("run/com:caracteres!estranhos", 1);
        assert!(
            id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
            "saiu {id}"
        );
    }

    #[test]
    fn a_compra_leva_stop_a_mercado_no_gatilho() {
        let stop = preco_do_stop(Side::Buy, dec!(81233.70), dec!(0.02)).unwrap();
        let c = corpo_de_criacao(
            "BTCUSDT",
            &ordem(Side::Buy, dec!(0.000615)),
            "trade-x-1",
            Some(stop),
        );
        assert!(c.contains(r#""stopLoss":"79609.03""#), "corpo: {c}");
        assert!(
            c.contains(r#""slOrderType":"Market""#),
            "stop limitado seria cancelado pela banda de preço: {c}"
        );
    }

    #[test]
    fn a_venda_nao_leva_stop() {
        // Em spot comprado a venda é a saída. Um stop nela seria stop de uma
        // posição que a ordem está justamente encerrando.
        assert_eq!(preco_do_stop(Side::Sell, dec!(81233.70), dec!(0.02)), None);
    }

    #[test]
    fn o_corpo_declara_spot_e_quantidade_em_moeda_base() {
        let c = corpo_de_criacao(
            "BTCUSDT",
            &ordem(Side::Buy, dec!(0.000615)),
            "trade-x-1",
            None,
        );
        assert!(c.contains(r#""category":"spot""#), "{c}");
        assert!(
            c.contains(r#""marketUnit":"baseCoin""#),
            "sem isso a Bybit interpretaria a quantidade como valor em USDT: {c}"
        );
        assert!(c.contains(r#""qty":"0.000615""#), "{c}");
        assert!(
            !c.contains("stopLoss"),
            "sem stop pedido, sem stop no corpo"
        );
    }

    #[test]
    fn o_corpo_e_json_valido() {
        let c = corpo_de_criacao("BTCUSDT", &ordem(Side::Buy, dec!(1)), "id", Some(dec!(100)));
        let v: serde_json::Value = serde_json::from_str(&c).expect("corpo precisa ser JSON válido");
        assert_eq!(v["category"], "spot");
        assert_eq!(v["slOrderType"], "Market");
    }

    #[test]
    fn o_recebido_desconta_a_taxa_em_moeda_base() {
        assert_eq!(
            recebido(dec!(0.000615), dec!(0.000000615)),
            dec!(0.000614385)
        );
    }
}
