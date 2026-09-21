//! T050 — `trade porta2`.
//!
//! O relatório existe **antes** do dado que ele mede. Quando os trinta dias
//! correrem, a medição já terá sido exercitada — em vez de ser escrita no dia
//! em que o período mais caro do projeto termina.

use assert_cmd::Command;
use predicates::str::contains;

fn trade() -> Command {
    Command::cargo_bin("trade").unwrap()
}

#[test]
fn sem_sessao_paper_o_comando_diz_o_que_falta_e_como() {
    // Um erro que dissesse só "não há dados" deixaria quem lê descobrindo
    // sozinho o caminho. FR-022 vale aqui pelo mesmo motivo que vale nas
    // rotas: quem está diante do erro não tem o plano aberto ao lado.
    let dir = tempfile::tempdir().unwrap();
    trade()
        .args([
            "porta2",
            "--runs-db",
            dir.path().join("vazio.db").to_str().unwrap(),
        ])
        .assert()
        .code(3)
        .stderr(contains("modo `paper`"))
        .stderr(contains("trade paper verificar"))
        .stderr(contains("trade paper rodar"));
}

#[test]
fn os_limiares_sao_argumentos_e_nao_constantes() {
    // Valores de partida, ajustáveis — a constitution manda tratá-los assim.
    // Se estivessem embutidos, não haveria a flag para conferir.
    trade()
        .args(["porta2", "--help"])
        .assert()
        .success()
        .stdout(contains("--dias-exigidos"))
        .stdout(contains("--amostra-minima"));
}

#[test]
fn o_comando_nao_promove_nada() {
    // Princípio I: a promoção entre portões é ato humano registrado. Não
    // existe flag que promova, e é de propósito.
    trade()
        .args(["porta2", "--help"])
        .assert()
        .success()
        .stdout(predicates::function::function(|s: &str| {
            !s.contains("--promover") && !s.contains("--aprovar")
        }));
}
