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
///
/// **Enum exaustivo, sem variante coringa.** Acrescentar um ambiente quebra a
/// compilação em todo `match` que os trate, que é o comportamento desejado: um
/// ambiente novo que caísse num padrão genérico cairia no domínio errado, e o
/// domínio errado aqui é a conta de produção.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Ambiente {
    Testnet,
    /// Demo Trading — livro e preços de produção, saldo fictício.
    ///
    /// É o ambiente da Porta 2 desde a emenda **2.1.0**. A chave é emitida
    /// pela conta de produção, e `api-demo.bybit.com` difere de
    /// `api.bybit.com` por um prefixo: é por isso que o domínio não vem de
    /// configuração em lugar nenhum.
    Demo,
    Producao,
}

impl Ambiente {
    /// O **único** lugar do código onde os domínios existem.
    ///
    /// `tests/um_dominio_so.rs` varre as fontes e falha se o literal de
    /// produção aparecer em qualquer outro arquivo. Um domínio que pudesse ser
    /// escrito em dois lugares poderia divergir num deles, e divergir aqui é
    /// enviar ordem para a corretora errada.
    pub const fn base_url(self) -> &'static str {
        match self {
            Ambiente::Testnet => "https://api-testnet.bybit.com",
            Ambiente::Demo => "https://api-demo.bybit.com",
            Ambiente::Producao => "https://api.bybit.com",
        }
    }

    /// Todos, para que um teste possa percorrer o conjunto em vez de um
    /// exemplo. Acrescentar variante sem acrescentar aqui quebra os testes que
    /// contam.
    pub const fn todos() -> [Ambiente; 3] {
        [Ambiente::Testnet, Ambiente::Demo, Ambiente::Producao]
    }

    pub const fn nome(self) -> &'static str {
        match self {
            Ambiente::Testnet => "Testnet",
            Ambiente::Demo => "Demo Trading",
            Ambiente::Producao => "Produção",
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
        "credenciais ausentes: defina BYBIT_DEMO_KEY e BYBIT_DEMO_SECRET no \
         ambiente. O Demo Trading é o ambiente da Porta 2 desde a emenda \
         2.1.0, e o modo paper não inicia sem credencial."
    )]
    Ausente,
    #[error(
        "credenciais de {primeiro} e de {segundo} presentes no mesmo ambiente. \
         O Princípio VI exige que não coexistam: uma configuração que carrega \
         as duas está a uma troca de variável de enviar ordem para a corretora \
         errada."
    )]
    MaisDeUma {
        primeiro: &'static str,
        segundo: &'static str,
    },
    #[error("a variável {0} está definida mas vazia")]
    Vazia(&'static str),
}

/// As variáveis de cada ambiente, na ordem em que os ambientes são listados.
const VARIAVEIS: [(Ambiente, &str, &str); 3] = [
    (
        Ambiente::Testnet,
        "BYBIT_TESTNET_KEY",
        "BYBIT_TESTNET_SECRET",
    ),
    (Ambiente::Demo, "BYBIT_DEMO_KEY", "BYBIT_DEMO_SECRET"),
    (Ambiente::Producao, "BYBIT_KEY", "BYBIT_SECRET"),
];

/// Uma credencial encontrada no ambiente, ainda não validada.
struct Achada {
    ambiente: Ambiente,
    nome_chave: &'static str,
    nome_segredo: &'static str,
    chave: Option<String>,
    segredo: Option<String>,
}

impl Credencial {
    /// Lê do ambiente do processo.
    pub fn do_ambiente() -> Result<Self, CredencialError> {
        Self::a_partir_de(|k| std::env::var(k).ok())
    }

    /// Lê de onde a closure mandar. É esta que os testes exercitam.
    pub fn a_partir_de(ler: impl Fn(&str) -> Option<String>) -> Result<Self, CredencialError> {
        // Percorre os três ambientes em vez de nomear dois. Acrescentar um
        // quarto é acrescentar uma linha em `VARIAVEIS`, e a regra de não
        // coexistência passa a valer para ele sem que ninguém se lembre.
        let mut presentes: Vec<Achada> = Vec::new();
        for (ambiente, nome_chave, nome_segredo) in VARIAVEIS {
            let chave = ler(nome_chave);
            let segredo = ler(nome_segredo);
            if chave.is_some() || segredo.is_some() {
                presentes.push(Achada {
                    ambiente,
                    nome_chave,
                    nome_segredo,
                    chave,
                    segredo,
                });
            }
        }

        // Qualquer par entre os ambientes reconhecidos, e não só testnet
        // contra produção (constitution 2.1.0). Com o Demo Trading as duas
        // credenciais saem da mesma conta, e separá-las por nome de variável
        // virou convenção — a barreira precisa ser a recusa.
        if presentes.len() > 1 {
            return Err(CredencialError::MaisDeUma {
                primeiro: presentes[0].ambiente.nome(),
                segundo: presentes[1].ambiente.nome(),
            });
        }

        let Some(a) = presentes.pop() else {
            return Err(CredencialError::Ausente);
        };
        // Meia credencial não é credencial: passaria adiante e falharia na
        // assinatura, com mensagem genérica da corretora.
        let (Some(chave), Some(segredo)) = (a.chave, a.segredo) else {
            return Err(CredencialError::Ausente);
        };
        if chave.trim().is_empty() {
            return Err(CredencialError::Vazia(a.nome_chave));
        }
        if segredo.trim().is_empty() {
            return Err(CredencialError::Vazia(a.nome_segredo));
        }

        Ok(Self {
            chave,
            segredo: Segredo(segredo),
            ambiente: a.ambiente,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    const TESTNET_KEY: &str = VARIAVEIS[0].1;
    const TESTNET_SECRET: &str = VARIAVEIS[0].2;
    const PROD_KEY: &str = VARIAVEIS[2].1;
    const PROD_SECRET: &str = VARIAVEIS[2].2;

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
        assert!(matches!(e, CredencialError::MaisDeUma { .. }), "veio {e:?}");
    }

    #[test]
    fn uma_variavel_de_producao_solta_ja_recusa() {
        let e = Credencial::a_partir_de(ambiente(&[
            (TESTNET_KEY, "k"),
            (TESTNET_SECRET, "s"),
            (PROD_KEY, "kp"),
        ]))
        .unwrap_err();
        assert!(matches!(e, CredencialError::MaisDeUma { .. }), "veio {e:?}");
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
