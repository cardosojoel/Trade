//! Cliente HTTP autenticado contra a Bybit.
//!
//! Vive nesta crate e não na do paper porque é aqui que o HTTP mora — o
//! `tests/architecture.rs` cobra que estratégia, risco e backtest não alcancem
//! rede, e concentrar o cliente num lugar é o que torna essa trava
//! verificável.
//!
//! O segredo entra apenas na assinatura, e a assinatura vai no cabeçalho.
//! Nenhum caminho deste módulo imprime, registra ou devolve o segredo.

use crate::auth::{Cabecalhos, RECV_WINDOW_MS, dentro_da_janela};
use crate::credencial::Credencial;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HttpError {
    #[error("falha transitória: {0}")]
    Transitoria(String),
    #[error("falha permanente: {0}")]
    Permanente(String),
    #[error("resultado desconhecido: {0}")]
    Desconhecido(String),
}

pub struct ClienteAutenticado {
    credencial: Credencial,
    agent: ureq::Agent,
}

/// `Debug` manual: o derivado imprimiria a credencial inteira.
impl std::fmt::Debug for ClienteAutenticado {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClienteAutenticado")
            .field("ambiente", &self.credencial.ambiente)
            .field("chave", &"(redigida)")
            .finish()
    }
}

fn agora_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl ClienteAutenticado {
    pub fn novo(credencial: Credencial) -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(20)))
            .user_agent("trade-bot/0.1 (+https://github.com/cardosojoel/Trade)")
            .build()
            .into();
        Self { credencial, agent }
    }

    pub fn base_url(&self) -> &'static str {
        self.credencial.ambiente.base_url()
    }

    /// GET autenticado. `query` já vem montada, porque é ela que é assinada.
    pub fn get(&self, caminho: &str, query: &str) -> Result<String, HttpError> {
        let ts = agora_ms();
        if ts == 0 {
            return Err(HttpError::Permanente(
                "relógio do sistema antes da época Unix".into(),
            ));
        }
        let h = Cabecalhos::montar(&self.credencial, ts, query);

        let url = if query.is_empty() {
            format!("{}{caminho}", self.base_url())
        } else {
            format!("{}{caminho}?{query}", self.base_url())
        };

        let mut req = self.agent.get(&url);
        for (nome, valor) in h.pares() {
            req = req.header(nome, valor);
        }
        Self::ler(req.call())
    }

    /// POST autenticado. O corpo enviado é byte a byte o corpo assinado.
    pub fn post(&self, caminho: &str, corpo: &str) -> Result<String, HttpError> {
        let ts = agora_ms();
        let h = Cabecalhos::montar(&self.credencial, ts, corpo);
        let url = format!("{}{caminho}", self.base_url());

        let mut req = self
            .agent
            .post(&url)
            .header("Content-Type", "application/json");
        for (nome, valor) in h.pares() {
            req = req.header(nome, valor);
        }
        Self::ler(req.send(corpo))
    }

    fn ler(r: Result<ureq::http::Response<ureq::Body>, ureq::Error>) -> Result<String, HttpError> {
        let mut resposta = match r {
            Ok(r) => r,
            Err(ureq::Error::StatusCode(s)) if s == 429 || s >= 500 => {
                return Err(HttpError::Transitoria(format!("HTTP {s}")));
            }
            Err(ureq::Error::StatusCode(s)) => {
                return Err(HttpError::Permanente(format!("HTTP {s}")));
            }
            Err(ureq::Error::Timeout(_)) => {
                // A requisição saiu e a resposta não voltou. Não é transitória:
                // a ordem pode ter sido criada, e repetir cegamente duplica.
                return Err(HttpError::Desconhecido("tempo esgotado".into()));
            }
            Err(e) => return Err(HttpError::Transitoria(e.to_string())),
        };

        resposta
            .body_mut()
            .read_to_string()
            .map_err(|e| HttpError::Desconhecido(format!("resposta ilegível: {e}")))
    }

    /// Confere o relógio local contra o do servidor, antes de assinar algo.
    ///
    /// Endpoint público, sem assinatura — de propósito: se o relógio estiver
    /// fora da janela, uma requisição assinada seria recusada com mensagem
    /// genérica, e a causa levaria horas para aparecer.
    pub fn relogio_esta_sincronizado(&self) -> Result<bool, HttpError> {
        let url = format!("{}/v5/market/time", self.base_url());
        let mut r = self
            .agent
            .get(&url)
            .call()
            .map_err(|e| HttpError::Transitoria(e.to_string()))?;
        let corpo = r
            .body_mut()
            .read_to_string()
            .map_err(|e| HttpError::Transitoria(e.to_string()))?;

        let v: serde_json::Value = serde_json::from_str(&corpo)
            .map_err(|e| HttpError::Permanente(format!("resposta ilegível: {e}")))?;
        let nano = v["result"]["timeNano"]
            .as_str()
            .and_then(|s| s.parse::<u128>().ok())
            .ok_or_else(|| HttpError::Permanente("resposta sem timeNano".into()))?;

        Ok(dentro_da_janela(
            agora_ms(),
            (nano / 1_000_000) as u64,
            RECV_WINDOW_MS,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credencial::{Ambiente, Segredo};

    fn cliente() -> ClienteAutenticado {
        ClienteAutenticado::novo(Credencial {
            chave: "chave".into(),
            segredo: Segredo::from("segredo"),
            ambiente: Ambiente::Testnet,
        })
    }

    #[test]
    fn aponta_para_a_testnet() {
        assert_eq!(cliente().base_url(), "https://api-testnet.bybit.com");
    }

    #[test]
    fn o_debug_do_cliente_nao_carrega_a_credencial() {
        let d = format!("{:?}", cliente());
        assert!(!d.contains("segredo"), "segredo vazou: {d}");
        assert!(!d.contains("chave\""), "chave vazou: {d}");
        assert!(d.contains("redigida"));
    }
}
