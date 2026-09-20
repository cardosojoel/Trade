//! Registro que descarta eventos.
//!
//! Existe apenas enquanto a persistência da auditoria (US4) não está pronta.
//! **Não é configuração**: não há bandeira, variável de ambiente ou argumento
//! que o selecione. Quando o `SqliteAuditSink` chegar, este arquivo sai.
//!
//! Deixá-lo acessível como opção seria criar exatamente o que o Princípio IV
//! não admite — uma execução sem registro.

use trade_domain::AuditEvent;
use trade_ports::{AuditError, AuditSink};

#[derive(Debug)]
pub struct NoopAuditSink;

impl AuditSink for NoopAuditSink {
    fn record(&mut self, _event: AuditEvent) -> Result<(), AuditError> {
        Ok(())
    }

    fn flush(&mut self) -> Result<(), AuditError> {
        Ok(())
    }
}
