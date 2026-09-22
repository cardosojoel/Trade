//! Biblioteca do binário `trade`.
//!
//! O ponto de composição vive aqui, e não só em `main.rs`, para que seja
//! testável: um binário puro só se testa por fora, pelo processo, e isso é
//! lento demais para a maior parte do que precisa ser verificado.

/// A versão do código que este binário é, para o registro (P6/P9).
///
/// Capturada na compilação por `build.rs`: identificador do commit, com a
/// marca `-sujo` quando a árvore tinha alteração não commitada. `None` quando
/// o binário não soube dizer — e nulo no registro significa "não sei", que é
/// diferente de qualquer valor que se pudesse inventar.
///
/// Decisão 043 do Jev (`commit_com_marca_de_sujo`, 0,98 · confiança 0,98).
pub fn versao_do_codigo() -> Option<&'static str> {
    let v = env!("TRADE_CODE_VERSION");
    if v.is_empty() { None } else { Some(v) }
}

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
