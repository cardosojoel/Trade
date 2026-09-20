//! FR-032 — capital esgotado encerra de forma controlada.
//!
//! Em spot comprado e sem alavancagem o capital nunca chega a zero: sempre
//! resta posição valendo algo. O que existe é um **piso** abaixo do qual
//! continuar operando não faz sentido, e ele é configuração (`min_equity`),
//! não constante de código.

mod common;

use common::*;
use rust_decimal::dec;
use trade_backtest::{BacktestEngine, RunOutcome};
use trade_domain::Intent;

#[test]
fn capital_esgotado_encerra_e_reporta_o_instante() {
    // Compra tudo e o preço despenca para quase zero; ao vender, sobra pó.
    let candles = velas(&[
        (0, dec!(100)),
        (1, dec!(100)),
        (2, dec!(0.01)),
        (3, dec!(0.01)),
        (4, dec!(0.01)),
        (5, dec!(0.01)),
    ]);
    let mut estrategia = ScriptedStrategy::new(vec![
        (0, Intent::Buy, Some(dec!(10))),
        (2, Intent::Sell, Some(dec!(10))),
    ]);
    let mut rec = recorder();

    let r = BacktestEngine::new(config(dec!(1000), 60, dec!(0), dec!(0), limites_folgados()))
        .run(
            &mut estrategia,
            &fonte(candles),
            KillSwitch::disarmed(),
            &mut rec,
        )
        .unwrap();

    match r.outcome {
        RunOutcome::CapitalExhausted { at } => {
            // Minuto 2, não 3: o capital acabou quando o preço desabou, não
            // quando a venda (já impossível) se liquidaria. Reportar o instante
            // da venda esconderia um minuto inteiro operando quebrado.
            assert_eq!(at, t(2), "o instante exato precisa ser reportado");
        }
        outro => panic!("esperado capital esgotado, veio {outro:?}"),
    }
}

#[test]
fn periodo_completo_sem_esgotar_termina_como_completed() {
    let candles = velas(&[(0, dec!(100)), (1, dec!(100)), (2, dec!(100))]);
    let mut estrategia = ScriptedStrategy::new(vec![]);
    let mut rec = recorder();

    let r = BacktestEngine::new(config(dec!(1000), 60, dec!(0), dec!(0), limites_folgados()))
        .run(
            &mut estrategia,
            &fonte(candles),
            KillSwitch::disarmed(),
            &mut rec,
        )
        .unwrap();

    assert_eq!(r.outcome, RunOutcome::Completed);
}
