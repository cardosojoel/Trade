//! Emenda 2.1.0 — as salvaguardas que a troca para Demo Trading exige.
//!
//! Três, e cada uma percorre **todas** as possibilidades em vez de um exemplo
//! escolhido: é a diferença entre conferir e ilustrar.

use trade_bybit::credencial::{Ambiente, Credencial, CredencialError};

/// Lê de um mapa fixo, sem tocar o ambiente do processo.
fn de(pares: &[(&str, &str)]) -> Result<Credencial, CredencialError> {
    let mapa: std::collections::BTreeMap<&str, &str> = pares.iter().copied().collect();
    Credencial::a_partir_de(|k| mapa.get(k).map(|v| (*v).to_string()))
}

const T: [(&str, &str); 2] = [("BYBIT_TESTNET_KEY", "tk"), ("BYBIT_TESTNET_SECRET", "ts")];
const D: [(&str, &str); 2] = [("BYBIT_DEMO_KEY", "dk"), ("BYBIT_DEMO_SECRET", "ds")];
const P: [(&str, &str); 2] = [("BYBIT_KEY", "pk"), ("BYBIT_SECRET", "ps")];

// ------------------------------------------- salvaguarda 1: o domínio

#[test]
fn nenhum_ambiente_que_nao_seja_producao_aponta_para_producao() {
    // Percorre **todas** as variantes, e não um exemplo. Acrescentar um
    // ambiente novo sem pensar nisso faz este teste falhar.
    for a in Ambiente::todos() {
        if a == Ambiente::Producao {
            continue;
        }
        assert_ne!(
            a.base_url(),
            "https://api.bybit.com",
            "{a:?} resolve para o domínio de produção"
        );
    }
}

#[test]
fn cada_ambiente_tem_domínio_proprio_e_nenhum_se_repete() {
    let urls: std::collections::BTreeSet<&str> =
        Ambiente::todos().iter().map(|a| a.base_url()).collect();
    assert_eq!(
        urls.len(),
        Ambiente::todos().len(),
        "dois ambientes compartilham domínio: um erro de configuração \
         passaria a ser indetectável"
    );
}

#[test]
fn o_demo_aponta_para_o_dominio_do_demo() {
    // `api-demo.bybit.com` difere de `api.bybit.com` por um prefixo, e é essa
    // proximidade que a emenda 2.1.0 declara como o preço da troca.
    assert_eq!(Ambiente::Demo.base_url(), "https://api-demo.bybit.com");
    assert_eq!(
        Ambiente::Testnet.base_url(),
        "https://api-testnet.bybit.com"
    );
}

// ------------------------------- salvaguarda 3: a não coexistência

#[test]
fn das_oito_combinacoes_as_quatro_com_mais_de_uma_credencial_abortam() {
    // Oito combinações de presença entre três credenciais. As quatro com mais
    // de uma **têm** de abortar — conferir três casos escolhidos a dedo
    // deixaria de fora justamente o que ninguém pensou.
    for (tem_t, tem_d, tem_p) in [
        (false, false, false),
        (true, false, false),
        (false, true, false),
        (false, false, true),
        (true, true, false),
        (true, false, true),
        (false, true, true),
        (true, true, true),
    ] {
        let mut pares = Vec::new();
        if tem_t {
            pares.extend_from_slice(&T);
        }
        if tem_d {
            pares.extend_from_slice(&D);
        }
        if tem_p {
            pares.extend_from_slice(&P);
        }
        let quantas = u8::from(tem_t) + u8::from(tem_d) + u8::from(tem_p);
        let r = de(&pares);
        match quantas {
            0 => assert!(
                matches!(r, Err(CredencialError::Ausente)),
                "sem credencial nenhuma devia ser Ausente, veio {r:?}"
            ),
            1 => assert!(r.is_ok(), "uma só credencial devia passar, veio {r:?}"),
            _ => assert!(
                matches!(r, Err(CredencialError::MaisDeUma { .. })),
                "{quantas} credenciais presentes: devia abortar, veio {r:?}"
            ),
        }
    }
}

#[test]
fn a_recusa_por_coexistencia_diz_quais_ambientes_colidiram() {
    // Dizer "duas presentes" sem dizer quais deixa quem lê procurando.
    let mut pares = Vec::new();
    pares.extend_from_slice(&D);
    pares.extend_from_slice(&P);
    let e = de(&pares).unwrap_err();
    let texto = e.to_string();
    assert!(texto.contains("Demo"), "veio {texto}");
    assert!(
        texto.contains("Produção") || texto.contains("Producao"),
        "veio {texto}"
    );
}

#[test]
fn cada_credencial_sozinha_resolve_o_ambiente_certo() {
    assert_eq!(de(&T).unwrap().ambiente, Ambiente::Testnet);
    assert_eq!(de(&D).unwrap().ambiente, Ambiente::Demo);
    assert_eq!(de(&P).unwrap().ambiente, Ambiente::Producao);
}

#[test]
fn chave_sem_o_segredo_correspondente_nao_passa() {
    // Meia credencial não é credencial: passaria adiante e falharia na
    // assinatura, com mensagem genérica da corretora.
    assert!(de(&[("BYBIT_DEMO_KEY", "dk")]).is_err());
    assert!(de(&[("BYBIT_DEMO_SECRET", "ds")]).is_err());
}

#[test]
fn variavel_definida_e_vazia_e_recusada() {
    let r = de(&[("BYBIT_DEMO_KEY", ""), ("BYBIT_DEMO_SECRET", "ds")]);
    assert!(matches!(r, Err(CredencialError::Vazia(_))), "veio {r:?}");
}
