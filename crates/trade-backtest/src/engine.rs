//! O motor.

use crate::clock::BacktestClock;
use crate::executor::{CandleFeed, SimulatedExecutor};
use crate::run::{BacktestOutcome, RunOutcome};
use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use trade_domain::{
    AuditKind, FeeModel, Gap, Intent, Interval, LimitBreach, MarketContext, Money, Order, OrderId,
    Position, RiskLimits, RunMetrics, Side, Strategy, Symbol, Verdict,
};
use trade_ports::{Clock, MarketDataSource, MarketError, Recorder};
use trade_risk::{KillSwitch, RiskContext, RiskGuard};

/// Parâmetros de uma execução.
#[derive(Debug, Clone)]
pub struct BacktestConfig {
    pub symbol: Symbol,
    pub interval: Interval,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub initial_capital: Money,
    pub fees: FeeModel,
    pub limits: RiskLimits,
    /// Patrimônio abaixo do qual a execução encerra como capital esgotado.
    ///
    /// Existe porque, em spot comprado e sem alavancagem, **o capital nunca
    /// chega a zero**: sempre resta uma posição valendo algo. Comparar com
    /// zero satisfaria FR-032 no papel e nunca na prática. O que existe de
    /// verdade é um piso abaixo do qual continuar operando não faz sentido — e
    /// onde fica esse piso é decisão de quem opera, não constante de código.
    pub min_equity: Money,
}

/// Nome do estado da posição, para o registro de transições.
fn estado(p: &Position) -> &'static str {
    if p.is_flat() { "Flat" } else { "Long" }
}

pub struct BacktestEngine {
    config: BacktestConfig,
}

impl BacktestEngine {
    pub fn new(config: BacktestConfig) -> Self {
        BacktestEngine { config }
    }

