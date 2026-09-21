//! Fatia 1 — o servidor existe e recusa (T003 a T007).
//!
//! As recusas vêm antes de qualquer rota de dado porque são o que mais custa
//! errar: cada uma é um caminho por onde algo indevido passaria. Os critérios
//! `SC-003` a `SC-007` são todos sobre elas.

use trade_serve::servidor::Pedido;
use trade_serve::{Guarda, Motivo, Recusa, Servidor};

// ------------------------------------------------- T004, o corpo único

#[test]
fn toda_recusa_usa_o_mesmo_corpo() {
    // FR-023: a tela distingue uma recusa da outra pelo código, sem
    // interpretar texto. Se o formato variasse, ela teria que adivinhar.
    for m in Motivo::todos() {
        let corpo = Recusa::nova(m, "qualquer coisa").corpo();
        let erro = corpo
            .get("erro")
            .unwrap_or_else(|| panic!("{:?} não tem a chave `erro`", m));
        assert!(erro["codigo"].is_string(), "{m:?} sem código");
        assert!(erro["mensagem"].is_string(), "{m:?} sem mensagem");
        assert!(
            erro.get("comando").is_some(),
            "{m:?} omite `comando` — devia vir presente e nulo"
        );
    }
}

#[test]
fn cada_motivo_tem_codigo_e_status_proprios() {
    assert_eq!(Motivo::ModoAusente.codigo(), "modo_ausente");
    assert_eq!(Motivo::ModoAusente.status(), 400);
    assert_eq!(Motivo::ModoRecusado.codigo(), "modo_recusado");
    assert_eq!(Motivo::ModoRecusado.status(), 403);
    assert_eq!(Motivo::TokenAusente.status(), 401);
    assert_eq!(Motivo::TokenInvalido.status(), 401);
    assert_eq!(Motivo::OrigemRecusada.status(), 403);
    assert_eq!(Motivo::HistoricoInsuficiente.status(), 409);
    assert_eq!(Motivo::ExecucaoDesconhecida.status(), 404);
    assert_eq!(Motivo::RegistroIndisponivel.status(), 503);
}

#[test]
fn a_ausencia_de_modo_e_a_recusa_de_live_sao_distinguiveis() {
    // SC-004 pede mensagem distinta da anterior. Códigos diferentes é o que
    // torna isso verificável sem ler texto.
    assert_ne!(Motivo::ModoAusente.codigo(), Motivo::ModoRecusado.codigo());
    assert_ne!(Motivo::ModoAusente.status(), Motivo::ModoRecusado.status());
}

#[test]
fn o_comando_que_resolve_aparece_quando_existe() {
    let r = Recusa::nova(Motivo::HistoricoInsuficiente, "faltam velas")
        .com_comando("trade collect --symbol BTCUSDT --from 2026-01-01 --to 2026-01-08");
    let c = r.corpo();
    assert_eq!(
        c["erro"]["comando"],
        "trade collect --symbol BTCUSDT --from 2026-01-01 --to 2026-01-08"
    );
}

#[test]
fn sem_comando_que_resolva_o_campo_vem_presente_e_nulo() {
    // Mesma regra do FR-006: ausência declarada, nunca omitida.
    let c = Recusa::nova(Motivo::RegistroIndisponivel, "runs.db ilegível").corpo();
    assert!(c["erro"].get("comando").is_some(), "o campo existe");
    assert!(c["erro"]["comando"].is_null(), "e vem nulo");
}

#[test]
fn a_recusa_nao_ecoa_o_que_recebeu() {
    // FR-020: o servidor não devolve o token em mensagem nenhuma. O caso que
    // importa é o do token **errado**: se a recusa ecoasse o que veio, uma
    // resposta de erro viraria espelho de qualquer coisa enviada — e no caso
    // do token certo, entregaria o segredo a quem só o chutou.
    let s = Servidor::subir(0).expect("subir na porta livre");
    let certo = s.guarda().token().to_string();

    for enviado in [certo.as_str(), "chute-qualquer", "<script>alerta</script>"] {
        let r = s
            .despachar(&Pedido {
                metodo: "POST".into(),
                caminho: "/runs".into(),
                consulta: String::new(),
                token: Some(enviado.to_string()),
                origem: Some("https://exemplo.invalido".into()),
            })
            .expect_err("origem alheia é recusada");
        let texto = r.corpo().to_string();
        assert!(
            !texto.contains(enviado),
            "a recusa ecoou o que recebeu: {texto}"
        );
    }
}

