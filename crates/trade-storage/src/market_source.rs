//! Provedor de mercado a partir do histórico local.

use crate::decimal_sql::from_sql;
use chrono::{DateTime, TimeZone, Utc};
use rusqlite::Connection;
use std::collections::VecDeque;
use trade_domain::{Candle, Interval, Symbol};
use trade_ports::{MarketDataSource, MarketError};

/// Quantas velas são lidas por vez.
///
/// É o que torna a memória **constante em relação ao período**: um backtest de
/// cinco anos mantém em memória as mesmas mil velas que um de cinco dias. Ler
/// tudo de uma vez seria mais simples e transformaria período longo em
/// consumo de memória proporcional.
const PAGINA: usize = 1_000;

pub struct SqliteMarketDataSource {
    conn: Connection,
}

impl SqliteMarketDataSource {
    pub fn new(conn: Connection) -> Self {
        SqliteMarketDataSource { conn }
    }
}

fn ms(t: DateTime<Utc>) -> i64 {
    t.timestamp_millis()
}

fn de_ms(v: i64) -> Result<DateTime<Utc>, MarketError> {
    Utc.timestamp_millis_opt(v)
        .single()
        .ok_or_else(|| MarketError::InvalidCandle {
            at: v.to_string(),
            cause: "instante fora da faixa representável".into(),
        })
}

struct Paginado<'a> {
    conn: &'a Connection,
    symbol: String,
    interval: String,
    cursor_ms: i64,
    ate_ms: i64,
    buffer: VecDeque<Result<Candle, MarketError>>,
    esgotado: bool,
}

impl Paginado<'_> {
    fn carregar(&mut self) {
        let sql = "SELECT open_ms, open, high, low, close, volume, turnover \
                   FROM candle \
                   WHERE symbol = ?1 AND interval = ?2 AND open_ms > ?3 AND open_ms < ?4 \
                   ORDER BY open_ms LIMIT ?5";

        let mut stmt = match self.conn.prepare(sql) {
            Ok(s) => s,
            Err(e) => {
                self.esgotado = true;
                self.buffer
                    .push_back(Err(MarketError::Storage(e.to_string())));
                return;
            }
        };

        let linhas = stmt.query_map(
            rusqlite::params![
                &self.symbol,
                &self.interval,
                self.cursor_ms,
                self.ate_ms,
                PAGINA as i64
            ],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, String>(6)?,
                ))
            },
        );

        let linhas = match linhas {
            Ok(l) => l,
            Err(e) => {
                self.esgotado = true;
                self.buffer
                    .push_back(Err(MarketError::Storage(e.to_string())));
                return;
            }
        };

        let mut quantas = 0;
        for linha in linhas {
            match linha {
                Ok((open_ms, o, h, l, c, v, tv)) => {
                    quantas += 1;
                    self.cursor_ms = open_ms;
                    let montar = || -> Result<Candle, MarketError> {
                        Ok(Candle {
                            open_time: de_ms(open_ms)?,
                            open: campo("open", &o)?,
                            high: campo("high", &h)?,
                            low: campo("low", &l)?,
                            close: campo("close", &c)?,
                            volume: campo("volume", &v)?,
                            turnover: campo("turnover", &tv)?,
                        })
                    };
                    self.buffer.push_back(montar());
                }
                Err(e) => {
                    self.buffer
                        .push_back(Err(MarketError::Storage(e.to_string())));
                }
            }
        }

        if quantas < PAGINA {
            self.esgotado = true;
        }
    }
}

fn campo(nome: &str, raw: &str) -> Result<rust_decimal::Decimal, MarketError> {
    from_sql(nome, raw).map_err(|e| MarketError::InvalidCandle {
        at: nome.to_string(),
        cause: e.to_string(),
    })
}

impl Iterator for Paginado<'_> {
    type Item = Result<Candle, MarketError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.buffer.is_empty() && !self.esgotado {
            self.carregar();
        }
        self.buffer.pop_front()
    }
}

