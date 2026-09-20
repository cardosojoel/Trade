//! Tipos elementares do domínio.

use crate::{Money, Qty};
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

/// Par negociado, sempre em caixa alta.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Symbol(String);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("símbolo inválido: '{0}' — esperado alfanumérico em caixa alta, ex. BTCUSDT")]
pub struct SymbolError(String);

impl Symbol {
    pub fn new(s: &str) -> Result<Self, SymbolError> {
        let up = s.trim().to_ascii_uppercase();
        if up.is_empty() || !up.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(SymbolError(s.to_string()));
        }
        Ok(Symbol(up))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Granularidade da vela.
///
/// O domínio conhece a duração; a tradução para o código que cada corretora usa
/// vive no adaptador dela. É o Princípio V aplicado a um detalhe pequeno.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Interval {
    M1,
    M5,
    M15,
    H1,
    H4,
    D1,
}

impl Interval {
    pub const fn minutes(self) -> i64 {
        match self {
            Interval::M1 => 1,
            Interval::M5 => 5,
            Interval::M15 => 15,
            Interval::H1 => 60,
            Interval::H4 => 240,
            Interval::D1 => 1440,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Interval::M1 => "1m",
            Interval::M5 => "5m",
            Interval::M15 => "15m",
            Interval::H1 => "1h",
            Interval::H4 => "4h",
            Interval::D1 => "1d",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("granularidade '{0}' inválida — use 1m, 5m, 15m, 1h, 4h ou 1d")]
pub struct IntervalError(String);

impl FromStr for Interval {
    type Err = IntervalError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "1m" => Ok(Interval::M1),
            "5m" => Ok(Interval::M5),
            "15m" => Ok(Interval::M15),
            "1h" => Ok(Interval::H1),
            "4h" => Ok(Interval::H4),
            "1d" => Ok(Interval::D1),
            outro => Err(IntervalError(outro.to_string())),
        }
    }
}

impl fmt::Display for Interval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Direção de uma ordem.
///
/// Não há variante de venda a descoberto: no escopo spot comprado, `Sell` só é
/// válida contra quantidade detida (FR-005).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    Buy,
    Sell,
}

impl Side {
    pub const fn as_str(self) -> &'static str {
        match self {
            Side::Buy => "Buy",
            Side::Sell => "Sell",
        }
    }
}

/// Intenção da estratégia em um instante.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Intent {
    Buy,
    Sell,
    Hold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SignalId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OrderId(pub u64);

/// Os dados que produziram um sinal.
///
/// É o campo que torna a auditoria útil (FR-034). Registrar só a intenção
/// responde "o que o robô fez"; isto responde "por quê", que é a pergunta que
/// se faz quando algo dá errado.
///
/// [`BTreeMap`] e não `HashMap`: a ordem de iteração de `HashMap` é aleatorizada
/// por semente a cada processo, e isso vazaria para o registro, quebrando o
/// determinismo de FR-029.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SignalInputs {
    pub values: BTreeMap<String, String>,
    pub params: BTreeMap<String, String>,
}

impl SignalInputs {
    pub fn with_value(mut self, k: &str, v: impl fmt::Display) -> Self {
        self.values.insert(k.to_string(), v.to_string());
        self
    }

    pub fn with_param(mut self, k: &str, v: impl fmt::Display) -> Self {
        self.params.insert(k.to_string(), v.to_string());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signal {
    pub id: SignalId,
    pub at: DateTime<Utc>,
    pub intent: Intent,
    pub inputs: SignalInputs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    pub id: OrderId,
    pub signal_ref: SignalId,
    pub side: Side,
    pub qty: Qty,
    pub at: DateTime<Utc>,
}

/// Resultado de uma ordem no mercado.
///
/// Taxa e slippage são campos próprios, não embutidos no preço (FR-027):
/// quando uma estratégia lucrativa no papel dá prejuízo na simulação, a
/// primeira pergunta é quanto foi custo de transação, e ela precisa ser
/// respondível por consulta, não por dedução.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fill {
    pub order_ref: OrderId,
    pub price: Money,
    pub qty: Qty,
    pub fee: Money,
    pub slippage: Money,
    pub at: DateTime<Utc>,
}

/// Um ciclo fechado de compra e venda, com resultado realizado.
///
/// Cada venda — total ou parcial — gera uma `Trade` correspondente à porção
/// realizada. É o que faz a soma do extrato coincidir com o resultado líquido
/// (SC-009) sem depender de arredondamento.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trade {
    pub seq: u64,
    pub entry_at: DateTime<Utc>,
    pub entry_price: Money,
    pub exit_at: DateTime<Utc>,
    pub exit_price: Money,
    pub qty: Qty,
    pub fees: Money,
    pub pnl: Money,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simbolo_normaliza_para_caixa_alta() {
        assert_eq!(Symbol::new("btcusdt").unwrap().as_str(), "BTCUSDT");
    }

    #[test]
    fn simbolo_rejeita_vazio_e_pontuacao() {
        assert!(Symbol::new("").is_err());
        assert!(Symbol::new("BTC-USDT").is_err());
    }

    #[test]
    fn granularidade_ida_e_volta() {
        for i in [
            Interval::M1,
            Interval::M5,
            Interval::M15,
            Interval::H1,
            Interval::H4,
            Interval::D1,
        ] {
            assert_eq!(i.as_str().parse::<Interval>().unwrap(), i);
        }
    }

    #[test]
    fn granularidade_em_minutos() {
        assert_eq!(Interval::M1.minutes(), 1);
        assert_eq!(Interval::H4.minutes(), 240);
        assert_eq!(Interval::D1.minutes(), 1440);
    }

    #[test]
    fn inputs_de_sinal_tem_ordem_estavel() {
        // Ordem instável vazaria para o registro e quebraria o determinismo.
        let a = SignalInputs::default()
            .with_value("z", 1)
            .with_value("a", 2);
        let b = SignalInputs::default()
            .with_value("a", 2)
            .with_value("z", 1);
        let chaves: Vec<_> = a.values.keys().collect();
        assert_eq!(chaves, vec!["a", "z"]);
        assert_eq!(a, b);
    }
}

/// Intervalo do histórico em que a fonte não tem dados.
///
/// Registrado em vez de interpolado (FR-031): interpolar uma lacuna inventa
/// preço que não existiu, e o backtest não teria como saber.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gap {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

/// O que já está armazenado para um par e granularidade.
///
/// É sobre isto que a retomada de uma coleta decide por onde continuar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coverage {
    pub first: DateTime<Utc>,
    pub last: DateTime<Utc>,
    pub count: u64,
}
