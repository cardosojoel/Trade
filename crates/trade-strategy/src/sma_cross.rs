//! Estratégia de referência: cruzamento de médias móveis.
//!
//! **Não é recomendação de investimento** (FR-038). Existe para exercitar o
//! ciclo completo do motor com comportamento previsível o bastante para ser
//! conferido à mão — que é o que FR-037 pede. Cruzamento de médias é o exemplo
//! mais banal do gênero, e essa banalidade é proposital: reduz o risco de
//! alguém confundi-la com uma estratégia de verdade.

use rust_decimal::Decimal;
use std::collections::{BTreeMap, VecDeque};
use trade_domain::{Intent, MarketContext, Money, Signal, SignalId, SignalInputs, Strategy};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmaCrossParams {
    pub fast: usize,
    pub slow: usize,
    /// Fração do saldo aplicada a cada compra.
    ///
    /// A estratégia **não conhece a cerca** — não tem como, por desenho. Se
    /// esta fração for maior que o limite de posição, as ordens serão
    /// recusadas e isso aparecerá na contagem de recusas do relatório. Manter
    /// os dois coerentes é trabalho de quem configura, não do código.
    pub position_fraction: Money,
}

impl Default for SmaCrossParams {
    fn default() -> Self {
        SmaCrossParams {
            fast: 9,
            slow: 21,
            position_fraction: Decimal::new(10, 2), // 0,10
        }
    }
}

#[derive(Debug)]
pub struct SmaCross {
    params: SmaCrossParams,
    fechamentos: VecDeque<Money>,
    /// Última relação observada: rápida acima da lenta?
    acima: Option<bool>,
    proximo_id: u64,
}

impl SmaCross {
    pub fn new(params: SmaCrossParams) -> Self {
        SmaCross {
            fechamentos: VecDeque::with_capacity(params.slow),
            params,
            acima: None,
            proximo_id: 0,
        }
    }

    fn media(&self, periodo: usize) -> Option<Money> {
        if self.fechamentos.len() < periodo {
            return None;
        }
        let soma: Money = self.fechamentos.iter().rev().take(periodo).sum();
        Some(soma / Decimal::from(periodo))
    }
}

impl Strategy for SmaCross {
    fn name(&self) -> &str {
        "sma-cross"
    }

