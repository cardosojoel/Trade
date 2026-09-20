//! REQ-BYBIT-006 — o motor negocia apenas em múltiplos do passo do instrumento.

mod common;

use common::*;
use rust_decimal::dec;
use trade_backtest::BacktestEngine;
use trade_domain::{Instrumento, Intent};

#[test]
fn toda_ordem_e_multiplo_do_passo_do_instrumento() {
    // Um passo grosso deixa o efeito visível: 0,01 unidade. Sem truncar no
    // passo, o dimensionamento por saldo produz quantidade com oito casas —
    // uma ordem que a corretora recusaria.
    let candles = velas(&[(0, dec!(100)), (1, dec!(100)), (2, dec!(100))]);
    let mut estrategia = ScriptedStrategy::new(vec![(0, Intent::Buy, None)]);
    let mut rec = recorder();

    let mut cfg = config(
        dec!(1000),
        10,
        dec!(0.001),
        dec!(0.0005),
        limites_folgados(),
    );
    cfg.instrumento = Instrumento::novo(dec!(0.01), dec!(5)).unwrap();

    let _ = BacktestEngine::new(cfg)
        .run(
            &mut estrategia,
            &fonte(candles),
            KillSwitch::disarmed(),
            &mut rec,
        )
        .unwrap();

    // O que precisa ser múltiplo do passo é a **ordem**, não a posição: a
    // taxa da compra sai em moeda base depois do preenchimento, e o que ela
    // deixa abaixo do passo é justamente o resíduo. Exigir posição redonda
    // seria exigir que o resíduo não existisse.
    let ordens: Vec<_> = rec
        .sink()
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            trade_domain::AuditKind::Order(o) => Some(o.qty),
            _ => None,
        })
        .collect();
    assert!(!ordens.is_empty(), "nenhuma ordem foi emitida");

    for qty in ordens {
        assert_eq!(
            qty % dec!(0.01),
            dec!(0),
            "ordem fora do passo do instrumento: {qty}"
        );
    }
}

#[test]
fn ordem_abaixo_do_valor_minimo_nao_e_emitida() {
    // Com capital de 6 e mínimo de 5, a compra cabe; com mínimo de 50, não —
    // e o motor não deve emitir ordem que a corretora recusaria.
    let candles = velas(&[(0, dec!(100)), (1, dec!(100)), (2, dec!(100))]);
    let mut estrategia = ScriptedStrategy::new(vec![(0, Intent::Buy, None)]);
    let mut rec = recorder();

    let mut cfg = config(dec!(6), 10, dec!(0.001), dec!(0.0005), limites_folgados());
    cfg.instrumento = Instrumento::novo(dec!(0.000001), dec!(50)).unwrap();

    let r = BacktestEngine::new(cfg)
        .run(
            &mut estrategia,
            &fonte(candles),
            KillSwitch::disarmed(),
            &mut rec,
        )
        .unwrap();

    assert!(
        r.final_position.is_flat(),
        "ordem abaixo do valor mínimo não pode ter sido emitida"
    );
}
