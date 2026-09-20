//! Tentativa de obter o executor de dentro do RiskGuard.
//!
//! Este arquivo **não deve compilar**. É a forma de provar que FR-018 é uma
//! propriedade do tipo e não uma regra que alguém precisa lembrar de checar em
//! revisão: não existe `inner()`, não existe `Deref`, não existe campo
//! público. O executor foi movido para dentro do guard e não há referência
//! viva para ele em lugar nenhum.

use rust_decimal::dec;
use trade_ports::testing::StubOrderExecutor;
use trade_risk::{KillSwitch, RiskGuard};

fn main() {
    let guard = RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        trade_domain::RiskLimits::default(),
        KillSwitch::disarmed(),
    );

    // Campo privado: não existe acesso direto.
    let _escapou = guard.inner;

    // Nem por método: o acessador não existe.
    let _tambem_nao = guard.inner();
}
