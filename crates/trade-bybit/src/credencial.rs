//! Credenciais da Bybit, lidas do ambiente.
//!
//! O Princípio VI manda: nada de chave commitada, nada em log, permissão
//! mínima, e chave de testnet separada da de produção. Este módulo cobra as
//! quatro coisas antes de a primeira ordem existir.
//!
//! A leitura é por closure e não direto de `std::env` porque variável de
//! ambiente é estado global do processo: um teste que a escreva corrompe os
//! que rodam em paralelo. A função pública lê o ambiente; a testável recebe de
//! onde ler.

use std::fmt;

/// Contra qual ambiente a credencial opera.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ambiente {
    Testnet,
    Producao,
}

impl Ambiente {
    pub const fn base_url(self) -> &'static str {
        match self {
            Ambiente::Testnet => "https://api-testnet.bybit.com",
            Ambiente::Producao => "https://api.bybit.com",
        }
    }
}

/// Segredo que não se deixa imprimir.
///
/// `Debug` é o que vaza em produção: um `dbg!`, um `{:?}` num log de erro, um
/// `unwrap` que estoura carregando a struct inteira. Redigir aqui é o que
/// torna esse vazamento impossível em vez de improvável.
#[derive(Clone, PartialEq, Eq)]
pub struct Segredo(String);

impl Segredo {
    pub fn expor(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Segredo {
    fn from(s: &str) -> Self {
        Segredo(s.to_string())
    }
}

impl fmt::Debug for Segredo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Segredo(redigido)")
    }
}

impl fmt::Display for Segredo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("(redigido)")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credencial {
    pub chave: String,
    pub segredo: Segredo,
    pub ambiente: Ambiente,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CredencialError {
    #[error(
        "credenciais de testnet ausentes: defina BYBIT_TESTNET_KEY e \
         BYBIT_TESTNET_SECRET no ambiente. Modo paper não inicia sem elas."
    )]
    Ausente,
    #[error(
        "credenciais de testnet e de produção presentes no mesmo ambiente. O \
         Princípio VI exige que não coexistam: uma configuração que carrega as \
         duas é uma troca de variável de distância de um envio para a corretora \
         errada."
    )]
    AmbasPresentes,
    #[error("a variável {0} está definida mas vazia")]
    Vazia(&'static str),
}

const TESTNET_KEY: &str = "BYBIT_TESTNET_KEY";
const TESTNET_SECRET: &str = "BYBIT_TESTNET_SECRET";
const PROD_KEY: &str = "BYBIT_KEY";
const PROD_SECRET: &str = "BYBIT_SECRET";

impl Credencial {
    /// Lê do ambiente do processo.
    pub fn do_ambiente() -> Result<Self, CredencialError> {
        Self::a_partir_de(|k| std::env::var(k).ok())
    }

    /// Lê de onde a closure mandar. É esta que os testes exercitam.
    pub fn a_partir_de(ler: impl Fn(&str) -> Option<String>) -> Result<Self, CredencialError> {
        let tk = ler(TESTNET_KEY);
        let ts = ler(TESTNET_SECRET);
        let pk = ler(PROD_KEY);
        let ps = ler(PROD_SECRET);

        let tem_testnet = tk.is_some() || ts.is_some();
        let tem_producao = pk.is_some() || ps.is_some();

        if tem_testnet && tem_producao {
            return Err(CredencialError::AmbasPresentes);
        }
        let (Some(chave), Some(segredo)) = (tk, ts) else {
            return Err(CredencialError::Ausente);
        };
        if chave.trim().is_empty() {
            return Err(CredencialError::Vazia(TESTNET_KEY));
        }
        if segredo.trim().is_empty() {
            return Err(CredencialError::Vazia(TESTNET_SECRET));
        }

        Ok(Self {
            chave,
            segredo: Segredo(segredo),
            ambiente: Ambiente::Testnet,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn ambiente(pares: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + use<> {
        let m: HashMap<String, String> = pares
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |k| m.get(k).cloned()
    }

    #[test]
    fn sem_credencial_nao_ha_credencial() {
        let e = Credencial::a_partir_de(ambiente(&[])).unwrap_err();
        assert_eq!(e, CredencialError::Ausente);
    }

    #[test]
    fn so_a_chave_sem_o_segredo_nao_serve() {
        let e = Credencial::a_partir_de(ambiente(&[(TESTNET_KEY, "k")])).unwrap_err();
        assert_eq!(e, CredencialError::Ausente);
    }

    #[test]
    fn testnet_e_producao_no_mesmo_ambiente_e_recusado() {
        // FR-104. Não é zelo excessivo: com as duas carregadas, mandar para a
        // corretora errada vira erro de uma variável.
        let e = Credencial::a_partir_de(ambiente(&[
            (TESTNET_KEY, "k"),
            (TESTNET_SECRET, "s"),
            (PROD_KEY, "kp"),
            (PROD_SECRET, "sp"),
        ]))
        .unwrap_err();
        assert_eq!(e, CredencialError::AmbasPresentes);
    }

    #[test]
    fn uma_variavel_de_producao_solta_ja_recusa() {
        let e = Credencial::a_partir_de(ambiente(&[
            (TESTNET_KEY, "k"),
            (TESTNET_SECRET, "s"),
            (PROD_KEY, "kp"),
        ]))
        .unwrap_err();
        assert_eq!(e, CredencialError::AmbasPresentes);
    }

    #[test]
    fn variavel_vazia_e_erro_proprio() {
        let e = Credencial::a_partir_de(ambiente(&[(TESTNET_KEY, "  "), (TESTNET_SECRET, "s")]))
            .unwrap_err();
        assert_eq!(e, CredencialError::Vazia(TESTNET_KEY));
    }

    #[test]
    fn credencial_de_testnet_e_lida() {
        let c = Credencial::a_partir_de(ambiente(&[(TESTNET_KEY, "k"), (TESTNET_SECRET, "s")]))
            .unwrap();
        assert_eq!(c.chave, "k");
        assert_eq!(c.segredo.expor(), "s");
        assert_eq!(c.ambiente, Ambiente::Testnet);
        assert_eq!(c.ambiente.base_url(), "https://api-testnet.bybit.com");
    }

    #[test]
    fn o_segredo_nao_aparece_em_debug_nem_em_display() {
        // É o vazamento realista: um dbg!, um {:?} num log de erro, um unwrap
        // que estoura carregando a struct inteira.
        let c = Credencial::a_partir_de(ambiente(&[
            (TESTNET_KEY, "k"),
            (TESTNET_SECRET, "supersecreto"),
        ]))
        .unwrap();
        let d = format!("{c:?}");
        assert!(!d.contains("supersecreto"), "segredo vazou em Debug: {d}");
        assert!(d.contains("redigido"));
        assert!(!format!("{}", c.segredo).contains("supersecreto"));
    }
}
