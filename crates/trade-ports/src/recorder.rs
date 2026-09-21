//! Registrador de auditoria.
//!
//! Monta o envelope de FR-033 — `run_id`, `seq`, instante, modo — e entrega ao
//! [`AuditSink`]. O `seq` é mantido aqui, em um só lugar, porque precisa ser
//! **ordem total** dentro da execução: dois contadores independentes, um no
//! motor e outro na camada de risco, produziriam duas sequências e a cadeia
//! sinal → ordem → decisão → preenchimento perderia a ordem entre elas.
//!
//! A camada de risco recebe um `&mut dyn Recorder` como **parâmetro** de
//! `submit`. É deliberado: não há como submeter uma ordem sem fornecer onde
//! registrá-la. Auditoria não é opção que se possa deixar desligada, e o tipo
//! diz isso.

use crate::AuditSink;
use crate::errors::AuditError;
use chrono::{DateTime, Utc};
use trade_domain::{AuditEvent, AuditKind, ExecutionMode};

pub trait Recorder {
    /// Registra um evento, atribuindo o próximo `seq`.
    fn record(&mut self, at: DateTime<Utc>, kind: AuditKind) -> Result<(), AuditError>;
    /// Último `seq` atribuído.
    fn seq(&self) -> u64;
    /// Leva ao destino o que ainda está em memória.
    ///
    /// Está na trait, e não só no tipo concreto, porque quem fecha uma sessão
    /// precisa poder garantir que nada ficou no ar sem saber qual destino de
    /// registro está por trás. Deixar isto fora obrigaria todo chamador a
    /// lembrar — e a auditoria perdida em silêncio é o pior resultado
    /// possível.
    fn flush(&mut self) -> Result<(), AuditError>;
}

pub struct AuditRecorder<S: AuditSink> {
    run_id: String,
    mode: ExecutionMode,
    seq: u64,
    sink: S,
}

impl<S: AuditSink> AuditRecorder<S> {
    pub fn new(run_id: impl Into<String>, mode: ExecutionMode, sink: S) -> Self {
        AuditRecorder {
            run_id: run_id.into(),
            mode,
            seq: 0,
            sink,
        }
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn sink(&self) -> &S {
        &self.sink
    }
}

impl<S: AuditSink> Recorder for AuditRecorder<S> {
    fn record(&mut self, at: DateTime<Utc>, kind: AuditKind) -> Result<(), AuditError> {
        self.seq += 1;
        self.sink.record(AuditEvent {
            run_id: self.run_id.clone(),
            seq: self.seq,
            at,
            mode: self.mode,
            kind,
        })
    }

    fn seq(&self) -> u64 {
        self.seq
    }

    fn flush(&mut self) -> Result<(), AuditError> {
        self.sink.flush()
    }
}
