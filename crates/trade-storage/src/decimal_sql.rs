//! Conversão entre [`Decimal`] e a representação em `TEXT` do SQLite.
//!
//! **Nunca via `REAL`.** O tipo `REAL` do SQLite é IEEE-754 de 64 bits: gravar
//! um `Decimal` nele desfaria, já na primeira ida ao disco, toda a garantia que
//! `rust_decimal` existe para dar. Texto preserva o valor exatamente como veio
//! da fonte — que, no caso da Bybit, também o envia como string.
//!
//! Custa espaço e custa comparação numérica em SQL. Ambos aceitáveis diante de
//! correção monetária.

use rust_decimal::Decimal;
use std::str::FromStr;
use trade_ports::StorageError;

/// Serializa para gravação.
pub fn to_sql(v: Decimal) -> String {
    v.to_string()
}

/// Lê um valor gravado, falhando de forma explícita se estiver corrompido.
pub fn from_sql(campo: &str, raw: &str) -> Result<Decimal, StorageError> {
    Decimal::from_str(raw).map_err(|_| StorageError::Corrupt {
        campo: campo.to_string(),
        valor: raw.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::dec;

    #[test]
    fn ida_e_volta_preserva_o_valor_exato() {
        for v in [
            dec!(0),
            dec!(1),
            dec!(-1),
            dec!(63420.10),
            dec!(0.00000001),
            dec!(123456789.123456789),
            dec!(-0.00000001),
        ] {
            let voltou = from_sql("teste", &to_sql(v)).unwrap();
            assert_eq!(voltou, v, "perdeu exatidão em {v}");
        }
    }

    #[test]
    fn preserva_oito_casas_decimais() {
        // Precisão de satoshi: 1 BTC = 100_000_000 sat.
        let v = dec!(0.12345678);
        assert_eq!(from_sql("qty", &to_sql(v)).unwrap(), v);
        assert_eq!(to_sql(v), "0.12345678");
    }

    #[test]
    fn nao_passa_por_ponto_flutuante() {
        // 0.1 + 0.2 em f64 dá 0.30000000000000004. Em Decimal, e na ida e
        // volta pelo texto, dá exatamente 0.3.
        let soma = dec!(0.1) + dec!(0.2);
        assert_eq!(to_sql(soma), "0.3");
        assert_eq!(from_sql("soma", "0.3").unwrap(), dec!(0.3));
    }

    #[test]
    fn valor_corrompido_falha_de_forma_explicita() {
        let err = from_sql("preco", "não é número").unwrap_err();
        assert!(matches!(err, StorageError::Corrupt { .. }));
        assert!(err.to_string().contains("preco"));
    }

    #[test]
    fn valor_vazio_falha() {
        assert!(from_sql("preco", "").is_err());
    }
}
