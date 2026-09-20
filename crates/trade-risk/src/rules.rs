//! Avaliação dos limites.
//!
//! Cada regra devolve o [`LimitBreach`] que a violou, ou `None`. A ordem de
//! avaliação é deliberada e está documentada em [`evaluate`].

use rust_decimal::Decimal;
use trade_domain::{LimitBreach, Money, Order, Position, RiskLimits, Side};

/// O que a camada de risco precisa saber para julgar uma ordem.
pub struct Judgment<'a> {
    pub order: &'a Order,
    pub position: &'a Position,
    pub balance: Money,
    pub reference_price: Money,
    pub orders_in_window: u32,
    pub daily_loss_blocked: bool,
    pub kill_switch_engaged: bool,
}

/// Julga uma ordem contra a cerca.
///
/// Ordem de avaliação, do mais categórico ao mais específico:
///
/// 1. **Kill switch** — acionado, nada passa.
/// 2. **Perda diária** — bloqueia apenas o que *aumenta* exposição. Vender
///    continua permitido, porque o bloqueio existe para impedir risco novo,
///    não para prender o operador na posição (FR-022).
/// 3. **Frequência** — ordens por janela.
/// 4. **Escopo spot comprado** — venda acima do detido, compra sem saldo.
/// 5. **Tamanho e exposição** — o quanto a posição ficaria.
///
/// Para tamanho, exposição e frequência, o limite é **valor permitido**: a
/// violação é ultrapassá-lo, não atingi-lo. Para a perda diária vale o
/// inverso — atingir já aciona (FR-019a).
pub fn evaluate(j: &Judgment<'_>, limits: &RiskLimits) -> Option<LimitBreach> {
    if j.kill_switch_engaged {
        return Some(LimitBreach::KillSwitchEngaged);
    }

    if j.daily_loss_blocked && j.order.side == Side::Buy {
        return Some(LimitBreach::DailyLossReached);
    }

    if j.orders_in_window >= limits.max_orders_per_window {
        return Some(LimitBreach::MaxOrdersPerWindow);
    }

    match j.order.side {
        Side::Sell => {
            if j.order.qty > j.position.qty() {
                return Some(LimitBreach::SellExceedsHoldings);
            }
        }
        Side::Buy => {
            let custo = j.order.qty * j.reference_price;
            if custo > j.balance {
                return Some(LimitBreach::InsufficientBalance);
            }

            let nova_qty = j.position.qty() + j.order.qty;
            let novo_valor = nova_qty * j.reference_price;

            if novo_valor > limits.max_position_size {
                return Some(LimitBreach::MaxPositionSize);
            }
            if novo_valor > limits.max_total_exposure {
                return Some(LimitBreach::MaxTotalExposure);
            }
        }
    }

    debug_assert!(j.order.qty > Decimal::ZERO);
    None
}
