//! Motor de backtest.
//!
//! Percorre o histórico vela a vela e executa o ciclo completo:
//! sinal → verificação de risco → ordem → atualização de posição.
//!
//! Três propriedades governam o desenho:
//!
//! - **Sem informação futura** (FR-030): o preenchimento acontece na vela
//!   *seguinte* à do sinal. A estratégia decide com o que existia no instante,
//!   não com o que veio depois.
//! - **Determinismo** (FR-029): o tempo vem do [`clock::BacktestClock`], nunca
//!   do relógio da máquina; não há paralelismo nem iteração sobre `HashMap` em
//!   caminho que afete resultado.
//! - **Memória constante**: as velas são percorridas por iterador preguiçoso.
//!   Um backtest de cinco anos usa a mesma memória que um de cinco dias.

pub mod clock;
pub mod engine;
pub mod executor;
pub mod run;

pub use engine::{BacktestConfig, BacktestEngine};
pub use executor::SimulatedExecutor;
pub use run::{BacktestOutcome, RunOutcome};
