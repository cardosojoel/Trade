//! Duplos de teste.
//!
//! Sob a feature `testing`, para que não entrem em build de produção. Existem
//! porque o Princípio III exige que nenhum teste de lógica crítica dependa de
//! rede (SC-008): a corretora precisa ser substituível por algo previsível.

use crate::errors::{AuditError, ExecError, MarketError, StorageError};
use crate::{AccountView, AuditSink, Clock, MarketDataSource, OrderExecutor};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::collections::VecDeque;
use trade_domain::{AuditEvent, Candle, Fill, Interval, Money, Order, Position, Symbol};

/// Relógio controlado pelo teste.
#[derive(Debug, Clone)]
pub struct FakeClock {
    now: DateTime<Utc>,
}

impl FakeClock {
    pub fn at(now: DateTime<Utc>) -> Self {
        FakeClock { now }
    }

    pub fn set(&mut self, now: DateTime<Utc>) {
        self.now = now;
    }
}

impl Clock for FakeClock {
    fn now(&self) -> DateTime<Utc> {
        self.now
    }
}

/// Registro em memória, inspecionável pelo teste.
#[derive(Debug, Default)]
pub struct InMemoryAuditSink {
    pub events: Vec<AuditEvent>,
    pub flushes: usize,
}

impl InMemoryAuditSink {
    pub fn new() -> Self {
        Self::default()
    }

    /// Quantos eventos de um dado tipo foram registrados.
    ///
    /// É com isto que se verifica SC-002 por contagem: o número de eventos
    /// `order` tem de ser igual ao de `risk_decision`.
    pub fn count(&self, kind: &str) -> usize {
        self.events
            .iter()
            .filter(|e| e.kind.as_str() == kind)
            .count()
    }
}

impl AuditSink for InMemoryAuditSink {
    fn record(&mut self, event: AuditEvent) -> Result<(), AuditError> {
        self.events.push(event);
        Ok(())
    }

    fn flush(&mut self) -> Result<(), AuditError> {
        self.flushes += 1;
        Ok(())
    }
}

/// Executor de mentira.
///
/// Registra **toda** ordem que o alcança. É o que permite a um teste afirmar
/// algo mais forte que "a ordem foi recusada": que ela nunca chegou ao
/// mercado. Uma recusa que mesmo assim tocasse o executor não satisfaria
/// FR-018, e só este registro revela a diferença.
#[derive(Debug, Default)]
pub struct StubOrderExecutor {
    respostas: VecDeque<Result<Fill, ExecError>>,
    preco_padrao: Option<Money>,
    pub recebidas: Vec<Order>,
}

impl StubOrderExecutor {
    /// Preenche qualquer ordem ao preço dado, sem taxa nem slippage.
    pub fn always_fills_at(price: Money) -> Self {
        StubOrderExecutor {
            respostas: VecDeque::new(),
            preco_padrao: Some(price),
            recebidas: Vec::new(),
        }
    }

    /// Devolve respostas roteirizadas, em ordem.
    pub fn scripted(respostas: Vec<Result<Fill, ExecError>>) -> Self {
        StubOrderExecutor {
            respostas: respostas.into(),
            preco_padrao: None,
            recebidas: Vec::new(),
        }
    }

    /// Falha `n` vezes de forma transitória e depois preenche ao preço dado.
    pub fn fails_then_fills(n: u32, price: Money) -> Self {
        let mut respostas: Vec<Result<Fill, ExecError>> = (0..n)
            .map(|i| Err(ExecError::Unavailable(format!("indisponível ({})", i + 1))))
            .collect();
        respostas.push(Ok(Fill {
            order_ref: trade_domain::OrderId(0),
            price,
            qty: Decimal::ZERO,
            fee: Decimal::ZERO,
            slippage: Decimal::ZERO,
            at: DateTime::<Utc>::MIN_UTC,
        }));
        StubOrderExecutor::scripted(respostas)
    }

