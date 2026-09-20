//! Tipos da camada de risco.
//!
//! Vivem no domínio, e não em `trade-risk`, para que o registro de auditoria e
//! a persistência possam nomeá-los sem depender da camada de risco.

use crate::Money;
use crate::types::OrderId;
use chrono::NaiveDate;
use rust_decimal::Decimal;

/// A cerca. Todos os valores são configuração (FR-025), nunca constante
/// embutida — ver constitution v1.2.0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiskLimits {
    pub max_daily_loss: Money,
    pub max_position_size: Money,
    pub max_total_exposure: Money,
    pub max_orders_per_window: u32,
    pub window_minutes: i64,
    pub max_transient_retries: u32,
}

/// Estado corrente avaliado contra a cerca.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RiskState {
    pub daily_pnl: Money,
    pub exposure: Money,
    pub orders_in_window: u32,
    pub day: Option<NaiveDate>,
    pub daily_loss_blocked: bool,
}

/// Qual limite foi violado.
///
/// Enum exaustivo de propósito: acrescentar um limite no futuro quebra a
/// compilação em todo lugar que trate recusas. É o comportamento desejado —
/// um limite novo que passe despercebido em um `match` é um limite que não
/// protege nada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitBreach {
    MaxPositionSize,
    MaxTotalExposure,
    DailyLossReached,
    MaxOrdersPerWindow,
    InsufficientBalance,
    SellExceedsHoldings,
    KillSwitchEngaged,
}

impl LimitBreach {
    pub const fn as_str(self) -> &'static str {
        match self {
            LimitBreach::MaxPositionSize => "MaxPositionSize",
            LimitBreach::MaxTotalExposure => "MaxTotalExposure",
            LimitBreach::DailyLossReached => "DailyLossReached",
            LimitBreach::MaxOrdersPerWindow => "MaxOrdersPerWindow",
            LimitBreach::InsufficientBalance => "InsufficientBalance",
            LimitBreach::SellExceedsHoldings => "SellExceedsHoldings",
            LimitBreach::KillSwitchEngaged => "KillSwitchEngaged",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Accepted,
    Rejected(LimitBreach),
}

impl Verdict {
    pub const fn is_accepted(self) -> bool {
        matches!(self, Verdict::Accepted)
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Verdict::Accepted => "Accepted",
            Verdict::Rejected(_) => "Rejected",
        }
    }
}

/// O veredito sobre uma ordem, com fotografias do que havia no momento.
///
/// Os dois retratos existem porque auditar uma recusa exige saber contra o quê
/// ela foi avaliada. Sem eles, um registro cujo limite mudou depois vira
/// indecifrável: sabe-se que houve recusa, não contra o quê.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiskDecision {
    pub order_ref: OrderId,
    pub verdict: Verdict,
    pub limits: RiskLimits,
    pub state: RiskState,
}

/// Classificação de uma falha.
///
/// A distinção é a razão de o robô conseguir operar sozinho: parar por três
/// segundos de rede custa oportunidade sem comprar segurança; seguir operando
/// com a posição divergindo da corretora custa dinheiro.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Anomaly {
    /// Retenta automaticamente, sem intervenção humana (FR-024a).
    Transient { attempt: u32, max_retries: u32 },
    /// Para e exige ato humano explícito para retomar (FR-024b).
    Integrity(IntegrityCause),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrityCause {
    PositionDivergence,
    ImplausiblePrice,
    RetriesExhausted,
    KillSwitchEngaged,
}

impl IntegrityCause {
    pub const fn as_str(self) -> &'static str {
        match self {
            IntegrityCause::PositionDivergence => "PositionDivergence",
            IntegrityCause::ImplausiblePrice => "ImplausiblePrice",
            IntegrityCause::RetriesExhausted => "RetriesExhausted",
            IntegrityCause::KillSwitchEngaged => "KillSwitchEngaged",
        }
    }
}

impl Anomaly {
    /// Se a retomada depende de um ato humano.
    pub const fn requires_human(&self) -> bool {
        matches!(self, Anomaly::Integrity(_))
    }

    pub const fn classification(&self) -> &'static str {
        match self {
            Anomaly::Transient { .. } => "Transient",
            Anomaly::Integrity(_) => "Integrity",
        }
    }
}

impl Default for RiskLimits {
    /// Cerca fechada.
    ///
    /// O padrão é **tudo zero**, o que recusa qualquer ordem. É deliberado:
    /// limite ausente precisa significar "nada passa", nunca "tudo passa".
    /// Quem quer operar declara a cerca em `limits.toml`.
    fn default() -> Self {
        RiskLimits {
            max_daily_loss: Decimal::ZERO,
            max_position_size: Decimal::ZERO,
            max_total_exposure: Decimal::ZERO,
            max_orders_per_window: 0,
            window_minutes: 60,
            max_transient_retries: 0,
        }
    }
}
