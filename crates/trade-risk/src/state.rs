//! Estado corrente avaliado contra a cerca.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use trade_domain::{Money, RiskState};

/// Estado mutável do guard.
#[derive(Debug, Clone, Default)]
pub struct GuardState {
    /// Resultado **fechado** acumulado desde a virada do dia.
    realized_today: Money,
    /// Último resultado **aberto** marcado a mercado.
    unrealized: Money,
    /// Resultado aberto no instante da virada — a linha de base do dia.
    ///
    /// Sem ela, o prejuízo aberto herdado de ontem contaria de novo contra o
    /// limite de hoje, e a retomada automática da virada seria só nominal: o
    /// freio re-armaria na primeira marcação do dia novo.
    unrealized_at_day_start: Money,
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

    /// Resultado do dia: o fechado mais a **variação** do aberto desde a
    /// virada.
    ///
    /// O aberto entra por variação, não por valor absoluto, porque o limite
    /// mede o que se perdeu **hoje** — não o que a posição acumula desde que
    /// foi aberta.
    pub fn daily_pnl(&self) -> Money {
        self.realized_today + (self.unrealized - self.unrealized_at_day_start)
    }

    /// Aciona o bloqueio ao **atingir** o limite.
    ///
    /// Atingir, não ultrapassar: a constitution diz "atingido o limite de
    /// perda diária, o sistema MUST cessar". É a fronteira invertida em
    /// relação aos demais limites, e é deliberada (FR-019a).
    ///
    /// O bloqueio **trava**: uma vez atingido, só a virada do dia o solta. A
    /// constitution diz "cessar ... até o próximo período", e o preço voltar
    /// não desfaz o fato de o limite ter sido atingido.
    fn reavaliar(&mut self, max_daily_loss: Money) {
        if max_daily_loss > Decimal::ZERO && self.daily_pnl() <= -max_daily_loss {
            self.daily_loss_blocked = true;
        }
    }

    /// Acumula resultado realizado e remarca o aberto no mesmo passo.
    ///
    /// O aberto vem junto de propósito: uma venda que fecha operação move as
    /// duas parcelas ao mesmo tempo — o realizado sobe e o aberto cai. Somar
    /// uma sem atualizar a outra contaria o mesmo prejuízo duas vezes e
    /// acionaria o freio por um total que nunca existiu.
    pub fn record_realized(&mut self, pnl: Money, unrealized_now: Money, max_daily_loss: Money) {
        self.realized_today += pnl;
        self.unrealized = unrealized_now;
        self.reavaliar(max_daily_loss);
    }

    /// Remarca a posição aberta a mercado.
    ///
    /// É o que faz uma posição perdendo sozinha mover o contador: sem esta
    /// chamada o freio só enxergaria prejuízo depois de fechado.
    pub fn mark_to_market(&mut self, unrealized_now: Money, max_daily_loss: Money) {
        self.unrealized = unrealized_now;
        self.reavaliar(max_daily_loss);
    }

    /// Zera o dia. Retomada automática, sem ato humano (FR-022a).
    pub fn roll_day(&mut self, day: NaiveDate) -> bool {
        if self.day == Some(day) {
            return false;
        }
        self.day = Some(day);
        self.realized_today = Decimal::ZERO;
        self.unrealized_at_day_start = self.unrealized;
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
            daily_pnl: self.daily_pnl(),
            exposure,
            orders_in_window: self.orders_in_window(now, window_minutes),
            day: self.day,
            daily_loss_blocked: self.daily_loss_blocked,
        }
    }
}
