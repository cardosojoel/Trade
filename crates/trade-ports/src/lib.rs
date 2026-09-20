//! As fronteiras do sistema.
//!
//! Tudo que é substituível — corretora, banco, relógio, destino da auditoria —
//! entra por aqui. Nenhuma trait menciona Bybit, SQLite ou HTTP: é o que
//! sustenta o Princípio V da constitution.
//!
//! Contrato normativo em `specs/001-nucleo-execucao/contracts/ports.md`.
//!
//! A trait `Strategy` **não** está aqui — vive em `trade-domain`. Se estivesse,
//! `trade-strategy` precisaria depender desta crate e passaria a conseguir
//! nomear [`OrderExecutor`].

pub mod errors;
pub mod recorder;
#[cfg(feature = "testing")]
pub mod testing;

pub use errors::{AuditError, ExecError, MarketError, StorageError};
pub use recorder::{AuditRecorder, Recorder};

use chrono::{DateTime, Utc};
use trade_domain::{
    AuditEvent, Candle, Coverage, Fill, Gap, Interval, Money, Order, Position, Symbol,
};

/// Origem das velas.
pub trait MarketDataSource {
    /// Percorre as velas do período em ordem cronológica **crescente**.
    ///
    /// O iterador é preguiçoso: a memória usada não cresce com o tamanho do
    /// período. Um backtest de 5 anos usa a mesma memória que um de 5 dias.
    ///
    /// O item é `Result` e não `Candle` porque uma vela inválida no meio do
    /// percurso é informação, não motivo para abortar a leitura inteira em
    /// silêncio.
    fn candles<'a>(
        &'a self,
        symbol: &Symbol,
        interval: Interval,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Box<dyn Iterator<Item = Result<Candle, MarketError>> + 'a>, MarketError>;
}

/// Destino das ordens.
///
/// **Quem obtém um `OrderExecutor` obtém acesso ao mercado.** Por isso ele não
/// é acessível à estratégia, nem por empréstimo: na composição, o executor
/// concreto é *movido* para dentro do `RiskGuard` e não sobra referência viva
/// fora dele. É assim que FR-018 deixa de ser regra e vira propriedade do tipo.
pub trait OrderExecutor {
    fn execute(&mut self, order: &Order) -> Result<Fill, ExecError>;
}

/// Saldo e posição. Somente leitura.
///
/// A posição é alterada pela aplicação de um `Fill`, nunca por atribuição
/// direta — é o que impede que uma quantidade detida negativa seja construída
/// por engano, em vez de apenas detectada depois.
pub trait AccountView {
    fn balance(&self) -> Money;
    fn position(&self) -> &Position;
}

/// Tempo.
///
/// Existe para que nada no sistema chame o relógio do sistema. No backtest,
/// devolve o **instante simulado** da vela corrente — o que torna impossível,
/// por construção, datar um evento com a hora real da máquina e quebrar o
/// determinismo de FR-029.
pub trait Clock {
    fn now(&self) -> DateTime<Utc>;
}

/// Destino do registro de auditoria.
///
/// Porta **obrigatória** do motor: o motor não é construível sem um
/// `AuditSink`. Não há configuração que desligue a auditoria, porque o
/// Princípio IV não admite execução sem registro.
pub trait AuditSink {
    fn record(&mut self, event: AuditEvent) -> Result<(), AuditError>;
    fn flush(&mut self) -> Result<(), AuditError>;
}

/// Persistência do histórico coletado.
pub trait CandleRepository {
    /// Grava uma página inteira **em uma transação** e devolve quantas velas
    /// eram novas.
    ///
    /// Transação por página é o que garante FR-015: uma coleta interrompida no
    /// meio deixa páginas inteiras gravadas, nunca meia página.
    fn upsert_page(
        &mut self,
        symbol: &Symbol,
        interval: Interval,
        candles: &[Candle],
    ) -> Result<usize, StorageError>;

    fn record_gaps(
        &mut self,
        symbol: &Symbol,
        interval: Interval,
        gaps: &[Gap],
    ) -> Result<(), StorageError>;

    /// O que já existe. É sobre isto que a retomada decide por onde continuar.
    fn coverage(
        &self,
        symbol: &Symbol,
        interval: Interval,
    ) -> Result<Option<Coverage>, StorageError>;
}
