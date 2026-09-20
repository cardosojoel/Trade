//! Evento de auditoria.
//!
//! Esquema em `specs/001-nucleo-execucao/contracts/audit-event.md`. A
//! serialização é implementada na US4; aqui ficam os tipos, porque
//! `trade-ports` precisa nomeá-los para definir `AuditSink`.

use crate::mode::ExecutionMode;
use crate::position::Position;
use crate::risk_types::{Anomaly, RiskDecision};
use crate::types::{Fill, Order, Signal};
use chrono::{DateTime, Utc};

/// Envelope comum a todo evento (FR-033).
#[derive(Debug, Clone)]
pub struct AuditEvent {
    pub run_id: String,
    /// Ordem **total** dentro da execução, começando em 1.
    ///
    /// É o que permite reconstituir a cadeia sinal → ordem → decisão →
    /// preenchimento mesmo entre eventos do mesmo instante simulado. Sem ele,
    /// dois eventos na mesma vela ficariam sem ordem definida.
    pub seq: u64,
    /// Instante **simulado**, vindo do `Clock` — nunca o relógio da máquina.
    pub at: DateTime<Utc>,
    pub mode: ExecutionMode,
    pub kind: AuditKind,
}

#[derive(Debug, Clone)]
pub enum AuditKind {
    Signal(Signal),
    Order(Order),
    RiskDecision(RiskDecision),
    Fill(Fill),
    Halt {
        reason: String,
        anomaly: Anomaly,
    },
    Resume {
        trigger: String,
        automatic: bool,
    },
    Anomaly {
        anomaly: Anomaly,
        recovered: bool,
    },
    StateTransition {
        from: String,
        to: String,
        position: Position,
    },
}

impl AuditKind {
    /// Discriminante usado na coluna `kind` da tabela `audit_event`.
    pub const fn as_str(&self) -> &'static str {
        match self {
            AuditKind::Signal(_) => "signal",
            AuditKind::Order(_) => "order",
            AuditKind::RiskDecision(_) => "risk_decision",
            AuditKind::Fill(_) => "fill",
            AuditKind::Halt { .. } => "halt",
            AuditKind::Resume { .. } => "resume",
            AuditKind::Anomaly { .. } => "anomaly",
            AuditKind::StateTransition { .. } => "state_transition",
        }
    }
}
