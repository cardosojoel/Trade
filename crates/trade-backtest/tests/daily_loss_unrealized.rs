//! FR-019a — a perda diária conta o resultado ainda **aberto**.
//!
//! A camada de risco já sabe somar realizado e não realizado; estes testes
//! cobrem a outra metade: o motor precisa marcar a posição a mercado a cada
//! vela, senão o contador nunca recebe preço e o freio nunca dispara.

mod common;

use common::*;
use rust_decimal::dec;
use trade_backtest::BacktestEngine;
use trade_domain::{Intent, LimitBreach, RiskLimits};

fn limites(perda_maxima: rust_decimal::Decimal) -> RiskLimits {
    RiskLimits {
        max_daily_loss: perda_maxima,
        ..limites_folgados()
    }
}

#[test]
fn posicao_aberta_afundando_aciona_o_freio_sem_nenhuma_venda() {
    // Compra 5 a 100 e o preço cai a 80: -100 aberto, exatamente o limite.
    // Nenhuma venda acontece — o prejuízo nunca é realizado.
    let candles = velas(&[
        (0, dec!(100)),
        (1, dec!(100)),
        (2, dec!(80)),
        (3, dec!(80)),
        (4, dec!(80)),
    ]);
    let mut estrategia = ScriptedStrategy::new(vec![
        (0, Intent::Buy, Some(dec!(5))),
        // Tentativa de aumentar a posição depois da queda.
        (2, Intent::Buy, Some(dec!(1))),
    ]);
    let mut rec = recorder();

    let r = BacktestEngine::new(config(dec!(1000), 60, dec!(0), dec!(0), limites(dec!(100))))
        .run(
            &mut estrategia,
            &fonte(candles),
            KillSwitch::disarmed(),
            &mut rec,
        )
        .unwrap();

    assert!(
        r.rejections
            .iter()
            .any(|(b, _)| *b == LimitBreach::DailyLossReached),
        "prejuízo aberto de 100 com limite 100 precisa barrar a compra seguinte; veio {:?}",
        r.rejections
    );
    assert!(
        r.trades.is_empty(),
        "nada foi vendido — o freio veio do resultado aberto, não de operação fechada"
    );
}

#[test]
fn queda_dentro_do_limite_nao_barra_a_compra() {
    // Mesmo roteiro, queda menor: -50 aberto com limite 100. O freio não é
    // trigger-happy, e este teste é o que garante isso.
    let candles = velas(&[
        (0, dec!(100)),
        (1, dec!(100)),
        (2, dec!(90)),
        (3, dec!(90)),
        (4, dec!(90)),
    ]);
    let mut estrategia = ScriptedStrategy::new(vec![
        (0, Intent::Buy, Some(dec!(5))),
        (2, Intent::Buy, Some(dec!(1))),
    ]);
    let mut rec = recorder();

    let r = BacktestEngine::new(config(dec!(1000), 60, dec!(0), dec!(0), limites(dec!(100))))
        .run(
            &mut estrategia,
            &fonte(candles),
            KillSwitch::disarmed(),
            &mut rec,
        )
        .unwrap();

    assert!(
        r.rejections.is_empty(),
        "-50 aberto ainda cabe na cerca; veio {:?}",
        r.rejections
    );
    assert_eq!(r.final_position.qty(), dec!(6), "as duas compras passaram");
}

#[test]
fn bloqueado_pelo_aberto_a_saida_continua_possivel() {
    // O freio impede abrir, não impede sair. Barrar a venda prenderia o robô
    // dentro do prejuízo que acionou o freio.
    let candles = velas(&[
        (0, dec!(100)),
        (1, dec!(100)),
        (2, dec!(80)),
        (3, dec!(80)),
        (4, dec!(80)),
    ]);
    let mut estrategia = ScriptedStrategy::new(vec![
        (0, Intent::Buy, Some(dec!(5))),
        (2, Intent::Sell, Some(dec!(5))),
    ]);
    let mut rec = recorder();

    let r = BacktestEngine::new(config(dec!(1000), 60, dec!(0), dec!(0), limites(dec!(100))))
        .run(
            &mut estrategia,
            &fonte(candles),
            KillSwitch::disarmed(),
            &mut rec,
        )
        .unwrap();

    assert_eq!(
        r.trades.len(),
        1,
        "a venda passou mesmo com o freio acionado"
    );
    assert!(r.final_position.is_flat());
}
