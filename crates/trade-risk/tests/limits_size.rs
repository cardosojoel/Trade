//! FR-019a e FR-020 — tamanho máximo de posição.

mod common;

use common::*;
use rust_decimal::dec;
use trade_domain::{LimitBreach, Position, Side, Verdict};
use trade_ports::testing::StubOrderExecutor;
use trade_risk::{KillSwitch, RiskContext, RiskGuard};

fn ctx<'a>(pos: &'a Position, preco: rust_decimal::Decimal) -> RiskContext<'a> {
    RiskContext {
        position: pos,
        balance: dec!(1_000_000),
        reference_price: preco,
        now: at(1, 0),
    }
}

#[test]
fn valor_exatamente_no_limite_e_aceito() {
    let mut rec = recorder();
    // max_position_size = 1000. Comprar 10 a 100 dá exatamente 1000.
    let mut g = RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        limits(),
        KillSwitch::disarmed(),
    );
    let pos = Position::default();
    let r = g.submit(
        &order(1, Side::Buy, dec!(10), at(1, 0)),
        &ctx(&pos, dec!(100)),
        &mut rec,
    );
    assert_eq!(
        r.decision.verdict,
        Verdict::Accepted,
        "o limite é valor permitido, não proibido"
    );
}

#[test]
fn acima_do_limite_e_recusado() {
    let mut rec = recorder();
    let mut g = RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        limits(),
        KillSwitch::disarmed(),
    );
    let pos = Position::default();
    let r = g.submit(
        &order(1, Side::Buy, dec!(11), at(1, 0)),
        &ctx(&pos, dec!(100)),
        &mut rec,
    );
    assert_eq!(
        r.decision.verdict,
        Verdict::Rejected(LimitBreach::MaxPositionSize)
    );
}

#[test]
fn ordem_recusada_nao_e_reduzida_ao_teto() {
    let mut rec = recorder();
    // FR-020: recusar, não ajustar silenciosamente para caber. Uma ordem
    // reduzida sem aviso executa algo que a estratégia não pediu.
    let mut g = RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        limits(),
        KillSwitch::disarmed(),
    );
    let pos = Position::default();
    let r = g.submit(
        &order(1, Side::Buy, dec!(50), at(1, 0)),
        &ctx(&pos, dec!(100)),
        &mut rec,
    );
    assert!(r.fill.is_none(), "não pode haver preenchimento algum");
}

#[test]
fn ordem_recusada_nunca_alcanca_o_executor() {
    let mut rec = recorder();
    // A afirmação forte: não é que a ordem foi recusada, é que ela nunca
    // chegou ao mercado.
    // Executor sem nenhuma resposta roteirizada: se for alcançado, devolve
    // erro e o número de tentativas passa de zero. Se não for, ambos ficam
    // limpos — e é isso que separa "recusada" de "nunca chegou ao mercado".
    let mut g = RiskGuard::new(
        StubOrderExecutor::scripted(vec![]),
        limits(),
        KillSwitch::disarmed(),
    );
    let pos = Position::default();
    let r = g.submit(
        &order(1, Side::Buy, dec!(999), at(1, 0)),
        &ctx(&pos, dec!(100)),
        &mut rec,
    );

    assert_eq!(r.attempts, 0, "nenhuma tentativa de execução");
    assert!(
        r.error.is_none(),
        "o executor não chegou nem a ser consultado"
    );
    assert!(r.fill.is_none());
}

#[test]
fn posicao_existente_conta_para_o_limite() {
    let mut rec = recorder();
    // Já comprado 6 a 100 (valor 600); comprar mais 5 levaria a 1100.
    let mut g = RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        limits(),
        KillSwitch::disarmed(),
    );
    let pos = comprado(dec!(6), dec!(100));
    let r = g.submit(
        &order(1, Side::Buy, dec!(5), at(1, 0)),
        &ctx(&pos, dec!(100)),
        &mut rec,
    );
    assert_eq!(
        r.decision.verdict,
        Verdict::Rejected(LimitBreach::MaxPositionSize)
    );
}

#[test]
fn decisao_carrega_retrato_dos_limites_e_do_estado() {
    let mut rec = recorder();
    let mut g = RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        limits(),
        KillSwitch::disarmed(),
    );
    let pos = comprado(dec!(2), dec!(100));
    let r = g.submit(
        &order(1, Side::Buy, dec!(99), at(1, 0)),
        &ctx(&pos, dec!(100)),
        &mut rec,
    );
    assert_eq!(
        r.decision.limits,
        limits(),
        "sem o retrato, a recusa é indecifrável depois"
    );
    assert_eq!(r.decision.state.exposure, dec!(200));
}
