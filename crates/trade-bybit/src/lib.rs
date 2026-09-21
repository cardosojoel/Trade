//! Adaptador da Bybit.
//!
//! **Única crate do workspace com cliente HTTP.** É o que a trava em
//! `tests/architecture.rs` protege: estratégia, risco e backtest não declaram
//! esta crate nem `ureq`, e por isso não conseguem alcançar a rede.
//!
//! Escopo desta feature: apenas leitura de histórico público. Não há envio de
//! ordens e não há credencial — o endpoint `/v5/market/kline` é público, o que
//! mantém o Princípio VI intocado.

pub mod ao_vivo;
pub mod auth;
pub mod backoff;
pub mod client;
pub mod cliente_autenticado;
pub mod collector;
pub mod credencial;
pub mod errors;
pub mod gaps;
pub mod parse;

pub use ao_vivo::FonteAoVivo;
pub use client::{BybitClient, KlinePage};
pub use collector::{CollectOutcome, Collector};
