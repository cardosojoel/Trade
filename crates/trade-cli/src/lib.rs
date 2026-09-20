//! Biblioteca do binário `trade`.
//!
//! O ponto de composição vive aqui, e não só em `main.rs`, para que seja
//! testável: um binário puro só se testa por fora, pelo processo, e isso é
//! lento demais para a maior parte do que precisa ser verificado.

pub mod cli;
pub mod cmd_backtest;
pub mod config;
pub mod report;
