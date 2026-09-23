//! T044 — o comando que abre a sessão contínua.
//!
//! Nenhum teste aqui emite ordem nem alcança a rede: todos param antes, nas
//! recusas que vêm **antes** da primeira vela. É o que se pode verificar sem
//! credencial, e é deliberadamente o que mais importa — uma recusa que
//! falhasse deixaria dinheiro passar.

use assert_cmd::Command;
use predicates::str::contains;

fn trade() -> Command {
    let mut c = Command::cargo_bin("trade").unwrap();
    // Nenhum teste pode herdar credencial da máquina de quem roda.
    c.env_remove("BYBIT_TESTNET_KEY")
        .env_remove("BYBIT_TESTNET_SECRET")
        .env_remove("BYBIT_DEMO_KEY")
        .env_remove("BYBIT_DEMO_SECRET")
        .env_remove("BYBIT_KEY")
        .env_remove("BYBIT_SECRET");
    c
}

#[test]
fn sem_modo_nao_roda() {
    // Princípio I: o modo é obrigatório e sem padrão. Ausência de
    // configuração não pode resultar em nada que toque dinheiro.
    trade()
        .args(["paper", "rodar", "--capital", "1000"])
        .assert()
        .code(2)
        .stderr(contains("--mode"));
}

#[test]
fn live_e_recusado_no_proprio_argumento() {
    // A promoção para capital real é ato humano registrado. Ela não cabe num
    // argumento de linha de comando, e a recusa vem antes de qualquer leitura
    // de configuração ou credencial.
    trade()
        .args(["paper", "rodar", "--mode", "live", "--capital", "1000"])
        .assert()
        .code(2)
        .stderr(contains("não liberado"));
}

#[test]
fn backtest_nao_roda_sob_o_comando_da_sessao() {
    // Os dois regimes são diferentes: um percorre histórico e termina, o
    // outro acompanha o mercado. Aceitar a flag faria o comando prometer o
    // que não faz.
    trade()
        .args(["paper", "rodar", "--mode", "backtest", "--capital", "1000"])
        .assert()
        .code(2)
        .stderr(contains("só opera em modo 'paper'"));
}

#[test]
fn sem_a_cerca_nao_comeca() {
    // Limite ausente não pode virar "tudo passa": sem o arquivo, não há
    // execução. A recusa vem antes da credencial de propósito — é mais barato
    // descobrir que falta configuração do que descobrir que falta chave.
    trade()
        .args([
            "paper",
            "rodar",
            "--mode",
            "paper",
            "--capital",
            "1000",
            "--limits",
            "nao-existe.toml",
        ])
        .assert()
        .code(2)
        .stderr(contains("nao-existe.toml"));
}

#[test]
fn sem_credencial_nao_comeca() {
    // FR-102: ausência de credencial aborta a inicialização. Chega aqui só
    // depois de a cerca e o instrumento terem sido lidos.
    trade()
        .args([
            "paper",
            "rodar",
            "--mode",
            "paper",
            "--capital",
            "1000",
            "--limits",
            "examples/limits.toml",
            "--fees",
            "examples/fees.toml",
            "--instrumento",
            "examples/instrumento.toml",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR").to_string() + "/../..")
        .assert()
        .code(2);
}

fn rodar_com(chave: &str, segredo: &str) -> assert_cmd::assert::Assert {
    trade()
        .args([
            "paper",
            "rodar",
            "--mode",
            "paper",
            "--capital",
            "1000",
            "--limits",
            "examples/limits.toml",
            "--fees",
            "examples/fees.toml",
            "--instrumento",
            "examples/instrumento.toml",
        ])
        .env(chave, "k")
        .env(segredo, "s")
        .current_dir(env!("CARGO_MANIFEST_DIR").to_string() + "/../..")
        .assert()
}

#[test]
fn credencial_de_producao_nao_comeca_a_sessao_paper() {
    // Princípio I. Sem esta recusa, `BYBIT_KEY` sozinha levava a sessão
    // "paper" a `api.bybit.com`, com dinheiro real e `mode = 'paper'` no
    // registro. A recusa vem antes de qualquer requisição: a chave falsa
    // daqui nunca chega à rede.
    rodar_com("BYBIT_KEY", "BYBIT_SECRET")
        .code(2)
        .stderr(contains("só opera com credencial de Demo Trading"))
        .stderr(contains("Produção"));
}

#[test]
fn credencial_de_testnet_nao_comeca_a_sessao_paper() {
    // Emenda 2.1.0: a Porta 2 é o Demo Trading. Uma sessão na testnet
    // produziria trinta dias que não valem como evidência da porta.
    rodar_com("BYBIT_TESTNET_KEY", "BYBIT_TESTNET_SECRET")
        .code(2)
        .stderr(contains("só opera com credencial de Demo Trading"))
        .stderr(contains("Testnet"));
}
