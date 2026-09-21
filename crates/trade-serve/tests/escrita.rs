//! Fatia 5 — a escrita (T032 a T038).
//!
//! A única rota que escreve, e mesmo ela não emite ordem: inicia uma execução
//! pela composição que a CLI fornece. Tudo aqui é sobre o que **não** passa.

use std::sync::atomic::{AtomicUsize, Ordering};
use trade_serve::escrita::{IniciarExecucao, Parametros, TrabalhoId};
use trade_serve::servidor::{Pedido, Rota, Servidor};
use trade_serve::{Motivo, escrita};

/// Duplo do ponto de ligação: conta quantas vezes foi chamado.
#[derive(Default)]
struct LigacaoFalsa {
    chamadas: AtomicUsize,
}

impl IniciarExecucao for LigacaoFalsa {
    fn iniciar(&self, p: &Parametros) -> Result<TrabalhoId, String> {
        self.chamadas.fetch_add(1, Ordering::SeqCst);
        Ok(TrabalhoId {
            trabalho: format!("job_{}", p.simbolo),
            execucao: "01EXEC".into(),
        })
    }
}

fn corpo(modo: Option<&str>) -> serde_json::Value {
    let mut v = serde_json::json!({
        "simbolo": "BTCUSDT", "intervalo": "1m",
        "de_ms": 1_758_326_400_000i64, "ate_ms": 1_789_862_340_000i64,
        "capital_inicial": "10000", "estrategia": "sma-cross",
        "parametros": { "fast": "9", "slow": "21" }
    });
    if let Some(m) = modo {
        v["modo"] = serde_json::Value::String(m.to_string());
    }
    v
}

// ---------------------------------------------------------------- T033

#[test]
fn sem_modo_declarado_nenhuma_execucao_comeca() {
    // SC-003, em 100% das tentativas. E o que importa não é só o código de
    // recusa: é que o ponto de ligação **não foi chamado**.
    let l = LigacaoFalsa::default();
    let e = escrita::iniciar(&l, &corpo(None)).expect_err("sem modo é recusado");
    assert_eq!(e.status(), 400);
    assert_eq!(e.corpo()["erro"]["codigo"], "modo_ausente");
    assert_eq!(
        l.chamadas.load(Ordering::SeqCst),
        0,
        "nada começou: a recusa veio antes de qualquer trabalho"
    );
}

// ---------------------------------------------------------------- T034

#[test]
fn live_e_recusado_com_mensagem_distinta_da_ausencia() {
    // SC-004. A promoção para capital real é ato humano registrado, e não
    // cabe numa requisição HTTP.
    let l = LigacaoFalsa::default();
    let e = escrita::iniciar(&l, &corpo(Some("live"))).expect_err("live é recusado");
    assert_eq!(e.corpo()["erro"]["codigo"], "modo_recusado");
    assert_eq!(e.status(), 403);
    assert_ne!(
        e.corpo()["erro"]["codigo"],
        escrita::iniciar(&l, &corpo(None)).unwrap_err().corpo()["erro"]["codigo"],
        "as duas recusas se distinguem sem interpretar texto"
    );
    assert_eq!(l.chamadas.load(Ordering::SeqCst), 0);
}

#[test]
fn modo_desconhecido_tambem_e_recusado() {
    let l = LigacaoFalsa::default();
    assert!(escrita::iniciar(&l, &corpo(Some("producao"))).is_err());
    assert_eq!(l.chamadas.load(Ordering::SeqCst), 0);
}

// ---------------------------------------------------------------- T035

#[test]
fn a_resposta_e_imediata_com_identificador_e_estado() {
    // FR-017: responde sem esperar a conclusão. Um backtest de doze meses
    // roda em cerca de um segundo, mas "cerca de" não é contrato.
    let l = LigacaoFalsa::default();
    let v = escrita::iniciar(&l, &corpo(Some("backtest"))).expect("backtest é aceito");
    assert_eq!(v["trabalho"], "job_BTCUSDT");
    assert_eq!(v["execucao"], "01EXEC");
    assert_eq!(v["estado"], "em_andamento");
    assert_eq!(l.chamadas.load(Ordering::SeqCst), 1);
}

#[test]
fn o_modo_paper_tambem_e_aceito() {
    let l = LigacaoFalsa::default();
    assert!(escrita::iniciar(&l, &corpo(Some("paper"))).is_ok());
}

#[test]
fn falha_do_ponto_de_ligacao_vira_recusa_e_nao_panico() {
    struct SempreFalha;
    impl IniciarExecucao for SempreFalha {
        fn iniciar(&self, _: &Parametros) -> Result<TrabalhoId, String> {
            Err("o histórico não cobre o período".into())
        }
    }
    let e = escrita::iniciar(&SempreFalha, &corpo(Some("backtest"))).unwrap_err();
    assert!(
        e.corpo()["erro"]["mensagem"]
            .as_str()
            .unwrap()
            .contains("histórico")
    );
}

