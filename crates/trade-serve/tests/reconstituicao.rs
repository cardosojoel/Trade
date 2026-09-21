//! Fatia 3 — reconstituição (T019 a T025).
//!
//! É a US2 da spec: *reconstituir uma decisão até o sinal que a originou*, sem
//! escrever uma linha de SQL.

use rust_decimal::dec;
use serde_json::{Value, json};
use trade_domain::Trade;
use trade_serve::consultas;
use trade_storage::runs_repo::EventoLido;

fn em(dia: u32, h: u32) -> chrono::DateTime<chrono::Utc> {
    use chrono::TimeZone;
    chrono::Utc.with_ymd_and_hms(2026, 1, dia, h, 0, 0).unwrap()
}

/// Uma saída, com a entrada que a originou.
fn saida(seq: u64, entrada: (u32, u32), saida_em: (u32, u32), pnl: rust_decimal::Decimal) -> Trade {
    Trade {
        seq,
        entry_at: em(entrada.0, entrada.1),
        entry_price: dec!(100),
        exit_at: em(saida_em.0, saida_em.1),
        exit_price: dec!(110),
        qty: dec!(1),
        fees: dec!(0.25),
        pnl,
    }
}

fn evento(seq: u64, dia: u32, h: u32, kind: &str, payload: Value) -> EventoLido {
    EventoLido {
        seq,
        at: em(dia, h),
        kind: kind.to_string(),
        payload,
    }
}

// ---------------------------------------------------------------- T019

#[test]
fn as_saidas_da_mesma_entrada_viram_um_episodio_so() {
    // Um episódio abre uma vez e pode sair muitas. Na execução real, 146
    // episódios tiveram 28.327 saídas: contar as saídas como se fossem idas e
    // voltas multiplica a atividade por duzentos.
    let extrato = vec![
        saida(1, (1, 0), (1, 5), dec!(10)),
        saida(2, (1, 0), (1, 8), dec!(-4)),
        saida(3, (2, 0), (2, 3), dec!(7)),
    ];
    let eps = consultas::episodios(&extrato, &[]);
    assert_eq!(eps.len(), 2);
    assert_eq!(eps[0].saidas, 2);
    assert_eq!(eps[0].resultado, dec!(6), "as duas saídas somadas");
    assert_eq!(eps[0].taxas, dec!(0.5));
    assert_eq!(eps[1].saidas, 1);
}

#[test]
fn o_episodio_traz_entrada_fechamento_e_duracao() {
    let eps = consultas::episodios(&[saida(1, (1, 0), (3, 12), dec!(1))], &[]);
    let e = &eps[0];
    assert_eq!(e.entrada, em(1, 0));
    assert_eq!(e.fechamento, em(3, 12));
    assert_eq!(e.duracao_horas.to_string(), "60", "de 01 00h a 03 12h");
}

// ---------------------------------------------------------------- T020

#[test]
fn o_episodio_diz_o_que_o_fechou_quando_o_registro_sabe() {
    let extrato = vec![saida(1, (1, 0), (1, 5), dec!(10))];
    let fechamentos = vec![evento(
        9,
        1,
        5,
        "state_transition",
        json!({"from":"Long","to":"Flat","fechado_por":"prazo"}),
    )];
    let eps = consultas::episodios(&extrato, &fechamentos);
    assert_eq!(eps[0].fechado_por.as_deref(), Some("prazo"));
}

