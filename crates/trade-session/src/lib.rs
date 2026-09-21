//! O laço de uma sessão ao vivo.
//!
//! Mesma forma do motor de backtest, e pelo mesmo motivo: a crate declara a
//! camada de risco e o domínio, e **não** declara corretora nem cliente de
//! rede. O executor chega já movido para dentro do [`trade_risk::RiskGuard`],
//! de modo que não existe, nesta crate, tipo que alcance o mercado por fora da
//! cerca. `tests/architecture.rs` falha o build se a fronteira for cruzada.
//!
//! O que muda em relação ao backtest é o que a sessão **não** controla: as
//! velas chegam quando o mercado as fecha, o relógio é o do mundo, e o fim não
//! é uma data — é a fonte se encerrar, uma falha de integridade ou alguém
//! mandar parar.

pub mod laco;
pub mod relogio;

pub use laco::{Ambiente, FimDaSessao, Sessao, SessaoConfig, SessaoInicio, SessaoOutcome};
pub use relogio::RelogioReal;
