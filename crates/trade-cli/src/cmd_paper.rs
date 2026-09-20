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
            return Err((
                2,
                "o relógio local está fora da janela que a Bybit aceita. Toda requisição \
                 assinada seria recusada com mensagem genérica. Sincronize o relógio (NTP) \
                 antes de operar."
                    .into(),
            ));
        }
        Err(e) => return Err((2, format!("não foi possível conferir o relógio: {e}"))),
    }

    match verificar_sem_saque(&cliente) {
        Ok(()) => linhas.push("  Permissão da chave       negocia, não saca".into()),
        Err(e) => return Err((2, e.to_string())),
    }

    linhas.push(String::new());
    linhas.push("  Conta pronta para a Porta 2.".into());
    linhas.push(String::new());
    Ok(linhas.join("\n"))
}

/// Só o suficiente para conferir que é a chave certa, nunca a chave inteira.
fn prefixo(chave: &str) -> String {
    let visivel: String = chave.chars().take(4).collect();
    format!("{visivel}… ({} caracteres)", chave.chars().count())
}
