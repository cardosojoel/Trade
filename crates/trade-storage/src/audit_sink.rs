//! Registro de auditoria em SQLite.
//!
//! Escreve em lote, dentro de transação. Um backtest de doze meses produz
//! centenas de milhares de eventos, e uma transação por evento tornaria o
//! registro mais caro que a simulação — o que acabaria virando argumento para
//! desligá-lo, que é o que o Princípio IV não admite.

use rusqlite::Connection;
use trade_domain::AuditEvent;
use trade_ports::{AuditError, AuditSink};

/// Quantos eventos acumulam antes de uma gravação.
const LOTE: usize = 500;

pub struct SqliteAuditSink {
    conn: Connection,
    buffer: Vec<AuditEvent>,
    gravados: usize,
}

impl SqliteAuditSink {
    pub fn new(conn: Connection) -> Self {
        SqliteAuditSink {
            conn,
            buffer: Vec::with_capacity(LOTE),
            gravados: 0,
        }
    }

    pub fn gravados(&self) -> usize {
        self.gravados
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    fn descarregar(&mut self) -> Result<(), AuditError> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        let tx = self
            .conn
            .transaction()
            .map_err(|e| AuditError::Write(e.to_string()))?;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT OR REPLACE INTO audit_event (run_id, seq, at_ms, kind, payload_json) \
                     VALUES (?1,?2,?3,?4,?5)",
                )
                .map_err(|e| AuditError::Write(e.to_string()))?;

            for e in &self.buffer {
                stmt.execute(rusqlite::params![
                    e.run_id,
                    e.seq as i64,
                    e.at.timestamp_millis(),
                    e.kind.as_str(),
                    e.kind.payload_json(),
                ])
                .map_err(|err| AuditError::Write(err.to_string()))?;
            }
        }
        tx.commit().map_err(|e| AuditError::Write(e.to_string()))?;

        self.gravados += self.buffer.len();
        self.buffer.clear();
        Ok(())
    }
}

impl AuditSink for SqliteAuditSink {
    fn record(&mut self, event: AuditEvent) -> Result<(), AuditError> {
        self.buffer.push(event);
        if self.buffer.len() >= LOTE {
            self.descarregar()?;
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), AuditError> {
        self.descarregar()
    }
}

impl Drop for SqliteAuditSink {
    /// Rede de segurança: eventos em memória que nunca chegaram ao disco
    /// seriam auditoria perdida em silêncio, que é o pior resultado possível.
    fn drop(&mut self) {
        let _ = self.descarregar();
    }
}
