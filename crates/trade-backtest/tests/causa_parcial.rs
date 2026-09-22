//! T055 — a causa de um preenchimento parcial fica registrada (P3, achado 2).
//!
//! O achado: na execução `01M2ZG2N88…`, **quatro ordens em 28.618** foram
//! preenchidas em parte, e nenhum evento registrava a causa. A ordem 65 pediu
//! 0,008553 e recebeu 0,004391 — 51,3%; as outras três, 82,3%, 84,0% e 97,8%.
//! Não havia `anomaly` nem `halt` na execução inteira.
//!
//! O Princípio IV exige reconstituir a decisão a partir do registro, e um
//! preenchimento parcial sem causa registrada não se reconstitui. A causa
//! **era** conhecida — só não era gravada: quem preenche sabe por que
//! preencheu menos.

use rust_decimal::dec;
use trade_backtest::executor::{CandleFeed, SimulatedExecutor};
use trade_domain::{CausaParcial, FeeModel, Side};
use trade_domain::{Order, OrderId, SignalId};
use trade_ports::OrderExecutor;

fn ordem(id: u64, side: Side, qty: rust_decimal::Decimal) -> Order {
    use chrono::TimeZone;
    Order {
        id: OrderId(id),
        signal_ref: SignalId(id),
        side,
        qty,
        at: chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
    }
}

fn vela(volume: rust_decimal::Decimal) -> trade_domain::Candle {
    use chrono::TimeZone;
    trade_domain::Candle {
        open_time: chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        open: dec!(100),
        high: dec!(100),
        low: dec!(100),
        close: dec!(100),
        volume,
        turnover: dec!(100),
    }
}

fn sem_custo() -> FeeModel {
    FeeModel {
        taker_fee_rate: rust_decimal::Decimal::ZERO,
        slippage_rate: rust_decimal::Decimal::ZERO,
    }
}

#[test]
fn volume_insuficiente_preenche_em_parte_e_diz_por_que() {
    // A ordem pede 10, a vela tem 4. O que sai é 4, e o registro passa a
    // dizer que foi o volume da vela — em vez de deixar quem lê deduzir.
    let feed = CandleFeed::default();
    feed.set(vela(dec!(4)));
    let mut e = SimulatedExecutor::new(sem_custo(), feed);

    let f = e.execute(&ordem(1, Side::Buy, dec!(10))).unwrap();

    assert_eq!(f.qty, dec!(4));
    assert_eq!(
        f.causa_parcial,
        Some(CausaParcial::VolumeDaVela),
        "a causa é conhecida por quem preenche"
    );
}

#[test]
fn volume_suficiente_nao_tem_causa_parcial() {
    // `None` significa "não foi parcial", e não "não sei por quê". A
    // diferença é o que torna o campo útil.
    let feed = CandleFeed::default();
    feed.set(vela(dec!(100)));
    let mut e = SimulatedExecutor::new(sem_custo(), feed);

    let f = e.execute(&ordem(1, Side::Buy, dec!(10))).unwrap();

    assert_eq!(f.qty, dec!(10));
    assert_eq!(f.causa_parcial, None);
}

#[test]
fn a_proporcao_preenchida_se_reconstitui_do_registro() {
    // O caso da ordem 65: pediu 0,008553 e recebeu 0,004391, 51,3%. Com a
    // causa gravada, a pergunta "por que só metade?" passa a ter resposta
    // sem abrir o código.
    let feed = CandleFeed::default();
    feed.set(vela(dec!(0.004391)));
    let mut e = SimulatedExecutor::new(sem_custo(), feed);

    let f = e.execute(&ordem(65, Side::Buy, dec!(0.008553))).unwrap();

    assert_eq!(f.qty, dec!(0.004391));
    assert_eq!(f.causa_parcial, Some(CausaParcial::VolumeDaVela));
}
