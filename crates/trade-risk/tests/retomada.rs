//! T025 — os contadores de risco sobrevivem ao reinício (FR-112).
//!
//! O outro lado da recuperação: o registro devolve o estado, e o guard precisa
//! **voltar com ele**. Um contador recuperado que não chega ao guard é um
//! contador que não existe.

mod common;

use chrono::{Duration, NaiveDate};
use common::*;
use rust_decimal::dec;
use trade_domain::{EstadoRetomado, FeeModel, LimitBreach, Position, Side, Verdict};
use trade_ports::testing::StubOrderExecutor;
use trade_risk::{KillSwitch, RiskContext, RiskGuard};

fn estado() -> EstadoRetomado {
    EstadoRetomado {
        position: Position::default(),
        proximo_seq: 1,
        realized_today: dec!(0),
        unrealized: dec!(0),
        unrealized_at_day_start: dec!(0),
        daily_loss_blocked: false,
        day: Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
        aceitas: Vec::new(),
    }
}

fn retomado(e: EstadoRetomado) -> RiskGuard<StubOrderExecutor> {
    RiskGuard::retomar(
        StubOrderExecutor::always_fills_at(dec!(100)),
        limits(),
        KillSwitch::disarmed(),
        &e,
    )
}

#[test]
fn o_freio_volta_travado() {
    // O bloqueio só cai na virada do dia. Reiniciar o processo não é virada
    // de dia, e não pode ser a forma barata de destravá-lo.
    let g = retomado(EstadoRetomado {
        daily_loss_blocked: true,
        realized_today: dec!(-200),
        ..estado()
    });
    assert!(g.daily_loss_blocked());
}

#[test]
fn a_ordem_recuperada_conta_na_janela() {
    let mut g = retomado(EstadoRetomado {
        aceitas: (0..10).map(|i| at(1, 9) + Duration::minutes(i)).collect(),
        ..estado()
    });
    // A cerca dos testes admite dez ordens por janela de sessenta minutos, e
    // dez já voltaram do registro: a décima primeira é recusada.
    let mut rec = recorder();
    let pos = Position::default();
    let r = g.submit(
        &order(11, Side::Buy, dec!(1), at(1, 9) + Duration::minutes(30)),
        &RiskContext {
            position: &pos,
            balance: dec!(100000),
            reference_price: dec!(100),
            now: at(1, 9) + Duration::minutes(30),
            fees: FeeModel::default(),
        },
        &mut rec,
    );
    assert_eq!(
        r.decision.verdict,
        Verdict::Rejected(LimitBreach::MaxOrdersPerWindow),
        "reiniciar não zera a janela deslizante"
    );
}

#[test]
fn a_perda_ocorrida_durante_a_parada_conta_contra_o_dia() {
    // Decisão 030, o caso concreto que a decidiu. O processo caiu com a
    // posição valendo menos 10 no dia; voltou com ela valendo menos 300. A
    // diferença aconteceu com o robô desligado, e ainda assim é perda **do
    // dia** — contá-la é o que separa um limite de um limite decorativo.
    let mut g = retomado(EstadoRetomado {
        position: comprado(dec!(2), dec!(100)),
        realized_today: dec!(0),
        unrealized: dec!(-10),
        unrealized_at_day_start: dec!(0),
        ..estado()
    });
    assert!(
        !g.daily_loss_blocked(),
        "ao voltar, o freio ainda está solto"
    );

    g.mark_to_market(dec!(-300));
    assert!(
        g.daily_loss_blocked(),
        "menos 300 contra o limite de 200: o freio aciona na primeira remarcação"
    );
}

#[test]
fn a_linha_de_base_recuperada_nao_deixa_a_perda_de_ontem_contar_hoje() {
    // O aberto entra por **variação** desde a virada. Uma posição que já
    // estava perdendo 500 na virada não pode acionar o freio de hoje só por
    // continuar onde estava.
    let mut g = retomado(EstadoRetomado {
        position: comprado(dec!(2), dec!(100)),
        realized_today: dec!(0),
        unrealized: dec!(-500),
        unrealized_at_day_start: dec!(-500),
        ..estado()
    });
    g.mark_to_market(dec!(-500));
    assert!(
        !g.daily_loss_blocked(),
        "o prejuízo herdado de ontem não é perda de hoje"
    );

    g.mark_to_market(dec!(-701));
    assert!(
        g.daily_loss_blocked(),
        "a variação de 201 desde a virada é que atinge o limite de 200"
    );
}

#[test]
fn o_dia_recuperado_nao_e_virada_de_dia() {
    // Se o dia recuperado já é o corrente, `on_day_boundary` não pode
    // zerar contador nenhum: o reinício não inventa um dia novo.
    let mut g = retomado(EstadoRetomado {
        daily_loss_blocked: true,
        realized_today: dec!(-200),
        ..estado()
    });
    let mut rec = recorder();
    assert!(
        !g.on_day_boundary(at(1, 15), &mut rec),
        "mesmo dia: não virou"
    );
    assert!(g.daily_loss_blocked(), "e o freio continua travado");

    assert!(g.on_day_boundary(at(2, 0), &mut rec), "dia seguinte: virou");
    assert!(!g.daily_loss_blocked(), "e aí sim o freio cai sozinho");
}
