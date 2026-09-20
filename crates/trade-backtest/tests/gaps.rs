//! FR-031 — lacunas reportadas, nunca interpoladas.

mod common;

use common::*;
use rust_decimal::dec;
use trade_backtest::BacktestEngine;
use trade_domain::Intent;

#[test]
fn lacuna_no_historico_e_reportada() {
    // Faltam os minutos 2 a 9: o histórico salta de 1 para 10.
    let candles = velas(&[
        (0, dec!(100)),
        (1, dec!(100)),
        (10, dec!(100)),
        (11, dec!(100)),
    ]);
    let mut estrategia = ScriptedStrategy::new(vec![]);
    let mut rec = recorder();

    let r = BacktestEngine::new(config(dec!(1000), 60, dec!(0), dec!(0), limites_folgados()))
        .run(
            &mut estrategia,
            &fonte(candles),
            KillSwitch::disarmed(),
            &mut rec,
        )
        .unwrap();

    assert_eq!(r.gaps.len(), 1, "a lacuna precisa aparecer no resultado");
    assert_eq!(r.gaps[0].from, t(1));
    assert_eq!(r.gaps[0].to, t(10));
}

#[test]
fn historico_continuo_nao_produz_lacuna() {
    let candles = velas(&[(0, dec!(100)), (1, dec!(100)), (2, dec!(100))]);
    let mut estrategia = ScriptedStrategy::new(vec![]);
    let mut rec = recorder();

    let r = BacktestEngine::new(config(dec!(1000), 60, dec!(0), dec!(0), limites_folgados()))
        .run(
            &mut estrategia,
            &fonte(candles),
            KillSwitch::disarmed(),
            &mut rec,
        )
        .unwrap();

    assert!(r.gaps.is_empty());
}

#[test]
fn a_lacuna_nao_vira_continuidade_de_preco() {
    // Antes da lacuna o preço é 100; depois, 200. Interpolar produziria uma
    // subida suave que nunca existiu — e uma estratégia de tendência compraria
    // com base nela.
    let candles = velas(&[
        (0, dec!(100)),
        (1, dec!(100)),
        (10, dec!(200)),
        (11, dec!(200)),
    ]);
    let mut estrategia = ScriptedStrategy::new(vec![(1, Intent::Buy, Some(dec!(1)))]);
    let mut rec = recorder();

    let r = BacktestEngine::new(config(dec!(1000), 60, dec!(0), dec!(0), limites_folgados()))
        .run(
            &mut estrategia,
            &fonte(candles),
            KillSwitch::disarmed(),
            &mut rec,
        )
        .unwrap();

    assert_eq!(r.candles_seen, 4, "só as velas que existem são percorridas");
    assert_eq!(
        r.final_position.avg_price(),
        dec!(200),
        "preenche no preço real do outro lado da lacuna"
    );
}
