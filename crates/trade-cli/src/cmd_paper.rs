//! `trade paper verificar` — confere se a conta está pronta para a Porta 2.
//!
//! Roda antes de a primeira ordem existir e não emite ordem nenhuma. Três
//! coisas, nessa ordem, porque cada uma só faz sentido se a anterior passou:
//!
//! 1. **A credencial existe** e não convive com a de produção (FR-102, FR-104).
//! 2. **O relógio está sincronizado** com o servidor. Fora da janela, toda
//!    requisição assinada seria recusada com mensagem genérica — e a causa
//!    levaria horas para aparecer.
//! 3. **A chave não tem permissão de saque** (FR-103, Princípio VI).
//!
//! Nada aqui imprime, registra ou devolve o segredo.

use trade_bybit::cliente_autenticado::ClienteAutenticado;
use trade_bybit::credencial::Credencial;
use trade_paper::verificar_sem_saque;

pub fn run() -> Result<String, (u8, String)> {
    let credencial = Credencial::do_ambiente().map_err(|e| (2, e.to_string()))?;
    let ambiente = credencial.ambiente;
    let chave_visivel = prefixo(&credencial.chave);
    let cliente = ClienteAutenticado::novo(credencial);

    let mut linhas = vec![
        String::new(),
        format!("  Ambiente                 {ambiente:?}"),
        format!("  URL                      {}", cliente.base_url()),
        format!("  Chave                    {chave_visivel}"),
        String::new(),
    ];

    match cliente.relogio_esta_sincronizado() {
        Ok(true) => linhas.push("  Relógio                  sincronizado".into()),
        Ok(false) => {
            return Err(com_diagnostico(
                &linhas,
                "o relógio local está fora da janela que a Bybit aceita. Toda requisição \
                 assinada seria recusada com mensagem genérica. Sincronize o relógio (NTP) \
                 antes de operar.",
                &[],
            ));
        }
        Err(e) => {
            return Err(com_diagnostico(
                &linhas,
                &format!("não foi possível conferir o relógio: {e}"),
                &[
                    "a URL acima está alcançável desta máquina?",
                    "há proxy ou firewall entre esta máquina e a Bybit?",
                ],
            ));
        }
    }

    match verificar_sem_saque(&cliente) {
        Ok(()) => linhas.push("  Permissão da chave       negocia, não saca".into()),
        Err(e) => {
            return Err(com_diagnostico(
                &linhas,
                &e.to_string(),
                &[
                    "a chave foi criada em bybit.com → Demo Trading → avatar → API?",
                    "não é chave de testnet? A Porta 2 usa Demo Trading desde a emenda \
                     2.1.0, e a chave de testnet não vale no domínio de demo.",
                    "não foi criada no modo demo **de dentro da testnet**? Esse quarto \
                     ambiente não tem domínio de API publicado, e a chave dele não \
                     funciona em lugar nenhum.",
                    "a chave e o segredo foram copiados inteiros, sem espaço nas pontas?",
                    "a chave está ativa, e não expirada nem revogada?",
                ],
            ));
        }
    }

    linhas.push(String::new());
    linhas.push("  Conta pronta para a Porta 2.".into());
    linhas.push(String::new());
    Ok(linhas.join("\n"))
}

/// Devolve o erro **com o que já foi conferido**.
///
/// Sem isto, quem vê "API key is invalid" não sabe contra qual URL a tentativa
/// foi feita, nem quais checagens já haviam passado — e as duas coisas são o
/// que separa "a chave está errada" de "a chave é de outro ambiente". É a
/// mesma regra que o `FR-022` fixa para o servidor: o erro diz o que falta, e
/// não só que falhou.
fn com_diagnostico(linhas: &[String], causa: &str, perguntas: &[&str]) -> (u8, String) {
    let mut s = linhas.join("\n");
    s.push_str("\n\n  ");
    s.push_str(causa);
    if !perguntas.is_empty() {
        s.push_str("\n\n  Confira, nesta ordem:\n");
        for p in perguntas {
            s.push_str(&format!("    · {p}\n"));
        }
    }
    (2, s)
}

/// Só o suficiente para conferir que é a chave certa, nunca a chave inteira.
fn prefixo(chave: &str) -> String {
    let visivel: String = chave.chars().take(4).collect();
    format!("{visivel}… ({} caracteres)", chave.chars().count())
}
