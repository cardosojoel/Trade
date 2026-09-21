//! Token local e verificação de origem (FR-019).
//!
//! A decisão 021 do Jev escolheu **as duas** verificações, 0,44 contra 0,38
//! para só o token. O que decidiu não foi o juízo abstrato — perguntada assim,
//! a opção "o loopback basta" respondeu 0,51. Refeita com o caso concreto,
//! **uma página aberta no navegador disparando POST para `127.0.0.1`**, a
//! mesma opção caiu para 0,06.
//!
//! É por isso que as duas existem. O loopback não protege: qualquer página
//! alcança `127.0.0.1`. E o token sozinho também não, se o navegador tiver
//! como enviá-lo — a origem é o que separa "a interface pediu" de "uma aba
//! qualquer pediu".

use crate::erros::Motivo;

/// Tamanho do token em bytes de sorteio, antes de virar hexadecimal.
const BYTES: usize = 24;

/// Origens que a interface local pode ter.
const ORIGENS_LOCAIS: &[&str] = &["http://127.0.0.1", "http://localhost", "http://[::1]"];

pub struct Guarda {
    token: String,
}

impl Guarda {
    /// Sorteia um token novo.
    ///
    /// **Morre com o processo**: não há prazo de expiração próprio, porque o
    /// processo é o prazo. Reiniciar o servidor invalida o anterior sem que
    /// ninguém precise revogar nada.
    pub fn nova() -> Self {
        Guarda { token: sortear() }
    }

    /// O token, para ser impresso **uma vez** no terminal que subiu o
    /// servidor. Nunca vai para o registro, para log, nem para resposta de
    /// rota alguma (FR-020).
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Leitura não exige token.
    ///
    /// Quem lê não muda nada, e exigir token para ler faria a interface
    /// guardá-lo onde não precisa — ampliando a superfície sem comprar nada.
    pub const fn autorizar_leitura(&self, _origem: Option<&str>) -> Result<(), Motivo> {
        Ok(())
    }

    /// Escrita exige token **e** origem local.
    ///
    /// A ordem importa pouco para a segurança e muito para quem depura: sem
    /// token é a falha mais comum e a mais barata de explicar.
    pub fn autorizar_escrita(
        &self,
        token: Option<&str>,
        origem: Option<&str>,
    ) -> Result<(), Motivo> {
        match token {
            None => return Err(Motivo::TokenAusente),
            Some(t) if !igual_em_tempo_constante(t, &self.token) => {
                return Err(Motivo::TokenInvalido);
            }
            Some(_) => {}
        }

        // Ausência de origem é o caso do `curl` e do script local, que não
        // mandam cabeçalho nenhum. O que se recusa é origem **declarada e
        // alheia** — é ela que denuncia a página de terceiro.
        if let Some(o) = origem
            && !e_local(o)
        {
            return Err(Motivo::OrigemRecusada);
        }
        Ok(())
    }
}

impl Default for Guarda {
    fn default() -> Self {
        Self::nova()
    }
}

fn e_local(origem: &str) -> bool {
    ORIGENS_LOCAIS
        .iter()
        .any(|l| origem == *l || origem.starts_with(&format!("{l}:")))
}

/// Compara sem sair mais cedo no primeiro byte diferente.
///
/// O token é local e o atacante teria de estar na máquina, mas comparar em
/// tempo variável é o tipo de detalhe que se paga caro quando o contexto muda.
fn igual_em_tempo_constante(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

/// Sorteia bytes do sistema operacional e devolve em hexadecimal.
///
/// Lê de `/dev/urandom`: é a fonte do sistema, própria para uso
/// criptográfico, e não exige dependência nova. **Não** é derivado de caminho,
/// porta nem horário — o FR-019 proíbe exatamente isso, porque qualquer um dos
/// três é adivinhável por quem está na máquina.
fn sortear() -> String {
    use std::io::Read;
    let mut bytes = [0u8; BYTES];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .expect("sem fonte de aleatoriedade do sistema — o servidor não sobe sem token");
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
