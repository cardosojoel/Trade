//! O laço contínuo de uma sessão (T026 a T031).

use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use std::sync::atomic::{AtomicBool, Ordering};
use trade_domain::{
    AuditKind, CausaDoFechamento, FeeModel, Instrumento, Intent, MarketContext, Money, Order,
    OrderId, Position, Side, Signal, Strategy, Symbol, Trade, Verdict,
};
use trade_ports::{
    Clock, LiveCandleSource, MarketError, OrderExecutor, PrecoDeReferencia, Recorder,
};
use trade_risk::{RiskContext, RiskGuard};

/// A cerca de operação de uma sessão.
#[derive(Debug, Clone)]
pub struct SessaoConfig {
    pub symbol: Symbol,
    pub fees: FeeModel,
    /// Regras de quantidade da corretora (REQ-BYBIT-006).
    pub instrumento: Instrumento,
    /// Prazo máximo de posição (FR-114, emenda 2.0.0).
    ///
    /// Vem da configuração e não de constante: limiar de risco vive em
    /// `limits.toml`, nunca embutido no código.
    pub prazo_maximo: Duration,
}

/// O que está fora do laço e ele precisa consultar a cada volta.
///
/// Os três juntos num lugar só porque são a mesma coisa: o mundo de fora. O
/// relógio diz que horas são, a bandeira diz se é para continuar, e o preço é
/// o único fio até o executor — que está dentro do `RiskGuard` e, de resto,
/// inalcançável.
pub struct Ambiente<'a> {
    pub clock: &'a dyn Clock,
    pub parar: &'a AtomicBool,
    pub preco: &'a dyn PrecoDeReferencia,
}

/// De onde a sessão parte.
///
/// Numa sessão nova, tudo zerado. Numa retomada, o que o registro devolveu
/// (FR-112) — é por aqui que a recuperação da fatia 4 entra no laço.
#[derive(Debug, Clone)]
pub struct SessaoInicio {
    pub position: Position,
    pub balance: Money,
    /// Último identificador de ordem usado, para a numeração não recomeçar.
    pub proxima_ordem: u64,
}

/// Por que a sessão terminou.
#[derive(Debug)]
pub enum FimDaSessao {
    /// A fonte de velas se encerrou.
    FonteEncerrada,
    /// Alguém mandou parar, e a volta corrente terminou antes da saída.
    Interrompida,
    /// Falha de integridade: parar é mais barato que seguir sobre premissa
    /// errada.
    Parada { motivo: String },
}

#[derive(Debug)]
pub struct SessaoOutcome {
    pub fim: FimDaSessao,
    pub trades: Vec<Trade>,
    pub position: Position,
    pub balance: Money,
    pub velas: u64,
}

/// Nome do estado da posição, para o registro de transições.
fn estado(p: &Position) -> &'static str {
    if p.is_flat() { "Flat" } else { "Long" }
}

pub struct Sessao {
    config: SessaoConfig,
}

impl Sessao {
    pub fn new(config: SessaoConfig) -> Self {
        Sessao { config }
    }

