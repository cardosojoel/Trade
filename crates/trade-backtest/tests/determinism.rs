//! FR-029 e SC-004 — mesma entrada, mesmo resultado.

mod common;

use common::*;
use rust_decimal::dec;
use trade_backtest::BacktestEngine;
use trade_domain::Intent;

fn executa() -> trade_backtest::BacktestOutcome {
    let candles = velas(&[
        (0, dec!(100)),
        (1, dec!(101)),
        (2, dec!(99)),
        (3, dec!(103)),
        (4, dec!(97)),
        (5, dec!(105)),
        (6, dec!(100)),
    ]);
    let mut estrategia = ScriptedStrategy::new(vec![
        (0, Intent::Buy, Some(dec!(3))),
        (2, Intent::Sell, Some(dec!(3))),
        (3, Intent::Buy, Some(dec!(2))),
        (5, Intent::Sell, Some(dec!(2))),
    ]);
    let mut rec = recorder();
    BacktestEngine::new(config(
        dec!(1000),
        20,
        dec!(0.001),
        dec!(0.0005),
        limites_folgados(),
    ))
    .run(
        &mut estrategia,
        &fonte(candles),
        KillSwitch::disarmed(),
        &mut rec,
    )
    .unwrap()
}

#[test]
fn duas_execucoes_identicas_produzem_resultado_identico() {
    let a = executa();
    let b = executa();

    assert_eq!(a.metrics, b.metrics);
    assert_eq!(a.trades, b.trades);
    assert_eq!(a.final_balance, b.final_balance);
    assert_eq!(a.final_position, b.final_position);
}

#[test]
fn dez_execucoes_produzem_o_mesmo_resultado() {
    // Uma repetição pega o acaso grosseiro; dez pegam o que depende de ordem
    // de iteração ou de semente.
    let referencia = executa();
    for i in 1..=10 {
        let r = executa();
        assert_eq!(r.metrics, referencia.metrics, "divergiu na execução {i}");
        assert_eq!(
            r.trades, referencia.trades,
            "extrato divergiu na execução {i}"
        );
    }
}
