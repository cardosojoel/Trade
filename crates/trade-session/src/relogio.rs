//! O relógio de uma sessão ao vivo (FR-115).

use chrono::{DateTime, Utc};
use trade_ports::Clock;

/// Devolve a hora do mundo.
///
/// Contraparte do relógio de backtest, que devolve instante simulado. Os dois
/// implementam a mesma trait e são intercambiáveis no tipo — o que os separa
/// é o grafo de dependências: esta crate não declara `trade-backtest`, e por
/// isso o relógio simulado é **inalcançável** daqui. FR-115 deixa de depender
/// de ninguém escolher errado.
#[derive(Debug, Clone, Copy, Default)]
pub struct RelogioReal;

impl Clock for RelogioReal {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devolve_a_hora_do_mundo() {
        let antes = Utc::now();
        let agora = RelogioReal.now();
        let depois = Utc::now();
        assert!(agora >= antes && agora <= depois);
    }
}
