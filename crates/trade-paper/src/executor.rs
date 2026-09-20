//! `OrderExecutor` contra a testnet.
//!
//! O mesmo lugar que o `SimulatedExecutor` ocupa no backtest — dentro do
//! `RiskGuard`, inalcançável por quem o construiu. O que muda é que aqui existe
//! contraparte, e com ela três coisas que a simulação nunca teve:
//!
//! - o preenchimento chega **depois**, em outra requisição;
//! - a taxa vem na moeda que a corretora decide, e não na que presumimos;
//! - a resposta pode não chegar, e aí ninguém sabe se a ordem existe.
//!
//! A terceira é a que governa o desenho: `execute` nunca reenvia. Diante de
//! resultado desconhecido devolve falha de integridade, que o `RiskGuard`
//! trata como parada — e a retomada exige reconciliar (FR-107).

use crate::erros::{de_ret_code, de_transporte};
use crate::ordem::{CAMINHO_CRIAR, corpo_de_criacao, order_link_id, preco_do_stop};
use crate::transporte::Transporte;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::cell::RefCell;
use std::rc::Rc;
use trade_domain::{Fill, Money, Order, Qty, Side};
use trade_ports::{ExecError, OrderExecutor};

/// Caminho de consulta do preenchimento.
pub const CAMINHO_EXECUCOES: &str = "/v5/execution/list";

/// Canal por onde o laço informa o preço de referência corrente.
///
/// Mesmo papel do `CandleFeed` do backtest, e pela mesma razão: o executor
/// está dentro do `RiskGuard` e não é alcançável, mas precisa saber sobre que
/// preço calcular o stop.
#[derive(Debug, Clone, Default)]
pub struct PrecoCorrente(Rc<RefCell<Option<Money>>>);

impl PrecoCorrente {
    pub fn set(&self, p: Money) {
        *self.0.borrow_mut() = Some(p);
    }
    pub fn get(&self) -> Option<Money> {
        *self.0.borrow()
    }
}

pub struct PaperExecutor<T: Transporte> {
    transporte: T,
    simbolo: String,
    run_id: String,
    stop_fracao: Decimal,
    preco: PrecoCorrente,
}

impl<T: Transporte> PaperExecutor<T> {
    pub fn novo(
        transporte: T,
        simbolo: impl Into<String>,
        run_id: impl Into<String>,
        stop_fracao: Decimal,
        preco: PrecoCorrente,
    ) -> Self {
        Self {
            transporte,
            simbolo: simbolo.into(),
            run_id: run_id.into(),
            stop_fracao,
            preco,
        }
    }
}

#[derive(Deserialize)]
struct Envelope<R> {
    #[serde(rename = "retCode")]
    ret_code: i64,
    #[serde(rename = "retMsg")]
    ret_msg: String,
    result: Option<R>,
}

#[derive(Deserialize)]
struct Criada {
    #[serde(rename = "orderLinkId")]
    order_link_id: String,
}

#[derive(Deserialize)]
struct ListaDeExecucoes {
    list: Vec<Execucao>,
}

#[derive(Deserialize)]
struct Execucao {
    #[serde(rename = "execPrice")]
    exec_price: String,
    #[serde(rename = "execQty")]
    exec_qty: String,
    #[serde(rename = "execFee")]
    exec_fee: String,
    #[serde(rename = "feeCurrency", default)]
    fee_currency: String,
}

fn decimal(campo: &str, v: &str) -> Result<Decimal, ExecError> {
    v.parse::<Decimal>().map_err(|_| {
        ExecError::Integrity(format!(
            "campo `{campo}` da corretora não é decimal: {v:?}. Não dá para \
             reconciliar posição sobre um número que não se sabe ler."
        ))
    })
}