#[test]
fn episodio_gravado_antes_do_campo_existir_traz_nulo_e_a_resposta_avisa() {
    // FR-010: presente e nulo enquanto o registro não distinguir. O aviso ao
    // lado existe para que quem lê não conclua que ninguém sabe — o registro
    // é de antes de o campo existir, e isso é dizível.
    let extrato = vec![saida(1, (1, 0), (1, 5), dec!(10))];
    let fechamentos = vec![evento(
        9,
        1,
        5,
        "state_transition",
        json!({"from":"Long","to":"Flat","fechado_por": Value::Null}),
    )];
    let eps = consultas::episodios(&extrato, &fechamentos);
    assert_eq!(eps[0].fechado_por, None);

    let corpo = consultas::episodios_json("01A", &eps);
    assert!(
        corpo["episodios"][0].get("fechado_por").is_some(),
        "presente"
    );
    assert!(corpo["episodios"][0]["fechado_por"].is_null(), "e nulo");
    assert!(
        corpo["aviso"].as_str().unwrap().contains("não distingue"),
        "a resposta diz por que está nulo: {}",
        corpo["aviso"]
    );
}

#[test]
fn sem_episodio_nulo_nao_ha_aviso() {
    let extrato = vec![saida(1, (1, 0), (1, 5), dec!(10))];
    let fechamentos = vec![evento(
        9,
        1,
        5,
        "state_transition",
        json!({"from":"Long","to":"Flat","fechado_por":"sinal"}),
    )];
    let eps = consultas::episodios(&extrato, &fechamentos);
    let corpo = consultas::episodios_json("01A", &eps);
    assert!(corpo["aviso"].is_null(), "não há o que avisar");
}

// ---------------------------------------------------------------- T021

#[test]
fn cada_saida_traz_a_taxa_ao_lado_do_resultado() {
    // REQ-UI-030. Sem a taxa ao lado, o resultado de uma saída não se
    // explica: parte dele é custo de transação, e a tela não teria como
    // dizer quanto.
    let extrato = vec![
        saida(1, (1, 0), (1, 5), dec!(10)),
        saida(2, (1, 0), (1, 8), dec!(-4)),
    ];
    let c = consultas::saidas_json("01A", em(1, 0), &extrato, 500);
    let linhas = c["saidas"].as_array().unwrap();
    assert_eq!(linhas.len(), 2);
    assert_eq!(linhas[0]["resultado"], "10");
    assert_eq!(linhas[0]["taxas"], "0.25");
    assert!(linhas[0]["preco_de_saida"].is_string());
}

#[test]
fn as_saidas_pedidas_sao_so_as_do_episodio_pedido() {
    let extrato = vec![
        saida(1, (1, 0), (1, 5), dec!(10)),
        saida(2, (2, 0), (2, 5), dec!(99)),
    ];
    let c = consultas::saidas_json("01A", em(2, 0), &extrato, 500);
    let linhas = c["saidas"].as_array().unwrap();
    assert_eq!(linhas.len(), 1);
    assert_eq!(linhas[0]["resultado"], "99");
}

// ---------------------------------------------------------------- T025

#[test]
fn a_colecao_declara_o_teto_e_diz_quando_cortou() {
    // FR-025. O maior episódio observado teve 432 saídas e o teto de partida
    // é 500 — mas uma resposta cortada em silêncio faria a tela somar errado
    // sem saber que somou errado.
    let extrato: Vec<Trade> = (1..=10)
        .map(|i| saida(i, (1, 0), (1, u32::try_from(i).unwrap()), dec!(1)))
        .collect();

    let inteira = consultas::saidas_json("01A", em(1, 0), &extrato, 500);
    assert_eq!(inteira["teto"], 500);
    assert_eq!(inteira["cortou"], false);
    assert_eq!(inteira["total"], 10);

    let cortada = consultas::saidas_json("01A", em(1, 0), &extrato, 4);
    assert_eq!(cortada["saidas"].as_array().unwrap().len(), 4);
    assert_eq!(cortada["cortou"], true);
    assert_eq!(
        cortada["total"], 10,
        "o total é o que existe, não o que coube"
    );
}

// ---------------------------------------------------------------- T022

