//! US1 — resultado conferido contra cálculo feito à mão.

mod common;

use common::*;
use rust_decimal::dec;
use trade_backtest::{BacktestEngine, RunOutcome};
use trade_domain::Intent;

/// Cenário de referência, com a conta explícita:
///
/// | minuto | preço | o que acontece                                  |
/// |--------|-------|-------------------------------------------------|
/// | 0      | 100   | sinal de compra de 5                             |
/// | 1      | 100   | **preenche** a compra: 5 a 100 → custo 500       |
/// | 2      | 110   | sinal de venda                                   |
/// | 3      | 110   | **preenche** a venda: 5 a 110 → recebe 550       |
/// | 4      | 90    | nada                                             |
///
/// Sem taxa e sem slippage: resultado 50, uma operação, nenhuma perdedora.
fn cenario() -> (Vec<trade_domain::Candle>, ScriptedStrategy) {
    let candles = velas(&[
        (0, dec!(100)),
        (1, dec!(100)),
        (2, dec!(110)),
        (3, dec!(110)),
        (4, dec!(90)),
    ]);
    let estrategia = ScriptedStrategy::new(vec![
        (0, Intent::Buy, Some(dec!(5))),
        (2, Intent::Sell, Some(dec!(5))),
    ]);
    (candles, estrategia)
}

#[test]
fn metricas_batem_com_a_conta_feita_a_mao() {
    let (candles, mut estrategia) = cenario();
    let mut rec = recorder();

    let r = BacktestEngine::new(config(dec!(1000), 10, dec!(0), dec!(0), limites_folgados()))
        .run(
            &mut estrategia,
            &fonte(candles),
            KillSwitch::disarmed(),
            &mut rec,
        )
        .unwrap();

    assert_eq!(r.metrics.net_result, dec!(50));
    assert_eq!(r.metrics.trade_count, 1);
    assert_eq!(r.metrics.gross_profit, dec!(50));
    assert_eq!(r.metrics.gross_loss, dec!(0));
    assert_eq!(r.metrics.profit_factor, None, "nenhuma operação perdedora");
    assert_eq!(r.metrics.max_drawdown, dec!(0));
    assert_eq!(r.outcome, RunOutcome::Completed);
}

#[test]
fn o_extrato_tem_a_operacao_com_entrada_e_saida_corretas() {
    let (candles, mut estrategia) = cenario();
    let mut rec = recorder();

    let r = BacktestEngine::new(config(dec!(1000), 10, dec!(0), dec!(0), limites_folgados()))
        .run(
            &mut estrategia,
            &fonte(candles),
            KillSwitch::disarmed(),
            &mut rec,
        )
        .unwrap();

    assert_eq!(r.trades.len(), 1);
    let op = &r.trades[0];
    assert_eq!(op.entry_price, dec!(100));
    assert_eq!(op.exit_price, dec!(110));
    assert_eq!(op.qty, dec!(5));
    assert_eq!(op.pnl, dec!(50));
    assert_eq!(op.entry_at, t(1), "entrada na vela seguinte ao sinal");
    assert_eq!(op.exit_at, t(3), "saída na vela seguinte ao sinal");
}

#[test]
fn o_saldo_final_reflete_o_ciclo() {
    let (candles, mut estrategia) = cenario();
    let mut rec = recorder();

    let r = BacktestEngine::new(config(dec!(1000), 10, dec!(0), dec!(0), limites_folgados()))
        .run(
            &mut estrategia,
            &fonte(candles),
            KillSwitch::disarmed(),
            &mut rec,
        )
        .unwrap();

    // 1000 − 500 (compra) + 550 (venda) = 1050
    assert_eq!(r.final_balance, dec!(1050));
    assert!(r.final_position.is_flat());
}

#[test]
fn operacao_perdedora_produz_profit_factor_definido() {
    let candles = velas(&[
        (0, dec!(100)),
        (1, dec!(100)),
        (2, dec!(110)),
        (3, dec!(110)), // vende com +50
        (4, dec!(110)),
        (5, dec!(110)), // compra de novo
        (6, dec!(100)),
        (7, dec!(100)), // vende com −50
    ]);
    let mut estrategia = ScriptedStrategy::new(vec![
        (0, Intent::Buy, Some(dec!(5))),
        (2, Intent::Sell, Some(dec!(5))),
        (4, Intent::Buy, Some(dec!(5))),
        (6, Intent::Sell, Some(dec!(5))),
    ]);
    let mut rec = recorder();

    let r = BacktestEngine::new(config(dec!(1000), 20, dec!(0), dec!(0), limites_folgados()))
        .run(
            &mut estrategia,
            &fonte(candles),
            KillSwitch::disarmed(),
            &mut rec,
        )
        .unwrap();

    assert_eq!(r.metrics.trade_count, 2);
    assert_eq!(r.metrics.gross_profit, dec!(50));
    assert_eq!(r.metrics.gross_loss, dec!(50));
    assert_eq!(r.metrics.profit_factor, Some(dec!(1)));
    assert_eq!(r.metrics.net_result, dec!(0));
    assert_eq!(r.metrics.max_drawdown, dec!(50), "pico 50, vale 0");
}
