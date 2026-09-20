//! Posição detida e sua evolução.

use crate::types::{Fill, Side, Trade};
use crate::{Money, Qty};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

/// Exposição corrente em BTC.
///
/// A invariante central é que [`Position::qty`] **nunca fica negativa**
/// (SC-010). No escopo spot comprado não existe posição vendida, e o tipo não a
/// representa: uma venda acima do detido é erro, não uma posição negativa.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Position {
    qty: Qty,
    avg_price: Money,
    /// Taxas pagas na abertura e ainda não realizadas, para rateio proporcional
    /// quando a posição é vendida em partes.
    open_fees: Money,
    opened_at: Option<DateTime<Utc>>,
    realized_pnl: Money,
    trades: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PositionError {
    #[error(
        "venda de {requested} acima da quantidade detida {held} — não há posição vendida no escopo spot comprado"
    )]
    SellExceedsHoldings { held: String, requested: String },
    #[error("quantidade do preenchimento deve ser positiva, veio {0}")]
    NonPositiveQty(String),
}

impl Position {
    pub fn qty(&self) -> Qty {
        self.qty
    }

    pub fn avg_price(&self) -> Money {
        self.avg_price
    }

    pub fn realized_pnl(&self) -> Money {
        self.realized_pnl
    }

    pub fn is_flat(&self) -> bool {
        self.qty.is_zero()
    }

    /// Valor de mercado da posição ao preço informado.
    pub fn exposure_at(&self, price: Money) -> Money {
        self.qty * price
    }

    /// Resultado não realizado ao preço informado.
    pub fn unrealized_at(&self, price: Money) -> Money {
        if self.qty.is_zero() {
            Decimal::ZERO
        } else {
            (price - self.avg_price) * self.qty - self.open_fees
        }
    }

