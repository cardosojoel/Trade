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
pub mod metrics;
pub mod mode;
pub mod position;
pub mod risk_types;
pub mod strategy;
pub mod types;

pub use audit::{AuditEvent, AuditKind};
pub use candle::{Candle, CandleError};
pub use metrics::RunMetrics;
pub use mode::{ExecutionMode, ModeError};
pub use position::{Position, PositionError};
pub use risk_types::{
    Anomaly, IntegrityCause, LimitBreach, RiskDecision, RiskLimits, RiskState, Verdict,
};
pub use strategy::{MarketContext, Strategy};
pub use types::{
    Coverage, Fill, Gap, Intent, Interval, Order, OrderId, Side, Signal, SignalId, SignalInputs,
    Symbol, Trade,
};

/// Valor monetário. Alias sobre [`rust_decimal::Decimal`] para que a intenção
/// apareça na assinatura: `Money` e `Qty` são coisas diferentes que por acaso
/// têm a mesma representação.
pub type Money = rust_decimal::Decimal;

/// Quantidade de ativo.
pub type Qty = rust_decimal::Decimal;
