//! FR-022a e SC-014 — retomada automática na virada do dia.

mod common;

use common::*;
use rust_decimal::dec;
use trade_domain::{FeeModel, Position, Side, Verdict};
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
    g.record_realized(dec!(-300), dec!(0));
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
    g.record_realized(dec!(-300), dec!(0));
    g.on_day_boundary(at(2, 0), &mut rec);

    let pos = Position::default();
    let r = g.submit(
        &order(1, Side::Buy, dec!(1), at(2, 0)),
        &RiskContext {
            position: &pos,
            balance: dec!(1_000_000),
            reference_price: dec!(100),
            now: at(2, 0),
            fees: FeeModel::default(),
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
    g.record_realized(dec!(-150), dec!(0));
    g.on_day_boundary(at(2, 0), &mut rec);
    g.record_realized(dec!(-150), dec!(0));
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

// --- Linha de base do não realizado ----------------------------------------
//
// Com o não realizado dentro do contador, a virada do dia precisa de uma
// linha de base. Sem ela, uma posição carregada no prejuízo re-arma o freio
// todo dia na abertura, e a retomada automática que a constitution exige
// seria só nominal.

#[test]
fn posicao_carregada_no_prejuizo_nao_rearma_o_freio_na_virada() {
    let mut rec = recorder();
    let mut g = guard();
    g.on_day_boundary(at(1, 0), &mut rec);

    let pos = comprado(dec!(10), dec!(100));
    g.mark_to_market(pos.unrealized_at(dec!(70))); // -300
    assert!(g.daily_loss_blocked());

    g.on_day_boundary(at(2, 0), &mut rec);
    g.mark_to_market(pos.unrealized_at(dec!(70))); // mesmo preço, novo dia

    assert!(
        !g.daily_loss_blocked(),
        "o prejuízo de ontem é a linha de base de hoje, não a perda de hoje"
    );
}

#[test]
fn apos_a_virada_conta_so_a_variacao_do_novo_dia() {
    let mut rec = recorder();
    let mut g = guard();
    g.on_day_boundary(at(1, 0), &mut rec);

    let pos = comprado(dec!(10), dec!(100));
    g.mark_to_market(pos.unrealized_at(dec!(70))); // -300, base do dia 2
    g.on_day_boundary(at(2, 0), &mut rec);

    g.mark_to_market(pos.unrealized_at(dec!(55))); // -450: caiu 150 hoje
    assert!(!g.daily_loss_blocked(), "-150 no dia ainda cabe");

    g.mark_to_market(pos.unrealized_at(dec!(48))); // -520: caiu 220 hoje
    assert!(
        g.daily_loss_blocked(),
        "-220 no dia ultrapassa o limite de 200"
    );
}
