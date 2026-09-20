//! Relógio do backtest.

use chrono::{DateTime, Utc};
use trade_ports::Clock;

/// Devolve o **instante simulado** da vela corrente.
///
/// Existe para que nada no motor consulte o relógio da máquina. Com ele, datar
/// um evento com a hora real passa a ser impossível por construção, e não algo
/// a evitar por cuidado — o que é o que sustenta o determinismo de FR-029 e a
/// exigência de instante simulado no registro de auditoria.
#[derive(Debug, Clone)]
pub struct BacktestClock {
    now: DateTime<Utc>,
}

impl BacktestClock {
    pub fn new(inicio: DateTime<Utc>) -> Self {
        BacktestClock { now: inicio }
    }

    pub fn advance_to(&mut self, t: DateTime<Utc>) {
        self.now = t;
    }
}

impl Clock for BacktestClock {
    fn now(&self) -> DateTime<Utc> {
        self.now
    }
}
