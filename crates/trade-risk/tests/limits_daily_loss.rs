//! FR-019a e FR-022 — perda máxima diária.

mod common;

use common::*;
use rust_decimal::dec;
use trade_domain::{FeeModel, LimitBreach, Position, Side, Verdict};
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
    g.record_realized(dec!(-200), dec!(0));
    assert!(
        g.daily_loss_blocked(),
        "perda de exatamente 200 com limite 200 já bloqueia"
    );
}

#[test]
fn abaixo_do_limite_nao_bloqueia() {
    let mut g = guard();
    g.record_realized(dec!(-199.99), dec!(0));
    assert!(!g.daily_loss_blocked());
}

#[test]
fn perdas_se_acumulam_no_dia() {
    let mut g = guard();
    g.record_realized(dec!(-150), dec!(0));
    assert!(!g.daily_loss_blocked());
    g.record_realized(dec!(-50), dec!(0));
    assert!(g.daily_loss_blocked());
}

#[test]
fn lucro_compensa_e_desbloqueia_o_acumulado() {
    let mut g = guard();
    g.record_realized(dec!(-150), dec!(0));
    g.record_realized(dec!(100), dec!(0));
    g.record_realized(dec!(-100), dec!(0));
    assert!(
        !g.daily_loss_blocked(),
        "acumulado é -150, ainda dentro do limite"
    );
}

#[test]
fn bloqueado_nenhuma_compra_passa() {
    let mut rec = recorder();
    let mut g = guard();
    g.record_realized(dec!(-200), dec!(0));
    let pos = Position::default();
    let r = g.submit(
        &order(1, Side::Buy, dec!(1), at(1, 0)),
        &RiskContext {
            position: &pos,
            balance: dec!(1_000_000),
            reference_price: dec!(100),
            now: at(1, 0),
            fees: FeeModel::default(),
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
    g.record_realized(dec!(-250), dec!(0));
    let pos = comprado(dec!(5), dec!(100));
    let r = g.submit(
        &order(1, Side::Sell, dec!(5), at(1, 0)),
        &RiskContext {
            position: &pos,
            balance: dec!(0),
            reference_price: dec!(100),
            now: at(1, 0),
            fees: FeeModel::default(),
        },
        &mut rec,
    );
    assert_eq!(r.decision.verdict, Verdict::Accepted);
}

// --- Não realizado ---------------------------------------------------------
//
// A perda diária conta o resultado **aberto**, não só o fechado. Sem isto,
// uma posição perdendo 50% não move o contador e o freio nunca dispara.

#[test]
fn posicao_aberta_no_prejuizo_aciona_o_freio_sem_nenhuma_venda() {
    let mut g = guard();
    let pos = comprado(dec!(10), dec!(100));
    // Preço cai de 100 para 80: não realizado -200, exatamente o limite.
    g.mark_to_market(pos.unrealized_at(dec!(80)));
    assert!(
        g.daily_loss_blocked(),
        "prejuízo aberto de 200 com limite 200 já aciona o freio"
    );
}

#[test]
fn oscilacao_dentro_do_limite_nao_aciona() {
    let mut g = guard();
    let pos = comprado(dec!(10), dec!(100));
    g.mark_to_market(pos.unrealized_at(dec!(81)));
    assert!(!g.daily_loss_blocked(), "-190 ainda cabe na cerca");
}

#[test]
fn recuperacao_no_mesmo_dia_nao_solta_o_freio() {
    // A constitution diz "cessar ... até o próximo período". O bloqueio trava:
    // o preço voltar não desfaz o fato de o limite ter sido atingido.
    let mut g = guard();
    let pos = comprado(dec!(10), dec!(100));
    g.mark_to_market(pos.unrealized_at(dec!(80)));
    assert!(g.daily_loss_blocked());

    g.mark_to_market(pos.unrealized_at(dec!(105)));
    assert!(
        g.daily_loss_blocked(),
        "atingido o limite, o dia está encerrado para abertura — não oscila de volta"
    );
}

#[test]
fn realizado_e_nao_realizado_somam_no_mesmo_contador() {
    let mut g = guard();
    let pos = comprado(dec!(10), dec!(100));
    // Fecha uma operação com -150 enquanto carrega outra posição.
    g.record_realized(dec!(-150), pos.unrealized_at(dec!(100)));
    assert!(!g.daily_loss_blocked());
    // A posição carregada passa a perder 50: o total do dia chega a -200.
    g.mark_to_market(pos.unrealized_at(dec!(95)));
    assert!(
        g.daily_loss_blocked(),
        "-150 fechado e -50 aberto somam os 200 do limite"
    );
}

#[test]
fn ganho_aberto_compensa_perda_fechada() {
    let mut g = guard();
    let pos = comprado(dec!(10), dec!(100));
    g.record_realized(dec!(-150), pos.unrealized_at(dec!(100)));
    g.mark_to_market(pos.unrealized_at(dec!(110)));
    assert!(
        !g.daily_loss_blocked(),
        "-150 fechado com +100 aberto é -50 no dia"
    );
}

#[test]
fn posicao_zerada_nao_contribui() {
    let mut g = guard();
    let pos = Position::default();
    g.mark_to_market(pos.unrealized_at(dec!(1)));
    assert!(!g.daily_loss_blocked());
}

#[test]
fn bloqueado_por_prejuizo_aberto_a_venda_continua_permitida() {
    // Barrar a venda prenderia o operador dentro do prejuízo que acionou o
    // freio — exatamente o pior momento para trancar a porta.
    let mut rec = recorder();
    let mut g = guard();
    let pos = comprado(dec!(10), dec!(100));
    g.mark_to_market(pos.unrealized_at(dec!(80)));
    assert!(g.daily_loss_blocked());

    let r = g.submit(
        &order(1, Side::Sell, dec!(10), at(1, 0)),
        &RiskContext {
            position: &pos,
            balance: dec!(0),
            reference_price: dec!(80),
            now: at(1, 0),
            fees: FeeModel::default(),
        },
        &mut rec,
    );
    assert_eq!(r.decision.verdict, Verdict::Accepted);
}

#[test]
fn bloqueado_por_prejuizo_aberto_nenhuma_compra_passa() {
    let mut rec = recorder();
    let mut g = guard();
    let pos = comprado(dec!(10), dec!(100));
    g.mark_to_market(pos.unrealized_at(dec!(80)));

    let r = g.submit(
        &order(1, Side::Buy, dec!(1), at(1, 0)),
        &RiskContext {
            position: &pos,
            balance: dec!(1_000_000),
            reference_price: dec!(80),
            now: at(1, 0),
            fees: FeeModel::default(),
        },
        &mut rec,
    );
    assert_eq!(
        r.decision.verdict,
        Verdict::Rejected(LimitBreach::DailyLossReached)
    );
}
