//! FR-030 — nenhuma decisão usa informação posterior ao instante simulado.

mod common;

use common::*;
use rust_decimal::dec;
use trade_backtest::BacktestEngine;
use trade_domain::Intent;

#[test]
fn o_preenchimento_acontece_na_vela_seguinte_ao_sinal() {
    // Preço salta de 100 para 200 logo depois do sinal. Se o motor preenchesse
    // na vela do sinal, a compra sairia a 100 — o que seria comprar sabendo do
    // salto. Preenchendo na seguinte, sai a 200.
    let candles = velas(&[(0, dec!(100)), (1, dec!(200)), (2, dec!(200))]);
    let mut estrategia = ScriptedStrategy::new(vec![(0, Intent::Buy, Some(dec!(1)))]);
    let mut rec = recorder();

    let r = BacktestEngine::new(config(dec!(1000), 10, dec!(0), dec!(0), limites_folgados()))
        .run(
            &mut estrategia,
            &fonte(candles),
            KillSwitch::disarmed(),
            &mut rec,
        )
        .unwrap();

    assert_eq!(
        r.final_position.avg_price(),
        dec!(200),
        "preencheu a 100 — isso é comprar sabendo do salto"
    );
}

#[test]
fn sinal_na_ultima_vela_nao_e_executado() {
    // Não existe vela seguinte: a ordem simplesmente não acontece. Executá-la
    // sobre a própria vela seria inventar um instante que o histórico não tem.
    let candles = velas(&[(0, dec!(100)), (1, dec!(100))]);
    let mut estrategia = ScriptedStrategy::new(vec![(1, Intent::Buy, Some(dec!(1)))]);
    let mut rec = recorder();

    let r = BacktestEngine::new(config(dec!(1000), 10, dec!(0), dec!(0), limites_folgados()))
        .run(
            &mut estrategia,
            &fonte(candles),
            KillSwitch::disarmed(),
            &mut rec,
        )
        .unwrap();

    assert!(r.final_position.is_flat());
    assert_eq!(r.trades.len(), 0);
}
