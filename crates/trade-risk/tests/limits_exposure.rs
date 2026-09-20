//! FR-019 — exposição máxima total.

mod common;

use common::*;
use rust_decimal::dec;
use trade_domain::{FeeModel, LimitBreach, Position, RiskLimits, Side, Verdict};
use trade_ports::testing::StubOrderExecutor;
use trade_risk::{KillSwitch, RiskContext, RiskGuard};

/// Tamanho de posição folgado, exposição apertada — para isolar qual limite
/// está sendo exercido.
fn limites_com_exposicao_apertada() -> RiskLimits {
    RiskLimits {
        max_position_size: dec!(1_000_000),
        max_total_exposure: dec!(500),
        ..limits()
    }
}

#[test]
fn ordem_que_ultrapassaria_a_exposicao_e_recusada() {
    let mut rec = recorder();
    let mut g = RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        limites_com_exposicao_apertada(),
        KillSwitch::disarmed(),
    );
    let pos = Position::default();
    let r = g.submit(
        &order(1, Side::Buy, dec!(6), at(1, 0)),
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
        Verdict::Rejected(LimitBreach::MaxTotalExposure)
    );
}

#[test]
fn exposicao_exatamente_no_limite_e_aceita() {
    let mut rec = recorder();
    let mut g = RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        limites_com_exposicao_apertada(),
        KillSwitch::disarmed(),
    );
    let pos = Position::default();
    let r = g.submit(
        &order(1, Side::Buy, dec!(5), at(1, 0)),
        &RiskContext {
            position: &pos,
            balance: dec!(1_000_000),
            reference_price: dec!(100),
            now: at(1, 0),
            fees: FeeModel::default(),
        },
        &mut rec,
    );
    assert_eq!(r.decision.verdict, Verdict::Accepted);
}

#[test]
fn venda_nao_e_barrada_por_exposicao() {
    let mut rec = recorder();
    // Vender reduz exposição; barrá-la seria prender o operador na posição.
    let mut g = RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        limites_com_exposicao_apertada(),
        KillSwitch::disarmed(),
    );
    let pos = comprado(dec!(50), dec!(100));
    let r = g.submit(
        &order(1, Side::Sell, dec!(10), at(1, 0)),
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
