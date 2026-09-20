//! SC-003 — a estratégia imprudente não ultrapassa nenhum limite.
//!
//! Não usa motor de backtest: as ordens são submetidas direto ao guard, que é
//! exatamente o que torna a US2 testável de forma independente.

mod common;

use common::*;
use rust_decimal::{Decimal, dec};
use trade_domain::{AuditKind, FeeModel, LimitBreach, Position, Side, Verdict};
use trade_ports::Recorder;
use trade_ports::testing::StubOrderExecutor;
use trade_risk::{KillSwitch, RiskContext, RiskGuard};

#[test]
fn nenhuma_tentativa_imprudente_ultrapassa_a_cerca() {
    let mut rec = recorder();
    let mut g = RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        limits(),
        KillSwitch::disarmed(),
    );

    let mut posicao = Position::default();
    let mut saldo = dec!(10_000);
    let mut recusas = 0;
    let mut aceitas = 0;

    // Cem tentativas: tamanho absurdo, rajada de ordens, insistência após
    // perda. Nenhuma pode passar da cerca.
    for i in 1..=100u64 {
        let hora = (i % 24) as u32;
        // Alterna entre uma ordem pagável e uma absurda: sem as pagáveis, o
        // teste só exercitaria saldo insuficiente e nunca chegaria perto do
        // teto de posição.
        let qty = if i % 2 == 0 { dec!(500) } else { dec!(1) };
        let ordem = order(i, Side::Buy, qty, at(1, hora));

        let r = g.submit(
            &ordem,
            &RiskContext {
                position: &posicao,
                balance: saldo,
                reference_price: dec!(100),
                now: at(1, hora),
                fees: FeeModel::default(),
            },
            &mut rec,
        );

        match r.decision.verdict {
            Verdict::Accepted => {
                aceitas += 1;
                let fill = r.fill.expect("aceita sem preenchimento");
                saldo -= fill.price * fill.qty;
                posicao.apply_fill(Side::Buy, &fill).unwrap();
            }
            Verdict::Rejected(_) => recusas += 1,
        }

        // A invariante verificada a cada passo, não só no fim.
        let valor = posicao.exposure_at(dec!(100));
        assert!(
            valor <= limits().max_position_size,
            "posição {valor} passou do teto na tentativa {i}"
        );
        assert!(
            valor <= limits().max_total_exposure,
            "exposição estourou na tentativa {i}"
        );
        assert!(saldo >= Decimal::ZERO, "saldo negativo na tentativa {i}");
        assert!(posicao.qty() >= Decimal::ZERO);
    }

    assert!(recusas > 0, "o teste não exercitou recusa nenhuma");
    assert!(aceitas > 0, "o teste não exercitou aceitação nenhuma");
    assert_eq!(aceitas + recusas, 100);
}

#[test]
fn toda_ordem_tem_decisao_registrada() {
    // SC-002 verificado por contagem: é consequência de submit exigir o
    // registrador e gravar a decisão antes de qualquer execução.
    let mut rec = recorder();
    let mut g = RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        limits(),
        KillSwitch::disarmed(),
    );
    let posicao = Position::default();

    for i in 1..=30u64 {
        g.submit(
            &order(i, Side::Buy, dec!(500), at(1, 0)),
            &RiskContext {
                position: &posicao,
                balance: dec!(10_000),
                reference_price: dec!(100),
                now: at(1, 0),
                fees: FeeModel::default(),
            },
            &mut rec,
        );
    }

    let decisoes = rec.sink().count("risk_decision");
    assert_eq!(decisoes, 30, "trinta ordens, trinta decisões");
    assert_eq!(
        rec.seq() as usize,
        rec.sink().events.len(),
        "seq sem buraco"
    );
}

#[test]
fn a_recusa_registrada_diz_qual_limite_e_contra_o_que() {
    let mut rec = recorder();
    let mut g = RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        limits(),
        KillSwitch::disarmed(),
    );
    let posicao = Position::default();

    // Saldo folgado de propósito: com saldo curto, a recusa viria por
    // InsufficientBalance e este teste não exercitaria o teto de posição.
    g.submit(
        &order(1, Side::Buy, dec!(500), at(1, 0)),
        &RiskContext {
            position: &posicao,
            balance: dec!(1_000_000),
            reference_price: dec!(100),
            now: at(1, 0),
            fees: FeeModel::default(),
        },
        &mut rec,
    );

    let evento = rec.sink().events.first().expect("nenhum evento registrado");
    match &evento.kind {
        AuditKind::RiskDecision(d) => {
            assert_eq!(d.verdict, Verdict::Rejected(LimitBreach::MaxPositionSize));
            assert_eq!(
                d.limits,
                limits(),
                "sem o retrato dos limites, a recusa é indecifrável"
            );
            assert_eq!(d.state.exposure, Decimal::ZERO);
        }
        outro => panic!("esperado risk_decision, veio {}", outro.as_str()),
    }
}

#[test]
fn a_retomada_automatica_e_registrada_como_automatica() {
    // SC-015 pelo lado positivo: a retomada que pode ser automática é marcada
    // como tal, e por isso uma consulta consegue distinguir das que não podem.
    let mut rec = recorder();
    let mut g = RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        limits(),
        KillSwitch::disarmed(),
    );

    g.on_day_boundary(at(1, 0), &mut rec);
    g.record_realized(dec!(-500));
    assert!(g.daily_loss_blocked());
    g.on_day_boundary(at(2, 0), &mut rec);

    let resumes: Vec<_> = rec
        .sink()
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            AuditKind::Resume { trigger, automatic } => Some((trigger.clone(), *automatic)),
            _ => None,
        })
        .collect();

    assert_eq!(resumes, vec![("DayBoundary".to_string(), true)]);
}
