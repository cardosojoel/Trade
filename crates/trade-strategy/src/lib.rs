//! Estratégias.
//!
//! Esta crate **não depende de `trade-ports`**, e é por isso que ela não
//! consegue nomear `OrderExecutor`. A estratégia devolve intenção; quem
//! converte intenção em ordem é o motor, e o caminho do motor até o mercado
//! atravessa a camada de risco.
//!
//! Verificado por `tests/architecture.rs` na raiz do workspace.

#[cfg(feature = "testing")]
pub mod reckless;
