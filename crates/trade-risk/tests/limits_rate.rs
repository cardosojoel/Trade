//! FR-019 — número máximo de ordens por janela.

mod common;

use common::*;
use rust_decimal::dec;
use trade_domain::{FeeModel, LimitBreach, Position, RiskLimits, Side, Verdict};
use trade_ports::testing::StubOrderExecutor;
use trade_risk::{KillSwitch, RiskContext, RiskGuard};

fn guard_com_teto(n: u32) -> RiskGuard<StubOrderExecutor> {
    RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        RiskLimits {
            max_orders_per_window: n,
            window_minutes: 60,
            ..limits()
        },
        KillSwitch::disarmed(),
    )
}

fn compra(g: &mut RiskGuard<StubOrderExecutor>, id: u64, hora: u32) -> trade_domain::Verdict {
    let mut rec = recorder();
    let pos = Position::default();
    g.submit(
        &order(id, Side::Buy, dec!(1), at(1, hora)),
        &RiskContext {
            position: &pos,
            balance: dec!(1_000_000),
            reference_price: dec!(100),
            now: at(1, hora),
            fees: FeeModel::default(),
        },
        &mut rec,
    )
    .decision
    .verdict
}

#[test]
fn ordens_ate_o_teto_sao_aceitas_e_a_seguinte_nao() {
    let mut g = guard_com_teto(3);
    for id in 1..=3 {
        assert_eq!(compra(&mut g, id, 0), Verdict::Accepted, "ordem {id}");
    }
    assert_eq!(
        compra(&mut g, 4, 0),
        Verdict::Rejected(LimitBreach::MaxOrdersPerWindow)
    );
}

#[test]
fn a_janela_desliza_com_o_tempo() {
    let mut g = guard_com_teto(2);
    assert_eq!(compra(&mut g, 1, 0), Verdict::Accepted);
    assert_eq!(compra(&mut g, 2, 0), Verdict::Accepted);
    assert_eq!(
        compra(&mut g, 3, 0),
        Verdict::Rejected(LimitBreach::MaxOrdersPerWindow)
    );
    // Duas horas depois, a janela de 60 minutos já não contém as anteriores.
    assert_eq!(compra(&mut g, 4, 2), Verdict::Accepted);
}

#[test]
fn ordens_recusadas_nao_consomem_a_janela() {
    let mut rec = recorder();
    // O limite existe para delimitar atividade no mercado, e ordem recusada
    // nunca chega lá. Contá-la puniria a estratégia duas vezes pelo mesmo erro.
    let mut g = guard_com_teto(2);
    let pos = Position::default();
    for id in 1..=5 {
        g.submit(
            &order(id, Side::Buy, dec!(99_999), at(1, 0)),
            &RiskContext {
                position: &pos,
                balance: dec!(1),
                reference_price: dec!(100),
                now: at(1, 0),
                fees: FeeModel::default(),
            },
            &mut rec,
        );
    }
    assert_eq!(
        compra(&mut g, 6, 0),
        Verdict::Accepted,
        "as cinco recusas não consumiram a janela"
    );
}