fn cadeia_completa() -> Vec<EventoLido> {
    vec![
        evento(
            40,
            1,
            3,
            "signal",
            json!({"signal_id":7,"intent":"Buy","qty":"1"}),
        ),
        evento(
            41,
            1,
            3,
            "order",
            json!({"order_id":12,"signal_ref":7,"side":"Buy","qty":"1"}),
        ),
        evento(
            42,
            1,
            3,
            "risk_decision",
            json!({
                "order_ref":12, "verdict":"Accepted", "breach": Value::Null,
                "limits": {"max_position_size":"1000.00","max_daily_loss":"200.00",
                           "max_total_exposure":"2000.00","max_orders_per_window":10,
                           "window_minutes":60},
                "state": {"daily_pnl":"-12.50","exposure":"110.00",
                          "orders_in_window":3,"daily_loss_blocked":false}
            }),
        ),
        evento(
            43,
            1,
            3,
            "fill",
            json!({"order_ref":12,"price":"110","qty":"0.4","fee":"0.1","fee_base":"0","slippage":"0"}),
        ),
        evento(
            44,
            1,
            3,
            "state_transition",
            json!({"from":"Flat","to":"Long","qty":"0.4","avg_price":"110","fechado_por":Value::Null}),
        ),
    ]
}

#[test]
fn a_cadeia_devolve_os_cinco_elos_em_ordem_de_seq() {
    let c = consultas::cadeia(&cadeia_completa(), 43).expect("o fill existe");
    let tipos: Vec<&str> = c["elos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["tipo"].as_str().unwrap())
        .collect();
    assert_eq!(
        tipos,
        vec![
            "signal",
            "order",
            "risk_decision",
            "fill",
            "state_transition"
        ]
    );
    let seqs: Vec<u64> = c["elos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["seq"].as_u64().unwrap())
        .collect();
    assert_eq!(seqs, vec![40, 41, 42, 43, 44]);
}

#[test]
fn a_cadeia_de_um_seq_que_nao_existe_e_nada() {
    assert!(consultas::cadeia(&cadeia_completa(), 999).is_none());
}

// ---------------------------------------------------------------- T023

#[test]
fn a_decisao_de_risco_traz_o_observado_ao_lado_do_teto() {
    // REQ-UI-025: o veredito diz que passou, a folga diz o quanto faltava.
    // Uma resposta que só dissesse "Accepted" esconderia que a ordem passou
    // raspando.
    let c = consultas::cadeia(&cadeia_completa(), 43).unwrap();
    let risco = &c["elos"][2];
    assert_eq!(risco["veredito"], "Accepted");
    let limites = risco["limites"].as_array().unwrap();
    let exposicao = limites
        .iter()
        .find(|l| l["limite"] == "max_total_exposure")
        .expect("exposição está entre os limites");
    assert_eq!(exposicao["teto"], "2000.00");
    assert_eq!(exposicao["observado"], "110.00");
    let janela = limites
        .iter()
        .find(|l| l["limite"] == "max_orders_per_window")
        .unwrap();
    assert_eq!(janela["teto"], 10);
    assert_eq!(janela["observado"], 3);
}

// ---------------------------------------------------------------- T024

#[test]
fn o_preenchimento_parcial_e_assinalado_com_a_proporcao() {
    // FR-014. Na execução real foram quatro ordens em 28.618, e nenhum
    // evento registra a causa — a interface mostra a divergência, e não pode
    // mostrar a causa porque a causa não está gravada.
    let c = consultas::cadeia(&cadeia_completa(), 43).unwrap();
    let fill = &c["elos"][3];
    assert_eq!(fill["divergente"], true);
    assert_eq!(fill["pedido"], "1");
    assert_eq!(fill["obtido"], "0.4");
    assert_eq!(fill["proporcao"], "0.4");
}

#[test]
fn preenchimento_integral_nao_e_assinalado() {
    let mut eventos = cadeia_completa();
    eventos[3].payload["qty"] = json!("1");
    let c = consultas::cadeia(&eventos, 43).unwrap();
    assert_eq!(c["elos"][3]["divergente"], false);
    assert_eq!(c["elos"][3]["proporcao"], "1");
}
