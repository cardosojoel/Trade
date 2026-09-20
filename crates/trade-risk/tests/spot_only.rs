//! FR-005 e FR-006 — escopo spot comprado.

mod common;

use common::*;
use rust_decimal::dec;
use trade_domain::{LimitBreach, Position, Side, Verdict};
use trade_ports::testing::StubOrderExecutor;
use trade_risk::{KillSwitch, RiskContext, RiskGuard};

fn guard() -> RiskGuard<StubOrderExecutor> {
    RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        limits(),
        KillSwitch::disarmed(),
    )
}

#[test]
fn venda_acima_do_detido_e_recusada() {
    let mut rec = recorder();
    // Não existe posição vendida no escopo spot comprado.
    let mut g = guard();
    let pos = comprado(dec!(2), dec!(100));
    let r = g.submit(
        &order(1, Side::Sell, dec!(3), at(1, 0)),
        &RiskContext {
            position: &pos,
            balance: dec!(0),
            reference_price: dec!(100),
            now: at(1, 0),
        },
        &mut rec,
    );
    assert_eq!(
        r.decision.verdict,
        Verdict::Rejected(LimitBreach::SellExceedsHoldings)
    );
}

#[test]
fn venda_exatamente_do_detido_e_aceita() {
    let mut rec = recorder();
    let mut g = guard();
    let pos = comprado(dec!(2), dec!(100));
    let r = g.submit(
        &order(1, Side::Sell, dec!(2), at(1, 0)),
        &RiskContext {
            position: &pos,
            balance: dec!(0),
            reference_price: dec!(100),
            now: at(1, 0),
        },
        &mut rec,
    );
    assert_eq!(r.decision.verdict, Verdict::Accepted);
}

#[test]
fn venda_sem_posicao_e_recusada() {
    let mut rec = recorder();
    let mut g = guard();
    let pos = Position::default();
    let r = g.submit(
        &order(1, Side::Sell, dec!(1), at(1, 0)),
        &RiskContext {
            position: &pos,
            balance: dec!(1000),
            reference_price: dec!(100),
            now: at(1, 0),
        },
        &mut rec,
    );
    assert_eq!(
        r.decision.verdict,
        Verdict::Rejected(LimitBreach::SellExceedsHoldings)
    );
}

#[test]
fn compra_sem_saldo_e_recusada() {
    let mut rec = recorder();
    let mut g = guard();
    let pos = Position::default();
    let r = g.submit(
        &order(1, Side::Buy, dec!(5), at(1, 0)),
        &RiskContext {
            position: &pos,
            balance: dec!(100),
            reference_price: dec!(100),
            now: at(1, 0),
        },
        &mut rec,
    );
    assert_eq!(
        r.decision.verdict,
        Verdict::Rejected(LimitBreach::InsufficientBalance)
    );
}

#[test]
fn compra_com_saldo_exato_e_aceita() {
    let mut rec = recorder();
    let mut g = guard();
    let pos = Position::default();
    let r = g.submit(
        &order(1, Side::Buy, dec!(5), at(1, 0)),
        &RiskContext {
            position: &pos,
            balance: dec!(500),
            reference_price: dec!(100),
            now: at(1, 0),
        },
        &mut rec,
    );
    assert_eq!(r.decision.verdict, Verdict::Accepted);
}
