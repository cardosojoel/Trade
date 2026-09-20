//! Estratégia deliberadamente imprudente, para teste.
//!
//! Existe para provar SC-003: submetida a cerca, ela não consegue ultrapassar
//! limite nenhum, em nenhuma tentativa. Tenta posição muito acima do teto,
//! insiste depois de estourar a perda diária e opera em rajada.
//!
//! Repare no que ela **não consegue nem tentar**: alterar os próprios limites.
//! Não há campo, método ou referência neste arquivo que alcance `RiskLimits`,
//! porque `trade-strategy` não depende de `trade-risk` nem de `trade-ports`.
//! A tentativa não é recusada em tempo de execução — ela não é expressável.

use rust_decimal::Decimal;
use std::collections::BTreeMap;
use trade_domain::{Intent, MarketContext, Signal, SignalId, SignalInputs, Strategy};

/// Compra sem parar, em tamanho absurdo.
#[derive(Debug, Default)]
pub struct RecklessStrategy {
    proximo_id: u64,
    /// Multiplicador sobre o saldo. Acima de 1 já é impossível de pagar.
    pub multiplicador: u32,
}

impl RecklessStrategy {
    pub fn new(multiplicador: u32) -> Self {
        RecklessStrategy {
            proximo_id: 0,
            multiplicador,
        }
    }
}

impl Strategy for RecklessStrategy {
    fn name(&self) -> &str {
        "reckless"
    }

    fn params(&self) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert("multiplicador".to_string(), self.multiplicador.to_string());
        m
    }

    fn on_candle(&mut self, ctx: &MarketContext<'_>) -> Option<Signal> {
        self.proximo_id += 1;

        // Tamanho proporcional ao saldo, multiplicado por um fator que torna a
        // ordem impagável de propósito.
        let qty = if ctx.candle.close > Decimal::ZERO {
            (ctx.balance / ctx.candle.close) * Decimal::from(self.multiplicador)
        } else {
            Decimal::ONE
        };

        Some(Signal {
            id: SignalId(self.proximo_id),
            at: ctx.candle.open_time,
            intent: Intent::Buy,
            qty: Some(qty),
            inputs: SignalInputs::default()
                .with_value("close", ctx.candle.close)
                .with_value("balance", ctx.balance)
                .with_param("multiplicador", self.multiplicador),
        })
    }
}
