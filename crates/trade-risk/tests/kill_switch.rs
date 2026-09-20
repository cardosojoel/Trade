//! FR-023 e FR-023a — kill switch.

mod common;

use common::*;
use rust_decimal::dec;
use tempfile::tempdir;
use trade_domain::{FeeModel, LimitBreach, Position, Side, Verdict};
use trade_ports::testing::StubOrderExecutor;
use trade_risk::{KillSwitch, RiskContext, RiskGuard};

#[test]
fn acionado_nenhuma_ordem_e_aceita() {
    let mut rec = recorder();
    let mut g = RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        limits(),
        KillSwitch::engaged(),
    );
    let pos = comprado(dec!(1), dec!(100));
    for side in [Side::Buy, Side::Sell] {
        let r = g.submit(
            &order(1, side, dec!(1), at(1, 0)),
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
            Verdict::Rejected(LimitBreach::KillSwitchEngaged)
        );
        assert!(r.fill.is_none());
    }
}

#[test]
fn acionado_a_posicao_aberta_e_mantida_e_reportada() {
    let mut rec = recorder();
    // FR-023a: vender sob pânico em mercado desordenado pode custar mais que
    // a posição mantida. Liquidar é decisão do mantenedor, não do robô.
    let mut g = RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        limits(),
        KillSwitch::engaged(),
    );
    let pos = comprado(dec!(3), dec!(100));
    let r = g.submit(
        &order(1, Side::Sell, dec!(3), at(1, 0)),
        &RiskContext {
            position: &pos,
            balance: dec!(0),
            reference_price: dec!(100),
            now: at(1, 0),
            fees: FeeModel::default(),
        },
        &mut rec,
    );
    assert!(
        r.fill.is_none(),
        "o guard não liquida — nem sequer executa a venda"
    );
    assert_eq!(pos.qty(), dec!(3), "a posição segue intacta");
    assert_eq!(
        r.decision.state.exposure,
        dec!(300),
        "e é reportada na decisão"
    );
}

#[test]
fn e_acionavel_sem_alteracao_de_codigo_por_arquivo_sentinela() {
    let mut rec = recorder();
    // O ponto de FR-023: acionável de qualquer terminal, sem recompilar,
    // sem reiniciar, com efeito sobre um processo já em operação.
    let dir = tempdir().unwrap();
    let sentinela = dir.path().join("KILL");
    let ks = KillSwitch::sentinel(&sentinela);

    let mut g = RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        limits(),
        ks.clone(),
    );
    let pos = Position::default();
    let ctx = |now| RiskContext {
        position: &pos,
        balance: dec!(1_000_000),
        reference_price: dec!(100),
        now,
        fees: FeeModel::default(),
    };

    assert_eq!(
        g.submit(
            &order(1, Side::Buy, dec!(1), at(1, 0)),
            &ctx(at(1, 0)),
            &mut rec
        )
        .decision
        .verdict,
        Verdict::Accepted
    );

    ks.engage().unwrap();

    assert_eq!(
        g.submit(
            &order(2, Side::Buy, dec!(1), at(1, 1)),
            &ctx(at(1, 1)),
            &mut rec
        )
        .decision
        .verdict,
        Verdict::Rejected(LimitBreach::KillSwitchEngaged),
        "o mesmo guard, já em operação, passa a recusar"
    );
}

#[test]
fn liberar_e_ato_explicito() {
    let dir = tempdir().unwrap();
    let ks = KillSwitch::sentinel(dir.path().join("KILL"));
    ks.engage().unwrap();
    assert!(ks.is_engaged());
    ks.release().unwrap();
    assert!(!ks.is_engaged());
}