    /// Percorre o período executando o ciclo completo.
    ///
    /// O sinal emitido na vela *i* é executado na vela *i+1* — é assim que
    /// FR-030 deixa de depender de cuidado: no instante do preenchimento, a
    /// vela que originou o sinal já passou, e nada no caminho dá acesso ao
    /// futuro.
    pub fn run(
        &self,
        strategy: &mut dyn Strategy,
        source: &dyn MarketDataSource,
        kill_switch: KillSwitch,
        audit: &mut dyn Recorder,
    ) -> Result<BacktestOutcome, MarketError> {
        let cfg = &self.config;

        let feed = CandleFeed::default();
        let mut guard = RiskGuard::new(
            SimulatedExecutor::new(cfg.fees.clone(), feed.clone()),
            cfg.limits.clone(),
            kill_switch,
        );

        let mut clock = BacktestClock::new(cfg.from);
        let mut position = Position::default();
        let mut balance = cfg.initial_capital;

        let mut trades = Vec::new();
        let mut gaps = Vec::new();
        let mut rejections: Vec<(LimitBreach, usize)> = Vec::new();
        let mut total_slippage = Decimal::ZERO;

        let mut pendente: Option<trade_domain::Signal> = None;
        let mut ultimo_instante: Option<DateTime<Utc>> = None;
        let mut proxima_ordem = 0u64;
        let mut candles_seen = 0u64;
        let mut outcome = RunOutcome::Completed;

        let passo = Duration::minutes(cfg.interval.minutes());

        for item in source.candles(&cfg.symbol, cfg.interval, cfg.from, cfg.to)? {
            let candle = item?;
            candle.validate().map_err(|e| MarketError::InvalidCandle {
                at: candle.open_time.to_rfc3339(),
                cause: e.to_string(),
            })?;

            candles_seen += 1;
            clock.advance_to(candle.open_time);
            feed.set(candle.clone());

            // Lacuna reportada, nunca interpolada (FR-031): interpolar
            // inventa preço que não existiu, e a estratégia decidiria sobre
            // um mercado imaginário.
            if let Some(anterior) = ultimo_instante
                && candle.open_time > anterior + passo
            {
                gaps.push(Gap {
                    from: anterior,
                    to: candle.open_time,
                });
            }
            ultimo_instante = Some(candle.open_time);

            // Retomada automática na virada do dia, sem ato humano (FR-022a).
            guard.on_day_boundary(candle.open_time, audit);

            // Execução do sinal da vela anterior.
            if let Some(sinal) = pendente.take() {
                let side = match sinal.intent {
                    Intent::Buy => Some(Side::Buy),
                    Intent::Sell => Some(Side::Sell),
                    Intent::Hold => None,
                };

                if let Some(side) = side {
                    let referencia = candle.open;
                    let qty = sinal.qty.unwrap_or_else(|| match side {
                        // Sem tamanho sugerido, o motor usa o que há. A cerca
                        // recusa o que passar dela — não ajusta para caber.
                        Side::Buy if referencia > Decimal::ZERO => balance / referencia,
                        Side::Buy => Decimal::ZERO,
                        Side::Sell => position.qty(),
                    });

                    // Quantidade quantizada à escala do ativo: uma ordem de
                    // 0,0170935254328020703426968481 BTC não existe em lugar
                    // nenhum, e simulá-la é simular outro mercado.
                    let qty = trade_domain::quantizar(qty);

                    if qty > Decimal::ZERO {
                        proxima_ordem += 1;
                        let ordem = Order {
                            id: OrderId(proxima_ordem),
                            signal_ref: sinal.id,
                            side,
                            qty,
                            at: clock.now(),
                        };
                        let _ = audit.record(clock.now(), AuditKind::Order(ordem.clone()));

                        let resultado = guard.submit(
                            &ordem,
                            &RiskContext {
                                position: &position,
                                balance,
                                reference_price: referencia,
                                now: clock.now(),
                            },
                            audit,
                        );

                        match resultado.decision.verdict {
                            Verdict::Rejected(breach) => {
                                // Recusa não interrompe o ciclo (FR-020a).
                                match rejections.iter_mut().find(|(b, _)| *b == breach) {
                                    Some((_, n)) => *n += 1,
                                    None => rejections.push((breach, 1)),
                                }
                            }
                            Verdict::Accepted => {
                                if let Some(fill) = resultado.fill {
                                    total_slippage += fill.slippage;
                                    let bruto = fill.price * fill.qty;
                                    let estado_antes = estado(&position);

                                    match position.apply_fill(side, &fill) {
                                        Ok(Some(operacao)) => {
                                            balance += bruto - fill.fee;
                                            guard.record_realized(operacao.pnl);
                                            let _ = audit.record(
                                                clock.now(),
                                                AuditKind::StateTransition {
                                                    from: estado_antes.into(),
                                                    to: estado(&position).into(),
                                                    position: position.clone(),
                                                },
                                            );
                                            trades.push(operacao);
                                        }
                                        Ok(None) => {
                                            balance -= bruto + fill.fee;
                                            let _ = audit.record(
                                                clock.now(),
                                                AuditKind::StateTransition {
                                                    from: estado_antes.into(),
                                                    to: estado(&position).into(),
                                                    position: position.clone(),
                                                },
                                            );
                                        }
                                        Err(e) => {
                                            // A cerca já recusou venda acima do
                                            // detido; chegar aqui é divergência
                                            // entre camadas, não erro comum.
                                            outcome = RunOutcome::Halted {
                                                reason: e.to_string(),
                                                at: clock.now(),
                                            };
                                            break;
                                        }
                                    }
                                }
                            }
                        }

                        if let Some(a) = &resultado.anomaly
                            && a.requires_human()
                        {
                            outcome = RunOutcome::Halted {
                                reason: resultado
                                    .error
                                    .map(|e| e.to_string())
                                    .unwrap_or_else(|| "falha de integridade".into()),
                                at: clock.now(),
                            };
                            break;
                        }
                    }
                }
            }

            // Patrimônio: saldo mais posição marcada ao preço corrente.
            let patrimonio = balance + position.exposure_at(candle.close);
            if patrimonio <= cfg.min_equity {
                outcome = RunOutcome::CapitalExhausted { at: clock.now() };
                break;
            }

            // A estratégia enxerga a vela corrente e devolve intenção.
            if let Some(sinal) = strategy.on_candle(&MarketContext {
                candle: &candle,
                position: &position,
                balance,
            }) {
                let _ = audit.record(clock.now(), AuditKind::Signal(sinal.clone()));
                pendente = Some(sinal);
            }
        }

        rejections.sort_by_key(|(b, _)| b.as_str());

        Ok(BacktestOutcome {
            metrics: RunMetrics::from_trades(&trades, total_slippage),
            trades,
            outcome,
            final_position: position,
            final_balance: balance,
            gaps,
            rejections,
            candles_seen,
        })
    }
}
