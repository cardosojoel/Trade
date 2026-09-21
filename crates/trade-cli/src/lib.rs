//! Biblioteca do binário `trade`.
//!
//! O ponto de composição vive aqui, e não só em `main.rs`, para que seja
//! testável: um binário puro só se testa por fora, pelo processo, e isso é
//! lento demais para a maior parte do que precisa ser verificado.

pub mod cli;
pub mod cmd_backtest;
pub mod cmd_collect;
pub mod cmd_paper;
pub mod cmd_paper_rodar;
pub mod cmd_perfil;
pub mod cmd_porta2;
pub mod cmd_serve;
pub mod config;
pub mod parada;
pub mod report;
