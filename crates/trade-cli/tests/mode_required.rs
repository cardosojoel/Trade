//! FR-001 a FR-003 e SC-007 — o modo de execução não tem padrão.
//!
//! É aqui que o Princípio I vira comportamento observável. Os três casos são
//! distintos de propósito: ausência, não implementado e desconhecido dizem
//! coisas diferentes a quem digitou.

use assert_cmd::Command;
use predicates::str::contains;

fn trade() -> Command {
    Command::cargo_bin("trade").unwrap()
}

/// Argumentos mínimos, sem o modo.
fn base() -> Vec<&'static str> {
    vec![
        "backtest",
        "--symbol",
        "BTCUSDT",
        "--from",
        "2026-01-01",
        "--to",
        "2026-01-02",
        "--capital",
        "1000",
    ]
}

#[test]
fn sem_modo_falha_com_codigo_dois() {
    trade()
        .args(base())
        .assert()
        .code(2)
        .stderr(contains("--mode"));
}

#[test]
fn modo_paper_diz_que_ainda_nao_existe() {
    // Não é "valor inválido": isso sugeriria erro de digitação e convidaria a
    // tentar de novo. "Ainda não implementado" informa o estado do sistema.
    trade()
        .args(base())
        .args(["--mode", "paper"])
        .assert()
        .code(2)
        .stderr(contains("ainda não implementado"));
}

#[test]
fn modo_live_diz_que_ainda_nao_existe() {
    trade()
        .args(base())
        .args(["--mode", "live"])
        .assert()
        .code(2)
        .stderr(contains("ainda não implementado"));
}

#[test]
fn a_mensagem_de_recusa_diz_qual_modo_existe() {
    trade()
        .args(base())
        .args(["--mode", "live"])
        .assert()
        .code(2)
        .stderr(contains("backtest"));
}

#[test]
fn modo_desconhecido_e_distinto_de_nao_implementado() {
    trade()
        .args(base())
        .args(["--mode", "xpto"])
        .assert()
        .code(2)
        .stderr(contains("desconhecido"));
}
