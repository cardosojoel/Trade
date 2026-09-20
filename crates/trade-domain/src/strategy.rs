//! A fronteira que mais importa.
//!
//! A trait vive em `trade-domain`, e não em `trade-ports`, e a diferença é o
//! ponto inteiro: se vivesse em ports, a crate de estratégias precisaria
//! declarar `trade-ports` como dependência e passaria a **conseguir nomear**
//! `OrderExecutor`. Vivendo aqui, `trade-strategy` depende só do domínio e o
//! executor é, literalmente, um nome que ela não alcança.
//!
//! Verificado por `tests/architecture.rs`.

use crate::Money;
use crate::candle::Candle;
use crate::position::Position;
use crate::types::{Signal, SignalInputs};
use std::collections::BTreeMap;

/// O que a estratégia enxerga.
///
/// Repare no que **não** está aqui: nenhum executor de ordens, nenhum limite
/// mutável, nenhuma corretora, nenhum acesso ao futuro. A vela é a **corrente**;
/// o preenchimento acontece na seguinte (FR-030), e o iterador já passou.
pub struct MarketContext<'a> {
    pub candle: &'a Candle,
    pub position: &'a Position,
    pub balance: Money,
}

/// Uma estratégia de negociação.
///
/// Devolve intenção, não ordem. Quem converte intenção em ordem é o motor, e o
/// caminho do motor até o mercado atravessa obrigatoriamente a camada de risco.
/// A estratégia não pode emitir ordem porque não tem a quem emitir, e não pode
/// afrouxar limite porque não tem o que alterar — FR-018 e FR-021 satisfeitos
/// pela ausência, que é a única forma de proibição que não depende de vigilância.
pub trait Strategy {
    /// Nome estável, usado no registro da execução.
    fn name(&self) -> &str;

    /// Parâmetros vigentes, para o registro e para a reconstituição.
    fn params(&self) -> BTreeMap<String, String>;

    /// Decide o que fazer neste instante.
    fn on_candle(&mut self, ctx: &MarketContext<'_>) -> Option<Signal>;

    /// Os dados que sustentaram a última decisão (FR-034).
    fn last_inputs(&self) -> SignalInputs {
        SignalInputs::default()
    }
}
