//! Persistência do histórico coletado.

use crate::decimal_sql::{from_sql, to_sql};
use chrono::{DateTime, TimeZone, Utc};
use rusqlite::Connection;
use trade_domain::{Candle, Coverage, Gap, Interval, Symbol};
use trade_ports::{CandleRepository, StorageError};

pub struct SqliteCandleRepository {
    conn: Connection,
}

fn escrita(e: rusqlite::Error) -> StorageError {
    StorageError::Write(e.to_string())
}

fn consulta(e: rusqlite::Error) -> StorageError {
    StorageError::Query(e.to_string())
}

fn de_ms(v: i64) -> Result<DateTime<Utc>, StorageError> {
    Utc.timestamp_millis_opt(v)
        .single()
        .ok_or_else(|| StorageError::Corrupt {
            campo: "open_ms".into(),
            valor: v.to_string(),
        })
}

impl SqliteCandleRepository {
    pub fn new(conn: Connection) -> Self {
        SqliteCandleRepository { conn }
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Registra a procedência do conjunto (FR-017).
    pub fn record_dataset(
        &mut self,
        symbol: &Symbol,
        interval: Interval,
        source: &str,
        coletado_em: DateTime<Utc>,
    ) -> Result<(), StorageError> {
        let Some(c) = self.coverage(symbol, interval)? else {
            return Ok(());
        };

        self.conn
            .execute(
                "INSERT INTO dataset (symbol, interval, source, first_open_ms, last_open_ms, collected_at) \
                 VALUES (?1,?2,?3,?4,?5,?6) \
                 ON CONFLICT(symbol, interval) DO UPDATE SET \
                   source = excluded.source, \
                   first_open_ms = excluded.first_open_ms, \
                   last_open_ms = excluded.last_open_ms, \
                   collected_at = excluded.collected_at",
                rusqlite::params![
                    symbol.as_str(),
                    interval.as_str(),
                    source,
                    c.first.timestamp_millis(),
                    c.last.timestamp_millis(),
                    coletado_em.timestamp_millis(),
                ],
            )
            .map_err(escrita)?;
        Ok(())
    }

    /// Lê uma vela, para conferência em teste.
    pub fn get(
        &self,
        symbol: &Symbol,
        interval: Interval,
        open_time: DateTime<Utc>,
    ) -> Result<Option<Candle>, StorageError> {
        let r = self.conn.query_row(
            "SELECT open, high, low, close, volume, turnover FROM candle \
             WHERE symbol=?1 AND interval=?2 AND open_ms=?3",
            rusqlite::params![
                symbol.as_str(),
                interval.as_str(),
                open_time.timestamp_millis()
            ],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                ))
            },
        );

        match r {
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(consulta(e)),
            Ok((o, h, l, c, v, t)) => Ok(Some(Candle {
                open_time,
                open: from_sql("open", &o)?,
                high: from_sql("high", &h)?,
                low: from_sql("low", &l)?,
                close: from_sql("close", &c)?,
                volume: from_sql("volume", &v)?,
                turnover: from_sql("turnover", &t)?,
            })),
        }
    }
}

impl CandleRepository for SqliteCandleRepository {
    /// Grava a página inteira **em uma transação**.
    ///
    /// É o que garante FR-015: uma coleta interrompida no meio deixa páginas
    /// inteiras gravadas, nunca meia página. E `INSERT OR IGNORE` sobre a chave
    /// primária torna regravar o que já existe uma operação vazia — a retomada
    /// de FR-014 vem do esquema, não de código de deduplicação.
    fn upsert_page(
        &mut self,
        symbol: &Symbol,
        interval: Interval,
        candles: &[Candle],
    ) -> Result<usize, StorageError> {
        let tx = self.conn.transaction().map_err(escrita)?;
        let mut novas = 0usize;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT OR IGNORE INTO candle \
                     (symbol, interval, open_ms, open, high, low, close, volume, turnover) \
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                )
                .map_err(escrita)?;

            for c in candles {
                novas += stmt
                    .execute(rusqlite::params![
                        symbol.as_str(),
                        interval.as_str(),
                        c.open_time.timestamp_millis(),
                        to_sql(c.open),
                        to_sql(c.high),
                        to_sql(c.low),
                        to_sql(c.close),
                        to_sql(c.volume),
                        to_sql(c.turnover),
                    ])
                    .map_err(escrita)?;
            }
        }
        tx.commit().map_err(escrita)?;
        Ok(novas)
    }

    fn record_gaps(
        &mut self,
        symbol: &Symbol,
        interval: Interval,
        gaps: &[Gap],
    ) -> Result<(), StorageError> {
        let agora = Utc::now().timestamp_millis();
        let tx = self.conn.transaction().map_err(escrita)?;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT OR REPLACE INTO gap (symbol, interval, from_ms, to_ms, detected_at) \
                     VALUES (?1,?2,?3,?4,?5)",
                )
                .map_err(escrita)?;
            for g in gaps {
                stmt.execute(rusqlite::params![
                    symbol.as_str(),
                    interval.as_str(),
                    g.from.timestamp_millis(),
                    g.to.timestamp_millis(),
                    agora,
                ])
                .map_err(escrita)?;
            }
        }
        tx.commit().map_err(escrita)?;
        Ok(())
    }

    fn coverage(
        &self,
        symbol: &Symbol,
        interval: Interval,
    ) -> Result<Option<Coverage>, StorageError> {
        let r: (Option<i64>, Option<i64>, i64) = self
            .conn
            .query_row(
                "SELECT MIN(open_ms), MAX(open_ms), COUNT(*) FROM candle \
                 WHERE symbol=?1 AND interval=?2",
                rusqlite::params![symbol.as_str(), interval.as_str()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map_err(consulta)?;

        match r {
            (Some(min), Some(max), n) if n > 0 => Ok(Some(Coverage {
                first: de_ms(min)?,
                last: de_ms(max)?,
                count: u64::try_from(n).unwrap_or(0),
            })),
            _ => Ok(None),
        }
    }
}
