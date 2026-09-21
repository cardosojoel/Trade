//! T039 a T041 — o comando `trade serve`, contra o processo de verdade.
//!
//! Sobe o binário, fala HTTP com ele por socket, e derruba. Nenhum teste
//! alcança rede externa: o servidor é local e os dois bancos são arquivos.

use assert_cmd::Command as Assert;
use predicates::str::contains;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};

/// Um servidor rodando, que morre com o teste.
struct Servidor {
    processo: Child,
    porta: u16,
    token: String,
    _dir: tempfile::TempDir,
}

impl Drop for Servidor {
    fn drop(&mut self) {
        let _ = self.processo.kill();
        let _ = self.processo.wait();
    }
}

impl Servidor {
    fn subir() -> Servidor {
        let dir = tempfile::tempdir().unwrap();
        // Porta livre, pedida ao sistema e devolvida antes de o servidor usá-la.
        let porta = {
            let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            l.local_addr().unwrap().port()
        };
        let raiz = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
        let mut p = Command::new(env!("CARGO_BIN_EXE_trade"))
            .current_dir(raiz)
            .args([
                "serve",
                "--porta",
                &porta.to_string(),
                "--porta1",
                "examples/porta1.toml",
                "--limits",
                "examples/limits.toml",
                "--fees",
                "examples/fees.toml",
                "--instrumento",
                "examples/instrumento.toml",
                "--runs-db",
                dir.path().join("runs.db").to_str().unwrap(),
                "--market-db",
                dir.path().join("market.db").to_str().unwrap(),
            ])
            .stdout(Stdio::piped())
            .spawn()
            .expect("subir o servidor");

        // O token é impresso **uma vez**, e é daqui que o teste o lê — como
        // quem opera leria.
        let saida = BufReader::new(p.stdout.take().unwrap());
        let mut token = String::new();
        for linha in saida.lines().map_while(Result::ok) {
            if let Some(t) = linha.split_whitespace().last()
                && linha.contains("Token de escrita")
            {
                token = t.to_string();
                break;
            }
        }
        assert!(!token.is_empty(), "o servidor não imprimiu o token");
        Servidor {
            processo: p,
            porta,
            token,
            _dir: dir,
        }
    }

    fn pedir(
        &self,
        metodo: &str,
        caminho: &str,
        cabecalhos: &[(&str, &str)],
        corpo: &str,
    ) -> (u16, String) {
        let mut s = TcpStream::connect(("127.0.0.1", self.porta)).expect("conectar");
        let mut req = format!(
            "{metodo} {caminho} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\
             Content-Length: {}\r\n",
            corpo.len()
        );
        for (k, v) in cabecalhos {
            req.push_str(&format!("{k}: {v}\r\n"));
        }
        req.push_str("\r\n");
        req.push_str(corpo);
        s.write_all(req.as_bytes()).unwrap();

        let mut bruto = String::new();
        s.read_to_string(&mut bruto).unwrap();
        let status = bruto
            .split_whitespace()
            .nth(1)
            .and_then(|c| c.parse().ok())
            .unwrap_or(0);
        let corpo = bruto
            .split_once("\r\n\r\n")
            .map_or("", |(_, c)| c)
            .to_string();
        (status, corpo)
    }
}

// ---------------------------------------------------------------- T040

#[test]
fn o_token_e_impresso_uma_vez_e_nao_aparece_em_resposta_nenhuma() {
    // FR-019 e FR-020. Ele sai no terminal de quem subiu o servidor, e em
    // lugar nenhum mais.
    let s = Servidor::subir();
    assert!(s.token.len() >= 32);

    for caminho in ["/runs", "/datasets", "/nao-existe"] {
        let (_, corpo) = s.pedir("GET", caminho, &[], "");
        assert!(
            !corpo.contains(&s.token),
            "{caminho} devolveu o token: {corpo}"
        );
    }
    let (_, corpo) = s.pedir("POST", "/runs", &[("X-Trade-Token", "chute")], "{}");
    assert!(!corpo.contains(&s.token), "a recusa devolveu o token");
}