impl<T: Transporte> PaperExecutor<T> {
    fn enviar(&self, order: &Order, link_id: &str) -> Result<(), ExecError> {
        let referencia = self.preco.get().ok_or_else(|| {
            ExecError::Integrity(
                "sem preço de referência: o stop não pode ser calculado, e sem stop \
                 aceito a posição não existe (REQ-EXEC-011)."
                    .into(),
            )
        })?;
        let stop = preco_do_stop(order.side, referencia, self.stop_fracao);
        if order.side == Side::Buy && stop.is_none() {
            return Err(ExecError::Integrity(
                "entrada sem stop: a posição não pode ser aberta (REQ-EXEC-011)".into(),
            ));
        }

        let corpo = corpo_de_criacao(&self.simbolo, order, link_id, stop);
        let bruto = self
            .transporte
            .post(CAMINHO_CRIAR, &corpo)
            .map_err(|e| de_transporte(&e))?;

        let env: Envelope<Criada> = serde_json::from_str(&bruto).map_err(|e| {
            ExecError::Integrity(format!(
                "resposta ilegível da criação de ordem ({e}): a ordem pode existir. \
                 Reconciliar antes de qualquer ordem nova."
            ))
        })?;
        if env.ret_code != 0 {
            return Err(de_ret_code(env.ret_code, &env.ret_msg));
        }
        match env.result {
            Some(c) if c.order_link_id == link_id => Ok(()),
            Some(c) => Err(ExecError::Integrity(format!(
                "a corretora devolveu o identificador {:?} para a ordem {link_id:?}",
                c.order_link_id
            ))),
            None => Err(ExecError::Integrity(
                "criação sem result: a ordem pode existir. Reconciliar.".into(),
            )),
        }
    }

    fn buscar_execucao(&self, link_id: &str) -> Result<(Money, Qty, Money, String), ExecError> {
        let query = format!(
            "category=spot&orderLinkId={link_id}&symbol={}",
            self.simbolo
        );
        let bruto = self
            .transporte
            .get(CAMINHO_EXECUCOES, &query)
            .map_err(|e| de_transporte(&e))?;

        let env: Envelope<ListaDeExecucoes> = serde_json::from_str(&bruto)
            .map_err(|e| ExecError::Integrity(format!("lista de execuções ilegível: {e}")))?;
        if env.ret_code != 0 {
            return Err(de_ret_code(env.ret_code, &env.ret_msg));
        }
        let lista = env.result.map(|r| r.list).unwrap_or_default();
        if lista.is_empty() {
            // A ordem foi criada e ainda não preencheu. Não é erro: é estado.
            // Tratar como recusa apagaria uma ordem que existe na corretora.
            return Err(ExecError::Timeout);
        }

        // Preenchimento parcial chega em pedaços. O preço é a média ponderada
        // pela quantidade — a média simples mentiria quando os pedaços têm
        // tamanhos diferentes.
        let mut qty = Decimal::ZERO;
        let mut valor = Decimal::ZERO;
        let mut taxa = Decimal::ZERO;
        let mut moeda = String::new();
        for e in &lista {
            let q = decimal("execQty", &e.exec_qty)?;
            let p = decimal("execPrice", &e.exec_price)?;
            taxa += decimal("execFee", &e.exec_fee)?;
            qty += q;
            valor += p * q;
            if moeda.is_empty() {
                moeda = e.fee_currency.clone();
            }
        }
        if qty <= Decimal::ZERO {
            return Err(ExecError::Integrity(
                "execuções com quantidade total zero".into(),
            ));
        }
        Ok((valor / qty, qty, taxa, moeda))
    }
}