#[test]
fn nenhuma_mensagem_do_servidor_carrega_o_token() {
    let s = Servidor::subir(0).expect("subir na porta livre");
    let segredo = s.guarda().token().to_string();
    for (metodo, caminho) in [
        ("POST", "/runs"),
        ("GET", "/nao-existe"),
        ("DELETE", "/runs"),
    ] {
        let Err(r) = s.despachar(&Pedido {
            metodo: metodo.into(),
            caminho: caminho.into(),
            consulta: String::new(),
            token: None,
            origem: None,
        }) else {
            continue;
        };
        assert!(
            !r.corpo().to_string().contains(&segredo),
            "{metodo} {caminho} vazou o token"
        );
    }
}

// ------------------------------------------------- T003, o socket

#[test]
fn escuta_em_127_0_0_1_e_nao_em_toda_interface() {
    // SC-007. O endereço é lido do **sistema**, e não repetido do que
    // pedimos: é a diferença entre verificar e confiar.
    let s = Servidor::subir(0).expect("subir na porta livre");
    let addr = s.endereco().expect("endereço IP");
    assert_eq!(
        addr.ip().to_string(),
        "127.0.0.1",
        "ligou em {addr} — fora do loopback"
    );
    assert_ne!(
        addr.port(),
        0,
        "porta 0 pede uma livre; ligou numa de verdade"
    );
}

// ------------------------------------------------- T007, o desconhecido

#[test]
fn caminho_desconhecido_usa_o_mesmo_corpo_de_recusa() {
    let s = Servidor::subir(0).expect("subir");
    let r = s
        .despachar(&Pedido {
            metodo: "GET".into(),
            caminho: "/nao-existe".into(),
            consulta: String::new(),
            token: None,
            origem: None,
        })
        .expect_err("caminho desconhecido é recusado");
    assert!(r.corpo()["erro"]["codigo"].is_string());
    assert!(r.corpo()["erro"].get("comando").is_some());
}

#[test]
fn metodo_errado_numa_rota_que_existe_tambem_e_recusado() {
    // `/runs` existe para GET e POST. Para DELETE, não existe — e a recusa
    // não pode vazar que a rota existe para outro método.
    let s = Servidor::subir(0).expect("subir");
    for metodo in ["DELETE", "PUT", "PATCH"] {
        assert!(
            s.despachar(&Pedido {
                metodo: metodo.into(),
                caminho: "/runs".into(),
                consulta: String::new(),
                token: None,
                origem: None,
            })
            .is_err(),
            "{metodo} /runs devia ser recusado"
        );
    }
}

#[test]
fn so_uma_rota_escreve() {
    // FR-005: a superfície de escrita é **uma**. Nenhuma emite ordem, altera
    // limite ou taxa, toca o kill switch, ou retoma execução parada.
    use trade_serve::servidor::Rota;
    let escrevem: Vec<Rota> = [
        ("GET", "/runs"),
        ("POST", "/runs"),
        ("GET", "/runs/X"),
        ("GET", "/runs/X/daily"),
        ("GET", "/runs/X/episodes"),
        ("GET", "/runs/X/episodes/3/saidas"),
        ("GET", "/runs/X/chain/9"),
        ("GET", "/runs/compare"),
        ("GET", "/runs/match"),
        ("GET", "/datasets"),
        ("GET", "/jobs/J"),
    ]
    .into_iter()
    .filter_map(|(m, c)| Rota::de(m, c))
    .filter(|r| r.escreve())
    .collect();
    assert_eq!(escrevem, vec![Rota::Iniciar], "só `POST /runs` escreve");
}