// ---------------------------------------------------------------- T036

#[test]
fn o_progresso_sai_por_consulta_e_os_eventos_tem_teto() {
    // FR-018: nenhuma conexão fica aberta empurrando evento. E FR-025: os
    // 100 mais recentes, com a resposta dizendo que cortou.
    let mut q = escrita::Quadro::novo("job_1", "01EXEC");
    for i in 0..150 {
        q.registrar(format!("evento {i}"));
    }
    let v = q.estado_json(100);
    assert_eq!(v["trabalho"], "job_1");
    assert_eq!(v["estado"], "em_andamento");
    assert_eq!(v["eventos"].as_array().unwrap().len(), 100);
    assert_eq!(v["cortou"], true);
    assert_eq!(v["total_de_eventos"], 150);
    assert_eq!(
        v["eventos"][99], "evento 149",
        "os **mais recentes**, e não os primeiros"
    );
}

#[test]
fn o_trabalho_concluido_diz_que_concluiu() {
    let mut q = escrita::Quadro::novo("job_1", "01EXEC");
    q.concluir();
    assert_eq!(q.estado_json(100)["estado"], "concluido");
}

#[test]
fn o_trabalho_que_falhou_diz_por_que() {
    let mut q = escrita::Quadro::novo("job_1", "01EXEC");
    q.falhar("preço implausível");
    let v = q.estado_json(100);
    assert_eq!(v["estado"], "falhou");
    assert_eq!(v["motivo"], "preço implausível");
}

// ---------------------------------------------------------------- T037

#[test]
fn nenhum_campo_da_requisicao_carrega_credencial() {
    // FR-020 e Princípio VI. A verificação é por **nome de campo**: os
    // parâmetros aceitos são uma lista de aceitação, e o que não está nela
    // não é lido.
    let l = LigacaoFalsa::default();
    let mut c = corpo(Some("backtest"));
    c["api_key"] = serde_json::json!("nao-devia-estar-aqui");
    c["secret"] = serde_json::json!("nem-isto");
    let v = escrita::iniciar(&l, &c).unwrap();
    let texto = v.to_string();
    assert!(
        !texto.contains("nao-devia-estar-aqui"),
        "ecoou a chave: {texto}"
    );
    assert!(!texto.contains("nem-isto"));
}

#[test]
fn a_resposta_de_progresso_nao_carrega_credencial() {
    let mut q = escrita::Quadro::novo("job_1", "01EXEC");
    q.registrar("BYBIT_TESTNET_KEY=abc".into());
    let v = q.estado_json(100);
    assert!(
        !v.to_string().contains("abc"),
        "um evento com segredo não pode sair como veio"
    );
}

// ---------------------------------------------------------------- T038

#[test]
fn nao_existe_rota_que_emita_ordem_ou_mexa_na_cerca() {
    // FR-005: a superfície de escrita é **uma**. Nenhuma rota emite ordem,
    // altera limite ou taxa, toca o kill switch, ou retoma execução parada.
    let proibidas = [
        ("POST", "/orders"),
        ("POST", "/runs/X/orders"),
        ("PUT", "/limits"),
        ("POST", "/limits"),
        ("PATCH", "/limits"),
        ("PUT", "/fees"),
        ("POST", "/kill"),
        ("DELETE", "/kill"),
        ("POST", "/runs/X/resume"),
        ("POST", "/runs/X/halt"),
        ("DELETE", "/runs/X"),
        ("PUT", "/runs/X"),
    ];
    for (m, c) in proibidas {
        assert!(Rota::de(m, c).is_none(), "{m} {c} não pode existir");
    }
}

#[test]
fn so_a_rota_de_iniciar_exige_token() {
    // E a recíproca: nenhuma rota de leitura o exige (FR-019).
    let s = Servidor::subir(0).expect("subir");
    let leituras = [
        "/runs",
        "/runs/X",
        "/runs/X/daily",
        "/runs/X/episodes",
        "/runs/X/chain/9",
        "/runs/compare",
        "/datasets",
        "/runs/match",
        "/jobs/J",
    ];
    for c in leituras {
        assert!(
            s.despachar(&Pedido {
                metodo: "GET".into(),
                caminho: c.into(),
                consulta: String::new(),
                token: None,
                origem: None,
            })
            .is_ok(),
            "GET {c} não devia exigir token"
        );
    }
    let escreve = s.despachar(&Pedido {
        metodo: "POST".into(),
        caminho: "/runs".into(),
        consulta: String::new(),
        token: None,
        origem: None,
    });
    assert_eq!(
        escreve.unwrap_err().corpo()["erro"]["codigo"],
        "token_ausente"
    );
    assert_eq!(Motivo::TokenAusente.status(), 401);
}
