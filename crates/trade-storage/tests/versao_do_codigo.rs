//! P6/P9 — a versão do código na tabela `run` (decisão 009, implementada pela 043).
//!
//! O defeito que isto corrige é medido, não hipotético: duas das nove
//! execuções gravadas têm símbolo, período, estratégia, parâmetros, limites e
//! taxas **iguais** às outras seis, e resultado diferente — divergindo em
//! 14.299 das 14.308 operações. A causa foi um commit que mudou o
//! arredondamento da quantidade, e ela estava fora do registro.
//!
//! Numa execução de trinta dias corridos, um registro que não diz qual código
//! o produziu é um registro que não sustenta a Porta 3.

use chrono::{Duration, TimeZone, Utc};
use rust_decimal::dec;
use std::collections::BTreeMap;
use tempfile::tempdir;
use trade_domain::{ExecutionMode, FeeModel, Interval, RiskLimits, Symbol};
use trade_storage::open_runs;
use trade_storage::runs_repo::{RunHeader, RunsRepository};

fn t() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
}

fn gravar(r: &mut RunsRepository, run_id: &str, versao: Option<&str>) {
    let sym = Symbol::new("BTCUSDT").unwrap();
    let lim = RiskLimits::default();
    let fees = FeeModel::default();
    let params = BTreeMap::new();
    r.start_run(&RunHeader {
        run_id,
        mode: ExecutionMode::Backtest,
        symbol: &sym,
        interval: Interval::M1,
        from: t(),
        to: t() + Duration::days(1),
        initial_capital: dec!(10000),
        limits: &lim,
        fees: &fees,
        strategy: "sma-cross",
        strategy_params: &params,
        started_at: t(),
        code_version: versao,
    })
    .unwrap();
}

fn repo() -> (tempfile::TempDir, RunsRepository) {
    let dir = tempdir().unwrap();
    let conn = open_runs(dir.path().join("runs.db")).unwrap();
    (dir, RunsRepository::new(conn))
}

#[test]
fn a_versao_e_gravada_e_lida_de_volta() {
    let (_d, mut r) = repo();
    gravar(&mut r, "R1", Some("a1b2c3d4e5f6"));
    assert_eq!(
        r.execucao("R1").unwrap().unwrap().code_version.as_deref(),
        Some("a1b2c3d4e5f6")
    );
}

#[test]
fn a_marca_de_arvore_suja_sobrevive_ao_disco() {
    // Um binário construído de árvore modificada afirma um commit que não
    // descreve o que rodou. A marca é o que impede essa afirmação.
    let (_d, mut r) = repo();
    gravar(&mut r, "R1", Some("a1b2c3d4e5f6-sujo"));
    let v = r.execucao("R1").unwrap().unwrap().code_version.unwrap();
    assert!(v.ends_with("-sujo"), "veio {v}");
}

#[test]
fn execucao_sem_versao_volta_como_none_e_nao_como_texto_vazio() {
    // Presente e nulo, nunca uma string vazia que a tela mostraria como se
    // fosse uma versão de nome estranho.
    let (_d, mut r) = repo();
    gravar(&mut r, "R1", None);
    assert_eq!(r.execucao("R1").unwrap().unwrap().code_version, None);
}

#[test]
fn banco_gravado_antes_da_coluna_existir_continua_legivel() {
    // O `runs.db` deste repositório tem nove execuções sem a coluna. O
    // registro é insubstituível: uma migração que o quebrasse destruiria
    // auditoria que não se reconstrói de fonte nenhuma.
    let dir = tempdir().unwrap();
    let caminho = dir.path().join("runs.db");

    // Esquema antigo, escrito à mão, sem a coluna.
    {
        let conn = rusqlite::Connection::open(&caminho).unwrap();
        conn.execute_batch(
            "CREATE TABLE run (
                 run_id TEXT PRIMARY KEY, mode TEXT NOT NULL, symbol TEXT NOT NULL,
                 interval TEXT NOT NULL, from_ms INTEGER NOT NULL, to_ms INTEGER NOT NULL,
                 initial_capital TEXT NOT NULL, limits_json TEXT NOT NULL,
                 fees_json TEXT NOT NULL, strategy TEXT NOT NULL,
                 strategy_params_json TEXT NOT NULL, started_at INTEGER NOT NULL,
                 ended_at INTEGER, outcome TEXT, halt_reason TEXT);
             INSERT INTO run VALUES ('ANTIGA','backtest','BTCUSDT','1m',0,1,'10000',
                 '{}','{}','sma-cross','{}',0,NULL,NULL,NULL);",
        )
        .unwrap();
    }

    // Abrir aplica a migração, sem perder o que havia.
    let conn = open_runs(&caminho).unwrap();
    let r = RunsRepository::new(conn);
    let antiga = r
        .execucao("ANTIGA")
        .unwrap()
        .expect("a linha antiga sobreviveu");
    assert_eq!(
        antiga.code_version, None,
        "execução anterior à coluna não ganha versão inventada"
    );

    // E a partir daí grava normalmente.
    let mut r = r;
    gravar(&mut r, "NOVA", Some("abc123"));
    assert_eq!(
        r.execucao("NOVA").unwrap().unwrap().code_version.as_deref(),
        Some("abc123")
    );
    assert_eq!(r.listar_execucoes().unwrap().len(), 2);
}

#[test]
fn abrir_duas_vezes_nao_duplica_a_coluna() {
    let dir = tempdir().unwrap();
    let caminho = dir.path().join("runs.db");
    let _ = open_runs(&caminho).unwrap();
    let conn = open_runs(&caminho).unwrap();
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('run') WHERE name = 'code_version'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 1, "a migração é idempotente");
}
