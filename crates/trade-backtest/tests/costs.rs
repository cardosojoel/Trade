//! FR-027 — taxa e slippage discriminados.

mod common;

use common::*;
use rust_decimal::dec;
use trade_backtest::BacktestEngine;
use trade_domain::Intent;

#[test]
fn taxa_e_slippage_aparecem_separados_no_resultado() {
    // Compra 5 a 100 com slippage 0,5% → preço 100,5; bruto 502,50.
    // Taxa 1% sobre o bruto → 5,025. Slippage discriminado → 0,5 × 5 = 2,50.
    let candles = velas(&[
        (0, dec!(100)),
        (1, dec!(100)),
        (2, dec!(100)),
        (3, dec!(100)),
    ]);
    let mut estrategia = ScriptedStrategy::new(vec![
        (0, Intent::Buy, Some(dec!(5))),
        (2, Intent::Sell, Some(dec!(5))),
    ]);
    let mut rec = recorder();

    let r = BacktestEngine::new(config(
        dec!(1000),
        10,
        dec!(0.01),
        dec!(0.005),
        limites_folgados(),
    ))
    .run(
        &mut estrategia,
        &fonte(candles),
        KillSwitch::disarmed(),
        &mut rec,
    )
    .unwrap();

    assert!(
        r.metrics.total_fees > dec!(0),
        "taxa não pode ficar invisível"
    );
    assert!(
        r.metrics.total_slippage > dec!(0),
        "slippage não pode ficar invisível"
    );
    assert_eq!(
        r.metrics.total_slippage,
        dec!(5),
        "0,50 por unidade, 5 na compra e 5 na venda"
    );
}

#[test]
fn o_custo_de_transacao_transforma_operacao_neutra_em_prejuizo() {
    // Comprar e vender ao mesmo preço dá zero antes dos custos. Depois deles,
    // dá negativo — e é exatamente isso que um backtest sem custos esconde.
    let candles = velas(&[
        (0, dec!(100)),
        (1, dec!(100)),
        (2, dec!(100)),
        (3, dec!(100)),
    ]);
    let mut estrategia = ScriptedStrategy::new(vec![
        (0, Intent::Buy, Some(dec!(5))),
        (2, Intent::Sell, Some(dec!(5))),
    ]);
    let mut rec = recorder();

    let r = BacktestEngine::new(config(
        dec!(1000),
        10,
        dec!(0.01),
        dec!(0.005),
        limites_folgados(),
    ))
    .run(
        &mut estrategia,
        &fonte(candles),
        KillSwitch::disarmed(),
        &mut rec,
    )
    .unwrap();

    assert!(
        r.metrics.net_result < dec!(0),
        "resultado {} deveria ser negativo",
        r.metrics.net_result
    );
}

#[test]
fn o_slippage_e_aplicado_contra_quem_opera() {
    let candles = velas(&[(0, dec!(100)), (1, dec!(100))]);
    let mut estrategia = ScriptedStrategy::new(vec![(0, Intent::Buy, Some(dec!(1)))]);
    let mut rec = recorder();

    let r = BacktestEngine::new(config(
        dec!(1000),
        10,
        dec!(0),
        dec!(0.01),
        limites_folgados(),
    ))
    .run(
        &mut estrategia,
        &fonte(candles),
        KillSwitch::disarmed(),
        &mut rec,
    )
    .unwrap();

    assert_eq!(
        r.final_position.avg_price(),
        dec!(101),
        "a compra sai mais cara; modelar a favor produz backtest otimista"
    );
}