    fn params(&self) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert("fast".into(), self.params.fast.to_string());
        m.insert("slow".into(), self.params.slow.to_string());
        m.insert(
            "position_fraction".into(),
            self.params.position_fraction.to_string(),
        );
        m
    }

    fn on_candle(&mut self, ctx: &MarketContext<'_>) -> Option<Signal> {
        self.fechamentos.push_back(ctx.candle.close);
        while self.fechamentos.len() > self.params.slow {
            self.fechamentos.pop_front();
        }

        let rapida = self.media(self.params.fast)?;
        let lenta = self.media(self.params.slow)?;
        let agora_acima = rapida > lenta;

        let anterior = self.acima.replace(agora_acima);

        // Só o **cruzamento** gera sinal. Sem isso, a estratégia emitiria
        // ordem em toda vela em que a relação se mantém, e o limite de
        // frequência recusaria quase tudo.
        let cruzou = matches!(anterior, Some(a) if a != agora_acima);
        if !cruzou {
            return None;
        }

        let (intent, qty) = if agora_acima {
            let alvo = ctx.balance * self.params.position_fraction;
            let qty = if ctx.candle.close > Decimal::ZERO {
                alvo / ctx.candle.close
            } else {
                Decimal::ZERO
            };
            (Intent::Buy, Some(qty))
        } else {
            (Intent::Sell, Some(ctx.position.qty()))
        };

        if qty.is_some_and(|q| q <= Decimal::ZERO) {
            return None;
        }

        self.proximo_id += 1;
        Some(Signal {
            id: SignalId(self.proximo_id),
            at: ctx.candle.open_time,
            intent,
            qty,
            inputs: SignalInputs::default()
                .with_value("sma_fast", rapida)
                .with_value("sma_slow", lenta)
                .with_value("close", ctx.candle.close)
                .with_param("fast", self.params.fast)
                .with_param("slow", self.params.slow),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use rust_decimal::dec;
    use trade_domain::{Candle, Position};

    fn vela(preco: Money) -> Candle {
        let t = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        Candle {
            open_time: t,
            open: preco,
            high: preco,
            low: preco,
            close: preco,
            volume: dec!(1000),
            turnover: preco * dec!(1000),
        }
    }

    /// Posição comprada, montada pelo único caminho que existe.
    fn comprado(qty: Money) -> Position {
        use trade_domain::{Fill, OrderId, Side};
        let mut p = Position::default();
        p.apply_fill(
            Side::Buy,
            &Fill {
                order_ref: OrderId(0),
                price: dec!(100),
                qty,
                fee: Decimal::ZERO,
                fee_base: Decimal::ZERO,
                slippage: Decimal::ZERO,
                at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            },
        )
        .unwrap();
        p
    }

    fn roda_com(precos: &[i64], pos: Position) -> Vec<Intent> {
        let mut s = SmaCross::new(SmaCrossParams {
            fast: 2,
            slow: 4,
            position_fraction: dec!(0.5),
        });
        let mut sinais = Vec::new();
        for p in precos {
            let c = vela(Decimal::from(*p));
            if let Some(sig) = s.on_candle(&MarketContext {
                candle: &c,
                position: &pos,
                balance: dec!(1000),
            }) {
                sinais.push(sig.intent);
            }
        }
        sinais
    }

    fn roda(precos: &[i64]) -> Vec<Intent> {
        roda_com(precos, Position::default())
    }

    #[test]
    fn nao_emite_sinal_antes_de_ter_historico_suficiente() {
        assert!(
            roda(&[100, 101, 102]).is_empty(),
            "média lenta de 4 não existe ainda"
        );
    }

    #[test]
    fn subida_sustentada_gera_uma_compra_e_nao_uma_por_vela() {
        // A relação se estabelece uma vez; manter-se não é cruzamento.
        let sinais = roda(&[100, 100, 100, 100, 110, 120, 130, 140, 150]);
        assert_eq!(sinais.iter().filter(|i| **i == Intent::Buy).count(), 1);
    }

    #[test]
    fn reversao_gera_venda_quando_ha_posicao() {
        let sinais = roda_com(
            &[100, 100, 100, 100, 130, 140, 150, 100, 80, 60, 40],
            comprado(dec!(5)),
        );
        assert!(sinais.contains(&Intent::Buy));
        assert!(sinais.contains(&Intent::Sell));
    }

    #[test]
    fn reversao_sem_posicao_nao_gera_venda() {
        // Emitir venda sem ter o que vender só produziria ordem recusada por
        // SellExceedsHoldings, poluindo a contagem de recusas do relatório com
        // algo que a estratégia já sabia ser impossível.
        let sinais = roda(&[100, 100, 100, 100, 130, 140, 150, 100, 80, 60, 40]);
        assert!(!sinais.contains(&Intent::Sell));
    }

    #[test]
    fn os_inputs_do_sinal_carregam_as_duas_medias() {
        // FR-034: sem isto, o registro diz o que foi feito e não por quê.
        let mut s = SmaCross::new(SmaCrossParams {
            fast: 2,
            slow: 4,
            position_fraction: dec!(0.5),
        });
        let pos = Position::default();
        let mut ultimo = None;
        for p in [100, 100, 100, 100, 200, 300] {
            let c = vela(Decimal::from(p));
            if let Some(sig) = s.on_candle(&MarketContext {
                candle: &c,
                position: &pos,
                balance: dec!(1000),
            }) {
                ultimo = Some(sig);
            }
        }
        let sig = ultimo.expect("nenhum sinal emitido");
        assert!(sig.inputs.values.contains_key("sma_fast"));
        assert!(sig.inputs.values.contains_key("sma_slow"));
        assert_eq!(sig.inputs.params.get("fast").map(String::as_str), Some("2"));
    }
}
