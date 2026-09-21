//! Tipos do domínio do robô de trade.
//!
//! Esta crate não tem nenhuma dependência de infraestrutura — nem banco, nem
//! HTTP, nem relógio do sistema. É o que permite testá-la inteira sem rede,
//! como o Princípio III da constitution exige (SC-008).
//!
//! Todo valor monetário e de quantidade é [`rust_decimal::Decimal`]. Nenhum é
//! ponto flutuante, em nenhum ponto. Ver `research.md` R-002 e a trava em
//! `tests/no_float.rs`.

pub mod audit;
pub mod candle;
pub mod instrumento;
pub mod metrics;
pub mod mode;
pub mod perfil;
pub mod position;
pub mod retomada;
pub mod risk_types;
pub mod strategy;
pub mod types;

pub use audit::{AuditEvent, AuditKind};
pub use candle::{Candle, CandleError};
pub use instrumento::{Instrumento, InstrumentoError};
pub use metrics::RunMetrics;
pub use mode::{ExecutionMode, ModeError};
pub use perfil::{ParametrosDerivacao, Perfil, PerfilError};
pub use position::{Position, PositionError};
pub use retomada::EstadoRetomado;
pub use risk_types::{
    Anomaly, FeeModel, IntegrityCause, LimitBreach, RiskDecision, RiskLimits, RiskState, Verdict,
};
pub use strategy::{MarketContext, Strategy};
pub use types::{
    CausaDoFechamento, Coverage, Fill, Gap, Intent, Interval, Order, OrderId, Side, Signal,
    SignalId, SignalInputs, Symbol, Trade,
};

/// Valor monetário. Alias sobre [`rust_decimal::Decimal`] para que a intenção
/// apareça na assinatura: `Money` e `Qty` são coisas diferentes que por acaso
/// têm a mesma representação.
pub type Money = rust_decimal::Decimal;

/// Quantidade de ativo.
pub type Qty = rust_decimal::Decimal;

/// Casas decimais de quantidade e de valor realizado.
///
/// Oito casas é a precisão do satoshi, e é também o que corretoras aceitam em
/// BTC. Sem quantizar, as quantidades saem de uma divisão
/// (`saldo × fração ÷ preço`) carregando 28 dígitos — números que nenhuma
/// corretora aceitaria e que fazem a soma do extrato divergir do resultado
/// reportado quando refeita por fora, em SQL. A divergência é ínfima, muito
/// abaixo de um centavo, mas SC-009 pede zero, e zero é verificável.
pub const ESCALA: u32 = 8;

/// Trunca para a escala monetária, sempre **para baixo em módulo**.
///
/// Truncar e não arredondar: arredondar para cima uma quantidade a comprar
/// criaria fração que o saldo não paga.
pub fn quantizar(v: rust_decimal::Decimal) -> rust_decimal::Decimal {
    v.trunc_with_scale(ESCALA)
}
