//! Persistência em SQLite.
//!
//! Dois arquivos com ciclos de vida distintos: `market.db` é cache
//! reconstruível da fonte, `runs.db` é auditoria insubstituível. Em um arquivo
//! só, "limpar o cache" e "destruir a auditoria" seriam a mesma operação — o
//! que o Princípio IV não admite.

pub mod db;
pub mod decimal_sql;

pub use db::{open_market, open_runs};