    /// Roda até a fonte se encerrar, alguém mandar parar, ou uma falha de
    /// integridade aparecer.
    ///
    /// O `guard` chega montado de fora, com o executor já dentro dele: é o
    /// ponto de composição que decide contra qual mercado a sessão opera, e é
    /// por isso que esta crate nunca precisa nomear corretora nenhuma.
    ///
    /// O relógio é consultado **uma vez por vela**, e o instante vale para
    /// todos os eventos daquela volta. É o que faz a cadeia sinal → ordem →
    /// decisão → preenchimento compartilhar um instante só, como no backtest.
    pub fn executar<E: OrderExecutor>(
        &self,
        strategy: &mut dyn Strategy,
        fonte: &mut dyn LiveCandleSource,
        guard: &mut RiskGuard<E>,
        ambiente: &Ambiente<'_>,
        inicio: SessaoInicio,
        audit: &mut dyn Recorder,
    ) -> Result<SessaoOutcome, MarketError> {
        let cfg = &self.config;

        let mut position = inicio.position;
        let mut balance = inicio.balance;
        let mut proxima_ordem = inicio.proxima_ordem;

        let mut trades = Vec::new();
        let mut pendente: Option<Signal> = None;
        let mut velas = 0u64;
        let mut fim = FimDaSessao::FonteEncerrada;

        loop {
            // Antes de pedir a próxima vela: pedir primeiro deixaria o laço
            // bloqueado na fonte esperando algo que já não vai ser usado.
            if ambiente.parar.load(Ordering::SeqCst) {
                fim = FimDaSessao::Interrompida;
                break;
            }

            let Some(candle) = fonte.proxima()? else {
                break;
            };
            candle.validate().map_err(|e| MarketError::InvalidCandle {
                at: candle.open_time.to_rfc3339(),
                cause: e.to_string(),
            })?;
            velas += 1;

            let agora = ambiente.clock.now();
            // O executor calcula o stop sobre este preço. É o único dado que
            // atravessa a cerca em direção a ele.
            ambiente.preco.definir(candle.close);

            // Retomada automática na virada do dia, sem ato humano (FR-022a).
            // A posição aberta atravessa: o dia mudou, o prazo dela não
            // (FR-019b).
            guard.on_day_boundary(agora, audit);

            // Execução do sinal da vela anterior. O sinal emitido na vela i só
            // vira ordem na vela i+1 — no instante do preenchimento, a vela
            // que o originou já passou.
            if let Some(sinal) = pendente.take() {
                let side = match sinal.intent {
                    Intent::Buy => Some(Side::Buy),
                    Intent::Sell => Some(Side::Sell),
                    Intent::Hold => None,
                };
                if let Some(side) = side {
                    let qty = quantidade(side, &sinal, &position, balance, candle.open, cfg);
                    if let Some(parada) = self.despachar(
                        Despacho {
                            side,
                            qty,
                            signal_ref: sinal.id,
                            referencia: candle.open,
                            agora,
                            causa: CausaDoFechamento::Sinal,
                        },
                        &mut proxima_ordem,
                        &mut position,
                        &mut balance,
                        &mut trades,
                        guard,
                        audit,
                    ) {
                        fim = parada;
                        break;
                    }
                }
            }

            // Prazo máximo de posição (FR-114). Vem depois do sinal pendente:
            // se a estratégia já mandou sair, não há prazo a fazer cumprir.
            if let Some(aberta_em) = position.opened_at()
                && agora - aberta_em >= cfg.prazo_maximo
            {
                let qty = cfg.instrumento.truncar_no_passo(position.qty());
                if let Some(parada) = self.despachar(
                    Despacho {
                        side: Side::Sell,
                        qty,
                        signal_ref: sinal_do_prazo(),
                        referencia: candle.close,
                        agora,
                        causa: CausaDoFechamento::Prazo,
                    },
                    &mut proxima_ordem,
                    &mut position,
                    &mut balance,
                    &mut trades,
                    guard,
                    audit,
                ) {
                    fim = parada;
                    break;
                }
            }

            // Posição aberta marcada a mercado: é o que faz um prejuízo ainda
            // não realizado mover o contador de perda diária.
            guard.mark_to_market(position.unrealized_at(candle.close));

            if let Some(sinal) = strategy.on_candle(&MarketContext {
                candle: &candle,
                position: &position,
                balance,
            }) {
                let _ = audit.record(agora, AuditKind::Signal(sinal.clone()));
                pendente = Some(sinal);
            }
        }

        // Encerramento limpo: nada em memória que não tenha chegado ao
        // registro. Sem isto, uma sessão interrompida perderia o último lote.
        let _ = audit.flush();

        Ok(SessaoOutcome {
            fim,
            trades,
            position,
            balance,
            velas,
        })
    }