impl MarketDataSource for SqliteMarketDataSource {
    fn candles<'a>(
        &'a self,
        symbol: &Symbol,
        interval: Interval,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Box<dyn Iterator<Item = Result<Candle, MarketError>> + 'a>, MarketError> {
        Ok(Box::new(Paginado {
            conn: &self.conn,
            symbol: symbol.as_str().to_string(),
            interval: interval.as_str().to_string(),
            // Exclusivo à esquerda no SQL, então recua um milissegundo para
            // que a primeira vela do período entre.
            cursor_ms: ms(from) - 1,
            ate_ms: ms(to),
            buffer: VecDeque::new(),
            esgotado: false,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_market;
    use crate::decimal_sql::to_sql;
    use rust_decimal::dec;
    use tempfile::tempdir;

    fn popular(conn: &Connection, quantas: i64) {
        let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        for i in 0..quantas {
            let t = base + chrono::Duration::minutes(i);
            let p = dec!(100) + rust_decimal::Decimal::from(i);
            conn.execute(
                "INSERT INTO candle VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                rusqlite::params![
                    "BTCUSDT",
                    "1m",
                    t.timestamp_millis(),
                    to_sql(p),
                    to_sql(p),
                    to_sql(p),
                    to_sql(p),
                    to_sql(dec!(10)),
                    to_sql(p * dec!(10))
                ],
            )
            .unwrap();
        }
    }

    #[test]
    fn percorre_em_ordem_cronologica_crescente() {
        let dir = tempdir().unwrap();
        let conn = open_market(dir.path().join("m.db")).unwrap();
        popular(&conn, 10);
        let src = SqliteMarketDataSource::new(conn);

        let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let velas: Vec<_> = src
            .candles(
                &Symbol::new("BTCUSDT").unwrap(),
                Interval::M1,
                base,
                base + chrono::Duration::hours(1),
            )
            .unwrap()
            .map(Result::unwrap)
            .collect();

        assert_eq!(velas.len(), 10);
        assert!(velas.windows(2).all(|w| w[0].open_time < w[1].open_time));
        assert_eq!(velas[0].open, dec!(100));
    }

    #[test]
    fn atravessa_varias_paginas() {
        // Mais que uma página: se a paginação estivesse errada, o percurso
        // pararia em 1000 ou repetiria a primeira página para sempre.
        let dir = tempdir().unwrap();
        let conn = open_market(dir.path().join("m.db")).unwrap();
        popular(&conn, 2_500);
        let src = SqliteMarketDataSource::new(conn);

        let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let n = src
            .candles(
                &Symbol::new("BTCUSDT").unwrap(),
                Interval::M1,
                base,
                base + chrono::Duration::days(3),
            )
            .unwrap()
            .count();
        assert_eq!(n, 2_500);
    }

    #[test]
    fn respeita_a_fronteira_do_periodo() {
        let dir = tempdir().unwrap();
        let conn = open_market(dir.path().join("m.db")).unwrap();
        popular(&conn, 10);
        let src = SqliteMarketDataSource::new(conn);

        let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let n = src
            .candles(
                &Symbol::new("BTCUSDT").unwrap(),
                Interval::M1,
                base + chrono::Duration::minutes(3),
                base + chrono::Duration::minutes(7),
            )
            .unwrap()
            .count();
        assert_eq!(
            n, 4,
            "minutos 3, 4, 5 e 6 — início inclusivo, fim exclusivo"
        );
    }

    #[test]
    fn periodo_vazio_devolve_iterador_vazio() {
        let dir = tempdir().unwrap();
        let conn = open_market(dir.path().join("m.db")).unwrap();
        let src = SqliteMarketDataSource::new(conn);
        let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(
            src.candles(
                &Symbol::new("BTCUSDT").unwrap(),
                Interval::M1,
                base,
                base + chrono::Duration::hours(1)
            )
            .unwrap()
            .count(),
            0
        );
    }
}
