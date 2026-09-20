//! Persistência de execuções, extrato e métricas.

use crate::decimal_sql::to_sql;
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use std::collections::BTreeMap;
use trade_domain::{
    ExecutionMode, FeeModel, Interval, Money, RiskLimits, RunMetrics, Symbol, Trade,
};
use trade_ports::StorageError;

/// Cabeçalho de uma execução.
pub struct RunHeader<'a> {
    pub run_id: &'a str,
    pub mode: ExecutionMode,
    pub symbol: &'a Symbol,
    pub interval: Interval,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub initial_capital: Money,
    pub limits: &'a RiskLimits,
    pub fees: &'a FeeModel,
    pub strategy: &'a str,
    pub strategy_params: &'a BTreeMap<String, String>,
    pub started_at: DateTime<Utc>,
}

pub struct RunsRepository {
    conn: Connection,
}

/// Serializa um mapa como JSON com chaves em ordem.
///
/// Feito à mão para não arrastar `serde` até aqui por tão pouco, e com
/// [`BTreeMap`] na entrada para que a ordem seja estável: ordem instável
/// vazaria para o banco e duas execuções idênticas gravariam textos
/// diferentes, quebrando a comparação que SC-004 exige.
fn json_map(m: &BTreeMap<String, String>) -> String {
    let corpo: Vec<String> = m
        .iter()
        .map(|(k, v)| format!("{}:{}", escapar(k), escapar(v)))
        .collect();
    format!("{{{}}}", corpo.join(","))
}

