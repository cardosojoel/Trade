//! A camada de risco: o funil obrigatório entre a estratégia e o mercado.
//!
//! O Princípio II da constitution é não-negociável, e esta crate é a sua
//! encarnação. A garantia central não é de disciplina, é de tipo: o executor
//! de ordens é **movido** para dentro do [`RiskGuard`] e não sobra referência
//! viva fora dele. Quem quiser o executor tem que atravessar o guard, porque
//! não há outro caminho até ele.
//!
//! A segunda face, igualmente obrigatória: dentro da cerca, o robô **não pode
//! parar para pedir permissão**. Ordem recusada por limite não interrompe o
//! ciclo; bloqueio por perda diária cai sozinho na virada do dia; falha
//! transitória retenta sozinha. Só falha de integridade e kill switch exigem
//! ato humano.

pub mod anomaly;
pub mod guard;
pub mod kill_switch;
pub mod rules;
pub mod state;

pub use guard::{RiskContext, RiskGuard, SubmitOutcome};
pub use kill_switch::KillSwitch;
