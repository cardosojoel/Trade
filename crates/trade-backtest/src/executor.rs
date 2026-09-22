//! Execução simulada de ordens.

use rust_decimal::Decimal;
use std::cell::RefCell;
use std::rc::Rc;
use trade_domain::{Candle, CausaParcial, FeeModel, Fill, Money, Order, Side};
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
        //
        // A **causa** vai junto desde a T055. Antes ela ficava aqui, no código,
        // e quem lesse o registro veria uma quantidade menor sem saber por
        // quê — foi o achado 2, quatro ordens em 28.618 sem explicação. Quem
        // preenche sabe por que preencheu menos, e passa a dizer.
        let qty = order.qty.min(vela.volume);
        let causa_parcial = (qty < order.qty).then_some(CausaParcial::VolumeDaVela);

        let ajuste = referencia * self.fees.slippage_rate;
        let preco = match order.side {
            Side::Buy => referencia + ajuste,
            Side::Sell => referencia - ajuste,
        };

        let bruto: Money = preco * qty;

        // A moeda da taxa depende do lado, e não é detalhe contábil: na compra
        // ela sai em moeda base, reduzindo o que chega à conta, e é daí que
        // nasce o resíduo abaixo do passo negociável. Na venda sai em caixa.
        let (fee, fee_base) = match order.side {
            Side::Buy => (
                Decimal::ZERO,
                trade_domain::quantizar(qty * self.fees.taker_fee_rate),
            ),
            Side::Sell => (
                trade_domain::quantizar(bruto * self.fees.taker_fee_rate),
                Decimal::ZERO,
            ),
        };

        Ok(Fill {
            order_ref: order.id,
            price: preco,
            qty,
            fee,
            fee_base,
            slippage: trade_domain::quantizar(ajuste * qty),
            at: vela.open_time,
            causa_parcial,
        })
    }
}
