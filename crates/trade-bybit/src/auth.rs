//! Assinatura das requisições autenticadas da Bybit V5.
//!
//! A documentação define a string a assinar como, para GET,
//! `timestamp + api_key + recv_window + queryString`, e para POST,
//! `timestamp + api_key + recv_window + jsonBodyString`. O resultado é
//! HMAC-SHA256 em hexadecimal minúsculo, enviado em `X-BAPI-SIGN`.
//!
//! A ordem importa e não é adivinhável: trocar dois campos de lugar produz uma
//! assinatura que a corretora recusa com uma mensagem genérica, e o erro custa
//! horas. Por isso a montagem da string é testada separadamente da primitiva
//! criptográfica — uma coisa é o algoritmo estar certo, outra é estarmos
//! alimentando-o com o que a Bybit espera.

use crate::credencial::Credencial;
use hmac::{Hmac, Mac};
use sha2::Sha256;

/// Janela de validade da requisição, em milissegundos.
///
/// A Bybit exige `server_time - recv_window <= timestamp < server_time + 1000`.
/// Cinco segundos é o padrão dela e é folgado para REST; apertar isso troca um
/// risco que não temos por recusas em atraso de rede.
pub const RECV_WINDOW_MS: u64 = 5_000;

/// A string que vai para o HMAC, na ordem que a Bybit define.
pub fn string_a_assinar(
    timestamp_ms: u64,
    chave: &str,
    recv_window_ms: u64,
    corpo: &str,
) -> String {
    format!("{timestamp_ms}{chave}{recv_window_ms}{corpo}")
}

/// HMAC-SHA256 em hexadecimal minúsculo.
pub fn assinar(segredo: &str, mensagem: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(segredo.as_bytes())
        .expect("HMAC aceita chave de qualquer tamanho");
    mac.update(mensagem.as_bytes());
    mac.finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Os cabeçalhos de uma requisição autenticada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cabecalhos {
    pub api_key: String,
    pub timestamp: String,
    pub recv_window: String,
    pub sign: String,
}

impl Cabecalhos {
    pub fn montar(c: &Credencial, timestamp_ms: u64, corpo: &str) -> Self {
        let msg = string_a_assinar(timestamp_ms, &c.chave, RECV_WINDOW_MS, corpo);
        Self {
            api_key: c.chave.clone(),
            timestamp: timestamp_ms.to_string(),
            recv_window: RECV_WINDOW_MS.to_string(),
            sign: assinar(c.segredo.expor(), &msg),
        }
    }

    /// Pares prontos para o cliente HTTP, na ordem em que a Bybit os nomeia.
    pub fn pares(&self) -> [(&'static str, &str); 4] {
        [
            ("X-BAPI-API-KEY", &self.api_key),
            ("X-BAPI-TIMESTAMP", &self.timestamp),
            ("X-BAPI-RECV-WINDOW", &self.recv_window),
            ("X-BAPI-SIGN", &self.sign),
        ]
    }
}

/// O timestamp está dentro da janela que a corretora aceita?
///
/// Conferir antes de enviar troca uma recusa remota — que custa uma ida e
/// volta e chega com mensagem genérica — por um erro local que diz o que
/// houve. Relógio fora de sincronia é falha de integridade, não transitória:
/// retentar com o mesmo relógio errado só repete o erro.
pub fn dentro_da_janela(timestamp_ms: u64, hora_do_servidor_ms: u64, recv_window_ms: u64) -> bool {
    timestamp_ms + recv_window_ms >= hora_do_servidor_ms
        && timestamp_ms < hora_do_servidor_ms + 1_000
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credencial::{Ambiente, Credencial, Segredo};

    #[test]
    fn hmac_sha256_bate_com_vetor_conhecido() {
        // RFC 4231 / vetor clássico de HMAC-SHA256. Valida a primitiva, e não
        // o nosso uso dela: se esta falhar, o problema é a biblioteca.
        assert_eq!(
            assinar("key", "The quick brown fox jumps over the lazy dog"),
            "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8"
        );
    }

    #[test]
    fn a_ordem_da_string_e_a_que_a_bybit_define() {
        // timestamp + api_key + recv_window + corpo. Trocar dois campos produz
        // assinatura que a corretora recusa com mensagem genérica — o erro
        // custa horas, e é por isso que a ordem é testada sozinha.
        assert_eq!(
            string_a_assinar(
                1700000000000,
                "MINHA_CHAVE",
                5000,
                r#"{"symbol":"BTCUSDT"}"#
            ),
            r#"1700000000000MINHA_CHAVE5000{"symbol":"BTCUSDT"}"#
        );
    }

    #[test]
    fn corpo_vazio_ainda_assina_os_tres_primeiros_campos() {
        assert_eq!(string_a_assinar(1, "k", 5000, ""), "1k5000");
    }

    fn credencial() -> Credencial {
        Credencial::a_partir_de(|k| match k {
            "BYBIT_TESTNET_KEY" => Some("chave".into()),
            "BYBIT_TESTNET_SECRET" => Some("segredo".into()),
            _ => None,
        })
        .unwrap()
    }

    #[test]
    fn cabecalhos_carregam_os_quatro_campos_exigidos() {
        let h = Cabecalhos::montar(&credencial(), 1700000000000, "");
        let nomes: Vec<&str> = h.pares().iter().map(|(n, _)| *n).collect();
        assert_eq!(
            nomes,
            vec![
                "X-BAPI-API-KEY",
                "X-BAPI-TIMESTAMP",
                "X-BAPI-RECV-WINDOW",
                "X-BAPI-SIGN"
            ]
        );
        assert_eq!(h.recv_window, "5000");
        assert_eq!(h.sign.len(), 64, "hex de SHA-256 tem 64 caracteres");
        assert!(
            h.sign
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()),
            "a Bybit exige hexadecimal minúsculo"
        );
    }

    #[test]
    fn a_assinatura_muda_com_o_corpo() {
        let c = credencial();
        let a = Cabecalhos::montar(&c, 1700000000000, r#"{"qty":"1"}"#);
        let b = Cabecalhos::montar(&c, 1700000000000, r#"{"qty":"2"}"#);
        assert_ne!(a.sign, b.sign, "corpo diferente, assinatura diferente");
    }

    #[test]
    fn o_segredo_nao_vaza_nos_cabecalhos() {
        let h = Cabecalhos::montar(&credencial(), 1700000000000, "");
        let d = format!("{h:?}");
        assert!(!d.contains("segredo"), "segredo vazou: {d}");
    }

    #[test]
    fn timestamp_fora_da_janela_e_recusado_antes_do_envio() {
        let servidor = 1_700_000_000_000u64;
        assert!(dentro_da_janela(servidor, servidor, 5_000), "igual passa");
        assert!(
            dentro_da_janela(servidor - 5_000, servidor, 5_000),
            "no limite de trás passa"
        );
        assert!(
            !dentro_da_janela(servidor - 5_001, servidor, 5_000),
            "atrasado demais não passa"
        );
        assert!(
            !dentro_da_janela(servidor + 1_000, servidor, 5_000),
            "adiantado demais não passa"
        );
    }

    #[test]
    fn segredo_e_ambiente_nao_afetam_a_ordem_da_string() {
        // Guarda contra uma refatoração que passasse a incluir a URL base ou o
        // ambiente na assinatura: a Bybit não os assina.
        let c = Credencial {
            chave: "k".into(),
            segredo: Segredo::from("s"),
            ambiente: Ambiente::Producao,
        };
        let h = Cabecalhos::montar(&c, 7, "corpo");
        assert_eq!(h.sign, assinar("s", "7k5000corpo"));
    }
}
