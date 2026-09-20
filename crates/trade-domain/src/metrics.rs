//! Métricas de uma execução.

use crate::Money;
use crate::types::Trade;
use rust_decimal::Decimal;

/// O que a Porta 1 de promoção exige medir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunMetrics {
    /// Lucro bruto dividido pelo prejuízo bruto.
    ///
    /// É [`None`] — **indefinido**, não infinito — quando não houve nenhuma
    /// operação perdedora. Uma execução sem perdas não tem profit factor, e
    /// fingir que tem produziria um número que induz erro na Porta 1.
    pub profit_factor: Option<Decimal>,
    /// Maior queda do pico ao vale na curva de capital, em valor absoluto.
    pub max_drawdown: Money,
    pub trade_count: usize,
    pub net_result: Money,
    pub gross_profit: Money,
    pub gross_loss: Money,
    pub total_fees: Money,
    pub total_slippage: Money,
}

impl RunMetrics {
    /// Calcula as métricas a partir do extrato.
    ///
    /// `net_result` é **a soma do extrato**, por construção — não um valor
    /// calculado por outro caminho que depois se espera coincidir. É o que faz
    /// SC-009 valer com divergência zero em vez de depender de arredondamento.
    pub fn from_trades(trades: &[Trade], total_slippage: Money) -> Self {
        let mut gross_profit = Decimal::ZERO;
        let mut gross_loss = Decimal::ZERO;
        let mut total_fees = Decimal::ZERO;

        // Curva de capital relativa ao início. O pico parte de zero porque a
        // primeira operação perdedora já é um recuo em relação ao ponto de
        // partida.
        let mut acumulado = Decimal::ZERO;
        let mut pico = Decimal::ZERO;
        let mut max_drawdown = Decimal::ZERO;

        for t in trades {
            if t.pnl > Decimal::ZERO {
                gross_profit += t.pnl;
            } else {
                gross_loss += -t.pnl;
            }
            total_fees += t.fees;

            acumulado += t.pnl;
            if acumulado > pico {
                pico = acumulado;
            }
            let queda = pico - acumulado;
            if queda > max_drawdown {
                max_drawdown = queda;
            }
        }

        RunMetrics {
            // Indefinido, nunca infinito: sem operação perdedora não há razão
            // entre lucro e prejuízo.
            profit_factor: if gross_loss.is_zero() {
                None
            } else {
                Some(gross_profit / gross_loss)
            },
            max_drawdown,
            trade_count: trades.len(),
            // Por construção, a soma do extrato — não um valor calculado por
            // outro caminho que depois se espera coincidir (SC-009).
            net_result: acumulado,
            gross_profit,
            gross_loss,
            total_fees,
            total_slippage,
        }
    }

    /// Drawdown máximo como fração do capital inicial.
    pub fn drawdown_ratio(&self, initial_capital: Money) -> Option<Decimal> {
        if initial_capital.is_zero() {
            None
        } else {
            Some(self.max_drawdown / initial_capital)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use rust_decimal::dec;

    fn trade(seq: u64, pnl: Money, fees: Money) -> Trade {
        let t = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        Trade {
            seq,
            entry_at: t,
            entry_price: dec!(100),
            exit_at: t,
            exit_price: dec!(100) + pnl,
            qty: dec!(1),
            fees,
            pnl,
        }
    }

    #[test]
    fn resultado_liquido_e_a_soma_do_extrato() {
        let ts = [
            trade(1, dec!(10), dec!(1)),
            trade(2, dec!(-4), dec!(1)),
            trade(3, dec!(7), dec!(1)),
        ];
        let m = RunMetrics::from_trades(&ts, dec!(0));
        assert_eq!(m.net_result, dec!(13));
        assert_eq!(m.trade_count, 3);
    }

    #[test]
    fn profit_factor_e_lucro_sobre_prejuizo() {
        let ts = [trade(1, dec!(30), dec!(0)), trade(2, dec!(-10), dec!(0))];
        let m = RunMetrics::from_trades(&ts, dec!(0));
        assert_eq!(m.gross_profit, dec!(30));
        assert_eq!(m.gross_loss, dec!(10));
        assert_eq!(m.profit_factor, Some(dec!(3)));
    }

    #[test]
    fn profit_factor_e_indefinido_sem_operacao_perdedora() {
        let ts = [trade(1, dec!(10), dec!(0)), trade(2, dec!(5), dec!(0))];
        let m = RunMetrics::from_trades(&ts, dec!(0));
        assert_eq!(
            m.profit_factor, None,
            "sem perdas não há profit factor — nunca infinito"
        );
    }

    #[test]
    fn profit_factor_e_indefinido_sem_nenhuma_operacao() {
        let m = RunMetrics::from_trades(&[], dec!(0));
        assert_eq!(m.profit_factor, None);
        assert_eq!(m.net_result, dec!(0));
        assert_eq!(m.max_drawdown, dec!(0));
    }

    #[test]
    fn drawdown_maximo_e_a_maior_queda_do_pico_ao_vale() {
        // Curva acumulada: +10, +4 (queda de 6), +14, +2 (queda de 12).
        let ts = [
            trade(1, dec!(10), dec!(0)),
            trade(2, dec!(-6), dec!(0)),
            trade(3, dec!(10), dec!(0)),
            trade(4, dec!(-12), dec!(0)),
        ];
        let m = RunMetrics::from_trades(&ts, dec!(0));
        assert_eq!(m.max_drawdown, dec!(12));
    }

    #[test]
    fn drawdown_e_zero_quando_a_curva_so_sobe() {
        let ts = [trade(1, dec!(5), dec!(0)), trade(2, dec!(5), dec!(0))];
        assert_eq!(RunMetrics::from_trades(&ts, dec!(0)).max_drawdown, dec!(0));
    }

    #[test]
    fn taxas_sao_somadas_e_slippage_vem_de_fora() {
        let ts = [trade(1, dec!(10), dec!(2)), trade(2, dec!(-4), dec!(3))];
        let m = RunMetrics::from_trades(&ts, dec!(7));
        assert_eq!(m.total_fees, dec!(5));
        assert_eq!(m.total_slippage, dec!(7));
    }

    #[test]
    fn drawdown_como_fracao_do_capital() {
        let ts = [trade(1, dec!(-15), dec!(0))];
        let m = RunMetrics::from_trades(&ts, dec!(0));
        assert_eq!(m.drawdown_ratio(dec!(100)), Some(dec!(0.15)));
    }
}
