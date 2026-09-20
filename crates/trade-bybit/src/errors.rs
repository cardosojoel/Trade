//! Tradução dos erros da Bybit.
//!
//! Nenhum código da corretora vaza desta crate (FR-010). Quem está fora vê
//! `MarketError::Unavailable`, não "retCode 10006" — é o que permite ao mesmo
//! tratamento de falha servir para o simulador de backtest e para a corretora
//! ao vivo, sem alteração no motor.

use trade_ports::MarketError;

/// Excesso de requisições. Documentado como `retCode: 10006`,
/// `retMsg: "Too many visits!"`, acompanhado de HTTP 403.
pub const RET_CODE_TOO_MANY_VISITS: i64 = 10006;

/// Se o erro é candidato a retentativa automática.
///
/// Candidato, não garantido: esgotado o número de tentativas, ele vira falha de
/// integridade (FR-024b). A decisão final não é desta crate.
pub fn is_transient(e: &MarketError) -> bool {
    matches!(e, MarketError::Unavailable(_))
}

pub fn too_many_visits() -> MarketError {
    MarketError::Unavailable("excesso de requisições na fonte (retCode 10006)".into())
}

pub fn http_status(status: u16) -> MarketError {
    match status {
        // 403 acompanha o excesso de requisições; 429 é o caso genérico.
        403 | 429 => too_many_visits(),
        500..=599 => MarketError::Unavailable(format!("fonte indisponível (HTTP {status})")),
        outro => MarketError::Source(format!("resposta HTTP {outro} da fonte")),
    }
}

pub fn ret_code(code: i64, msg: &str) -> MarketError {
    if code == RET_CODE_TOO_MANY_VISITS {
        too_many_visits()
    } else {
        MarketError::Source(format!("fonte recusou a consulta (código {code}): {msg}"))
    }
}
