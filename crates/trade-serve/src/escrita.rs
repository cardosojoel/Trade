//! A única rota que escreve, e o progresso do que ela inicia.
//!
//! **Ela não emite ordem.** Inicia uma execução pela composição que o ponto
//! de ligação fornece — a mesma que o `trade backtest` usa (FR-001). O
//! servidor não sabe montar executor nenhum, e é por isso que esta crate não
//! declara a corretora, o risco nem a CLI.

use crate::erros::{Motivo, Recusa};
use serde_json::{Value, json};
use std::collections::VecDeque;

/// O que se pede ao iniciar.
///
/// Lista de **aceitação**: o que não está aqui não é lido. Um campo a mais na
/// requisição — uma chave de API colada por engano, por exemplo — não tem por
/// onde entrar nem por onde voltar (FR-020).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parametros {
    pub modo: String,
    pub simbolo: String,
    pub intervalo: String,
    pub de_ms: i64,
    pub ate_ms: i64,
    pub capital_inicial: String,
    pub estrategia: String,
    pub parametros: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrabalhoId {
    pub trabalho: String,
    pub execucao: String,
}

/// O ponto de ligação com a composição.
///
/// Existe para que o `FR-001` e o `SC-008` valham juntos: o servidor inicia
/// execução **pela mesma composição** que a CLI usa, e mesmo assim não
/// declara a CLI. Quem implementa é o `trade-cli`; o servidor chama e não sabe
/// o que há do outro lado.
pub trait IniciarExecucao {
    /// Começa e devolve imediatamente. Quem implementa **não** espera a
    /// execução terminar: o progresso sai por consulta (FR-017, FR-018).
    fn iniciar(&self, p: &Parametros) -> Result<TrabalhoId, String>;
}

/// `POST /runs`.
///
/// A ordem das verificações importa: o modo é conferido **antes** de qualquer
/// outra coisa, e a recusa acontece sem que o ponto de ligação seja chamado.
pub fn iniciar(l: &dyn IniciarExecucao, corpo: &Value) -> Result<Value, Recusa> {
    let modo = corpo.get("modo").and_then(Value::as_str).ok_or_else(|| {
        Recusa::nova(
            Motivo::ModoAusente,
            "o modo de execução é obrigatório e não tem padrão. \
             A ausência de configuração não pode resultar em algo que toque dinheiro.",
        )
    })?;

    // `live` tem recusa própria, com código e status distintos da ausência:
    // são situações diferentes e a tela precisa distingui-las (SC-004).
    match modo {
        "backtest" | "paper" => {}
        "live" => {
            return Err(Recusa::nova(
                Motivo::ModoRecusado,
                "`live` não é iniciável pelo servidor. A promoção para capital real é \
                 ato humano registrado, e não cabe numa requisição.",
            ));
        }
        outro => {
            return Err(Recusa::nova(
                Motivo::ModoAusente,
                format!("modo '{outro}' desconhecido — disponíveis: backtest, paper"),
            ));
        }
    }

    let p = Parametros {
        modo: modo.to_string(),
        simbolo: texto(corpo, "simbolo"),
        intervalo: texto(corpo, "intervalo"),
        de_ms: corpo.get("de_ms").and_then(Value::as_i64).unwrap_or(0),
        ate_ms: corpo.get("ate_ms").and_then(Value::as_i64).unwrap_or(0),
        capital_inicial: texto(corpo, "capital_inicial"),
        estrategia: texto(corpo, "estrategia"),
        parametros: corpo
            .get("parametros")
            .and_then(Value::as_object)
            .map(|o| {
                o.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
                    .collect()
            })
            .unwrap_or_default(),
    };

    let id = l
        .iniciar(&p)
        .map_err(|e| Recusa::nova(Motivo::HistoricoInsuficiente, e))?;

    Ok(json!({
        "trabalho": id.trabalho,
        "execucao": id.execucao,
        "estado": "em_andamento",
    }))
}

fn texto(v: &Value, campo: &str) -> String {
    v.get(campo)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// O estado de um trabalho em andamento.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Estado {
    EmAndamento,
    Concluido,
    Falhou,
}

/// O quadro de um trabalho: o que já aconteceu e em que pé está.
///
/// Consultado por `GET /jobs/{id}`. Não há conexão mantida aberta empurrando
/// evento (FR-018) — quem acompanha pergunta de novo.
#[derive(Debug, Clone)]
pub struct Quadro {
    trabalho: String,
    execucao: String,
    estado: Estado,
    motivo: Option<String>,
    /// Os mais recentes. A fila tem fundo porque um backtest de doze meses
    /// produz centenas de milhares de eventos, e devolvê-los todos a cada
    /// consulta custaria mais que a execução.
    eventos: VecDeque<String>,
    total: usize,
}

/// Quantos eventos ficam em memória. Acima disso, o mais antigo sai.
const GUARDADOS: usize = 200;

impl Quadro {
    pub fn novo(trabalho: impl Into<String>, execucao: impl Into<String>) -> Self {
        Quadro {
            trabalho: trabalho.into(),
            execucao: execucao.into(),
            estado: Estado::EmAndamento,
            motivo: None,
            eventos: VecDeque::new(),
            total: 0,
        }
    }

    pub fn registrar(&mut self, evento: String) {
        self.total += 1;
        self.eventos.push_back(evento);
        while self.eventos.len() > GUARDADOS {
            self.eventos.pop_front();
        }
    }

    pub fn concluir(&mut self) {
        self.estado = Estado::Concluido;
    }

    pub fn falhar(&mut self, motivo: impl Into<String>) {
        self.estado = Estado::Falhou;
        self.motivo = Some(motivo.into());
    }

    pub fn estado_json(&self, teto: usize) -> Value {
        let recentes: Vec<String> = self
            .eventos
            .iter()
            .rev()
            .take(teto)
            .rev()
            .map(|e| redigir(e))
            .collect();
        json!({
            "trabalho": self.trabalho,
            "execucao": self.execucao,
            "estado": match self.estado {
                Estado::EmAndamento => "em_andamento",
                Estado::Concluido => "concluido",
                Estado::Falhou => "falhou",
            },
            "motivo": self.motivo,
            "eventos": recentes,
            "teto": teto,
            "cortou": self.total > teto,
            "total_de_eventos": self.total,
        })
    }
}

/// Apaga o que parece segredo antes de o evento sair.
///
/// O Princípio VI não admite credencial em log nem em resposta, e um evento
/// de progresso é as duas coisas. A verificação é sobre o **nome** que
/// antecede o valor: procurar o valor em si exigiria conhecê-lo.
fn redigir(evento: &str) -> String {
    const SUSPEITOS: &[&str] = &[
        "key",
        "secret",
        "token",
        "password",
        "passphrase",
        "signature",
        "credential",
        "authorization",
    ];
    match evento.split_once('=') {
        Some((nome, _)) => {
            let n = nome.to_ascii_lowercase();
            if SUSPEITOS.iter().any(|s| n.contains(s)) {
                format!("{nome}=<redigido>")
            } else {
                evento.to_string()
            }
        }
        None => evento.to_string(),
    }
}
