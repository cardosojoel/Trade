//! Modo de execução.

use std::fmt;
use std::str::FromStr;

/// Em que regime o sistema está operando.
///
/// **Não implementa [`Default`], deliberadamente.** FR-001 proíbe valor padrão
/// para o modo de execução; em Rust, a forma de proibir um padrão é não
/// fornecer o trait que o daria. Um `#[derive(Default)]` aqui seria a violação.
///
/// Nesta versão só existe [`ExecutionMode::Backtest`]. `paper` e `live` são
/// reconhecidos no parse **apenas para serem recusados com mensagem própria**
/// (FR-003): dizer "valor inválido" sugeriria erro de digitação e convidaria a
/// tentar de novo; dizer "ainda não implementado" informa o estado real.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutionMode {
    Backtest,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ModeError {
    #[error("modo '{0}' ainda não implementado — apenas 'backtest' está disponível nesta versão")]
    NotImplemented(String),
    #[error("modo '{0}' desconhecido — modos disponíveis: backtest")]
    Unknown(String),
}

impl ExecutionMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            ExecutionMode::Backtest => "backtest",
        }
    }
}

impl FromStr for ExecutionMode {
    type Err = ModeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "backtest" => Ok(ExecutionMode::Backtest),
            m @ ("paper" | "live") => Err(ModeError::NotImplemented(m.to_string())),
            outro => Err(ModeError::Unknown(outro.to_string())),
        }
    }
}

impl fmt::Display for ExecutionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backtest_faz_parse() {
        assert_eq!(
            "backtest".parse::<ExecutionMode>().unwrap(),
            ExecutionMode::Backtest
        );
        assert_eq!(
            "BACKTEST".parse::<ExecutionMode>().unwrap(),
            ExecutionMode::Backtest
        );
    }

    #[test]
    fn paper_e_live_sao_recusados_como_nao_implementados() {
        for m in ["paper", "live"] {
            match m.parse::<ExecutionMode>() {
                Err(ModeError::NotImplemented(nome)) => assert_eq!(nome, m),
                outro => panic!("{m} deveria ser NotImplemented, veio {outro:?}"),
            }
        }
    }

    #[test]
    fn erro_de_nao_implementado_e_distinto_de_desconhecido() {
        // A distinção é o ponto de FR-003: um informa o estado do sistema,
        // o outro sugere erro de digitação.
        let nao_impl = "live".parse::<ExecutionMode>().unwrap_err();
        let desconhecido = "xpto".parse::<ExecutionMode>().unwrap_err();
        assert!(matches!(nao_impl, ModeError::NotImplemented(_)));
        assert!(matches!(desconhecido, ModeError::Unknown(_)));
        assert_ne!(nao_impl.to_string(), desconhecido.to_string());
    }

    #[test]
    fn mensagem_de_nao_implementado_menciona_o_modo_disponivel() {
        let msg = "live".parse::<ExecutionMode>().unwrap_err().to_string();
        assert!(msg.contains("backtest"), "mensagem inútil: {msg}");
    }
}
