//! FR-022a e SC-014 — retomada automática na virada do dia.

mod common;

use common::*;
use rust_decimal::dec;
use trade_domain::{Position, Side, Verdict};
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
fn virada_do_dia_libera_o_bloqueio_sem_ato_humano() {
    let mut rec = recorder();
    let mut g = guard();
    g.on_day_boundary(at(1, 0), &mut rec);
    g.record_realized(dec!(-300));
    assert!(g.daily_loss_blocked());

    let retomou = g.on_day_boundary(at(2, 0), &mut rec);

    assert!(
        retomou,
        "a virada deve ser reportada, para virar evento resume"
    );
    assert!(
        !g.daily_loss_blocked(),
        "o bloqueio cai sozinho — nenhuma chamada humana envolvida"
    );
}

#[test]
fn apos_a_virada_a_operacao_volta_a_ser_aceita() {
    let mut rec = recorder();
    let mut g = guard();
    g.on_day_boundary(at(1, 0), &mut rec);
    g.record_realized(dec!(-300));
    g.on_day_boundary(at(2, 0), &mut rec);

    let pos = Position::default();
    let r = g.submit(
        &order(1, Side::Buy, dec!(1), at(2, 0)),
        &RiskContext {
            position: &pos,
            balance: dec!(1_000_000),
            reference_price: dec!(100),
            now: at(2, 0),
        },
        &mut rec,
    );
    assert_eq!(r.decision.verdict, Verdict::Accepted);
}

#[test]
fn a_perda_acumulada_zera_na_virada() {
    let mut rec = recorder();
    let mut g = guard();
    g.on_day_boundary(at(1, 0), &mut rec);
    g.record_realized(dec!(-150));
    g.on_day_boundary(at(2, 0), &mut rec);
    g.record_realized(dec!(-150));
    assert!(
        !g.daily_loss_blocked(),
        "os -150 de ontem não contam para hoje"
    );
}

#[test]
fn dentro_do_mesmo_dia_nao_ha_retomada() {
    let mut rec = recorder();
    let mut g = guard();
    g.on_day_boundary(at(1, 0), &mut rec);
    assert!(
        !g.on_day_boundary(at(1, 23), &mut rec),
        "mesma data, nada a retomar"
    );
}

#[test]
fn a_fronteira_e_o_dia_em_utc() {
    let mut rec = recorder();
    // 23h de um dia e 0h do seguinte são dias diferentes; 0h e 23h do mesmo
    // dia não são. A fronteira precisa ser inequívoca.
    let mut g = guard();
    assert!(g.on_day_boundary(at(1, 23), &mut rec));
    assert!(g.on_day_boundary(at(2, 0), &mut rec));
    assert!(!g.on_day_boundary(at(2, 23), &mut rec));
}
