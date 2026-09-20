//! Resultado de uma execução.

use chrono::{DateTime, Utc};
use trade_domain::{Gap, LimitBreach, Money, Position, RunMetrics, Trade};

/// Como a execução terminou.
///
/// As três variantes são distintas de propósito: duas delas terminam **sem**
/// completar o período, e um consumidor que só perguntasse "terminou?" trataria
/// as três como sucesso.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunOutcome {
    /// Percorreu todo o período.
    Completed,
    /// Interrompido por falha de integridade — exige ato humano (FR-024b).
    Halted { reason: String, at: DateTime<Utc> },
    /// Capital simulado esgotado (FR-032).
    CapitalExhausted { at: DateTime<Utc> },
}

impl RunOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            RunOutcome::Completed => "completed",
            RunOutcome::Halted { .. } => "halted",
            RunOutcome::CapitalExhausted { .. } => "capital_exhausted",
        }
    }
}

/// Tudo que uma execução produz.
#[derive(Debug, Clone)]
pub struct BacktestOutcome {
    pub metrics: RunMetrics,
    pub trades: Vec<Trade>,
    pub outcome: RunOutcome,
    pub final_position: Position,
    pub final_balance: Money,
    /// Lacunas encontradas no histórico, reportadas e não interpoladas (FR-031).
    pub gaps: Vec<Gap>,
    /// Recusas por limite, para o relatório. `Vec` ordenado e não `HashMap`:
    /// ordem de iteração instável vazaria para a saída.
    pub rejections: Vec<(LimitBreach, usize)>,
    pub candles_seen: u64,
}

impl BacktestOutcome {
    pub fn rejections_total(&self) -> usize {
        self.rejections.iter().map(|(_, n)| n).sum()
    }
}
