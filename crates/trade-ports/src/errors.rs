//! Erros das portas.
//!
//! São deliberadamente **agnósticos de corretora** (FR-010): a estratégia e o
//! motor veem `Unavailable`, não "erro 10006 da Bybit". A tradução de códigos
//! específicos acontece dentro do adaptador, que é o único lugar que conhece a
//! corretora. É o que permite o mesmo tratamento de falha servir para o
//! simulador de backtest e para a Bybit ao vivo, sem alteração no motor.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MarketError {
    #[error("histórico não encontrado para {symbol} {interval} no período pedido")]
    NotFound { symbol: String, interval: String },
    #[error("vela inválida em {at}: {cause}")]
    InvalidCandle { at: String, cause: String },
    #[error("lacuna no histórico entre {from} e {to}")]
    Gap { from: String, to: String },
    #[error("fonte indisponível: {0}")]
    Unavailable(String),
    /// A fonte respondeu, mas de forma que não dá para usar — resposta
    /// malformada, campo ausente, recusa da consulta. Distinta de
    /// `Unavailable` porque **não se resolve sozinha**: retentar devolve o
    /// mesmo problema.
    #[error("resposta inutilizável da fonte: {0}")]
    Source(String),
    #[error("falha de armazenamento: {0}")]
    Storage(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ExecError {
    /// A corretora recusou a ordem.
    #[error("ordem recusada: {0}")]
    Rejected(String),
    /// Indisponibilidade momentânea — candidata a falha transitória.
    #[error("execução indisponível: {0}")]
    Unavailable(String),
    #[error("tempo esgotado aguardando a execução")]
    Timeout,
    /// Divergência entre o que o sistema calculou e o que a fonte reporta.
    /// Nunca é transitória: seguir operando aqui custa dinheiro.
    #[error("falha de integridade: {0}")]
    Integrity(String),
    #[error("saldo ou posição insuficiente: {0}")]
    Insufficient(String),
}

impl ExecError {
    /// Se a falha é candidata a retentativa automática.
    ///
    /// Candidata, não garantida: esgotado o número de tentativas, ela vira
    /// falha de integridade (FR-024b). A decisão final é do `RiskGuard`.
    pub const fn is_transient_candidate(&self) -> bool {
        matches!(self, ExecError::Unavailable(_) | ExecError::Timeout)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StorageError {
    #[error("falha ao abrir o banco {path}: {cause}")]
    Open { path: String, cause: String },
    #[error("falha de consulta: {0}")]
    Query(String),
    #[error("falha de escrita: {0}")]
    Write(String),
    #[error("valor inválido gravado em `{campo}`: {valor}")]
    Corrupt { campo: String, valor: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AuditError {
    #[error("falha ao registrar evento: {0}")]
    Write(String),
    #[error("falha ao descarregar o registro: {0}")]
    Flush(String),
}
