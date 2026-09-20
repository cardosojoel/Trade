//! Preenchimento parcial e ausência de contraparte, ambos explícitos.

mod common;

use common::*;
use rust_decimal::dec;
use trade_backtest::BacktestEngine;
use trade_domain::Intent;

#[test]
fn volume_insuficiente_produz_preenchimento_parcial_explicito() {
    // Pede 10, a vela tem volume 3: preenche 3. A diferença aparece na
    // posição, não desaparece em silêncio.
    let mut candles = velas(&[(0, dec!(100)), (1, dec!(100)), (2, dec!(100))]);
    candles[1].volume = dec!(3);
    let mut estrategia = ScriptedStrategy::new(vec![(0, Intent::Buy, Some(dec!(10)))]);
    let mut rec = recorder();

    let r = BacktestEngine::new(config(
        dec!(10_000),
        60,
        dec!(0),
        dec!(0),
        limites_folgados(),
    ))
    .run(
        &mut estrategia,
        &fonte(candles),
        KillSwitch::disarmed(),
        &mut rec,
    )
    .unwrap();

    assert_eq!(r.final_position.qty(), dec!(3), "preencheu só o que havia");
}

#[test]
fn vela_sem_volume_nao_tem_contraparte() {
    let mut candles = velas(&[(0, dec!(100)), (1, dec!(100)), (2, dec!(100))]);
    candles[1].volume = dec!(0);
    let mut estrategia = ScriptedStrategy::new(vec![(0, Intent::Buy, Some(dec!(1)))]);
    let mut rec = recorder();

    let r = BacktestEngine::new(config(
        dec!(10_000),
        60,
        dec!(0),
        dec!(0),
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
        r.final_position.is_flat(),
        "sem contraparte, nada é executado"
    );
}
