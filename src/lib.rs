//! Raiz do workspace Trade.
//!
//! Este pacote não contém lógica. Ele existe para hospedar os testes de
//! arquitetura em `tests/`, que verificam mecanicamente as fronteiras que a
//! constitution exige:
//!
//! - `tests/architecture.rs` — estratégia, risco e backtest não alcançam a
//!   corretora nem a rede (Princípio V, FR-018)
//! - `tests/no_float.rs` — nenhum ponto flutuante em caminho monetário (R-002)
//!
//! A lógica vive em `crates/`.