impl<T: Transporte> OrderExecutor for PaperExecutor<T> {
    fn execute(&mut self, order: &Order) -> Result<Fill, ExecError> {
        let link_id = order_link_id(&self.run_id, order.id.0);
        self.enviar(order, &link_id)?;

        let (preco, qty, taxa, moeda) = self.buscar_execucao(&link_id)?;
        let referencia = self.preco.get().unwrap_or(preco);

        // A moeda da taxa decide em qual campo ela entra, e quem decide é a
        // corretora — não nós. No spot a compra paga em moeda base e a venda em
        // caixa, mas conferir em vez de presumir é o que faz o extrato bater
        // quando a corretora mudar de ideia.
        let base = self.simbolo.trim_end_matches("USDT");
        let em_moeda_base = moeda.eq_ignore_ascii_case(base);
        let (fee, fee_base) = if em_moeda_base {
            (Decimal::ZERO, taxa)
        } else {
            (taxa, Decimal::ZERO)
        };

        Ok(Fill {
            order_ref: order.id,
            price: preco,
            qty,
            fee,
            fee_base,
            // Slippage medido, e não modelado: é a diferença entre o preço de
            // referência no instante da decisão e o obtido. É o número que a
            // Porta 2 existe para produzir.
            slippage: ((preco - referencia).abs() * qty).round_dp(8),
            at: order.at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transporte::TransporteError;
    use crate::transporte::testing::TransporteFalso;
    use chrono::{TimeZone, Utc};
    use rust_decimal::dec;
    use trade_domain::{OrderId, SignalId};

    fn ordem(side: Side, qty: Qty) -> Order {
        Order {
            id: OrderId(1),
            signal_ref: SignalId(1),
            side,
            qty,
            at: Utc.with_ymd_and_hms(2026, 9, 20, 12, 0, 0).unwrap(),
        }
    }

    const CRIADA: &str = r#"{"retCode":0,"retMsg":"OK","result":{"orderLinkId":"trade-run-1"}}"#;

    fn execucao(preco: &str, qty: &str, taxa: &str, moeda: &str) -> String {
        format!(
            r#"{{"retCode":0,"retMsg":"OK","result":{{"list":[{{"execPrice":"{preco}","execQty":"{qty}","execFee":"{taxa}","feeCurrency":"{moeda}"}}]}}}}"#
        )
    }

    fn executor(respostas: Vec<Result<String, TransporteError>>) -> PaperExecutor<TransporteFalso> {
        let preco = PrecoCorrente::default();
        preco.set(dec!(81233.70));
        PaperExecutor::novo(
            TransporteFalso::com(respostas),
            "BTCUSDT",
            "run",
            dec!(0.02),
            preco,
        )
    }

    #[test]
    fn a_compra_envia_stop_junto_da_entrada() {
        let mut e = executor(vec![
            Ok(CRIADA.into()),
            Ok(execucao("81250", "0.000615", "0.000000615", "BTC")),
        ]);
        e.execute(&ordem(Side::Buy, dec!(0.000615))).unwrap();
        let corpo = &e.transporte.chamadas.borrow()[0].1;
        assert!(corpo.contains("stopLoss"), "entrada sem stop: {corpo}");
        assert!(corpo.contains(r#""slOrderType":"Market""#), "{corpo}");
    }

    #[test]
    fn a_taxa_em_moeda_base_vai_para_fee_base() {
        let mut e = executor(vec![
            Ok(CRIADA.into()),
            Ok(execucao("81250", "0.000615", "0.000000615", "BTC")),
        ]);
        let f = e.execute(&ordem(Side::Buy, dec!(0.000615))).unwrap();
        assert_eq!(f.fee_base, dec!(0.000000615));
        assert_eq!(f.fee, dec!(0));
    }

    #[test]
    fn a_taxa_em_caixa_vai_para_fee() {
        let mut e = executor(vec![
            Ok(CRIADA.into()),
            Ok(execucao("81200", "0.000614", "0.0499", "USDT")),
        ]);
        let f = e.execute(&ordem(Side::Sell, dec!(0.000614))).unwrap();
        assert_eq!(f.fee, dec!(0.0499));
        assert_eq!(f.fee_base, dec!(0));
    }

    #[test]
    fn o_slippage_e_medido_contra_o_preco_de_referencia() {
        // É o número que a Porta 2 existe para produzir: hoje 0,05% é analogia.
        let mut e = executor(vec![
            Ok(CRIADA.into()),
            Ok(execucao("81250", "0.001", "0", "BTC")),
        ]);
        let f = e.execute(&ordem(Side::Buy, dec!(0.001))).unwrap();
        assert_eq!(f.slippage, dec!(0.01630000), "(81250 − 81233,70) × 0,001");
    }

    #[test]
    fn preenchimento_parcial_em_pedacos_vira_media_ponderada() {
        let parcial = r#"{"retCode":0,"retMsg":"OK","result":{"list":[
            {"execPrice":"100","execQty":"3","execFee":"0","feeCurrency":"BTC"},
            {"execPrice":"200","execQty":"1","execFee":"0","feeCurrency":"BTC"}]}}"#;
        let mut e = executor(vec![Ok(CRIADA.into()), Ok(parcial.into())]);
        let f = e.execute(&ordem(Side::Buy, dec!(4))).unwrap();
        assert_eq!(f.qty, dec!(4));
        assert_eq!(f.price, dec!(125), "média ponderada, não simples");
    }

    #[test]
    fn resultado_desconhecido_nao_vira_retentativa() {
        // O caso perigoso: repetir pode duplicar a ordem. Vira integridade, que
        // o RiskGuard trata como parada até reconciliar (FR-107).
        let mut e = executor(vec![Err(TransporteError::Desconhecido(
            "sem resposta".into(),
        ))]);
        let err = e.execute(&ordem(Side::Buy, dec!(1))).unwrap_err();
        assert!(matches!(err, ExecError::Integrity(_)), "{err}");
        assert!(!err.is_transient_candidate());
        assert_eq!(
            e.transporte.chamadas.borrow().len(),
            1,
            "não pode ter havido segunda tentativa"
        );
    }

    #[test]
    fn identificador_devolvido_diferente_e_integridade() {
        // Se a corretora responde sobre outra ordem, não dá para saber o que
        // aconteceu com a nossa.
        let outro = r#"{"retCode":0,"retMsg":"OK","result":{"orderLinkId":"outra-coisa"}}"#;
        let mut e = executor(vec![Ok(outro.into())]);
        assert!(matches!(
            e.execute(&ordem(Side::Buy, dec!(1))).unwrap_err(),
            ExecError::Integrity(_)
        ));
    }

    #[test]
    fn sem_preco_de_referencia_a_entrada_nao_sai() {
        let mut e = PaperExecutor::novo(
            TransporteFalso::com(vec![]),
            "BTCUSDT",
            "run",
            dec!(0.02),
            PrecoCorrente::default(),
        );
        let err = e.execute(&ordem(Side::Buy, dec!(1))).unwrap_err();
        assert!(matches!(err, ExecError::Integrity(_)), "{err}");
        assert!(
            e.transporte.chamadas.borrow().is_empty(),
            "nada pode ter sido enviado"
        );
    }

    #[test]
    fn ordem_criada_e_ainda_nao_preenchida_e_timeout_e_nao_recusa() {
        // Tratar como recusa apagaria uma ordem que existe na corretora.
        let vazia = r#"{"retCode":0,"retMsg":"OK","result":{"list":[]}}"#;
        let mut e = executor(vec![Ok(CRIADA.into()), Ok(vazia.into())]);
        assert!(matches!(
            e.execute(&ordem(Side::Buy, dec!(1))).unwrap_err(),
            ExecError::Timeout
        ));
    }

    #[test]
    fn numero_ilegivel_da_corretora_e_integridade() {
        let sujo = execucao("oitenta-mil", "0.001", "0", "BTC");
        let mut e = executor(vec![Ok(CRIADA.into()), Ok(sujo)]);
        let err = e.execute(&ordem(Side::Buy, dec!(1))).unwrap_err();
        assert!(matches!(err, ExecError::Integrity(_)), "{err}");
    }
}
