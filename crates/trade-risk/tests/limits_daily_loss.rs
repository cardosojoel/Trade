//! FR-019a e FR-022 — perda máxima diária.

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
fn atingir_o_limite_ja_bloqueia() {
    // FR-019a: aqui a fronteira é invertida em relação aos demais limites.
    // A constitution diz "atingido o limite, o sistema MUST cessar".
    let mut g = guard();
    g.record_realized(dec!(-200));
    assert!(
        g.daily_loss_blocked(),
        "perda de exatamente 200 com limite 200 já bloqueia"
    );
}

#[test]
fn abaixo_do_limite_nao_bloqueia() {
    let mut g = guard();
    g.record_realized(dec!(-199.99));
    assert!(!g.daily_loss_blocked());
}

#[test]
fn perdas_se_acumulam_no_dia() {
    let mut g = guard();
    g.record_realized(dec!(-150));
    assert!(!g.daily_loss_blocked());
    g.record_realized(dec!(-50));
    assert!(g.daily_loss_blocked());
}

#[test]
fn lucro_compensa_e_desbloqueia_o_acumulado() {
    let mut g = guard();
    g.record_realized(dec!(-150));
    g.record_realized(dec!(100));
    g.record_realized(dec!(-100));
    assert!(
        !g.daily_loss_blocked(),
        "acumulado é -150, ainda dentro do limite"
    );
}

#[test]
fn bloqueado_nenhuma_compra_passa() {
    let mut rec = recorder();
    let mut g = guard();
    g.record_realized(dec!(-200));
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
    assert_eq!(
        r.decision.verdict,
        Verdict::Rejected(LimitBreach::DailyLossReached)
    );
}

#[test]
fn bloqueado_a_venda_continua_permitida() {
    let mut rec = recorder();
    // O bloqueio impede abrir exposição nova, não impede reduzir a existente.
    // Barrar a venda prenderia o operador exatamente no pior momento.
    let mut g = guard();
    g.record_realized(dec!(-250));
    let pos = comprado(dec!(5), dec!(100));
    let r = g.submit(
        &order(1, Side::Sell, dec!(5), at(1, 0)),
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
