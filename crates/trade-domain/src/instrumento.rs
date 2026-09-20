//! Regras de quantidade do instrumento negociado.
//!
//! A corretora não aceita qualquer quantidade: só múltiplos de um passo, e só
//! ordens acima de um valor mínimo. Para BTCUSDT na Bybit o passo é
//! `0,000001` BTC — cerca de oito centavos de dólar ao preço de 2026-09-20 — e
//! o mínimo é 5 USDT.
//!
//! Esses dois números não são constantes do projeto: a Bybit os revisa nos
//! dias 3 e 17 de cada mês. Por isso vivem aqui como valor lido, nunca como
//! literal no código (REQ-BYBIT-006).
//!
//! O passo é o que cria o **resíduo**: a taxa da compra é cobrada em moeda
//! base, então a quantidade recebida não é múltiplo do passo, e a diferença
//! não é vendável. Ela não é perda — é saldo que se soma à ordem seguinte
//! (REQ-BYBIT-005).

use crate::{Money, Qty};
use rust_decimal::Decimal;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InstrumentoError {
    #[error("passo de quantidade deve ser positivo, veio {0}")]
    PassoNaoPositivo(String),
    #[error("valor mínimo de ordem não pode ser negativo, veio {0}")]
    MinimoNegativo(String),
}

/// Restrições de quantidade e valor de uma ordem, lidas da corretora.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instrumento {
    passo_qty: Qty,
    valor_minimo_ordem: Money,
}

impl Instrumento {
    pub fn novo(passo_qty: Qty, valor_minimo_ordem: Money) -> Result<Self, InstrumentoError> {
        if passo_qty <= Decimal::ZERO {
            return Err(InstrumentoError::PassoNaoPositivo(passo_qty.to_string()));
        }
        if valor_minimo_ordem < Decimal::ZERO {
            return Err(InstrumentoError::MinimoNegativo(
                valor_minimo_ordem.to_string(),
            ));
        }
        Ok(Self {
            passo_qty,
            valor_minimo_ordem,
        })
    }

    pub fn passo_qty(&self) -> Qty {
        self.passo_qty
    }

    pub fn valor_minimo_ordem(&self) -> Money {
        self.valor_minimo_ordem
    }

    /// Maior múltiplo do passo que não excede `v`.
    ///
    /// Sempre para baixo: arredondar para cima uma quantidade a comprar cria
    /// fração que o saldo não paga, e uma quantidade a vender cria posição que
    /// não se detém.
    pub fn truncar_no_passo(&self, v: Qty) -> Qty {
        if v <= Decimal::ZERO {
            return Decimal::ZERO;
        }
        (v / self.passo_qty).floor() * self.passo_qty
    }

    /// O que sobra abaixo do passo — o resíduo que não é vendável agora.
    pub fn residuo(&self, v: Qty) -> Qty {
        if v <= Decimal::ZERO {
            return Decimal::ZERO;
        }
        v - self.truncar_no_passo(v)
    }

    /// Uma ordem só existe se for múltiplo do passo e alcançar o valor mínimo.
    pub fn negociavel(&self, qty: Qty, preco: Money) -> bool {
        qty > Decimal::ZERO && self.residuo(qty).is_zero() && qty * preco >= self.valor_minimo_ordem
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::dec;

    fn btcusdt() -> Instrumento {
        Instrumento::novo(dec!(0.000001), dec!(5)).unwrap()
    }

    #[test]
    fn passo_nao_positivo_e_erro() {
        assert!(Instrumento::novo(dec!(0), dec!(5)).is_err());
        assert!(Instrumento::novo(dec!(-0.001), dec!(5)).is_err());
    }

    #[test]
    fn truncar_desce_ate_o_multiplo_do_passo() {
        let i = btcusdt();
        assert_eq!(i.truncar_no_passo(dec!(0.000614385)), dec!(0.000614));
        assert_eq!(i.truncar_no_passo(dec!(0.000614)), dec!(0.000614));
        assert_eq!(i.truncar_no_passo(dec!(0.0000009)), dec!(0));
    }

    #[test]
    fn residuo_e_o_que_nao_cabe_no_passo() {
        let i = btcusdt();
        assert_eq!(i.residuo(dec!(0.000614385)), dec!(0.000000385));
        assert_eq!(i.residuo(dec!(0.000614)), dec!(0));
    }

    #[test]
    fn ordem_abaixo_do_valor_minimo_nao_e_negociavel() {
        let i = btcusdt();
        let preco = dec!(81233.70);
        // 0,00006 BTC valem US$ 4,87 — abaixo do mínimo de US$ 5.
        assert!(!i.negociavel(dec!(0.00006), preco));
        assert!(i.negociavel(dec!(0.000062), preco));
    }

    #[test]
    fn quantidade_fora_do_passo_nao_e_negociavel() {
        let i = btcusdt();
        assert!(!i.negociavel(dec!(0.0006143855), dec!(81233.70)));
    }
}
