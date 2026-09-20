//! Verificação da permissão da chave, antes da primeira ordem.
//!
//! O Princípio VI exige chave com permissão mínima: negociação sim, **saque
//! não**. E exige que a ausência de saque seja verificada antes da primeira
//! execução — não confiada à lembrança de quem criou a chave.
//!
//! O racional está na constitution: a chave com permissão de saque transforma
//! qualquer comprometimento — do código, da máquina, de uma dependência — em
//! perda total em vez de perda limitada.

use crate::transporte::{Transporte, TransporteError};
use serde::Deserialize;

/// Endpoint que descreve a própria chave.
pub const CAMINHO: &str = "/v5/user/query-api";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PermissaoError {
    #[error(
        "a chave tem permissão de saque ({0}). O Princípio VI exige permissão mínima: \
         negociação sim, saque não. Uma chave com saque transforma qualquer \
         comprometimento em perda total em vez de perda limitada. Recrie a chave sem \
         essa permissão."
    )]
    ComSaque(String),
    #[error("a chave não tem permissão de negociação em spot: sem isso não há o que operar")]
    SemNegociacao,
    #[error("não foi possível verificar a permissão da chave: {0}")]
    NaoVerificavel(String),
}

#[derive(Deserialize)]
struct Resposta {
    #[serde(rename = "retCode")]
    ret_code: i64,
    #[serde(rename = "retMsg")]
    ret_msg: String,
    result: Option<Resultado>,
}

#[derive(Deserialize)]
struct Resultado {
    #[serde(default)]
    permissions: Permissoes,
}

#[derive(Deserialize, Default)]
struct Permissoes {
    #[serde(rename = "Wallet", default)]
    wallet: Vec<String>,
    #[serde(rename = "Spot", default)]
    spot: Vec<String>,
}

/// Confere que a chave negocia e **não** saca.
pub fn verificar_sem_saque(t: &impl Transporte) -> Result<(), PermissaoError> {
    let bruto = t
        .get(CAMINHO, "")
        .map_err(|e| PermissaoError::NaoVerificavel(descrever(&e)))?;

    let r: Resposta = serde_json::from_str(&bruto)
        .map_err(|e| PermissaoError::NaoVerificavel(format!("resposta ilegível: {e}")))?;

    if r.ret_code != 0 {
        return Err(PermissaoError::NaoVerificavel(format!(
            "retCode {} — {}",
            r.ret_code, r.ret_msg
        )));
    }

    let p = r
        .result
        .ok_or_else(|| PermissaoError::NaoVerificavel("resposta sem result".into()))?
        .permissions;

    // Qualquer permissão de carteira que permita mover fundos para fora é
    // recusada. A lista é de negação e não de aceitação porque a Bybit
    // acrescenta nomes: um nome novo que dê saque MUST ser recusado, e uma
    // lista de aceitação o deixaria passar por não estar nela.
    if let Some(perm) = r_saque(&p.wallet) {
        return Err(PermissaoError::ComSaque(perm));
    }
    if p.spot.is_empty() {
        return Err(PermissaoError::SemNegociacao);
    }
    Ok(())
}

fn r_saque(wallet: &[String]) -> Option<String> {
    wallet
        .iter()
        .find(|p| {
            let p = p.to_ascii_lowercase();
            p.contains("withdraw") || p.contains("transfer")
        })
        .cloned()
}

fn descrever(e: &TransporteError) -> String {
    e.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transporte::testing::TransporteFalso;

    fn resposta(wallet: &str, spot: &str) -> String {
        format!(
            r#"{{"retCode":0,"retMsg":"OK","result":{{"permissions":{{"Wallet":[{wallet}],"Spot":[{spot}]}}}}}}"#
        )
    }

    #[test]
    fn chave_sem_saque_e_com_spot_passa() {
        let t = TransporteFalso::responde(&resposta("", r#""SpotTrade""#));
        assert!(verificar_sem_saque(&t).is_ok());
    }

    #[test]
    fn chave_com_saque_e_recusada() {
        let t = TransporteFalso::responde(&resposta(r#""Withdraw""#, r#""SpotTrade""#));
        let e = verificar_sem_saque(&t).unwrap_err();
        assert!(matches!(e, PermissaoError::ComSaque(_)), "{e}");
    }

    #[test]
    fn permissao_de_transferencia_tambem_e_saque() {
        // Transferir para outra conta tira o dinheiro daqui tão bem quanto
        // sacar. A lista é de negação por isso.
        let t = TransporteFalso::responde(&resposta(r#""AccountTransfer""#, r#""SpotTrade""#));
        assert!(matches!(
            verificar_sem_saque(&t).unwrap_err(),
            PermissaoError::ComSaque(_)
        ));
    }

    #[test]
    fn nome_novo_de_saque_e_recusado_sem_precisar_ser_previsto() {
        let t = TransporteFalso::responde(&resposta(r#""SuperWithdrawV2""#, r#""SpotTrade""#));
        assert!(matches!(
            verificar_sem_saque(&t).unwrap_err(),
            PermissaoError::ComSaque(_)
        ));
    }

    #[test]
    fn chave_sem_spot_nao_tem_o_que_operar() {
        let t = TransporteFalso::responde(&resposta("", ""));
        assert!(matches!(
            verificar_sem_saque(&t).unwrap_err(),
            PermissaoError::SemNegociacao
        ));
    }

    #[test]
    fn ret_code_de_erro_e_nao_verificavel_e_nao_aprovacao() {
        // O caso perigoso: tratar falha de verificação como "deve estar tudo
        // bem" seria exatamente o fallback que autoriza, e o Princípio II
        // proíbe fallback que resulte em autorização.
        let t = TransporteFalso::responde(r#"{"retCode":10003,"retMsg":"API key invalid"}"#);
        let e = verificar_sem_saque(&t).unwrap_err();
        assert!(matches!(e, PermissaoError::NaoVerificavel(_)), "{e}");
        assert!(e.to_string().contains("10003"));
    }

    #[test]
    fn falha_de_rede_nao_aprova_a_chave() {
        let t = TransporteFalso::com(vec![Err(TransporteError::Transitoria("timeout".into()))]);
        assert!(matches!(
            verificar_sem_saque(&t).unwrap_err(),
            PermissaoError::NaoVerificavel(_)
        ));
    }
}
