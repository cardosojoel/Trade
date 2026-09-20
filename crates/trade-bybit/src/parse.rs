//! Leitura da resposta do endpoint `/v5/market/kline`.
//!
//! Formato verificado na documentação oficial em 2026-09-20. Cada vela é um
//! vetor de **sete** elementos, nesta ordem:
//!
//! `[startTime, open, high, low, close, volume, turnover]`
//!
//! Todos vêm como **string**, inclusive os preços — o que é uma sorte: a
//! conversão para `Decimal` é direta e exata, sem passar por ponto flutuante
//! em momento algum.

use chrono::{DateTime, TimeZone, Utc};
use rust_decimal::Decimal;
use std::str::FromStr;
use trade_domain::{Candle, Money};
use trade_ports::MarketError;

/// Resposta bruta, antes de virar velas do domínio.
#[derive(Debug)]
pub struct RawResponse {
    pub ret_code: i64,
    pub ret_msg: String,
    pub list: Vec<Vec<String>>,
}

fn campo(v: &serde_json::Value, nome: &str) -> Result<String, MarketError> {
    v.get(nome)
        .and_then(|x| x.as_str())
        .map(str::to_string)
        .ok_or_else(|| MarketError::Source(format!("campo '{nome}' ausente na resposta")))
}

pub fn parse_response(corpo: &str) -> Result<RawResponse, MarketError> {
    let v: serde_json::Value = serde_json::from_str(corpo)
        .map_err(|e| MarketError::Source(format!("resposta não é JSON válido: {e}")))?;

    let ret_code = v
        .get("retCode")
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| MarketError::Source("campo 'retCode' ausente na resposta".into()))?;

    let ret_msg = campo(&v, "retMsg").unwrap_or_default();

    let list = v
        .get("result")
        .and_then(|r| r.get("list"))
        .and_then(serde_json::Value::as_array)
        .map(|linhas| {
            linhas
                .iter()
                .filter_map(|l| {
                    l.as_array().map(|c| {
                        c.iter()
                            .map(|x| x.as_str().unwrap_or_default().to_string())
                            .collect()
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(RawResponse {
        ret_code,
        ret_msg,
        list,
    })
}

fn decimal(nome: &str, raw: &str) -> Result<Money, MarketError> {
    Decimal::from_str(raw).map_err(|_| MarketError::InvalidCandle {
        at: nome.to_string(),
        cause: format!("'{raw}' não é um número decimal"),
    })
}

/// Converte uma linha de sete elementos em [`Candle`].
pub fn parse_candle(linha: &[String]) -> Result<Candle, MarketError> {
    if linha.len() < 7 {
        return Err(MarketError::InvalidCandle {
            at: "linha".into(),
            cause: format!("esperados 7 campos, vieram {}", linha.len()),
        });
    }

    let ms: i64 = linha[0].parse().map_err(|_| MarketError::InvalidCandle {
        at: linha[0].clone(),
        cause: "instante não é inteiro em milissegundos".into(),
    })?;

    let open_time: DateTime<Utc> =
        Utc.timestamp_millis_opt(ms)
            .single()
            .ok_or_else(|| MarketError::InvalidCandle {
                at: ms.to_string(),
                cause: "instante fora da faixa representável".into(),
            })?;

    let c = Candle {
        open_time,
        open: decimal("open", &linha[1])?,
        high: decimal("high", &linha[2])?,
        low: decimal("low", &linha[3])?,
        close: decimal("close", &linha[4])?,
        volume: decimal("volume", &linha[5])?,
        turnover: decimal("turnover", &linha[6])?,
    };

    c.validate().map_err(|e| MarketError::InvalidCandle {
        at: open_time.to_rfc3339(),
        cause: e.to_string(),
    })?;

    Ok(c)
}

/// Converte a página inteira, **invertendo a ordem**.
///
/// A Bybit documenta a lista como ordenada do mais recente para o mais antigo
/// ("sort in reverse by startTime"). O motor percorre em ordem cronológica
/// crescente, e um histórico gravado invertido seria consumido sem que nada
/// percebesse — produzindo um backtest sobre um mercado que rodou ao contrário.
pub fn parse_page(list: &[Vec<String>]) -> Result<Vec<Candle>, MarketError> {
    let mut velas: Vec<Candle> = list
        .iter()
        .map(|l| parse_candle(l))
        .collect::<Result<_, _>>()?;
    velas.reverse();
    Ok(velas)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::dec;

    const RESPOSTA: &str = r#"{
      "retCode": 0,
      "retMsg": "OK",
      "result": {
        "symbol": "BTCUSDT",
        "category": "spot",
        "list": [
          ["1767225720000","63500.10","63510.00","63495.00","63505.00","1.5","95258.0"],
          ["1767225660000","63450.00","63505.00","63440.00","63500.10","2.0","127000.0"],
          ["1767225600000","63400.00","63455.00","63390.00","63450.00","1.0","63420.0"]
        ]
      },
      "time": 1767225780000
    }"#;

    #[test]
    fn le_o_codigo_de_retorno_e_a_lista() {
        let r = parse_response(RESPOSTA).unwrap();
        assert_eq!(r.ret_code, 0);
        assert_eq!(r.list.len(), 3);
    }

    #[test]
    fn a_pagina_e_invertida_para_ordem_cronologica() {
        // A fonte devolve do mais recente para o mais antigo. Gravar assim
        // produziria um histórico que roda ao contrário, sem erro visível.
        let r = parse_response(RESPOSTA).unwrap();
        let velas = parse_page(&r.list).unwrap();
        assert!(velas.windows(2).all(|w| w[0].open_time < w[1].open_time));
        assert_eq!(velas[0].open, dec!(63400.00));
        assert_eq!(velas[2].open, dec!(63500.10));
    }

    #[test]
    fn os_precos_viram_decimal_sem_passar_por_float() {
        let r = parse_response(RESPOSTA).unwrap();
        let velas = parse_page(&r.list).unwrap();
        assert_eq!(velas[1].close, dec!(63500.10));
        assert_eq!(velas[1].turnover, dec!(127000.0));
    }

    #[test]
    fn linha_curta_e_recusada() {
        let curta = vec!["1767225600000".to_string(), "1".to_string()];
        assert!(matches!(
            parse_candle(&curta),
            Err(MarketError::InvalidCandle { .. })
        ));
    }

    #[test]
    fn vela_incoerente_e_recusada_na_fronteira() {
        // Mínima acima da máxima: dado corrompido não entra no banco.
        let ruim: Vec<String> = ["1767225600000", "100", "90", "110", "100", "1", "100"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(matches!(
            parse_candle(&ruim),
            Err(MarketError::InvalidCandle { .. })
        ));
    }

    #[test]
    fn resposta_de_erro_preserva_o_codigo() {
        let r =
            parse_response(r#"{"retCode":10006,"retMsg":"Too many visits!","result":{}}"#).unwrap();
        assert_eq!(r.ret_code, 10006);
        assert_eq!(r.ret_msg, "Too many visits!");
        assert!(r.list.is_empty());
    }

    #[test]
    fn corpo_invalido_falha_de_forma_explicita() {
        assert!(matches!(
            parse_response("não é json"),
            Err(MarketError::Source(_))
        ));
    }
}
