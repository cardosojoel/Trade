//! SC-009 — a conta fecha.

mod common;

use common::*;
use rust_decimal::{Decimal, dec};
use trade_backtest::BacktestEngine;
use trade_domain::Intent;

#[test]
fn o_resultado_liquido_coincide_com_a_soma_do_extrato() {
    let candles = velas(&[
        (0, dec!(100)),
        (1, dec!(100)),
        (2, dec!(107)),
        (3, dec!(107)),
        (4, dec!(107)),
        (5, dec!(107)),
        (6, dec!(94)),
        (7, dec!(94)),
    ]);
    let mut estrategia = ScriptedStrategy::new(vec![
        (0, Intent::Buy, Some(dec!(3))),
        (2, Intent::Sell, Some(dec!(3))),
        (4, Intent::Buy, Some(dec!(3))),
        (6, Intent::Sell, Some(dec!(3))),
    ]);
    let mut rec = recorder();

    let r = BacktestEngine::new(config(
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
    .unwrap();

    let soma: Decimal = r.trades.iter().map(|t| t.pnl).sum();
    assert_eq!(
        r.metrics.net_result, soma,
        "divergência zero, não aproximação"
    );
    assert_eq!(r.metrics.trade_count, r.trades.len());
}

#[test]
fn os_valores_do_extrato_cabem_na_escala_monetaria() {
    // Reproduz o defeito encontrado rodando de verdade: sem quantizar, a
    // quantidade saía de uma divisão com 28 dígitos, o resultado de cada
    // operação herdava essa cauda, e a soma do extrato refeita por fora — em
    // SQL — divergia do net_result reportado. Divergência ínfima, mas SC-009
    // pede zero, e só é zero se a soma puder ser refeita com outra aritmética.
    let candles = velas(&[
        (0, dec!(63421.17)),
        (1, dec!(63421.17)),
        (2, dec!(63887.93)),
        (3, dec!(63887.93)),
    ]);
    let mut estrategia = ScriptedStrategy::new(vec![
        (0, Intent::Buy, Some(dec!(0.157894736842105263157894737))),
        (2, Intent::Sell, None),
    ]);
    let mut rec = recorder();

    let r = BacktestEngine::new(config(
        // Capital folgado: com 10.000 a compra de 0,157 BTC a 63.421 custaria
        // mais que o saldo e seria recusada antes de virar operação.
        dec!(100_000),
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
    .unwrap();

    assert!(
        !r.trades.is_empty(),
        "o cenário precisa produzir ao menos uma operação"
    );
    for t in &r.trades {
        assert!(
            t.pnl.scale() <= 8,
            "pnl {} tem escala {}",
            t.pnl,
            t.pnl.scale()
        );
        assert!(
            t.fees.scale() <= 8,
            "taxas {} têm escala {}",
            t.fees,
            t.fees.scale()
        );
        assert!(
            t.qty.scale() <= 8,
            "quantidade {} tem escala {}",
            t.qty,
            t.qty.scale()
        );
    }
    assert!(
        r.metrics.net_result.scale() <= 8,
        "resultado líquido {} tem escala {}",
        r.metrics.net_result,
        r.metrics.net_result.scale()
    );
}

#[test]
fn o_saldo_final_nunca_fica_negativo() {
    let candles = velas(&[(0, dec!(100)), (1, dec!(100)), (2, dec!(100))]);
    let mut estrategia = ScriptedStrategy::new(vec![(0, Intent::Buy, Some(dec!(999999)))]);
    let mut rec = recorder();

    let r = BacktestEngine::new(config(
        dec!(1000),
        10,
        dec!(0.01),
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

    assert!(
        r.final_balance >= Decimal::ZERO,
        "saldo {}",
        r.final_balance
    );
}
