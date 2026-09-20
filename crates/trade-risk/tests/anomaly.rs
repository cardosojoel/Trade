//! FR-024a e FR-024b — classificação de anomalia e retentativa.

mod common;

use common::*;
use rust_decimal::dec;
use trade_domain::{Anomaly, IntegrityCause, Position, Side, Verdict};
use trade_ports::ExecError;
use trade_ports::testing::StubOrderExecutor;
use trade_risk::anomaly::{classify, implausible_price, position_divergence};
use trade_risk::{KillSwitch, RiskContext, RiskGuard};

#[test]
fn indisponibilidade_e_transitoria_ate_esgotar_as_tentativas() {
    assert_eq!(
        classify(&ExecError::Unavailable("x".into()), 1, 3),
        Some(Anomaly::Transient {
            attempt: 1,
            max_retries: 3
        })
    );
    assert_eq!(
        classify(&ExecError::Unavailable("x".into()), 3, 3),
        Some(Anomaly::Integrity(IntegrityCause::RetriesExhausted)),
        "esgotado o número de tentativas, deixa de ser transitória"
    );
}

#[test]
fn tempo_esgotado_tambem_e_transitorio() {
    assert!(matches!(
        classify(&ExecError::Timeout, 1, 3),
        Some(Anomaly::Transient { .. })
    ));
}

#[test]
fn divergencia_reportada_nunca_e_transitoria() {
    assert_eq!(
        classify(&ExecError::Integrity("posição divergente".into()), 1, 99),
        Some(Anomaly::Integrity(IntegrityCause::PositionDivergence)),
        "seguir operando aqui custa dinheiro — não há tentativa que conserte"
    );
}

#[test]
fn recusa_da_corretora_nao_e_anomalia() {
    // Uma ordem recusada é uma ordem que falhou, não um motivo para parar o
    // robô. Retentar não mudaria o resultado.
    assert_eq!(
        classify(&ExecError::Rejected("preço fora do book".into()), 1, 3),
        None
    );
}

#[test]
fn integridade_exige_ato_humano_e_transitoria_nao() {
    assert!(Anomaly::Integrity(IntegrityCause::PositionDivergence).requires_human());
    assert!(
        !Anomaly::Transient {
            attempt: 1,
            max_retries: 3
        }
        .requires_human()
    );
}

#[test]
fn preco_fora_da_faixa_e_implausivel() {
    // 20% de variação entre velas de um minuto em BTC não é movimento, é dado ruim.
    assert!(implausible_price(dec!(130), dec!(100), dec!(0.20)).is_some());
    assert!(implausible_price(dec!(70), dec!(100), dec!(0.20)).is_some());
    assert!(implausible_price(dec!(119), dec!(100), dec!(0.20)).is_none());
}

#[test]
fn preco_implausivel_e_falha_de_integridade() {
    assert_eq!(
        implausible_price(dec!(200), dec!(100), dec!(0.20)),
        Some(Anomaly::Integrity(IntegrityCause::ImplausiblePrice))
    );
}

#[test]
fn divergencia_de_posicao_e_detectada() {
    let pos = comprado(dec!(2), dec!(100));
    assert!(position_divergence(&pos, dec!(2)).is_none());
    assert_eq!(
        position_divergence(&pos, dec!(1.9)),
        Some(Anomaly::Integrity(IntegrityCause::PositionDivergence))
    );
}

#[test]
fn falha_transitoria_retenta_sozinha_e_prossegue() {
    let mut rec = recorder();
    // Duas falhas seguidas, depois sucesso. Com teto de 3, a operação não é
    // interrompida e nenhuma intervenção é necessária.
    let mut g = RiskGuard::new(
        StubOrderExecutor::fails_then_fills(2, dec!(100)),
        limits(),
        KillSwitch::disarmed(),
    );
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
    assert_eq!(r.decision.verdict, Verdict::Accepted);
    assert!(r.fill.is_some(), "retentou sozinha e conseguiu");
    assert_eq!(r.attempts, 3);
    assert!(r.anomaly.as_ref().is_none_or(|a| !a.requires_human()));
}

#[test]
fn tentativas_esgotadas_viram_falha_de_integridade() {
    let mut rec = recorder();
    let mut g = RiskGuard::new(
        StubOrderExecutor::fails_then_fills(10, dec!(100)),
        limits(),
        KillSwitch::disarmed(),
    );
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
    assert!(r.fill.is_none());
    assert_eq!(
        r.anomaly,
        Some(Anomaly::Integrity(IntegrityCause::RetriesExhausted))
    );
    assert!(
        r.anomaly.unwrap().requires_human(),
        "a partir daqui, só com ato humano"
    );
}
