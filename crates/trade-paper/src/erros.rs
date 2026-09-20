//! Tradução dos erros da Bybit para o vocabulário da porta.
//!
//! A distinção que importa não é a da corretora, é a do Princípio II: falha
//! **transitória** é retentada automaticamente dentro de um limite; falha de
//! **integridade** exige revisão humana antes da retomada. Classificar errado
//! custa nos dois sentidos — parar por uma queda de rede de três segundos
//! custa oportunidade, e seguir operando com a posição divergindo custa
//! dinheiro.
//!
//! O caso mais perigoso não é nenhum dos dois: é o **desconhecido**. A
//! requisição saiu, a resposta não voltou, e ninguém sabe se a ordem existe.
//! Retentar aí pode duplicar; desistir pode deixar posição órfã. A única saída
//! correta é reconciliar antes de qualquer ordem nova (FR-107).

use crate::transporte::TransporteError;
use trade_ports::ExecError;

/// Códigos da Bybit que descrevem indisponibilidade momentânea.
const TRANSITORIOS: &[i64] = &[
    10002,  // requisição fora da janela de tempo do servidor
    10006,  // rate limit
    10016,  // erro interno do servidor
    10429,  // too many visits
    170007, // timeout aguardando resposta do sistema de ordens
];

/// Códigos que descrevem saldo ou posição insuficiente.
const INSUFICIENTES: &[i64] = &[
    170131, // saldo insuficiente
    170133, // quantidade abaixo do mínimo
    170136, // valor da ordem abaixo do mínimo
];

/// Traduz uma resposta com `retCode` diferente de zero.
pub fn de_ret_code(ret_code: i64, ret_msg: &str) -> ExecError {
    let msg = format!("retCode {ret_code} — {ret_msg}");
    if TRANSITORIOS.contains(&ret_code) {
        ExecError::Unavailable(msg)
    } else if INSUFICIENTES.contains(&ret_code) {
        ExecError::Insufficient(msg)
    } else {
        // O padrão é recusa, e não indisponibilidade. Um código novo que a
        // Bybit acrescente será tratado como recusa — que interrompe aquela
        // ordem e deixa o ciclo seguir — e não como transitório, que faria o
        // sistema retentar em laço algo que nunca vai passar.
        ExecError::Rejected(msg)
    }
}

/// Traduz uma falha de transporte.
pub fn de_transporte(e: &TransporteError) -> ExecError {
    match e {
        TransporteError::Transitoria(m) => ExecError::Unavailable(m.clone()),
        TransporteError::Permanente(m) => ExecError::Rejected(m.clone()),
        // Desconhecido vira integridade de propósito: é o que impede a
        // retentativa automática e força a reconciliação. Classificá-lo como
        // transitório seria autorizar exatamente o reenvio cego que o FR-107
        // proíbe.
        TransporteError::Desconhecido(m) => ExecError::Integrity(format!(
            "resultado desconhecido ({m}): a ordem pode existir na corretora. \
             Reconciliar antes de qualquer ordem nova."
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_e_transitorio_e_retentavel() {
        let e = de_ret_code(10006, "Too many visits!");
        assert!(matches!(e, ExecError::Unavailable(_)), "{e}");
        assert!(e.is_transient_candidate(), "rate limit passa sozinho");
    }

    #[test]
    fn saldo_insuficiente_nao_e_retentavel() {
        let e = de_ret_code(170131, "Insufficient balance");
        assert!(matches!(e, ExecError::Insufficient(_)), "{e}");
        assert!(!e.is_transient_candidate(), "retentar não cria saldo");
    }

    #[test]
    fn codigo_desconhecido_vira_recusa_e_nao_transitorio() {
        // A Bybit acrescenta códigos. Um código novo tratado como transitório
        // faria o sistema retentar em laço algo que nunca vai passar; tratado
        // como recusa, interrompe aquela ordem e o ciclo segue.
        let e = de_ret_code(999999, "algo novo");
        assert!(matches!(e, ExecError::Rejected(_)), "{e}");
        assert!(!e.is_transient_candidate());
    }

    #[test]
    fn o_codigo_e_a_mensagem_sobrevivem_na_traducao() {
        // Sem isso, o registro diz "ordem recusada" e não diz por quê — e
        // reconstituir a decisão depois vira adivinhação.
        let e = de_ret_code(170133, "qty too small");
        assert!(e.to_string().contains("170133"), "{e}");
        assert!(e.to_string().contains("qty too small"), "{e}");
    }

    #[test]
    fn falha_transitoria_de_rede_e_retentavel() {
        let e = de_transporte(&TransporteError::Transitoria("timeout".into()));
        assert!(e.is_transient_candidate());
    }

    #[test]
    fn resultado_desconhecido_e_integridade_e_nao_transitorio() {
        // O caso perigoso. Retentar pode duplicar a ordem; desistir pode
        // deixar posição órfã. A única saída correta é reconciliar.
        let e = de_transporte(&TransporteError::Desconhecido("sem resposta".into()));
        assert!(matches!(e, ExecError::Integrity(_)), "{e}");
        assert!(
            !e.is_transient_candidate(),
            "desconhecido não pode ser retentado automaticamente"
        );
        assert!(e.to_string().contains("Reconciliar"), "{e}");
    }
}
