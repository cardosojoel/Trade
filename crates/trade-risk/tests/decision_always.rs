//! SC-002 — toda ordem produz exatamente uma decisão de risco.

mod common;

use common::*;
use rust_decimal::dec;
use trade_domain::{Position, Side};
use trade_ports::testing::StubOrderExecutor;
use trade_risk::{KillSwitch, RiskContext, RiskGuard};

#[test]
fn ordem_aceita_produz_decisao() {
    let mut rec = recorder();
    let mut g = RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        limits(),
        KillSwitch::disarmed(),
    );
    let pos = Position::default();
    let r = g.submit(
        &order(1, Side::Buy, dec!(1), at(1, 0)),
        &RiskContext {
            position: &pos,
            balance: dec!(1_000_000),
            reference_price: dec!(100),
            now: at(1, 0),
        },
        &mut rec,
    );
    assert_eq!(r.decision.order_ref, trade_domain::OrderId(1));
}

#[test]
fn ordem_recusada_produz_decisao() {
    let mut rec = recorder();
    let mut g = RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        limits(),
        KillSwitch::engaged(),
    );
    let pos = Position::default();
    let r = g.submit(
        &order(7, Side::Buy, dec!(1), at(1, 0)),
        &RiskContext {
            position: &pos,
            balance: dec!(1_000_000),
            reference_price: dec!(100),
            now: at(1, 0),
        },
        &mut rec,
    );
    assert_eq!(r.decision.order_ref, trade_domain::OrderId(7));
}

#[test]
fn ordem_autorizada_que_falha_na_execucao_ainda_produz_decisao() {
    let mut rec = recorder();
    // É por isto que submit não devolve Result: um Err perderia a decisão, e
    // a ordem ficaria no registro sem o veredito que a autorizou.
    let mut g = RiskGuard::new(
        StubOrderExecutor::fails_then_fills(99, dec!(100)),
        limits(),
        KillSwitch::disarmed(),
    );
    let pos = Position::default();
    let r = g.submit(
        &order(9, Side::Buy, dec!(1), at(1, 0)),
        &RiskContext {
            position: &pos,
            balance: dec!(1_000_000),
            reference_price: dec!(100),
            now: at(1, 0),
        },
        &mut rec,
    );
    assert_eq!(r.decision.order_ref, trade_domain::OrderId(9));
    assert!(
        r.decision.verdict.is_accepted(),
        "o risco autorizou; quem falhou foi o mercado"
    );
    assert!(r.fill.is_none());
    assert!(r.error.is_some());
}
