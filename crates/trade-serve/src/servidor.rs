//! O socket e o despacho (FR-003).
//!
//! Escuta **exclusivamente** em `127.0.0.1`. O endereço não vem de
//! configuração: é construído a partir de [`Ipv4Addr::LOCALHOST`], de modo que
//! não existe caminho em que um texto de fora vire o que se escuta. A porta é
//! explícita e sem padrão — um padrão seria um número que alguém acabaria
//! descobrindo estar aberto.

use crate::erros::{Motivo, Recusa};
use crate::guarda::Guarda;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use tiny_http::{Header, Response, Server};

pub struct Servidor {
    http: Server,
    guarda: Guarda,
}

/// O que o despacho precisa saber de uma requisição.
///
/// Extraído antes de qualquer trabalho, para que a decisão de recusar não
/// dependa de ler corpo nenhum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pedido {
    pub metodo: String,
    pub caminho: String,
    pub consulta: String,
    pub token: Option<String>,
    pub origem: Option<String>,
}

impl Servidor {
    /// Sobe o servidor na porta dada, em `127.0.0.1`.
    ///
    /// Porta `0` pede uma livre ao sistema, que é como os testes sobem sem
    /// disputar número com ninguém.
    pub fn subir(porta: u16) -> std::io::Result<Self> {
        let endereco = SocketAddrV4::new(Ipv4Addr::LOCALHOST, porta);
        let http = Server::http(endereco).map_err(std::io::Error::other)?;
        Ok(Servidor {
            http,
            guarda: Guarda::nova(),
        })
    }

    /// O endereço em que de fato ligou.
    ///
    /// Lido do sistema, e não repetido do que pedimos: é o que permite ao
    /// teste afirmar que ligou em `127.0.0.1` e não em `0.0.0.0` (SC-007).
    pub fn endereco(&self) -> Option<SocketAddr> {
        self.http.server_addr().to_ip()
    }

    pub fn guarda(&self) -> &Guarda {
        &self.guarda
    }

    /// Decide o que fazer com um pedido, **sem** tocar no corpo dele.
    ///
    /// Separado do laço de atendimento de propósito: assim toda recusa é
    /// verificável sem subir socket, e o que sobe socket é um laço curto que
    /// não decide nada.
    pub fn despachar(&self, p: &Pedido) -> Result<Rota, Recusa> {
        let rota = Rota::de(&p.metodo, &p.caminho).ok_or_else(|| {
            Recusa::nova(
                Motivo::ExecucaoDesconhecida,
                format!("nada responde a {} {}", p.metodo, p.caminho),
            )
        })?;

        let autorizado = if rota.escreve() {
            self.guarda
                .autorizar_escrita(p.token.as_deref(), p.origem.as_deref())
        } else {
            self.guarda.autorizar_leitura(p.origem.as_deref())
        };

        // A mensagem descreve o motivo e **nunca** ecoa o que veio: ecoar o
        // token numa recusa o entregaria a quem não o tem (FR-020).
        autorizado.map_err(|m| Recusa::nova(m, explicar(m)))?;
        Ok(rota)
    }

    /// Atende até o servidor ser derrubado. Por ora só responde as recusas —
    /// as rotas de dado entram nas fatias seguintes.
    pub fn atender(&self) {
        for req in self.http.incoming_requests() {
            let pedido = ler(&req);
            let (status, corpo) = match self.despachar(&pedido) {
                Ok(_) => (
                    501,
                    Recusa::nova(
                        Motivo::RegistroIndisponivel,
                        "rota reconhecida, ainda sem implementação",
                    )
                    .corpo(),
                ),
                Err(r) => (r.status(), r.corpo()),
            };
            let resposta = Response::from_string(corpo.to_string())
                .with_status_code(status)
                .with_header(json_header());
            let _ = req.respond(resposta);
        }
    }
}

