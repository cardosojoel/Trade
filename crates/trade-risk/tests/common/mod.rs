//! Apoio comum aos testes da camada de risco.
//!
//! `dead_code` é permitido porque este módulo é compilado dentro de cada
//! binário de teste, e nenhum deles usa todos os auxiliares.
#![allow(dead_code)]

use chrono::{DateTime, TimeZone, Utc};
use rust_decimal::{Decimal, dec};
use trade_domain::{
    ExecutionMode, Fill, Money, Order, OrderId, Position, RiskLimits, Side, SignalId,
};
use trade_ports::AuditRecorder;
use trade_ports::testing::InMemoryAuditSink;

pub fn at(dia: u32, hora: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, dia, hora, 0, 0).unwrap()
}

/// Cerca de referência dos testes. Valores redondos, para que a conta de
/// cabeça bata com a do código.
pub fn limits() -> RiskLimits {
    RiskLimits {
        max_daily_loss: dec!(200),
        max_position_size: dec!(1000),
        max_total_exposure: dec!(2000),
        max_orders_per_window: 10,
        window_minutes: 60,
        max_transient_retries: 3,
        max_price_deviation_ratio: dec!(0.20),
        max_position_hours: 72,
    }
}

pub fn order(id: u64, side: Side, qty: Money, quando: DateTime<Utc>) -> Order {
    Order {
        id: OrderId(id),
        signal_ref: SignalId(id),
        side,
        qty,
        at: quando,
    }
}

/// Posição comprada, montada pelo único caminho que existe: aplicar um fill.
pub fn comprado(qty: Money, preco: Money) -> Position {
    let mut p = Position::default();
    p.apply_fill(
        Side::Buy,
        &Fill {
            order_ref: OrderId(0),
            price: preco,
            qty,
            fee: Decimal::ZERO,
            fee_base: Decimal::ZERO,
            slippage: Decimal::ZERO,
            at: at(1, 0),
        },
    )
    .unwrap();
    p
}

/// Registrador de auditoria em memória, inspecionável pelo teste.
pub fn recorder() -> AuditRecorder<InMemoryAuditSink> {
    AuditRecorder::new(
        "run-de-teste",
        ExecutionMode::Backtest,
        InMemoryAuditSink::new(),
    )
}
