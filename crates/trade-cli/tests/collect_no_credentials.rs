//! FR-012 — a coleta não recebe credencial.

use assert_cmd::Command;
use predicates::str::contains;

fn trade() -> Command {
    Command::cargo_bin("trade").unwrap()
}

#[test]
fn o_comando_nao_expoe_nenhum_parametro_de_credencial() {
    // A garantia mais simples é não haver o que passar: se a opção não
    // existe, ninguém a preenche por engano e nada vaza para um log.
    let saida = trade().args(["collect", "--help"]).output().unwrap();
    let texto = String::from_utf8_lossy(&saida.stdout).to_lowercase();

    for proibido in ["api-key", "api_key", "secret", "token", "credential"] {
        assert!(!texto.contains(proibido), "a ajuda menciona '{proibido}'");
    }
}

#[test]
fn periodo_invertido_e_recusado_antes_de_tocar_a_rede() {
    trade()
        .args([
            "collect",
            "--from",
            "2026-01-10",
            "--to",
            "2026-01-01",
            "--db",
            "/tmp/nao-deve-ser-criado.db",
        ])
        .assert()
        .code(2)
        .stderr(contains("período vazio"));
}

#[test]
fn periodo_no_futuro_e_recusado() {
    trade()
        .args([
            "collect",
            "--from",
            "2099-01-01",
            "--to",
            "2099-01-02",
            "--db",
            "/tmp/nao-deve-ser-criado.db",
        ])
        .assert()
        .code(2)
        .stderr(contains("futuro"));
}
