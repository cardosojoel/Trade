//! Cliente HTTP do endpoint público de histórico.

use crate::backoff::Quota;
use crate::errors::{http_status, ret_code};
use crate::parse::{parse_page, parse_response};
use chrono::{DateTime, Utc};
use std::time::Duration;
use trade_domain::{Candle, Interval, Symbol};
use trade_ports::MarketError;

/// Página devolvida pela fonte, já em ordem cronológica crescente.
#[derive(Debug)]
pub struct KlinePage {
    pub candles: Vec<Candle>,
    pub quota: Quota,
}

/// Máximo documentado por requisição.
pub const LIMITE_MAXIMO: u32 = 1000;

pub struct BybitClient {
    base_url: String,
    agent: ureq::Agent,
}

/// Código da granularidade no formato da Bybit.
///
/// A tradução vive aqui, e não no domínio: é conhecimento da corretora, e o
/// Princípio V manda que ele não vaze para fora do adaptador.
fn codigo(interval: Interval) -> &'static str {
    match interval {
        Interval::M1 => "1",
        Interval::M5 => "5",
        Interval::M15 => "15",
        Interval::H1 => "60",
        Interval::H4 => "240",
        Interval::D1 => "D",
    }
}

fn cabecalho_u32(resposta: &ureq::http::Response<ureq::Body>, nome: &str) -> Option<u32> {
    resposta
        .headers()
        .get(nome)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse().ok())
}

impl BybitClient {
    pub fn new() -> Self {
        Self::with_base_url("https://api.bybit.com")
    }

    /// Aponta para outra origem — usado pelos testes contra servidor local.
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(20)))
            .user_agent("trade-bot/0.1 (+https://github.com/cardosojoel/Trade)")
            .build()
            .into();

        BybitClient {
            base_url: base_url.into(),
            agent,
        }
    }

    /// Busca uma página a partir de `start`, inclusive.
    ///
    /// `category=spot` é enviado **explicitamente**. A documentação diz que o
    /// parâmetro assume `linear` quando omitido — e `linear` é perpétuo, outro
    /// mercado. Omitir traria dados que parecem certos e não são.
    pub fn klines(
        &self,
        symbol: &Symbol,
        interval: Interval,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        limit: u32,
    ) -> Result<KlinePage, MarketError> {
        let url = format!("{}/v5/market/kline", self.base_url);
        let limit = limit.clamp(1, LIMITE_MAXIMO);

        let resposta = self
            .agent
            .get(&url)
            .query("category", "spot")
            .query("symbol", symbol.as_str())
            .query("interval", codigo(interval))
            .query("start", start.timestamp_millis().to_string())
            .query("end", end.timestamp_millis().to_string())
            .query("limit", limit.to_string())
            .call();

        let mut resposta = match resposta {
            Ok(r) => r,
            Err(ureq::Error::StatusCode(s)) => return Err(http_status(s)),
            Err(e) => return Err(MarketError::Unavailable(e.to_string())),
        };

        let quota = Quota {
            limite: cabecalho_u32(&resposta, "x-bapi-limit"),
            restante: cabecalho_u32(&resposta, "x-bapi-limit-status"),
        };

        let corpo = resposta
            .body_mut()
            .read_to_string()
            .map_err(|e| MarketError::Unavailable(format!("falha ao ler o corpo: {e}")))?;

        let bruta = parse_response(&corpo)?;
        if bruta.ret_code != 0 {
            return Err(ret_code(bruta.ret_code, &bruta.ret_msg));
        }

        Ok(KlinePage {
            candles: parse_page(&bruta.list)?,
            quota,
        })
    }
}

impl Default for BybitClient {
    fn default() -> Self {
        Self::new()
    }
}
