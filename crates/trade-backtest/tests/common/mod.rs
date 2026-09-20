//! Apoio comum aos testes do motor.
//!
//! `dead_code` permitido: o módulo é compilado dentro de cada binário de teste
//! e nenhum deles usa todos os auxiliares.
#![allow(dead_code)]

use chrono::{DateTime, TimeZone, Utc};
use rust_decimal::{Decimal, dec};
use std::collections::BTreeMap;
use trade_backtest::BacktestConfig;
use trade_domain::{
    Candle, ExecutionMode, FeeModel, Intent, Interval, MarketContext, Money, RiskLimits, Signal,
    SignalId, SignalInputs, Strategy, Symbol,
};
use trade_ports::AuditRecorder;
use trade_ports::testing::{InMemoryAuditSink, VecMarketDataSource};
pub use trade_risk::KillSwitch;

pub fn t(minuto: i64) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap() + chrono::Duration::minutes(minuto)
}

/// Vela plana: abertura, máxima, mínima e fechamento iguais.
///
/// Serve para que a conta do teste seja a conta do preço, sem ter de decidir
/// em que ponto da vela algo aconteceu.
pub fn vela(minuto: i64, preco: Money, volume: Money) -> Candle {
    Candle {
        open_time: t(minuto),
        open: preco,
        high: preco,
        low: preco,
        close: preco,
        volume,
        turnover: preco * volume,
    }
}

pub fn velas(precos: &[(i64, Money)]) -> Vec<Candle> {
    precos
        .iter()
        .map(|(m, p)| vela(*m, *p, dec!(1000)))
        .collect()
}

pub fn fonte(candles: Vec<Candle>) -> VecMarketDataSource {
    VecMarketDataSource::new(candles)
}

pub fn config(
    capital: Money,
    ate: i64,
    taxa: Decimal,
    slippage: Decimal,
    limits: RiskLimits,
) -> BacktestConfig {
    BacktestConfig {
        symbol: Symbol::new("BTCUSDT").unwrap(),
        interval: Interval::M1,
        from: t(0),
        to: t(ate),
        initial_capital: capital,
        fees: FeeModel {
            taker_fee_rate: taxa,
            slippage_rate: slippage,
        },
        limits,
        // Piso de 1% do capital: abaixo disso não há mais o que operar.
        min_equity: capital / dec!(100),
    }
}

pub fn limites_folgados() -> RiskLimits {
    RiskLimits {
        max_daily_loss: dec!(1_000_000),
        max_position_size: dec!(1_000_000),
        max_total_exposure: dec!(1_000_000),
        max_orders_per_window: 100_000,
        window_minutes: 60,
        max_transient_retries: 3,
        max_price_deviation_ratio: dec!(0.50),
    }
}

pub fn recorder() -> AuditRecorder<InMemoryAuditSink> {
    AuditRecorder::new(
        "run-de-teste",
        ExecutionMode::Backtest,
        InMemoryAuditSink::new(),
    )
}

/// Estratégia roteirizada: emite exatamente o que o teste mandar, no minuto
/// que o teste mandar. Previsível por construção, que é o que permite conferir
/// o resultado à mão.
#[derive(Debug, Default)]
pub struct ScriptedStrategy {
    /// (minuto da vela, intenção, quantidade sugerida)
    pub roteiro: Vec<(i64, Intent, Option<Money>)>,
    proximo_id: u64,
}

impl ScriptedStrategy {
    pub fn new(roteiro: Vec<(i64, Intent, Option<Money>)>) -> Self {
        ScriptedStrategy {
            roteiro,
            proximo_id: 0,
        }
    }
}

impl Strategy for ScriptedStrategy {
    fn name(&self) -> &str {
        "scripted"
    }

    fn params(&self) -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    fn on_candle(&mut self, ctx: &MarketContext<'_>) -> Option<Signal> {
        let minuto = (ctx.candle.open_time - t(0)).num_minutes();
        let (_, intent, qty) = self
            .roteiro
            .iter()
            .find(|(m, _, _)| *m == minuto)
            .copied()?;

        self.proximo_id += 1;
        Some(Signal {
            id: SignalId(self.proximo_id),
            at: ctx.candle.open_time,
            intent,
            qty,
            inputs: SignalInputs::default().with_value("close", ctx.candle.close),
        })
    }
}
