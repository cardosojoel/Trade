//! O estado com que uma sessão volta depois de um reinício (FR-112).
//!
//! Posição, ordens em aberto e contadores de risco vêm do registro, nunca da
//! memória de um processo que já morreu. O tipo vive aqui, e não na camada de
//! risco nem na de persistência, porque as duas precisam nomeá-lo e
//! `trade-risk` não pode declarar `trade-storage` — `tests/architecture.rs`
//! falha o build se declarar.

use crate::Money;
use crate::position::Position;
use chrono::{DateTime, NaiveDate, Utc};

/// O que o registro devolve ao início de uma sessão.
///
/// As três parcelas do resultado do dia vêm separadas porque é assim que o
/// contador de perda diária as mantém: o aberto entra por **variação** desde a
/// virada, e não por valor absoluto. Devolver só o total obrigaria a inventar
/// uma linha de base, e uma linha de base errada é um limite que não morde.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EstadoRetomado {
    pub position: Position,
    /// Primeiro `seq` livre, para que a numeração dos eventos continue sem
    /// buraco entre reinícios.
    pub proximo_seq: u64,
    /// Resultado **fechado** acumulado no dia corrente do registro.
    pub realized_today: Money,
    /// Último resultado **aberto** que o registro permite reconstruir.
    pub unrealized: Money,
    /// O aberto no instante da virada — a linha de base do dia.
    pub unrealized_at_day_start: Money,
    pub daily_loss_blocked: bool,
    pub day: Option<NaiveDate>,
    /// Instantes das ordens **aceitas** ainda dentro da janela deslizante.
    pub aceitas: Vec<DateTime<Utc>>,
}