    /// Aplica um preenchimento. Uma venda devolve a [`Trade`] realizada.
    ///
    /// Esta é a única forma de alterar a posição — não há atribuição direta de
    /// `qty`. É o que impede que uma quantidade negativa seja construída por
    /// engano, em vez de apenas detectada depois.
    pub fn apply_fill(&mut self, side: Side, fill: &Fill) -> Result<Option<Trade>, PositionError> {
        if fill.qty <= Decimal::ZERO {
            return Err(PositionError::NonPositiveQty(fill.qty.to_string()));
        }

        match side {
            Side::Buy => {
                let valor_total = self.avg_price * self.qty + fill.price * fill.qty;
                let nova_qty = self.qty + fill.qty;
                self.avg_price = valor_total / nova_qty;
                self.qty = nova_qty;
                self.open_fees += fill.fee;
                if self.opened_at.is_none() {
                    self.opened_at = Some(fill.at);
                }
                Ok(None)
            }

            Side::Sell => {
                // A verificação vem antes de qualquer mutação: uma tentativa
                // inválida não pode deixar rastro na posição.
                if fill.qty > self.qty {
                    return Err(PositionError::SellExceedsHoldings {
                        held: self.qty.to_string(),
                        requested: fill.qty.to_string(),
                    });
                }

                // Rateio proporcional das taxas de abertura ainda não
                // realizadas, para que uma venda parcial carregue apenas a
                // parte do custo de entrada que lhe corresponde.
                let proporcao = fill.qty / self.qty;
                let taxa_de_entrada = self.open_fees * proporcao;
                let preco_de_entrada = self.avg_price;
                let pnl = (fill.price - preco_de_entrada) * fill.qty - fill.fee - taxa_de_entrada;
                let entry_at = self.opened_at.unwrap_or(fill.at);

                self.open_fees -= taxa_de_entrada;
                self.qty -= fill.qty;
                self.realized_pnl += pnl;
                self.trades += 1;

                let trade = Trade {
                    seq: self.trades,
                    entry_at,
                    entry_price: preco_de_entrada,
                    exit_at: fill.at,
                    exit_price: fill.price,
                    qty: fill.qty,
                    fees: fill.fee + taxa_de_entrada,
                    pnl,
                };

                if self.qty.is_zero() {
                    self.avg_price = Decimal::ZERO;
                    self.open_fees = Decimal::ZERO;
                    self.opened_at = None;
                }

                Ok(Some(trade))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::OrderId;
    use chrono::TimeZone;
    use proptest::prelude::*;
    use rust_decimal::dec;

    fn em(h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, h, 0, 0).unwrap()
    }

    fn fill(price: Money, qty: Qty, fee: Money, h: u32) -> Fill {
        Fill {
            order_ref: OrderId(1),
            price,
            qty,
            fee,
            slippage: Decimal::ZERO,
            at: em(h),
        }
    }

    #[test]
    fn compra_define_preco_medio() {
        let mut p = Position::default();
        p.apply_fill(Side::Buy, &fill(dec!(100), dec!(2), dec!(0), 0))
            .unwrap();
        assert_eq!(p.qty(), dec!(2));
        assert_eq!(p.avg_price(), dec!(100));
    }

    #[test]
    fn compras_sucessivas_recalculam_o_preco_medio() {
        let mut p = Position::default();
        p.apply_fill(Side::Buy, &fill(dec!(100), dec!(1), dec!(0), 0))
            .unwrap();
        p.apply_fill(Side::Buy, &fill(dec!(200), dec!(1), dec!(0), 1))
            .unwrap();
        assert_eq!(p.qty(), dec!(2));
        assert_eq!(p.avg_price(), dec!(150));
    }

    #[test]
    fn venda_parcial_mantem_posicao_e_preco_medio() {
        let mut p = Position::default();
        p.apply_fill(Side::Buy, &fill(dec!(100), dec!(2), dec!(0), 0))
            .unwrap();
        let t = p
            .apply_fill(Side::Sell, &fill(dec!(110), dec!(1), dec!(0), 1))
            .unwrap()
            .unwrap();
        assert_eq!(p.qty(), dec!(1));
        assert_eq!(
            p.avg_price(),
            dec!(100),
            "venda não altera o preço médio do que resta"
        );
        assert_eq!(t.pnl, dec!(10));
        assert_eq!(t.qty, dec!(1));
    }

    #[test]
    fn venda_total_zera_a_posicao_e_gera_operacao() {
        let mut p = Position::default();
        p.apply_fill(Side::Buy, &fill(dec!(100), dec!(2), dec!(0), 0))
            .unwrap();
        let t = p
            .apply_fill(Side::Sell, &fill(dec!(120), dec!(2), dec!(0), 3))
            .unwrap()
            .unwrap();
        assert!(p.is_flat());
        assert_eq!(t.pnl, dec!(40));
        assert_eq!(t.entry_at, em(0));
        assert_eq!(t.exit_at, em(3));
        assert_eq!(p.realized_pnl(), dec!(40));
    }

    #[test]
    fn taxas_entram_no_resultado_e_sao_rateadas_na_venda_parcial() {
        let mut p = Position::default();
        // Compra 2 @ 100 com taxa 2. Vende 1 @ 110 com taxa 1.
        p.apply_fill(Side::Buy, &fill(dec!(100), dec!(2), dec!(2), 0))
            .unwrap();
        let t = p
            .apply_fill(Side::Sell, &fill(dec!(110), dec!(1), dec!(1), 1))
            .unwrap()
            .unwrap();
        // Ganho bruto 10, taxa de saída 1, metade da taxa de entrada 1 → 8.
        assert_eq!(t.pnl, dec!(8));
        assert_eq!(t.fees, dec!(2));
    }

    #[test]
    fn venda_acima_do_detido_e_erro_e_nao_altera_a_posicao() {
        let mut p = Position::default();
        p.apply_fill(Side::Buy, &fill(dec!(100), dec!(1), dec!(0), 0))
            .unwrap();
        let antes = p.clone();
        let err = p
            .apply_fill(Side::Sell, &fill(dec!(110), dec!(2), dec!(0), 1))
            .unwrap_err();
        assert!(matches!(err, PositionError::SellExceedsHoldings { .. }));
        assert_eq!(
            p, antes,
            "tentativa inválida não pode deixar rastro na posição"
        );
    }

    #[test]
    fn venda_sem_posicao_e_erro() {
        let mut p = Position::default();
        assert!(
            p.apply_fill(Side::Sell, &fill(dec!(100), dec!(1), dec!(0), 0))
                .is_err()
        );
    }

    #[test]
    fn compra_nao_gera_operacao() {
        let mut p = Position::default();
        assert!(
            p.apply_fill(Side::Buy, &fill(dec!(100), dec!(1), dec!(0), 0))
                .unwrap()
                .is_none()
        );
    }

    proptest! {
        /// SC-010: após qualquer sequência de preenchimentos válidos, a
        /// quantidade detida nunca fica negativa.
        #[test]
        fn quantidade_detida_nunca_fica_negativa(
            ops in prop::collection::vec((any::<bool>(), 1u32..50, 1u32..500), 0..80)
        ) {
            let mut p = Position::default();
            for (i, (compra, qtd, preco)) in ops.into_iter().enumerate() {
                let side = if compra { Side::Buy } else { Side::Sell };
                let f = fill(
                    Decimal::from(preco),
                    Decimal::from(qtd),
                    Decimal::ZERO,
                    (i % 24) as u32,
                );
                // Vendas acima do detido são recusadas; o ponto é que nem a
                // recusa nem a aceitação podem produzir quantidade negativa.
                let _ = p.apply_fill(side, &f);
                prop_assert!(p.qty() >= Decimal::ZERO, "quantidade negativa: {}", p.qty());
            }
        }
    }
}
