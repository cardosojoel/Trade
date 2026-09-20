//! O funil obrigatório.

use crate::anomaly::{classify, implausible_price};
use crate::kill_switch::KillSwitch;
use crate::rules::{Judgment, evaluate};
use crate::state::GuardState;
use chrono::{DateTime, Utc};
use trade_domain::AuditKind;
use trade_domain::{
    Anomaly, FeeModel, Fill, Money, Order, Position, RiskDecision, RiskLimits, Verdict,
};
use trade_ports::{ExecError, OrderExecutor, Recorder};

/// O que o motor informa ao guard sobre o instante corrente.
pub struct RiskContext<'a> {
    pub position: &'a Position,
    pub balance: Money,
    /// Preço usado para avaliar tamanho e exposição.
    pub reference_price: Money,
    /// Instante **simulado**, vindo do `Clock`.
    pub now: DateTime<Utc>,
    /// Custo de transação vigente, para a avaliação de saldo.
    pub fees: FeeModel,
}

/// O resultado de submeter uma ordem.
///
/// Não é `Result`: um `Err` perderia a [`RiskDecision`], e SC-002 exige que
/// **toda** ordem tenha decisão registrada, inclusive quando a execução falha
/// depois de autorizada. O erro vem ao lado da decisão, não no lugar dela.
#[derive(Debug)]
pub struct SubmitOutcome {
    pub decision: RiskDecision,
    pub fill: Option<Fill>,
    pub error: Option<ExecError>,
    pub anomaly: Option<Anomaly>,
    /// Quantas tentativas de execução foram feitas.
    pub attempts: u32,
}

impl SubmitOutcome {
    pub fn accepted(&self) -> bool {
        self.decision.verdict.is_accepted()
    }
}

/// A camada de risco.
///
/// Três propriedades que o tipo impõe, e não a disciplina de quem programa:
///
/// 1. `inner` é privado e foi **movido** para cá. Não existe `inner()`, nem
///    `Deref`, nem campo público: não há forma de obter o executor de volta.
///    FR-018 é estrutural.
/// 2. `limits` não tem setter. Alterar limites exige construir outro
///    `RiskGuard`, o que só o ponto de composição faz. FR-021 é estrutural.
/// 3. [`RiskGuard::submit`] devolve [`RiskDecision`] **sempre**. Não há
///    caminho que produza um `Fill` sem produzir a decisão que o autorizou.
pub struct RiskGuard<E: OrderExecutor> {
    inner: E,
    limits: RiskLimits,
    state: GuardState,
    kill_switch: KillSwitch,
}

impl<E: OrderExecutor> RiskGuard<E> {
    /// O executor é **movido** para cá. Quem o entregou não tem mais como
    /// alcançá-lo — é o que torna FR-018 uma propriedade do tipo.
    pub fn new(inner: E, limits: RiskLimits, kill_switch: KillSwitch) -> Self {
        RiskGuard {
            inner,
            limits,
            state: GuardState::default(),
            kill_switch,
        }
    }

    /// Limites vigentes. Somente leitura — não existe o setter correspondente.
    pub fn limits(&self) -> &RiskLimits {
        &self.limits
    }

    pub fn kill_switch(&self) -> &KillSwitch {
        &self.kill_switch
    }

