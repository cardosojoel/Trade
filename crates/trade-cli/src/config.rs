//! Carga de `limits.toml` e `fees.toml`.
//!
//! Os limiares são **configuração externa, nunca constante embutida** — é
//! exigência da constitution v1.2.0, que os registra como valores de partida
//! provisórios. Alterar um limiar é trocar uma linha de arquivo, não
//! recompilar.
//!
//! Todo valor monetário é declarado como **string** no TOML. Um número em TOML
//! é lido como inteiro ou float, e passar por float desfaria a exatidão antes
//! do primeiro cálculo. Declarar como string e converter para `Decimal` mantém
//! o valor exato desde o arquivo.

use rust_decimal::Decimal;
use serde::Deserialize;
use std::path::Path;
use std::str::FromStr;
use trade_domain::{Money, RiskLimits};

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("não foi possível ler {path}: {cause}")]
    Read { path: String, cause: String },
    #[error(
        "erro em {path}: {cause}\n\
         Valores monetários devem ser declarados como string, entre aspas — \
         por exemplo max_daily_loss = \"200.00\". Número em TOML passa por \
         ponto flutuante e perde exatidão."
    )]
    Parse { path: String, cause: String },
    #[error("campo `{campo}` em {path} não é um número decimal válido: {valor}")]
    NotDecimal {
        path: String,
        campo: &'static str,
        valor: String,
    },
    #[error("campo `{campo}` em {path} não pode ser negativo: {valor}")]
    Negative {
        path: String,
        campo: &'static str,
        valor: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LimitsFile {
    max_daily_loss: String,
    max_position_size: String,
    max_total_exposure: String,
    max_orders_per_window: u32,
    window_minutes: i64,
    max_transient_retries: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FeesFile {
    taker_fee_rate: String,
    slippage_rate: String,
}

/// Custo de transação configurado (FR-027).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fees {
    pub taker_fee_rate: Decimal,
    pub slippage_rate: Decimal,
}

fn decimal(path: &Path, campo: &'static str, raw: &str) -> Result<Money, ConfigError> {
    let v = Decimal::from_str(raw.trim()).map_err(|_| ConfigError::NotDecimal {
        path: path.display().to_string(),
        campo,
        valor: raw.to_string(),
    })?;
    if v < Decimal::ZERO {
        return Err(ConfigError::Negative {
            path: path.display().to_string(),
            campo,
            valor: raw.to_string(),
        });
    }
    Ok(v)
}

fn read(path: &Path) -> Result<String, ConfigError> {
    std::fs::read_to_string(path).map_err(|e| ConfigError::Read {
        path: path.display().to_string(),
        cause: e.to_string(),
    })
}

/// Carrega a cerca.
pub fn load_limits(path: impl AsRef<Path>) -> Result<RiskLimits, ConfigError> {
    let path = path.as_ref();
    let raw = read(path)?;
    let f: LimitsFile = toml::from_str(&raw).map_err(|e| ConfigError::Parse {
        path: path.display().to_string(),
        cause: e.message().to_string(),
    })?;

    Ok(RiskLimits {
        max_daily_loss: decimal(path, "max_daily_loss", &f.max_daily_loss)?,
        max_position_size: decimal(path, "max_position_size", &f.max_position_size)?,
        max_total_exposure: decimal(path, "max_total_exposure", &f.max_total_exposure)?,
        max_orders_per_window: f.max_orders_per_window,
        window_minutes: f.window_minutes,
        max_transient_retries: f.max_transient_retries,
    })
}

/// Carrega o custo de transação.
pub fn load_fees(path: impl AsRef<Path>) -> Result<Fees, ConfigError> {
    let path = path.as_ref();
    let raw = read(path)?;
    let f: FeesFile = toml::from_str(&raw).map_err(|e| ConfigError::Parse {
        path: path.display().to_string(),
        cause: e.message().to_string(),
    })?;

    Ok(Fees {
        taker_fee_rate: decimal(path, "taker_fee_rate", &f.taker_fee_rate)?,
        slippage_rate: decimal(path, "slippage_rate", &f.slippage_rate)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::dec;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn arquivo(conteudo: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(conteudo.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    const LIMITES_VALIDOS: &str = r#"
max_daily_loss        = "200.00"
max_position_size     = "1000.00"
max_total_exposure    = "2000.00"
max_orders_per_window = 10
window_minutes        = 60
max_transient_retries = 5
"#;

    #[test]
    fn carrega_limites_com_valores_exatos() {
        let f = arquivo(LIMITES_VALIDOS);
        let l = load_limits(f.path()).unwrap();
        assert_eq!(l.max_daily_loss, dec!(200.00));
        assert_eq!(l.max_position_size, dec!(1000.00));
        assert_eq!(l.max_total_exposure, dec!(2000.00));
        assert_eq!(l.max_orders_per_window, 10);
        assert_eq!(l.window_minutes, 60);
        assert_eq!(l.max_transient_retries, 5);
    }

    #[test]
    fn taxa_com_muitas_casas_nao_perde_exatidao() {
        let f = arquivo("taker_fee_rate = \"0.00075\"\nslippage_rate = \"0.000123456789\"\n");
        let fees = load_fees(f.path()).unwrap();
        assert_eq!(fees.taker_fee_rate, dec!(0.00075));
        assert_eq!(fees.slippage_rate, dec!(0.000123456789));
    }

    #[test]
    fn valor_numerico_e_recusado_com_explicacao() {
        // Este é o ponto: 200.00 sem aspas viraria float e perderia exatidão.
        let f = arquivo(&LIMITES_VALIDOS.replace("\"200.00\"", "200.00"));
        let err = load_limits(f.path()).unwrap_err();
        assert!(matches!(err, ConfigError::Parse { .. }));
        let msg = err.to_string();
        assert!(
            msg.contains("string"),
            "mensagem não explica o porquê: {msg}"
        );
        assert!(
            msg.contains("ponto flutuante"),
            "mensagem não explica o porquê: {msg}"
        );
    }

    #[test]
    fn valor_nao_decimal_e_recusado_apontando_o_campo() {
        let f = arquivo(&LIMITES_VALIDOS.replace("\"200.00\"", "\"duzentos\""));
        let err = load_limits(f.path()).unwrap_err();
        assert!(matches!(
            err,
            ConfigError::NotDecimal {
                campo: "max_daily_loss",
                ..
            }
        ));
    }

    #[test]
    fn limite_negativo_e_recusado() {
        let f = arquivo(&LIMITES_VALIDOS.replace("\"1000.00\"", "\"-1000.00\""));
        let err = load_limits(f.path()).unwrap_err();
        assert!(matches!(
            err,
            ConfigError::Negative {
                campo: "max_position_size",
                ..
            }
        ));
    }

    #[test]
    fn campo_ausente_e_recusado() {
        let f = arquivo("max_daily_loss = \"200.00\"\n");
        assert!(matches!(
            load_limits(f.path()).unwrap_err(),
            ConfigError::Parse { .. }
        ));
    }

    #[test]
    fn campo_desconhecido_e_recusado() {
        // Um campo com nome errado seria silenciosamente ignorado sem isto, e
        // o limite que o usuário pensou ter configurado não valeria nada.
        let f = arquivo(&format!("{LIMITES_VALIDOS}max_posicao = \"5\"\n"));
        assert!(matches!(
            load_limits(f.path()).unwrap_err(),
            ConfigError::Parse { .. }
        ));
    }

    #[test]
    fn arquivo_inexistente_e_recusado() {
        assert!(matches!(
            load_limits("/nao/existe/limits.toml").unwrap_err(),
            ConfigError::Read { .. }
        ));
    }
}
