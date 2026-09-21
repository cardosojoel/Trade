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
fn modo_paper_nao_roda_sob_o_comando_backtest() {
    // `paper` passou a existir, mas backtest e paper são regimes diferentes:
    // um percorre histórico, o outro opera continuamente. Aceitar a flag aqui
    // faria o comando prometer o que não faz.
    trade()
        .args(base())
        .args(["--mode", "paper"])
        .assert()
        .code(2)
        .stderr(contains("só roda em modo 'backtest'"));
}

#[test]
fn modo_live_e_recusado_e_diz_por_que() {
    // A mensagem mudou quando `paper` passou a rodar de verdade: `live` não
    // é "ainda não implementado", é **não liberado**. A promoção é ato humano
    // registrado, e dizer que falta código sugeriria que basta escrevê-lo.
    trade()
        .args(base())
        .args(["--mode", "live"])
        .assert()
        .code(2)
        .stderr(contains("não liberado"));
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
