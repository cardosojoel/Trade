//! O que a corretora diz que se tem (T041).
//!
//! A reconciliação compara o local com o reportado, e até aqui só o local
//! existia: `comparar` recebia a quantidade remota como parâmetro e nada a
//! produzia. Este módulo é o outro lado da comparação.
//!
//! O saldo vem como **string** e vira `Decimal` sem passar por ponto
//! flutuante. Ler "0.00061439" como `f64` e comparar com a posição local
//! produziria divergência onde não há — e divergência é falha de integridade,
//! que para o robô e chama um humano.

use crate::transporte::{Transporte, TransporteError};
use rust_decimal::Decimal;
use serde::Deserialize;
use std::str::FromStr;
use trade_domain::Qty;

const CAMINHO: &str = "/v5/account/wallet-balance";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ContaError {
    /// Não foi possível saber. Vira `Veredito::Desconhecido`, que bloqueia
    /// entrada nova sem afirmar que houve divergência.
    #[error("saldo não verificável: {0}")]
    NaoVerificavel(String),
}

#[derive(Deserialize)]
struct Resposta {
    #[serde(rename = "retCode")]
    ret_code: i64,
    #[serde(rename = "retMsg", default)]
    ret_msg: String,
    result: Option<Resultado>,
}

#[derive(Deserialize)]
struct Resultado {
    #[serde(default)]
    list: Vec<Carteira>,
}

#[derive(Deserialize)]
struct Carteira {
    #[serde(default)]
    coin: Vec<Moeda>,
}

#[derive(Deserialize)]
struct Moeda {
    coin: String,
    #[serde(rename = "walletBalance", default)]
    saldo: String,
}

/// Quanto da moeda a corretora diz que a conta detém.
///
/// Moeda ausente da carteira é **zero**, não erro: quem nunca comprou BTC não
/// tem a linha de BTC, e isso é informação, não falha.
pub fn saldo_de(t: &impl Transporte, moeda: &str) -> Result<Qty, ContaError> {
    let bruto = t
        .get(CAMINHO, "accountType=UNIFIED")
        .map_err(|e| ContaError::NaoVerificavel(descrever(&e)))?;

    let r: Resposta = serde_json::from_str(&bruto)
        .map_err(|e| ContaError::NaoVerificavel(format!("resposta ilegível: {e}")))?;

    if r.ret_code != 0 {
        return Err(ContaError::NaoVerificavel(format!(
            "retCode {} — {}",
            r.ret_code, r.ret_msg
        )));
    }

    let resultado = r
        .result
        .ok_or_else(|| ContaError::NaoVerificavel("resposta sem result".into()))?;

    for carteira in &resultado.list {
        for m in &carteira.coin {
            if m.coin.eq_ignore_ascii_case(moeda) {
                if m.saldo.trim().is_empty() {
                    return Ok(Decimal::ZERO);
                }
                return Decimal::from_str(m.saldo.trim()).map_err(|e| {
                    ContaError::NaoVerificavel(format!("saldo '{}' ilegível: {e}", m.saldo))
                });
            }
        }
    }
    Ok(Decimal::ZERO)
}

fn descrever(e: &TransporteError) -> String {
    e.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transporte::testing::TransporteFalso;
    use rust_decimal::dec;

    fn corpo(moedas: &str) -> String {
        format!(r#"{{"retCode":0,"retMsg":"OK","result":{{"list":[{{"coin":[{moedas}]}}]}}}}"#)
    }

    fn moeda(nome: &str, saldo: &str) -> String {
        format!(r#"{{"coin":"{nome}","walletBalance":"{saldo}"}}"#)
    }

    #[test]
    fn le_o_saldo_da_moeda_pedida() {
        let t = TransporteFalso::com(vec![Ok(corpo(&format!(
            "{},{}",
            moeda("USDT", "1000.5"),
            moeda("BTC", "0.00061439")
        )))]);
        assert_eq!(saldo_de(&t, "BTC").unwrap(), dec!(0.00061439));
    }

    #[test]
    fn nenhum_digito_se_perde_no_caminho() {
        // O saldo vem como string e fica Decimal. Passar por ponto flutuante
        // inventaria divergência — e divergência para o robô.
        let t = TransporteFalso::com(vec![Ok(corpo(&moeda("BTC", "0.123456789012345678")))]);
        assert_eq!(
            saldo_de(&t, "BTC").unwrap().to_string(),
            "0.123456789012345678"
        );
    }

    #[test]
    fn moeda_ausente_e_zero_e_nao_erro() {
        // Quem nunca comprou BTC não tem a linha de BTC. Isso é informação.
        let t = TransporteFalso::com(vec![Ok(corpo(&moeda("USDT", "1000")))]);
        assert_eq!(saldo_de(&t, "BTC").unwrap(), Decimal::ZERO);
    }

    #[test]
    fn o_nome_da_moeda_nao_depende_de_caixa() {
        let t = TransporteFalso::com(vec![Ok(corpo(&moeda("btc", "1.5")))]);
        assert_eq!(saldo_de(&t, "BTC").unwrap(), dec!(1.5));
    }

    #[test]
    fn falha_do_transporte_vira_nao_verificavel() {
        // Nunca zero: "não sei" e "não tem" decidem coisas diferentes.
        let t = TransporteFalso::com(vec![Err(TransporteError::Transitoria("rede".into()))]);
        assert!(matches!(
            saldo_de(&t, "BTC"),
            Err(ContaError::NaoVerificavel(_))
        ));
    }

    #[test]
    fn ret_code_de_erro_vira_nao_verificavel() {
        let t = TransporteFalso::com(vec![Ok(
            r#"{"retCode":10003,"retMsg":"chave inválida"}"#.to_string()
        )]);
        let e = saldo_de(&t, "BTC").unwrap_err();
        assert!(e.to_string().contains("10003"), "veio {e}");
    }
}