#[test]
fn as_onze_rotas_da_spec_sao_reconhecidas() {
    use trade_serve::servidor::Rota;
    let caminhos = [
        ("GET", "/runs"),
        ("GET", "/runs/01ABC"),
        ("GET", "/runs/01ABC/daily"),
        ("GET", "/runs/01ABC/episodes"),
        ("GET", "/runs/01ABC/episodes/3/saidas"),
        ("GET", "/runs/01ABC/chain/97"),
        ("GET", "/runs/compare"),
        ("GET", "/datasets"),
        ("GET", "/runs/match"),
        ("POST", "/runs"),
        ("GET", "/jobs/01XYZ"),
    ];
    for (m, c) in caminhos {
        assert!(Rota::de(m, c).is_some(), "{m} {c} não foi reconhecida");
    }
    let distintas: std::collections::BTreeSet<String> = caminhos
        .iter()
        .filter_map(|(m, c)| Rota::de(m, c))
        .map(|r| format!("{r:?}"))
        .collect();
    assert_eq!(distintas.len(), 11, "as onze rotas são distintas entre si");
}

// ------------------------------------------------- T005 e T006, a guarda

#[test]
fn a_escrita_sem_token_e_recusada() {
    // SC-005: recusada **antes** de qualquer trabalho começar.
    let g = Guarda::nova();
    assert_eq!(g.autorizar_escrita(None, None), Err(Motivo::TokenAusente));
}

#[test]
fn a_escrita_com_token_errado_e_recusada() {
    let g = Guarda::nova();
    assert_eq!(
        g.autorizar_escrita(Some("nao-e-o-token"), None),
        Err(Motivo::TokenInvalido)
    );
}

#[test]
fn a_escrita_com_token_certo_passa() {
    let g = Guarda::nova();
    let t = g.token().to_string();
    assert_eq!(g.autorizar_escrita(Some(&t), None), Ok(()));
}

#[test]
fn origem_de_outro_lugar_e_recusada_mesmo_com_token_valido() {
    // SC-006, e o caso concreto que derrubou "o loopback basta" de 0,51 para
    // 0,06 na decisão 021: uma página do navegador disparando POST para
    // 127.0.0.1. O token não a impede, porque o navegador o envia se o tiver.
    let g = Guarda::nova();
    let t = g.token().to_string();
    assert_eq!(
        g.autorizar_escrita(Some(&t), Some("https://exemplo.invalido")),
        Err(Motivo::OrigemRecusada)
    );
}

#[test]
fn origem_do_proprio_loopback_passa() {
    let g = Guarda::nova();
    let t = g.token().to_string();
    for origem in ["http://127.0.0.1:7878", "http://localhost:7878"] {
        assert_eq!(
            g.autorizar_escrita(Some(&t), Some(origem)),
            Ok(()),
            "origem {origem} devia passar"
        );
    }
}

#[test]
fn a_leitura_nao_exige_token() {
    // FR-019: quem lê não muda nada, e exigir token para ler faria a
    // interface guardá-lo onde não precisa.
    let g = Guarda::nova();
    assert_eq!(g.autorizar_leitura(None), Ok(()));
}

#[test]
fn cada_servidor_nasce_com_um_token_proprio() {
    // FR-019: gerado a cada início, e morre com o processo. Dois servidores
    // não compartilham token, e reiniciar invalida o anterior.
    let a = Guarda::nova();
    let b = Guarda::nova();
    assert_ne!(a.token(), b.token());
    assert!(a.token().len() >= 32, "token curto demais para ser sorteio");
    assert_eq!(
        a.autorizar_escrita(Some(b.token()), None),
        Err(Motivo::TokenInvalido),
        "o token de um servidor não vale no outro"
    );
}

#[test]
fn o_token_nao_e_derivado_de_porta_nem_de_horario() {
    // Dois sorteios seguidos no mesmo processo, mesma porta, mesmo segundo.
    // Se fossem derivados de qualquer um dos dois, coincidiriam.
    let tokens: Vec<String> = (0..8).map(|_| Guarda::nova().token().to_string()).collect();
    let unicos: std::collections::BTreeSet<&String> = tokens.iter().collect();
    assert_eq!(
        unicos.len(),
        tokens.len(),
        "houve repetição em oito sorteios"
    );
}
