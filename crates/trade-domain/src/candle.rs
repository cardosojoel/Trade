//! Vela histórica e sua validação.

use crate::{Money, Qty};
use chrono::{DateTime, Utc};

/// Um instante do mercado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candle {
    /// Início do intervalo. Junto com símbolo e granularidade, identifica a vela.
    pub open_time: DateTime<Utc>,
    pub open: Money,
    pub high: Money,
    pub low: Money,
    pub close: Money,
    pub volume: Qty,
    pub turnover: Money,
}

/// Motivo pelo qual uma vela é inválida.
///
/// Uma vela que viole as relações entre máxima e mínima indica dado corrompido.
/// Alimentar o motor com ela produz P&L sem significado, e por isso a validação
/// acontece na fronteira de leitura, não no meio do cálculo.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CandleError {
    #[error("preço não positivo em `{campo}`: {valor}")]
    NonPositivePrice { campo: &'static str, valor: String },
    #[error("valor negativo em `{campo}`: {valor}")]
    NegativeAmount { campo: &'static str, valor: String },
    #[error("mínima {low} acima da máxima {high}")]
    LowAboveHigh { low: String, high: String },
    #[error("`{campo}` ({valor}) fora da faixa [mínima {low}, máxima {high}]")]
    OutsideRange {
        campo: &'static str,
        valor: String,
        low: String,
        high: String,
    },
}

impl Candle {
    /// Verifica as invariantes de uma vela.
    pub fn validate(&self) -> Result<(), CandleError> {
        for (campo, valor) in [
            ("open", self.open),
            ("high", self.high),
            ("low", self.low),
            ("close", self.close),
        ] {
            if valor <= rust_decimal::Decimal::ZERO {
                return Err(CandleError::NonPositivePrice {
                    campo,
                    valor: valor.to_string(),
                });
            }
        }

        for (campo, valor) in [("volume", self.volume), ("turnover", self.turnover)] {
            if valor < rust_decimal::Decimal::ZERO {
                return Err(CandleError::NegativeAmount {
                    campo,
                    valor: valor.to_string(),
                });
            }
        }

        if self.low > self.high {
            return Err(CandleError::LowAboveHigh {
                low: self.low.to_string(),
                high: self.high.to_string(),
            });
        }

        for (campo, valor) in [("open", self.open), ("close", self.close)] {
            if valor < self.low || valor > self.high {
                return Err(CandleError::OutsideRange {
                    campo,
                    valor: valor.to_string(),
                    low: self.low.to_string(),
                    high: self.high.to_string(),
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use rust_decimal::dec;

    fn em(h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, h, 0, 0).unwrap()
    }

    fn vela(open: Money, high: Money, low: Money, close: Money) -> Candle {
        Candle {
            open_time: em(0),
            open,
            high,
            low,
            close,
            volume: dec!(1),
            turnover: dec!(1),
        }
    }

    #[test]
    fn aceita_vela_coerente() {
        let c = vela(dec!(100), dec!(110), dec!(95), dec!(105));
        assert!(c.validate().is_ok());
    }

    #[test]
    fn aceita_volume_e_turnover_zerados() {
        let mut c = vela(dec!(100), dec!(100), dec!(100), dec!(100));
        c.volume = dec!(0);
        c.turnover = dec!(0);
        assert!(
            c.validate().is_ok(),
            "vela sem negócio é possível, não é corrupção"
        );
    }

    #[test]
    fn rejeita_minima_acima_da_maxima() {
        let c = vela(dec!(100), dec!(90), dec!(110), dec!(100));
        assert!(matches!(
            c.validate(),
            Err(CandleError::LowAboveHigh { .. })
        ));
    }

    #[test]
    fn rejeita_abertura_fora_da_faixa() {
        let c = vela(dec!(120), dec!(110), dec!(95), dec!(105));
        assert!(matches!(
            c.validate(),
            Err(CandleError::OutsideRange { campo: "open", .. })
        ));
    }

    #[test]
    fn rejeita_fechamento_acima_da_maxima() {
        let c = vela(dec!(100), dec!(110), dec!(95), dec!(115));
        assert!(matches!(
            c.validate(),
            Err(CandleError::OutsideRange { campo: "close", .. })
        ));
    }

    #[test]
    fn rejeita_preco_zero_ou_negativo() {
        for (o, h, l, c) in [
            (dec!(0), dec!(110), dec!(0), dec!(105)),
            (dec!(100), dec!(110), dec!(-5), dec!(105)),
        ] {
            assert!(matches!(
                vela(o, h, l, c).validate(),
                Err(CandleError::NonPositivePrice { .. })
            ));
        }
    }

    #[test]
    fn rejeita_volume_negativo() {
        let mut c = vela(dec!(100), dec!(110), dec!(95), dec!(105));
        c.volume = dec!(-1);
        assert!(matches!(
            c.validate(),
            Err(CandleError::NegativeAmount {
                campo: "volume",
                ..
            })
        ));
    }
}
