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

/// Uma execução que começou e não foi encerrada.
///
/// Sem `ended_at` é o que fica quando o processo caiu — e é o que a sessão
/// procura ao subir para retomar de onde parou (decisão 035).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecucaoEmAberto {
    pub run_id: String,
    pub started_at: DateTime<Utc>,
}

/// Uma execução, como o registro a guarda.
///
/// Os JSON da cerca e das taxas vêm **crus**. Quem lê decide se os interpreta:
/// interpretá-los aqui obrigaria esta crate a conhecer a forma de `RiskLimits`
/// em toda versão que já foi gravada, e o registro é insubstituível — tem
/// linha de ontem e vai ter de amanhã.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecucaoLida {
    pub run_id: String,
    pub mode: String,
    pub symbol: String,
    pub interval: String,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub initial_capital: Money,
    pub limits_json: String,
    pub fees_json: String,
    pub strategy: String,
    pub strategy_params_json: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub outcome: Option<String>,
    pub halt_reason: Option<String>,
}

/// As métricas de uma execução.
///
/// `profit_factor` é `Option` porque **é** indefinido sem operação perdedora.
/// Convertê-lo em zero ou infinito produziria um número que induz erro na
/// Porta 1 de promoção (FR-021).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricasLidas {
    pub profit_factor: Option<Money>,
    pub max_drawdown: Money,
    pub trade_count: u64,
    pub net_result: Money,
    pub gross_profit: Money,
    pub gross_loss: Money,
    pub total_fees: Money,
    pub total_slippage: Money,
}