    /// Quantas ordens efetivamente alcançaram o executor.
    pub fn alcancado(&self) -> usize {
        self.recebidas.len()
    }
}

impl OrderExecutor for StubOrderExecutor {
    fn execute(&mut self, order: &Order) -> Result<Fill, ExecError> {
        self.recebidas.push(order.clone());

        if let Some(resposta) = self.respostas.pop_front() {
            return resposta.map(|mut f| {
                f.order_ref = order.id;
                if f.qty.is_zero() {
                    f.qty = order.qty;
                }
                if f.at == DateTime::<Utc>::MIN_UTC {
                    f.at = order.at;
                }
                f
            });
        }

        match self.preco_padrao {
            Some(price) => Ok(Fill {
                order_ref: order.id,
                price,
                qty: order.qty,
                fee: Decimal::ZERO,
                slippage: Decimal::ZERO,
                at: order.at,
            }),
            None => Err(ExecError::Unavailable(
                "stub sem resposta roteirizada".into(),
            )),
        }
    }
}

/// Conta em memória.
#[derive(Debug, Default)]
pub struct StubAccount {
    pub balance: Money,
    pub position: Position,
}

impl AccountView for StubAccount {
    fn balance(&self) -> Money {
        self.balance
    }

    fn position(&self) -> &Position {
        &self.position
    }
}

/// Provedor de mercado a partir de um vetor de velas.
///
/// Serve à US5: a mesma estratégia roda contra este e contra o provedor de
/// SQLite sem nenhuma alteração.
#[derive(Debug, Clone, Default)]
pub struct VecMarketDataSource {
    pub candles: Vec<Candle>,
}

impl VecMarketDataSource {
    pub fn new(candles: Vec<Candle>) -> Self {
        VecMarketDataSource { candles }
    }
}

impl MarketDataSource for VecMarketDataSource {
    fn candles<'a>(
        &'a self,
        _symbol: &Symbol,
        _interval: Interval,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Box<dyn Iterator<Item = Result<Candle, MarketError>> + 'a>, MarketError> {
        let it = self
            .candles
            .iter()
            .filter(move |c| c.open_time >= from && c.open_time < to)
            .map(|c| Ok(c.clone()));
        Ok(Box::new(it))
    }
}

/// Provedor que sempre falha, para verificar que a falha chega uniforme
/// independentemente de qual provedor falhou (FR-010).
#[derive(Debug, Clone)]
pub struct FailingMarketDataSource(pub String);

impl MarketDataSource for FailingMarketDataSource {
    fn candles<'a>(
        &'a self,
        _symbol: &Symbol,
        _interval: Interval,
        _from: DateTime<Utc>,
        _to: DateTime<Utc>,
    ) -> Result<Box<dyn Iterator<Item = Result<Candle, MarketError>> + 'a>, MarketError> {
        Err(MarketError::Unavailable(self.0.clone()))
    }
}

/// Repositório de velas que falha em toda escrita, para exercitar o caminho de
/// erro sem tocar o disco.
#[derive(Debug, Clone, Default)]
pub struct FailingCandleRepository;

impl crate::CandleRepository for FailingCandleRepository {
    fn upsert_page(
        &mut self,
        _symbol: &Symbol,
        _interval: Interval,
        _candles: &[Candle],
    ) -> Result<usize, StorageError> {
        Err(StorageError::Write("repositório de teste".into()))
    }

    fn record_gaps(
        &mut self,
        _symbol: &Symbol,
        _interval: Interval,
        _gaps: &[trade_domain::Gap],
    ) -> Result<(), StorageError> {
        Err(StorageError::Write("repositório de teste".into()))
    }

    fn coverage(
        &self,
        _symbol: &Symbol,
        _interval: Interval,
    ) -> Result<Option<trade_domain::Coverage>, StorageError> {
        Ok(None)
    }
}