#[test]
fn sem_o_arquivo_da_porta_1_o_servidor_nao_sobe() {
    // FR-024: subir julgando por números que ninguém escolheu seria pior que
    // não subir.
    Assert::cargo_bin("trade")
        .unwrap()
        .args(["serve", "--porta", "0", "--porta1", "nao-existe.toml"])
        .assert()
        .code(2)
        .stderr(contains("nao-existe.toml"));
}

// ---------------------------------------------------------------- T039

#[test]
fn as_recusas_chegam_pelo_socket_com_o_corpo_unico() {
    let s = Servidor::subir();

    // Sem token.
    let (status, corpo) = s.pedir("POST", "/runs", &[], r#"{"modo":"backtest"}"#);
    assert_eq!(status, 401);
    assert!(corpo.contains("token_ausente"), "veio {corpo}");

    // Com token e origem alheia.
    let (status, corpo) = s.pedir(
        "POST",
        "/runs",
        &[
            ("X-Trade-Token", &s.token),
            ("Origin", "https://exemplo.invalido"),
        ],
        r#"{"modo":"backtest"}"#,
    );
    assert_eq!(status, 403);
    assert!(corpo.contains("origem_recusada"), "veio {corpo}");

    // Com token, sem modo.
    let (status, corpo) = s.pedir("POST", "/runs", &[("X-Trade-Token", &s.token)], "{}");
    assert_eq!(status, 400);
    assert!(corpo.contains("modo_ausente"), "veio {corpo}");

    // Com token, pedindo `live`.
    let (status, corpo) = s.pedir(
        "POST",
        "/runs",
        &[("X-Trade-Token", &s.token)],
        r#"{"modo":"live"}"#,
    );
    assert_eq!(status, 403);
    assert!(corpo.contains("modo_recusado"), "veio {corpo}");
}

#[test]
fn a_leitura_responde_sem_token() {
    // FR-019: quem lê não muda nada. Exigir token para ler faria a interface
    // guardá-lo onde não precisa.
    let s = Servidor::subir();
    let (status, corpo) = s.pedir("GET", "/runs", &[], "");
    assert_eq!(status, 200);
    assert!(corpo.contains("\"grupos\""), "veio {corpo}");
    assert!(corpo.contains("\"origem\":\"runs.db\""), "veio {corpo}");
}

#[test]
fn o_historico_diz_que_e_cache_reconstruivel() {
    let s = Servidor::subir();
    let (status, corpo) = s.pedir("GET", "/datasets", &[], "");
    assert_eq!(status, 200);
    assert!(corpo.contains("\"origem\":\"market.db\""), "veio {corpo}");
    assert!(corpo.contains("\"reconstruivel\":true"), "veio {corpo}");
}

#[test]
fn execucao_que_nao_existe_responde_404_e_nao_500() {
    let s = Servidor::subir();
    let (status, corpo) = s.pedir("GET", "/runs/01NAOEXISTE", &[], "");
    assert_eq!(status, 404);
    assert!(corpo.contains("execucao_desconhecida"), "veio {corpo}");
}

#[test]
fn o_servidor_responde_no_loopback() {
    // SC-007 tem duas metades, e só uma cabe aqui. Que o socket **ligou** em
    // 127.0.0.1 é conferido lendo do sistema o endereço em que ligou — está
    // em `trade-serve/tests/recusas.rs`, e é o que impede o erro de
    // configuração.
    //
    // A outra metade — tentar de outra interface de rede e a tentativa
    // falhar — exige uma máquina com duas interfaces, e uma de CI pode não
    // ter. Fica no `quickstart.md`, onde há máquina de verdade. Um teste que
    // não pudesse afirmar isso e fingisse que sim seria pior que nenhum.
    let s = Servidor::subir();
    assert!(TcpStream::connect(("127.0.0.1", s.porta)).is_ok());
}