/// As onze rotas da spec. Enum exaustivo: uma rota nova quebra a compilação
/// em todo lugar que trate rotas, que é o comportamento desejado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rota {
    Execucoes,
    Execucao,
    Diario,
    Episodios,
    Saidas,
    Cadeia,
    Comparar,
    Datasets,
    Repetida,
    /// A **única** rota de escrita.
    Iniciar,
    Trabalho,
}

impl Rota {
    /// Se escreve. Só uma escreve, e é por isso que a pergunta cabe num
    /// `matches!` — a superfície de escrita é uma (FR-005).
    pub const fn escreve(self) -> bool {
        matches!(self, Rota::Iniciar)
    }

    /// Reconhece o caminho. Devolve `None` para o que não existe, e também
    /// para o método errado numa rota que existe.
    pub fn de(metodo: &str, caminho: &str) -> Option<Rota> {
        let partes: Vec<&str> = caminho.trim_matches('/').split('/').collect();
        match (metodo, partes.as_slice()) {
            ("GET", ["runs"]) => Some(Rota::Execucoes),
            ("POST", ["runs"]) => Some(Rota::Iniciar),
            ("GET", ["runs", "compare"]) => Some(Rota::Comparar),
            ("GET", ["runs", "match"]) => Some(Rota::Repetida),
            ("GET", ["runs", _]) => Some(Rota::Execucao),
            ("GET", ["runs", _, "daily"]) => Some(Rota::Diario),
            ("GET", ["runs", _, "episodes"]) => Some(Rota::Episodios),
            ("GET", ["runs", _, "episodes", _, "saidas"]) => Some(Rota::Saidas),
            ("GET", ["runs", _, "chain", _]) => Some(Rota::Cadeia),
            ("GET", ["datasets"]) => Some(Rota::Datasets),
            ("GET", ["jobs", _]) => Some(Rota::Trabalho),
            _ => None,
        }
    }
}

fn explicar(m: Motivo) -> &'static str {
    match m {
        Motivo::TokenAusente => {
            "esta rota escreve e exige o token local, no cabeçalho `X-Trade-Token`. \
             Ele é impresso uma vez no terminal que subiu o servidor."
        }
        Motivo::TokenInvalido => {
            "o token não confere. Reiniciar o servidor sorteia outro, e o anterior \
             deixa de valer — o processo é o prazo."
        }
        Motivo::OrigemRecusada => {
            "a requisição declara origem de outro lugar. O loopback não basta: \
             qualquer página aberta no navegador alcança 127.0.0.1."
        }
        Motivo::ModoAusente => "o modo de execução é obrigatório e não tem padrão.",
        Motivo::ModoRecusado => {
            "`live` não é iniciável pelo servidor. A promoção para capital real é \
             ato humano registrado."
        }
        Motivo::HistoricoInsuficiente => "falta vela no período pedido.",
        Motivo::ExecucaoDesconhecida => "não há o que responder aqui.",
        Motivo::RegistroIndisponivel => "o registro não pôde ser lido.",
    }
}

fn json_header() -> Header {
    Header::from_bytes(
        &b"Content-Type"[..],
        &b"application/json; charset=utf-8"[..],
    )
    .expect("cabeçalho constante")
}

fn ler(req: &tiny_http::Request) -> Pedido {
    let bruto = req.url().to_string();
    let (caminho, consulta) = match bruto.split_once('?') {
        Some((c, q)) => (c.to_string(), q.to_string()),
        None => (bruto, String::new()),
    };
    fn cabecalho(req: &tiny_http::Request, nome: &'static str) -> Option<String> {
        req.headers()
            .iter()
            .find(|h| h.field.equiv(nome))
            .map(|h| h.value.as_str().to_string())
    }
    Pedido {
        metodo: req.method().as_str().to_string(),
        caminho,
        consulta,
        token: cabecalho(req, "X-Trade-Token"),
        // `Referer` serve de reserva: nem todo cliente manda `Origin`, e o
        // que se procura é qualquer declaração de que a requisição veio de
        // uma página.
        origem: cabecalho(req, "Origin").or_else(|| cabecalho(req, "Referer")),
    }
}
