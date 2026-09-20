//! A fronteira com a rede.

/// O que o adaptador precisa de um transporte autenticado.
///
/// Duas operações. Mais que isso seria antecipar necessidade que ainda não
/// existe, e cada método a mais é um método a duplicar no duplo de teste.
pub trait Transporte {
    /// GET autenticado. `query` já vem montada e ordenada, porque é ela que
    /// entra na assinatura — remontá-la aqui abriria espaço para assinar uma
    /// string e enviar outra.
    fn get(&self, caminho: &str, query: &str) -> Result<String, TransporteError>;

    /// POST autenticado, com corpo JSON. O mesmo corpo que é assinado.
    fn post(&self, caminho: &str, corpo: &str) -> Result<String, TransporteError>;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TransporteError {
    /// Falha que passa: rede instável, indisponibilidade momentânea.
    #[error("falha transitória: {0}")]
    Transitoria(String),
    /// Falha que não passa sozinha: credencial, permissão, requisição malformada.
    #[error("falha permanente: {0}")]
    Permanente(String),
    /// A requisição saiu e a resposta não voltou. **Não é transitória**: o
    /// resultado é desconhecido, e repetir cegamente pode duplicar a ordem.
    #[error("resultado desconhecido: {0}")]
    Desconhecido(String),
}

#[cfg(feature = "testing")]
pub mod testing {
    use super::*;
    use std::cell::RefCell;

    /// Transporte de teste: devolve o que lhe mandarem, na ordem, e guarda o
    /// que recebeu.
    #[derive(Debug, Default)]
    pub struct TransporteFalso {
        respostas: RefCell<Vec<Result<String, TransporteError>>>,
        pub chamadas: RefCell<Vec<(String, String)>>,
    }

    impl TransporteFalso {
        pub fn com(respostas: Vec<Result<String, TransporteError>>) -> Self {
            Self {
                respostas: RefCell::new(respostas),
                chamadas: RefCell::new(Vec::new()),
            }
        }

        pub fn responde(corpo: &str) -> Self {
            Self::com(vec![Ok(corpo.to_string())])
        }

        fn proxima(&self, caminho: &str, corpo: &str) -> Result<String, TransporteError> {
            self.chamadas
                .borrow_mut()
                .push((caminho.to_string(), corpo.to_string()));
            let mut r = self.respostas.borrow_mut();
            if r.is_empty() {
                return Err(TransporteError::Permanente(
                    "duplo sem resposta configurada".into(),
                ));
            }
            r.remove(0)
        }
    }

    impl Transporte for TransporteFalso {
        fn get(&self, caminho: &str, query: &str) -> Result<String, TransporteError> {
            self.proxima(caminho, query)
        }
        fn post(&self, caminho: &str, corpo: &str) -> Result<String, TransporteError> {
            self.proxima(caminho, corpo)
        }
    }
}