fn escapar(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn limites_json(l: &RiskLimits) -> String {
    let mut m = BTreeMap::new();
    m.insert("max_daily_loss".into(), to_sql(l.max_daily_loss));
    m.insert("max_position_size".into(), to_sql(l.max_position_size));
    m.insert("max_total_exposure".into(), to_sql(l.max_total_exposure));
    m.insert(
        "max_orders_per_window".into(),
        l.max_orders_per_window.to_string(),
    );
    m.insert("window_minutes".into(), l.window_minutes.to_string());
    m.insert(
        "max_transient_retries".into(),
        l.max_transient_retries.to_string(),
    );
    m.insert(
        "max_price_deviation_ratio".into(),
        to_sql(l.max_price_deviation_ratio),
    );
    json_map(&m)
}

fn taxas_json(f: &FeeModel) -> String {
    let mut m = BTreeMap::new();
    m.insert("taker_fee_rate".into(), to_sql(f.taker_fee_rate));
    m.insert("slippage_rate".into(), to_sql(f.slippage_rate));
    json_map(&m)
}

fn escrita(e: rusqlite::Error) -> StorageError {
    StorageError::Write(e.to_string())
}

impl RunsRepository {
    pub fn new(conn: Connection) -> Self {
        RunsRepository { conn }
    }

    /// Grava o cabeçalho, **incluindo a cerca e as taxas sob as quais a
    /// execução correu** (FR-025). Comparar duas execuções exige saber sob que
    /// condições cada uma foi feita.
    pub fn start_run(&mut self, h: &RunHeader<'_>) -> Result<(), StorageError> {
        self.conn
            .execute(
                "INSERT INTO run (run_id, mode, symbol, interval, from_ms, to_ms, \
                 initial_capital, limits_json, fees_json, strategy, strategy_params_json, started_at) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
                rusqlite::params![
                    h.run_id,
                    h.mode.as_str(),
                    h.symbol.as_str(),
                    h.interval.as_str(),
                    h.from.timestamp_millis(),
                    h.to.timestamp_millis(),
                    to_sql(h.initial_capital),
                    limites_json(h.limits),
                    taxas_json(h.fees),
                    h.strategy,
                    json_map(h.strategy_params),
                    h.started_at.timestamp_millis(),
                ],
            )
            .map_err(escrita)?;
        Ok(())
    }

    pub fn finish_run(
        &mut self,
        run_id: &str,
        ended_at: DateTime<Utc>,
        outcome: &str,
        halt_reason: Option<&str>,
    ) -> Result<(), StorageError> {
        self.conn
            .execute(
                "UPDATE run SET ended_at = ?2, outcome = ?3, halt_reason = ?4 WHERE run_id = ?1",
                rusqlite::params![run_id, ended_at.timestamp_millis(), outcome, halt_reason],
            )
            .map_err(escrita)?;
        Ok(())
    }

    /// Grava o extrato inteiro em uma transação.
    pub fn save_trades(&mut self, run_id: &str, trades: &[Trade]) -> Result<(), StorageError> {
        let tx = self.conn.transaction().map_err(escrita)?;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO trade (run_id, trade_seq, entry_ms, entry_price, exit_ms, \
                     exit_price, qty, fees, pnl) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                )
                .map_err(escrita)?;
            for t in trades {
                stmt.execute(rusqlite::params![
                    run_id,
                    t.seq as i64,
                    t.entry_at.timestamp_millis(),
                    to_sql(t.entry_price),
                    t.exit_at.timestamp_millis(),
                    to_sql(t.exit_price),
                    to_sql(t.qty),
                    to_sql(t.fees),
                    to_sql(t.pnl),
                ])
                .map_err(escrita)?;
            }
        }
        tx.commit().map_err(escrita)?;
        Ok(())
    }

    pub fn save_metrics(&mut self, run_id: &str, m: &RunMetrics) -> Result<(), StorageError> {
        self.conn
            .execute(
                "INSERT INTO metrics (run_id, profit_factor, max_drawdown, trade_count, \
                 net_result, gross_profit, gross_loss, total_fees, total_slippage) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                rusqlite::params![
                    run_id,
                    // NULL quando indefinido. Gravar infinito produziria um
                    // número que induz erro na Porta 1 de promoção.
                    m.profit_factor.map(to_sql),
                    to_sql(m.max_drawdown),
                    m.trade_count as i64,
                    to_sql(m.net_result),
                    to_sql(m.gross_profit),
                    to_sql(m.gross_loss),
                    to_sql(m.total_fees),
                    to_sql(m.total_slippage),
                ],
            )
            .map_err(escrita)?;
        Ok(())
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_runs;
    use chrono::TimeZone;
    use rust_decimal::dec;
    use tempfile::tempdir;

    fn repo() -> (tempfile::TempDir, RunsRepository) {
        let dir = tempdir().unwrap();
        let conn = open_runs(dir.path().join("runs.db")).unwrap();
        (dir, RunsRepository::new(conn))
    }

    fn cabecalho<'a>(
        sym: &'a Symbol,
        lim: &'a RiskLimits,
        fees: &'a FeeModel,
        params: &'a BTreeMap<String, String>,
    ) -> RunHeader<'a> {
        let t = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        RunHeader {
            run_id: "R1",
            mode: ExecutionMode::Backtest,
            symbol: sym,
            interval: Interval::M1,
            from: t,
            to: t + chrono::Duration::days(1),
            initial_capital: dec!(10000),
            limits: lim,
            fees,
            strategy: "sma-cross",
            strategy_params: params,
            started_at: t,
        }
    }

    #[test]
    fn grava_a_cerca_junto_da_execucao() {
        let (_d, mut r) = repo();
        let sym = Symbol::new("BTCUSDT").unwrap();
        let lim = RiskLimits {
            max_daily_loss: dec!(200),
            ..RiskLimits::default()
        };
        let fees = FeeModel::default();
        let params = BTreeMap::new();
        r.start_run(&cabecalho(&sym, &lim, &fees, &params)).unwrap();

        let json: String = r
            .connection()
            .query_row("SELECT limits_json FROM run WHERE run_id='R1'", [], |x| {
                x.get(0)
            })
            .unwrap();
        assert!(json.contains("\"max_daily_loss\":\"200\""), "veio {json}");
    }

    #[test]
    fn profit_factor_indefinido_vira_null() {
        let (_d, mut r) = repo();
        let m = RunMetrics::from_trades(&[], dec!(0));
        r.save_metrics("R1", &m).unwrap();
        let valor: Option<String> = r
            .connection()
            .query_row(
                "SELECT profit_factor FROM metrics WHERE run_id='R1'",
                [],
                |x| x.get(0),
            )
            .unwrap();
        assert_eq!(valor, None, "indefinido é NULL, nunca um número inventado");
    }

    #[test]
    fn o_extrato_e_gravado_inteiro() {
        let (_d, mut r) = repo();
        let t = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let trades: Vec<Trade> = (1..=5)
            .map(|i| Trade {
                seq: i,
                entry_at: t,
                entry_price: dec!(100),
                exit_at: t,
                exit_price: dec!(110),
                qty: dec!(1),
                fees: dec!(0.1),
                pnl: dec!(9.9),
            })
            .collect();
        r.save_trades("R1", &trades).unwrap();

        let n: i64 = r
            .connection()
            .query_row("SELECT COUNT(*) FROM trade WHERE run_id='R1'", [], |x| {
                x.get(0)
            })
            .unwrap();
        assert_eq!(n, 5);
    }

    #[test]
    fn os_valores_monetarios_vao_como_texto_exato() {
        let (_d, mut r) = repo();
        let t = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        r.save_trades(
            "R1",
            &[Trade {
                seq: 1,
                entry_at: t,
                entry_price: dec!(63420.12345678),
                exit_at: t,
                exit_price: dec!(63500.00000001),
                qty: dec!(0.00000001),
                fees: dec!(0),
                pnl: dec!(0.0000000008),
            }],
        )
        .unwrap();

        let preco: String = r
            .connection()
            .query_row("SELECT entry_price FROM trade WHERE run_id='R1'", [], |x| {
                x.get(0)
            })
            .unwrap();
        assert_eq!(preco, "63420.12345678", "nenhum dígito perdido no disco");
    }
}
