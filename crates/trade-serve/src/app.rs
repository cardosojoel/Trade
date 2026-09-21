//! O que o servidor precisa para responder de verdade.
//!
//! Separado do socket de propósito: a [`Aplicacao`] decide **o quê**
//! responder e é verificável sem rede; o servidor decide **como** e não sabe
//! o que há nas rotas.

use crate::consultas::{self, ExigenciasPorta1};
use crate::erros::{Motivo, Recusa};
use crate::escrita::{self, IniciarExecucao, Quadro};
use crate::respostas;
use crate::servidor::{Pedido, Rota};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;
use trade_storage::open_runs;
use trade_storage::runs_repo::RunsRepository;

/// Tetos de partida das coleções (FR-025).
const TETO_SAIDAS: usize = 500;
const TETO_EVENTOS: usize = 100;

pub struct Aplicacao<'a> {
    pub runs_db: PathBuf,
    pub market_db: PathBuf,
    pub exigencias: ExigenciasPorta1,
    pub ligacao: &'a dyn IniciarExecucao,
    pub quadros: Mutex<BTreeMap<String, Quadro>>,
}

impl Aplicacao<'_> {
    /// Responde um pedido já autorizado.
    pub fn responder(&self, rota: Rota, p: &Pedido, corpo: &str) -> Result<Value, Recusa> {
        let repo = || {
            open_runs(&self.runs_db)
                .map(RunsRepository::new)
                // Indisponível, não inexistente: dizer "não existe" faria a
                // tela concluir que nunca houve execução.
                .map_err(|e| Recusa::nova(Motivo::RegistroIndisponivel, e.to_string()))
        };
        let falha = |e: trade_ports::StorageError| {
            Recusa::nova(Motivo::RegistroIndisponivel, e.to_string())
        };
        let id = || segmento(&p.caminho, 1).unwrap_or_default();
        let nao_achou = |o: Option<Value>| {
            o.ok_or_else(|| {
                Recusa::nova(
                    Motivo::ExecucaoDesconhecida,
                    format!("nenhuma execução com identificador `{}`", id()),
                )
            })
        };

        match rota {
            Rota::Execucoes => respostas::execucoes(&repo()?).map_err(falha),

            Rota::Execucao => {
                nao_achou(respostas::execucao(&repo()?, &id(), &self.exigencias).map_err(falha)?)
            }

            Rota::Diario => {
                let r = repo()?;
                let e =
                    nao_achou(respostas::execucao(&r, &id(), &self.exigencias).map_err(falha)?)?;
                let extrato = r.extrato(&id()).map_err(falha)?;
                // O limite vem da cerca **daquela execução**, e não da cerca
                // de hoje: um dia rompeu o limite que valia então.
                let limite = e["cerca"]["limits"]["max_daily_loss"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_default();
                Ok(consultas::por_dia_json(&consultas::por_dia(
                    &extrato, limite,
                )))
            }

            Rota::Episodios => {
                let r = repo()?;
                let extrato = r.extrato(&id()).map_err(falha)?;
                let eventos = r.linha_do_tempo(&id()).map_err(falha)?;
                let eps = consultas::episodios(&extrato, &eventos);
                Ok(consultas::episodios_json(&id(), &eps))
            }

            Rota::Saidas => {
                let r = repo()?;
                let entrada_ms: i64 = segmento(&p.caminho, 3)
                    .and_then(|s| s.parse().ok())
                    .ok_or_else(|| {
                        Recusa::nova(
                            Motivo::ExecucaoDesconhecida,
                            "o episódio é identificado pelo instante de entrada, em ms",
                        )
                    })?;
                let entrada =
                    chrono::DateTime::from_timestamp_millis(entrada_ms).ok_or_else(|| {
                        Recusa::nova(Motivo::ExecucaoDesconhecida, "instante inválido")
                    })?;
                let extrato = r.extrato(&id()).map_err(falha)?;
                Ok(consultas::saidas_json(
                    &id(),
                    entrada,
                    &extrato,
                    TETO_SAIDAS,
                ))
            }

            Rota::Cadeia => {
                let seq: u64 = segmento(&p.caminho, 3)
                    .and_then(|s| s.parse().ok())
                    .ok_or_else(|| Recusa::nova(Motivo::ExecucaoDesconhecida, "`seq` inválido"))?;
                let eventos = repo()?.linha_do_tempo(&id()).map_err(falha)?;
                consultas::cadeia(&eventos, seq).ok_or_else(|| {
                    Recusa::nova(
                        Motivo::ExecucaoDesconhecida,
                        format!("não há evento com seq {seq} nesta execução"),
                    )
                })
            }

            Rota::Comparar => {
                let q = parametros(&p.consulta);
                let (Some(a), Some(b)) = (q.get("a"), q.get("b")) else {
                    return Err(Recusa::nova(
                        Motivo::ExecucaoDesconhecida,
                        "a comparação precisa de duas execuções: ?a=…&b=…",
                    ));
                };
                nao_achou(respostas::comparar(&repo()?, a, b).map_err(falha)?)
            }

            Rota::Repetida => {
                let q = parametros(&p.consulta);
                respostas::repetida(
                    &repo()?,
                    &respostas::Pedido {
                        modo: q.get("modo").cloned().unwrap_or_default(),
                        simbolo: q.get("simbolo").cloned().unwrap_or_default(),
                        intervalo: q.get("intervalo").cloned().unwrap_or_default(),
                        de_ms: q.get("de_ms").and_then(|s| s.parse().ok()).unwrap_or(0),
                        ate_ms: q.get("ate_ms").and_then(|s| s.parse().ok()).unwrap_or(0),
                    },
                )
                .map_err(falha)
            }

            Rota::Datasets => Ok(consultas::datasets_json(&[])),

            Rota::Iniciar => {
                let v: Value = serde_json::from_str(corpo).map_err(|e| {
                    Recusa::nova(Motivo::ModoAusente, format!("corpo ilegível: {e}"))
                })?;
                let r = escrita::iniciar(self.ligacao, &v)?;
                if let (Some(t), Some(x)) = (r["trabalho"].as_str(), r["execucao"].as_str())
                    && let Ok(mut q) = self.quadros.lock()
                {
                    q.entry(t.to_string()).or_insert_with(|| Quadro::novo(t, x));
                }
                Ok(r)
            }

            Rota::Trabalho => {
                let t = segmento(&p.caminho, 1).unwrap_or_default();
                let q = self.quadros.lock().map_err(|_| {
                    Recusa::nova(Motivo::RegistroIndisponivel, "quadro indisponível")
                })?;
                q.get(&t)
                    .map(|x| x.estado_json(TETO_EVENTOS))
                    .ok_or_else(|| {
                        Recusa::nova(
                            Motivo::ExecucaoDesconhecida,
                            format!("nenhum trabalho com identificador `{t}`"),
                        )
                    })
            }
        }
    }
}

/// O n-ésimo segmento do caminho, contando de zero.
fn segmento(caminho: &str, n: usize) -> Option<String> {
    caminho
        .trim_matches('/')
        .split('/')
        .nth(n)
        .map(str::to_string)
}

/// Parâmetros de consulta. Sem decodificação de escape: os valores desta API
/// são identificadores e números, e inventar decodificação para o que não
/// aparece é complexidade sem caso de uso.
fn parametros(consulta: &str) -> BTreeMap<String, String> {
    consulta
        .split('&')
        .filter_map(|par| par.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}