/// Um evento do registro, como ele foi gravado.
#[derive(Debug, Clone, PartialEq)]
pub struct EventoLido {
    pub seq: u64,
    pub at: DateTime<Utc>,
    pub kind: String,
    pub payload: serde_json::Value,
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
    m.insert(
        "max_position_hours".into(),
        l.max_position_hours.to_string(),
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

    /// Uma execução pelo identificador.
    ///
    /// `None` é "não existe", e não "não deu para ler" — as duas decidem
    /// coisas diferentes, e o contrato de erro do servidor as separa em 404 e
    /// 503.
    pub fn execucao(&self, run_id: &str) -> Result<Option<ExecucaoLida>, StorageError> {
        let mut stmt = self
            .conn
            .prepare(&format!("{SELECT_EXECUCAO} WHERE run_id = ?1"))
            .map_err(consulta)?;
        let mut linhas = stmt.query_map([run_id], le_execucao).map_err(consulta)?;
        match linhas.next() {
            None => Ok(None),
            Some(l) => Ok(Some(l.map_err(consulta)??)),
        }
    }

    /// Todas as execuções, da mais recente para a mais antiga.
    ///
    /// Ordenadas por `run_id`, que é ULID e portanto ordenável por tempo. Não
    /// depende de o relógio da máquina ter andado para a frente entre duas
    /// execuções.
    pub fn listar_execucoes(&self) -> Result<Vec<ExecucaoLida>, StorageError> {
        let mut stmt = self
            .conn
            .prepare(&format!("{SELECT_EXECUCAO} ORDER BY run_id DESC"))
            .map_err(consulta)?;
        let linhas = stmt.query_map([], le_execucao).map_err(consulta)?;
        let mut out = Vec::new();
        for l in linhas {
            out.push(l.map_err(consulta)??);
        }
        Ok(out)
    }

    /// As métricas de uma execução, se já foram gravadas.
    pub fn metricas(&self, run_id: &str) -> Result<Option<MetricasLidas>, StorageError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT profit_factor, max_drawdown, trade_count, net_result, \
                 gross_profit, gross_loss, total_fees, total_slippage \
                 FROM metrics WHERE run_id = ?1",
            )
            .map_err(consulta)?;
        let mut linhas = stmt
            .query_map([run_id], |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, String>(6)?,
                    r.get::<_, String>(7)?,
                ))
            })
            .map_err(consulta)?;

        let Some(l) = linhas.next() else {
            return Ok(None);
        };
        let (pf, dd, n, net, gp, gl, fees, slip) = l.map_err(consulta)?;
        Ok(Some(MetricasLidas {
            profit_factor: pf.map(|s| decimal(&s, "profit_factor")).transpose()?,
            max_drawdown: decimal(&dd, "max_drawdown")?,
            trade_count: u64::try_from(n).unwrap_or(0),
            net_result: decimal(&net, "net_result")?,
            gross_profit: decimal(&gp, "gross_profit")?,
            gross_loss: decimal(&gl, "gross_loss")?,
            total_fees: decimal(&fees, "total_fees")?,
            total_slippage: decimal(&slip, "total_slippage")?,
        }))
    }

    /// O extrato de uma execução, em ordem.
    pub fn extrato(&self, run_id: &str) -> Result<Vec<Trade>, StorageError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT trade_seq, entry_ms, entry_price, exit_ms, exit_price, \
                 qty, fees, pnl FROM trade WHERE run_id = ?1 ORDER BY trade_seq",
            )
            .map_err(consulta)?;
        let linhas = stmt
            .query_map([run_id], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, String>(6)?,
                    r.get::<_, String>(7)?,
                ))
            })
            .map_err(consulta)?;

        let mut out = Vec::new();
        for l in linhas {
            let (seq, e_ms, e_px, x_ms, x_px, qty, fees, pnl) = l.map_err(consulta)?;
            out.push(Trade {
                seq: u64::try_from(seq).unwrap_or(0),
                entry_at: instante(e_ms)?,
                entry_price: decimal(&e_px, "entry_price")?,
                exit_at: instante(x_ms)?,
                exit_price: decimal(&x_px, "exit_price")?,
                qty: decimal(&qty, "qty")?,
                fees: decimal(&fees, "fees")?,
                pnl: decimal(&pnl, "pnl")?,
            });
        }
        Ok(out)
    }

    /// A linha do tempo de uma execução.
    ///
    /// Ordenada por `seq`, **nunca** por `at_ms` (FR-012). Dois eventos do
    /// mesmo instante simulado — e há muitos, porque a cadeia sinal → ordem →
    /// decisão → preenchimento acontece toda dentro de uma vela — só têm
    /// ordem definida pelo `seq`. Ordenar pelo instante devolveria a cadeia
    /// embaralhada dentro da vela, e ninguém notaria.
    pub fn linha_do_tempo(&self, run_id: &str) -> Result<Vec<EventoLido>, StorageError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT seq, at_ms, kind, payload_json FROM audit_event \
                 WHERE run_id = ?1 ORDER BY seq",
            )
            .map_err(consulta)?;
        let linhas = stmt
            .query_map([run_id], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })
            .map_err(consulta)?;

        let mut out = Vec::new();
        for l in linhas {
            let (seq, at_ms, kind, payload) = l.map_err(consulta)?;
            out.push(EventoLido {
                seq: u64::try_from(seq).unwrap_or(0),
                at: instante(at_ms)?,
                kind,
                payload: serde_json::from_str(&payload).map_err(|e| {
                    StorageError::Query(format!("carga do seq {seq} ilegível: {e}"))
                })?,
            });
        }
        Ok(out)
    }

    /// A última execução em modo `paper` que não foi encerrada.
    ///
    /// Ordenada por `run_id` e não por `started_at`: o identificador é ULID,
    /// ordenável por tempo por construção, e ordenar pelo que é chave não
    /// depende de o relógio da máquina ter andado para a frente.
    pub fn ultima_paper_em_aberto(&self) -> Result<Option<ExecucaoEmAberto>, StorageError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT run_id, started_at FROM run \
                 WHERE mode = 'paper' AND ended_at IS NULL \
                 ORDER BY run_id DESC LIMIT 1",
            )
            .map_err(|e| StorageError::Query(e.to_string()))?;

        let mut linhas = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
            .map_err(|e| StorageError::Query(e.to_string()))?;

        match linhas.next() {
            None => Ok(None),
            Some(linha) => {
                let (run_id, ms) = linha.map_err(|e| StorageError::Query(e.to_string()))?;
                let started_at = DateTime::from_timestamp_millis(ms)
                    .ok_or_else(|| StorageError::Query(format!("instante inválido: {ms}")))?;
                Ok(Some(ExecucaoEmAberto { run_id, started_at }))
            }
        }
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

#[cfg(test)]
mod testes_retomada {
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

    fn abrir(r: &mut RunsRepository, run_id: &str, mode: ExecutionMode, hora: u32) {
        let sym = Symbol::new("BTCUSDT").unwrap();
        let lim = RiskLimits::default();
        let fees = FeeModel::default();
        let params = BTreeMap::new();
        let t = Utc.with_ymd_and_hms(2026, 1, 1, hora, 0, 0).unwrap();
        r.start_run(&RunHeader {
            run_id,
            mode,
            symbol: &sym,
            interval: Interval::M1,
            from: t,
            to: t + chrono::Duration::days(1),
            initial_capital: dec!(10000),
            limits: &lim,
            fees: &fees,
            strategy: "sma-cross",
            strategy_params: &params,
            started_at: t,
        })
        .unwrap();
    }

