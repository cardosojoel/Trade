//! Paper trading contra a testnet da Bybit.
//!
//! O mesmo código que iria para `live` — o que muda é qual implementação de
//! `OrderExecutor` a CLI compõe, e contra qual URL base. É essa identidade que
//! dá sentido à Porta 2: trinta dias validando um sistema diferente do que vai
//! operar não validam nada.
//!
//! O transporte é uma trait, e não o cliente HTTP direto. Não é abstração
//! especulativa: sem ela, todo teste desta crate exigiria rede e credencial, e
//! a lógica crítica — identificador de ordem, stop obrigatório, resultado
//! desconhecido — ficaria sem teste até alguém ter uma chave.

pub mod permissao;
pub mod transporte;

pub use permissao::{PermissaoError, verificar_sem_saque};
pub use transporte::{Transporte, TransporteError};