    /// Monta a ordem, atravessa a cerca e aplica o que voltou.
    ///
    /// Devolve `Some` quando a sessão tem de parar.
    #[allow(clippy::too_many_arguments)]
    fn despachar<E: OrderExecutor>(
        &self,
        d: Despacho,
        proxima_ordem: &mut u64,
        position: &mut Position,
        balance: &mut Money,
        trades: &mut Vec<Trade>,
        guard: &mut RiskGuard<E>,
        audit: &mut dyn Recorder,
    ) -> Option<FimDaSessao> {
        let cfg = &self.config;
        let qty = cfg.instrumento.truncar_no_passo(d.qty);

        // Ordem abaixo do mínimo da corretora não é emitida: o resíduo espera
        // acumular em vez de virar recusa a cada vela.
        if !cfg.instrumento.negociavel(qty, d.referencia) {
            return None;
        }

        *proxima_ordem += 1;
        let ordem = Order {
            id: OrderId(*proxima_ordem),
            signal_ref: d.signal_ref,
            side: d.side,
            qty,
            at: d.agora,
        };
        let _ = audit.record(d.agora, AuditKind::Order(ordem.clone()));

        let resultado = guard.submit(
            &ordem,
            &RiskContext {
                position,
                balance: *balance,
                reference_price: d.referencia,
                now: d.agora,
                fees: cfg.fees.clone(),
            },
            audit,
        );

        if let Verdict::Accepted = resultado.decision.verdict
            && let Some(fill) = resultado.fill
        {
            let bruto = fill.price * fill.qty;
            let estado_antes = estado(position);
            match position.apply_fill(d.side, &fill) {
                Ok(Some(operacao)) => {
                    *balance += bruto - fill.fee;
                    guard.record_realized(operacao.pnl, position.unrealized_at(d.referencia));
                    let _ = audit.record(
                        d.agora,
                        AuditKind::StateTransition {
                            from: estado_antes.into(),
                            to: estado(position).into(),
                            position: position.clone(),
                            fechado_por: Some(d.causa),
                        },
                    );
                    trades.push(operacao);
                }
                Ok(None) => {
                    *balance -= bruto + fill.fee;
                    let _ = audit.record(
                        d.agora,
                        AuditKind::StateTransition {
                            from: estado_antes.into(),
                            to: estado(position).into(),
                            position: position.clone(),
                            // Compra não fecha episódio nenhum.
                            fechado_por: None,
                        },
                    );
                }
                Err(e) => {
                    // A cerca já recusa venda acima do detido; chegar aqui é
                    // divergência entre camadas, não erro comum.
                    return Some(FimDaSessao::Parada {
                        motivo: e.to_string(),
                    });
                }
            }
        }

        // Transitória já foi retentada dentro da cerca. O que chega aqui
        // exigindo ato humano é integridade, e integridade não se retenta
        // (FR-111).
        if let Some(a) = &resultado.anomaly
            && a.requires_human()
        {
            return Some(FimDaSessao::Parada {
                motivo: resultado
                    .error
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "falha de integridade".into()),
            });
        }

        None
    }
}

/// Os dados de uma ordem a despachar.
struct Despacho {
    side: Side,
    qty: Money,
    signal_ref: trade_domain::SignalId,
    referencia: Money,
    agora: DateTime<Utc>,
    causa: CausaDoFechamento,
}

/// O encerramento por prazo não nasce de sinal da estratégia.
///
/// Referência zero, que nenhum sinal usa: o elo aponta para o nada de
/// propósito, e quem ler o registro vê que aquela ordem não veio de decisão da
/// estratégia. A causa do fechamento diz o resto.
fn sinal_do_prazo() -> trade_domain::SignalId {
    trade_domain::SignalId(0)
}

/// Quanto a ordem pede, quando a estratégia não disse.
fn quantidade(
    side: Side,
    sinal: &Signal,
    position: &Position,
    balance: Money,
    referencia: Money,
    cfg: &SessaoConfig,
) -> Money {
    let pedido = sinal.qty.unwrap_or_else(|| match side {
        // Sem tamanho sugerido, usa o que há — descontado o que a ordem
        // custa, ou ela é recusada por saldo justamente por ter sido
        // dimensionada ignorando o próprio custo.
        Side::Buy if referencia > Decimal::ZERO => {
            let unitario = referencia
                * (Decimal::ONE + cfg.fees.slippage_rate)
                * (Decimal::ONE + cfg.fees.taker_fee_rate);
            balance / unitario
        }
        Side::Buy => Decimal::ZERO,
        Side::Sell => position.qty(),
    });

    // Venda limitada ao detido: no spot a taxa da compra sai em moeda base,
    // então quem comprou 5 detém 4,95. Mandar vender 5 é pedir o que não
    // existe, e sem o limite a posição nunca fecharia.
    match side {
        Side::Sell => pedido.min(position.qty()),
        Side::Buy => pedido,
    }
}
