//! Servidor HTTP local que imita o endpoint `/v5/market/kline`.
//!
//! Escrito à mão sobre `TcpListener` em vez de trazer uma dependência: precisa
//! responder JSON fixo e contar requisições, nada além disso. Um servidor de
//! verdade aqui seria peso sem ganho.
//!
//! Nenhum teste desta crate toca a rede de verdade.
#![allow(dead_code)]

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

/// O que o servidor responde à n-ésima requisição.
#[derive(Clone, Debug)]
pub enum Resposta {
    /// HTTP 200 com o corpo dado.
    Ok(String),
    /// Status HTTP de erro (403 para excesso de requisições).
    Status(u16),
}

pub struct ServidorFalso {
    pub base_url: String,
    pub requisicoes: Arc<AtomicUsize>,
    pub consultas: Arc<Mutex<Vec<String>>>,
}

impl ServidorFalso {
    /// Sobe o servidor com um roteiro de respostas. Esgotado o roteiro, a
    /// última resposta se repete.
    pub fn novo(roteiro: Vec<Resposta>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let porta = listener.local_addr().unwrap().port();
        let requisicoes = Arc::new(AtomicUsize::new(0));
        let consultas = Arc::new(Mutex::new(Vec::new()));

        let r = Arc::clone(&requisicoes);
        let q = Arc::clone(&consultas);

        thread::spawn(move || {
            for fluxo in listener.incoming() {
                let Ok(fluxo) = fluxo else { break };
                let n = r.fetch_add(1, Ordering::SeqCst);
                let resposta = roteiro
                    .get(n)
                    .or_else(|| roteiro.last())
                    .cloned()
                    .unwrap_or(Resposta::Status(500));
                atender(fluxo, &resposta, &q);
            }
        });

        ServidorFalso {
            base_url: format!("http://127.0.0.1:{porta}"),
            requisicoes,
            consultas,
        }
    }

    pub fn chamadas(&self) -> usize {
        self.requisicoes.load(Ordering::SeqCst)
    }

    /// Query string da n-ésima requisição.
    pub fn consulta(&self, n: usize) -> String {
        self.consultas
            .lock()
            .unwrap()
            .get(n)
            .cloned()
            .unwrap_or_default()
    }
}

fn atender(mut fluxo: TcpStream, resposta: &Resposta, consultas: &Arc<Mutex<Vec<String>>>) {
    let mut leitor = BufReader::new(fluxo.try_clone().expect("clone"));
    let mut linha = String::new();
    let _ = leitor.read_line(&mut linha);

    if let Some(alvo) = linha.split_whitespace().nth(1) {
        consultas.lock().unwrap().push(alvo.to_string());
    }

    // Consome os cabeçalhos até a linha em branco.
    loop {
        let mut l = String::new();
        if leitor.read_line(&mut l).unwrap_or(0) == 0 || l == "\r\n" || l == "\n" {
            break;
        }
    }

    let saida = match resposta {
        Resposta::Ok(corpo) => format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
             X-Bapi-Limit: 600\r\nX-Bapi-Limit-Status: 599\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{}",
            corpo.len(),
            corpo
        ),
        Resposta::Status(s) => format!(
            "HTTP/1.1 {s} Erro\r\nContent-Type: application/json\r\n\
             Content-Length: 2\r\nConnection: close\r\n\r\n{{}}"
        ),
    };

    let _ = fluxo.write_all(saida.as_bytes());
    let _ = fluxo.flush();
}

/// Monta um corpo de resposta com as velas dadas, **do mais recente para o
/// mais antigo**, como a fonte faz.
pub fn corpo(velas: &[(i64, &str)]) -> String {
    let mut linhas: Vec<String> = velas
        .iter()
        .map(|(ms, preco)| {
            format!(r#"["{ms}","{preco}","{preco}","{preco}","{preco}","10","{preco}0"]"#)
        })
        .collect();
    linhas.reverse();
    format!(
        r#"{{"retCode":0,"retMsg":"OK","result":{{"symbol":"BTCUSDT","category":"spot","list":[{}]}},"time":0}}"#,
        linhas.join(",")
    )
}

/// Servidor que imita a semântica **real** da Bybit, verificada contra a API:
/// devolve as `limit` velas mais recentes dentro de `[start, end]` — com
/// `end` **inclusivo** — contando de trás para frente, e não as mais antigas
/// a partir de `start`.
///
/// Existe porque a diferença entre as duas leituras é invisível num roteiro
/// fixo e custa 441 minutos de histórico numa coleta real.
pub struct ServidorRealista {
    pub base_url: String,
    pub requisicoes: Arc<AtomicUsize>,
}

impl ServidorRealista {
    /// `base_ms` é o instante da primeira vela disponível; `total` quantas
    /// existem, de minuto em minuto.
    pub fn novo(base_ms: i64, total: i64) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let porta = listener.local_addr().unwrap().port();
        let requisicoes = Arc::new(AtomicUsize::new(0));
        let r = Arc::clone(&requisicoes);

        thread::spawn(move || {
            for fluxo in listener.incoming() {
                let Ok(mut fluxo) = fluxo else { break };
                r.fetch_add(1, Ordering::SeqCst);

                let mut leitor = BufReader::new(fluxo.try_clone().expect("clone"));
                let mut linha = String::new();
                let _ = leitor.read_line(&mut linha);
                let alvo = linha.split_whitespace().nth(1).unwrap_or("").to_string();
                loop {
                    let mut l = String::new();
                    if leitor.read_line(&mut l).unwrap_or(0) == 0 || l == "\r\n" || l == "\n" {
                        break;
                    }
                }

                let q = |nome: &str| -> i64 {
                    alvo.split(['?', '&'])
                        .find_map(|p| p.strip_prefix(&format!("{nome}=")))
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0)
                };
                let (start, end, limit) = (q("start"), q("end"), q("limit").max(1));

                let disponiveis: Vec<i64> = (0..total)
                    .map(|i| base_ms + i * 60_000)
                    .filter(|ms| *ms >= start && *ms <= end)
                    .collect();

                // Aqui está o ponto: as MAIS RECENTES da janela.
                let inicio = disponiveis.len().saturating_sub(limit as usize);
                let selecionadas: Vec<(i64, &str)> = disponiveis[inicio..]
                    .iter()
                    .map(|ms| (*ms, "63000"))
                    .collect();

                let body = corpo(&selecionadas);
                let saida = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                     Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = fluxo.write_all(saida.as_bytes());
                let _ = fluxo.flush();
            }
        });

        ServidorRealista {
            base_url: format!("http://127.0.0.1:{porta}"),
            requisicoes,
        }
    }

    pub fn chamadas(&self) -> usize {
        self.requisicoes.load(Ordering::SeqCst)
    }
}