    #[test]
    fn sem_execucao_paper_nao_ha_o_que_retomar() {
        let (_d, mut r) = repo();
        abrir(&mut r, "B1", ExecutionMode::Backtest, 1);
        assert_eq!(r.ultima_paper_em_aberto().unwrap(), None);
    }

    #[test]
    fn a_execucao_encerrada_nao_e_retomada() {
        let (_d, mut r) = repo();
        abrir(&mut r, "P1", ExecutionMode::Paper, 1);
        r.finish_run("P1", Utc::now(), "completed", None).unwrap();
        assert_eq!(
            r.ultima_paper_em_aberto().unwrap(),
            None,
            "encerrada é encerrada: retomá-la duplicaria o registro dela"
        );
    }

    #[test]
    fn retoma_a_mais_recente_das_que_ficaram_abertas() {
        let (_d, mut r) = repo();
        abrir(&mut r, "P1", ExecutionMode::Paper, 1);
        abrir(&mut r, "P2", ExecutionMode::Paper, 2);
        let a = r.ultima_paper_em_aberto().unwrap().unwrap();
        assert_eq!(a.run_id, "P2");
    }

    #[test]
    fn um_backtest_em_aberto_nao_e_confundido_com_sessao() {
        let (_d, mut r) = repo();
        abrir(&mut r, "P1", ExecutionMode::Paper, 1);
        abrir(&mut r, "Z9", ExecutionMode::Backtest, 9);
        let a = r.ultima_paper_em_aberto().unwrap().unwrap();
        assert_eq!(a.run_id, "P1", "o modo separa, e o ULID maior não engana");
    }
}

const SELECT_EXECUCAO: &str = "SELECT run_id, mode, symbol, interval, from_ms, to_ms, \
     initial_capital, limits_json, fees_json, strategy, strategy_params_json, \
     started_at, ended_at, outcome, halt_reason FROM run";

fn consulta(e: rusqlite::Error) -> StorageError {
    StorageError::Query(e.to_string())
}

/// Lê um valor monetário gravado como texto.
///
/// Gravado como TEXT justamente para isto: um `REAL` no caminho perderia
/// dígito entre a escrita e a leitura, e o extrato deixaria de fechar contra a
/// soma feita por fora, em SQL (SC-009 da feature 001).
fn decimal(s: &str, campo: &str) -> Result<Money, StorageError> {
    use std::str::FromStr;
    Money::from_str(s).map_err(|e| StorageError::Query(format!("`{campo}` = {s:?}: {e}")))
}

fn instante(ms: i64) -> Result<DateTime<Utc>, StorageError> {
    DateTime::from_timestamp_millis(ms)
        .ok_or_else(|| StorageError::Query(format!("instante inválido: {ms}")))
}

/// Monta uma execução a partir da linha, adiando a conversão de decimal.
///
/// O `Result` de dentro existe porque `rusqlite` não sabe converter para
/// `Decimal`: o erro de conversão é nosso, não dele, e sai por fora do erro
/// dele.
#[allow(clippy::type_complexity)]
fn le_execucao(r: &rusqlite::Row<'_>) -> rusqlite::Result<Result<ExecucaoLida, StorageError>> {
    let run_id: String = r.get(0)?;
    let mode: String = r.get(1)?;
    let symbol: String = r.get(2)?;
    let interval: String = r.get(3)?;
    let from_ms: i64 = r.get(4)?;
    let to_ms: i64 = r.get(5)?;
    let capital: String = r.get(6)?;
    let limits_json: String = r.get(7)?;
    let fees_json: String = r.get(8)?;
    let strategy: String = r.get(9)?;
    let strategy_params_json: String = r.get(10)?;
    let started: i64 = r.get(11)?;
    let ended: Option<i64> = r.get(12)?;
    let outcome: Option<String> = r.get(13)?;
    let halt_reason: Option<String> = r.get(14)?;

    Ok((|| {
        Ok(ExecucaoLida {
            run_id,
            mode,
            symbol,
            interval,
            from: instante(from_ms)?,
            to: instante(to_ms)?,
            initial_capital: decimal(&capital, "initial_capital")?,
            limits_json,
            fees_json,
            strategy,
            strategy_params_json,
            started_at: instante(started)?,
            ended_at: ended.map(instante).transpose()?,
            outcome,
            halt_reason,
        })
    })())
}
