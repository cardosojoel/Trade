//! Estado corrente avaliado contra a cerca.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use trade_domain::{Money, RiskState};

/// Estado mutável do guard.
#[derive(Debug, Clone, Default)]
pub struct GuardState {
    pub daily_pnl: Money,
    pub daily_loss_blocked: bool,
    pub day: Option<NaiveDate>,
    /// Instantes das ordens **aceitas**, para a janela deslizante.
    aceitas: Vec<DateTime<Utc>>,
}

impl GuardState {
    /// Remove da janela o que já saiu dela e devolve quantas restam.
    ///
    /// Conta ordens **aceitas**, não submetidas: o limite existe para
    /// delimitar atividade no mercado, e ordem recusada nunca chega lá.
    pub fn orders_in_window(&mut self, now: DateTime<Utc>, window_minutes: i64) -> u32 {
        let inicio = now - Duration::minutes(window_minutes);
        self.aceitas.retain(|t| *t > inicio);
        u32::try_from(self.aceitas.len()).unwrap_or(u32::MAX)
    }

    pub fn register_accepted(&mut self, at: DateTime<Utc>) {
        self.aceitas.push(at);
    }

    /// Acumula resultado realizado e aciona o bloqueio ao **atingir** o limite.
    ///
    /// Atingir, não ultrapassar: a constitution diz "atingido o limite de perda
    /// diária, o sistema MUST cessar". É a fronteira invertida em relação aos
    /// demais limites, e é deliberada (FR-019a).
    pub fn record_realized(&mut self, pnl: Money, max_daily_loss: Money) {
        self.daily_pnl += pnl;
        if max_daily_loss > Decimal::ZERO && self.daily_pnl <= -max_daily_loss {
            self.daily_loss_blocked = true;
        }
    }

    /// Zera o dia. Retomada automática, sem ato humano (FR-022a).
    pub fn roll_day(&mut self, day: NaiveDate) -> bool {
        if self.day == Some(day) {
            return false;
        }
        self.day = Some(day);
        self.daily_pnl = Decimal::ZERO;
        self.daily_loss_blocked = false;
        true
    }

    /// Retrato para o registro de auditoria.
    pub fn snapshot(
        &mut self,
        exposure: Money,
        now: DateTime<Utc>,
        window_minutes: i64,
    ) -> RiskState {
        RiskState {
            daily_pnl: self.daily_pnl,
            exposure,
            orders_in_window: self.orders_in_window(now, window_minutes),
            day: self.day,
            daily_loss_blocked: self.daily_loss_blocked,
        }
    }
}