    /// Único caminho até o executor.
    ///
    /// O registrador é **parâmetro obrigatório**: não existe assinatura que
    /// permita submeter uma ordem sem dizer onde registrá-la. É o que faz
    /// SC-002 — toda ordem com decisão registrada — deixar de depender de o
    /// chamador lembrar.
    pub fn submit(
        &mut self,
        order: &Order,
        ctx: &RiskContext<'_>,
        audit: &mut dyn Recorder,
    ) -> SubmitOutcome {
        let janela = self.limits.window_minutes;
        let orders_in_window = self.state.orders_in_window(ctx.now, janela);

        let julgamento = Judgment {
            order,
            position: ctx.position,
            balance: ctx.balance,
            reference_price: ctx.reference_price,
            orders_in_window,
            daily_loss_blocked: self.state.daily_loss_blocked,
            kill_switch_engaged: self.kill_switch.is_engaged(),
            fees: ctx.fees.clone(),
        };

        let violacao = evaluate(&julgamento, &self.limits);
        let exposicao = ctx.position.exposure_at(ctx.reference_price);

        // A decisão é montada antes de qualquer execução e devolvida em todos
        // os caminhos. Não existe saída desta função sem ela.
        let decision = RiskDecision {
            order_ref: order.id,
            verdict: match violacao {
                Some(b) => Verdict::Rejected(b),
                None => Verdict::Accepted,
            },
            limits: self.limits.clone(),
            state: self.state.snapshot(exposicao, ctx.now, janela),
        };

        // Registrada antes de qualquer execução, e em todos os caminhos.
        let _ = audit.record(ctx.now, AuditKind::RiskDecision(decision.clone()));

        if violacao.is_some() {
            // Recusa não interrompe a operação e não consome a janela
            // (FR-020a): o ciclo segue no passo seguinte.
            return SubmitOutcome {
                decision,
                fill: None,
                error: None,
                anomaly: None,
                attempts: 0,
            };
        }

        self.state.register_accepted(ctx.now);

        let mut attempts = 0u32;
        loop {
            attempts += 1;
            match self.inner.execute(order) {
                Ok(fill) => {
                    // Preço fora da faixa plausível é falha de integridade,
                    // mesmo vindo de uma execução bem-sucedida: o dado está
                    // errado, e operar sobre ele é pior que parar.
                    if let Some(a) = implausible_price(
                        fill.price,
                        ctx.reference_price,
                        self.limits.max_price_deviation_ratio,
                    ) {
                        let _ = audit.record(
                            ctx.now,
                            AuditKind::Halt {
                                reason: format!(
                                    "preço {} fora da faixa plausível em torno de {}",
                                    fill.price, ctx.reference_price
                                ),
                                anomaly: a.clone(),
                            },
                        );
                        return SubmitOutcome {
                            decision,
                            fill: None,
                            error: Some(ExecError::Integrity(format!(
                                "preço {} fora da faixa plausível em torno de {}",
                                fill.price, ctx.reference_price
                            ))),
                            anomaly: Some(a),
                            attempts,
                        };
                    }

                    if attempts > 1 {
                        // Retentativa bem-sucedida ainda é sinal de que algo
                        // vai mal, e por isso entra no registro mesmo sem ter
                        // interrompido nada.
                        let _ = audit.record(
                            ctx.now,
                            AuditKind::Anomaly {
                                anomaly: Anomaly::Transient {
                                    attempt: attempts - 1,
                                    max_retries: self.limits.max_transient_retries,
                                },
                                recovered: true,
                            },
                        );
                    }
                    let _ = audit.record(ctx.now, AuditKind::Fill(fill.clone()));

                    return SubmitOutcome {
                        decision,
                        fill: Some(fill),
                        error: None,
                        anomaly: None,
                        attempts,
                    };
                }

                Err(e) => match classify(&e, attempts, self.limits.max_transient_retries) {
                    // Transitória: retenta sozinha, sem interromper nada e
                    // sem pedir permissão a ninguém (FR-024a).
                    Some(Anomaly::Transient { .. }) => continue,

                    outra => {
                        if let Some(a) = &outra
                            && a.requires_human()
                        {
                            let _ = audit.record(
                                ctx.now,
                                AuditKind::Halt {
                                    reason: e.to_string(),
                                    anomaly: a.clone(),
                                },
                            );
                        }
                        return SubmitOutcome {
                            decision,
                            fill: None,
                            error: Some(e),
                            anomaly: outra,
                            attempts,
                        };
                    }
                },
            }
        }
    }

    /// Acumula resultado realizado e remarca a posição no mesmo passo,
    /// acionando o bloqueio de perda diária ao atingir o limite.
    ///
    /// `unrealized_now` é o resultado aberto **depois** de aplicado o
    /// preenchimento. Pedi-lo aqui, em vez de deixar para uma chamada
    /// seguinte, fecha a janela em que o realizado já subiu e o aberto ainda
    /// não caiu — nela, o mesmo prejuízo apareceria nas duas parcelas.
    pub fn record_realized(&mut self, pnl: Money, unrealized_now: Money) {
        self.state
            .record_realized(pnl, unrealized_now, self.limits.max_daily_loss);
    }

    /// Remarca a posição aberta a mercado.
    ///
    /// O motor chama a cada vela fechada. É o que faz uma posição perdendo
    /// sozinha acionar o freio: sem isto, o contador só se moveria quando uma
    /// venda fechasse operação, e um prejuízo aberto poderia crescer sem
    /// limite com a cerca inteira intacta.
    pub fn mark_to_market(&mut self, unrealized_now: Money) {
        self.state
            .mark_to_market(unrealized_now, self.limits.max_daily_loss);
    }

    /// Virada do dia em UTC: o bloqueio de perda diária cai **sozinho**.
    ///
    /// Devolve `true` se houve retomada, para que o motor registre o evento
    /// `resume` com `automatic: true` (FR-022a, FR-024c).
    pub fn on_day_boundary(&mut self, at: DateTime<Utc>, audit: &mut dyn Recorder) -> bool {
        let estava_bloqueado = self.state.daily_loss_blocked;
        let virou = self.state.roll_day(at.date_naive());
        if virou && estava_bloqueado {
            let _ = audit.record(
                at,
                AuditKind::Resume {
                    trigger: "DayBoundary".to_string(),
                    automatic: true,
                },
            );
        }
        virou
    }

    pub fn daily_loss_blocked(&self) -> bool {
        self.state.daily_loss_blocked
    }
}
