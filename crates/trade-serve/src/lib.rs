//! Servidor HTTP local de leitura do registro.
//!
//! Onze rotas em `127.0.0.1`. Dez leem; uma escreve, e mesmo essa não emite
//! ordem — inicia uma execução pela composição que a CLI fornece.
//!
//! **De onde esta crate não alcança** é o que a define. Ela não declara a
//! corretora, não declara a camada de risco, não declara o motor de backtest e
//! não declara a própria CLI. Está na lista `ISOLATED` de
//! `tests/architecture.rs`, e o build falha se qualquer uma dessas linhas for
//! cruzada — é assim que o `SC-008` deixa de depender de revisão humana.

pub mod consultas;
pub mod erros;
pub mod guarda;
pub mod respostas;
pub mod servidor;

pub use erros::{Motivo, Recusa};
pub use guarda::Guarda;
pub use servidor::Servidor;
