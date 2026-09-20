//! Execução simulada de ordens.

use rust_decimal::Decimal;
use std::cell::RefCell;
use std::rc::Rc;
use trade_domain::{Candle, FeeModel, Fill, Money, Order, Side};
use trade_ports::{ExecError, OrderExecutor};

/// Canal por onde o motor informa ao executor qual é a vela corrente.
///
/// Existe porque o executor foi **movido** para dentro do `RiskGuard` e não é
/// mais alcançável — nem pelo motor, que o construiu. Este identificador
/// compartilhado dá ao motor o que ele precisa (dizer sobre que vela preencher)
/// sem lhe devolver o que ele não pode ter (o executor).
#[derive(Debug, Clone, Default)]
pub struct CandleFeed(Rc<RefCell<Option<Candle>>>);

impl CandleFeed {
    pub fn set(&self, c: Candle) {
        *self.0.borrow_mut() = Some(c);
    }

    pub fn current(&self) -> Option<Candle> {
        self.0.borrow().clone()
    }
}

/// Simula o preenchimento sobre a vela corrente, aplicando taxa e slippage.
///
/// Taxa e slippage entram **discriminados** no [`Fill`], nunca embutidos no
/// preço (FR-027): quando uma estratégia lucrativa no papel dá prejuízo na
/// simulação, a primeira pergunta é quanto foi custo de transação, e ela
/// precisa ser respondível por consulta.
///
/// O slippage é aplicado **contra** quem opera: compra sai mais cara, venda
/// sai mais barata. Modelar a favor produziria backtest otimista, que é o tipo
/// de erro que só aparece com dinheiro real.
#[derive(Debug, Clone)]
pub struct SimulatedExecutor {
    fees: FeeModel,
    feed: CandleFeed,
}

impl SimulatedExecutor {
    pub fn new(fees: FeeModel, feed: CandleFeed) -> Self {
        SimulatedExecutor { fees, feed }
    }
}

impl OrderExecutor for SimulatedExecutor {
    fn execute(&mut self, order: &Order) -> Result<Fill, ExecError> {
        let vela = self
            .feed
            .current()
            .ok_or_else(|| ExecError::Unavailable("sem vela para preencher".into()))?;

        if vela.volume <= Decimal::ZERO {
            return Err(ExecError::Rejected(
                "vela sem volume: não há contraparte".into(),
            ));
        }

        let referencia = vela.open;

        // Preenchimento parcial quando o volume da vela não comporta a ordem.
        // Explícito, e não silencioso: a diferença aparece na quantidade do
        // Fill e no extrato.
        let qty = order.qty.min(vela.volume);

        let ajuste = referencia * self.fees.slippage_rate;
        let preco = match order.side {
            Side::Buy => referencia + ajuste,
            Side::Sell => referencia - ajuste,
        };

        let bruto: Money = preco * qty;

        Ok(Fill {
            order_ref: order.id,
            price: preco,
            qty,
            fee: trade_domain::quantizar(bruto * self.fees.taker_fee_rate),
            slippage: trade_domain::quantizar(ajuste * qty),
            at: vela.open_time,
        })
    }
}
