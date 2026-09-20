//! US3 — coleta retomável, sem credencial e sem vela em formação.

mod common;

use chrono::{Duration, TimeZone, Utc};
use common::{Resposta, ServidorFalso, corpo};
use tempfile::tempdir;
use trade_bybit::{BybitClient, Collector};
use trade_domain::{Interval, Symbol};
use trade_ports::CandleRepository;
use trade_storage::{SqliteCandleRepository, open_market};

fn base_ms() -> i64 {
    Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
        .unwrap()
        .timestamp_millis()
}

fn minuto(n: i64) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap() + Duration::minutes(n)
}

fn repo() -> (tempfile::TempDir, SqliteCandleRepository) {
    let dir = tempdir().unwrap();
    let conn = open_market(dir.path().join("m.db")).unwrap();
    (dir, SqliteCandleRepository::new(conn))
}

fn btc() -> Symbol {
    Symbol::new("BTCUSDT").unwrap()
}

#[test]
fn a_pagina_e_gravada_em_ordem_cronologica() {
    let velas: Vec<(i64, &str)> = (0..5).map(|i| (base_ms() + i * 60_000, "63000")).collect();
    let s = ServidorFalso::novo(vec![Resposta::Ok(corpo(&velas)), Resposta::Ok(corpo(&[]))]);
    let (_d, mut r) = repo();

    let c = BybitClient::with_base_url(&s.base_url);
    let out = Collector::new(&c, &mut r, 3)
        .sem_espera()
        .collect(&btc(), Interval::M1, minuto(0), minuto(60), minuto(120))
        .unwrap();

    assert_eq!(out.velas_novas, 5);
    let cov = r.coverage(&btc(), Interval::M1).unwrap().unwrap();
    assert_eq!(cov.first, minuto(0));
    assert_eq!(cov.last, minuto(4));
}

#[test]
fn a_vela_em_formacao_e_descartada() {
    // Agora é 00:04:30, então a vela do minuto 4 ainda não fechou. Gravá-la
    // significaria gravar um fechamento que ainda vai mudar — duas coletas do
    // mesmo período dariam históricos diferentes, sem erro visível.
    let velas: Vec<(i64, &str)> = (0..5).map(|i| (base_ms() + i * 60_000, "63000")).collect();
    let s = ServidorFalso::novo(vec![Resposta::Ok(corpo(&velas)), Resposta::Ok(corpo(&[]))]);
    let (_d, mut r) = repo();

    let agora = minuto(4) + Duration::seconds(30);
    let c = BybitClient::with_base_url(&s.base_url);
    let out = Collector::new(&c, &mut r, 3)
        .sem_espera()
        .collect(&btc(), Interval::M1, minuto(0), minuto(60), agora)
        .unwrap();

    assert_eq!(out.descartadas_em_formacao, 1);
    assert_eq!(out.velas_novas, 4);
    assert_eq!(
        r.coverage(&btc(), Interval::M1).unwrap().unwrap().last,
        minuto(3)
    );
}

#[test]
fn repetir_a_coleta_nao_duplica_nada() {
    let velas: Vec<(i64, &str)> = (0..5).map(|i| (base_ms() + i * 60_000, "63000")).collect();
    let s = ServidorFalso::novo(vec![Resposta::Ok(corpo(&velas)), Resposta::Ok(corpo(&[]))]);
    let (_d, mut r) = repo();
    let c = BybitClient::with_base_url(&s.base_url);

    let primeira = Collector::new(&c, &mut r, 3)
        .sem_espera()
        .collect(&btc(), Interval::M1, minuto(0), minuto(60), minuto(120))
        .unwrap();
    assert_eq!(primeira.velas_novas, 5);

    let segunda = Collector::new(&c, &mut r, 3)
        .sem_espera()
        .collect(&btc(), Interval::M1, minuto(0), minuto(60), minuto(120))
        .unwrap();

    // Retoma do fim do que já existe: não há nada novo a buscar.
    assert_eq!(segunda.velas_novas, 0);

    let total: i64 = r
        .connection()
        .query_row("SELECT COUNT(*) FROM candle", [], |x| x.get(0))
        .unwrap();
    let distintos: i64 = r
        .connection()
        .query_row("SELECT COUNT(DISTINCT open_ms) FROM candle", [], |x| {
            x.get(0)
        })
        .unwrap();
    assert_eq!(
        total, distintos,
        "a chave primária torna duplicata impossível"
    );
    assert_eq!(total, 5);
}

#[test]
fn excesso_de_requisicoes_e_retentado_sozinho() {
    // Duas recusas com 403, depois sucesso. Com teto de 3, a coleta prossegue
    // sem interromper nada e sem pedir intervenção.
    let velas: Vec<(i64, &str)> = (0..3).map(|i| (base_ms() + i * 60_000, "63000")).collect();
    let s = ServidorFalso::novo(vec![
        Resposta::Status(403),
        Resposta::Status(403),
        Resposta::Ok(corpo(&velas)),
        Resposta::Ok(corpo(&[])),
    ]);
    let (_d, mut r) = repo();

    let c = BybitClient::with_base_url(&s.base_url);
    let out = Collector::new(&c, &mut r, 3)
        .sem_espera()
        .collect(&btc(), Interval::M1, minuto(0), minuto(60), minuto(120))
        .unwrap();

    assert_eq!(out.tentativas_extras, 2);
    assert_eq!(out.velas_novas, 3);
}

#[test]
fn tentativas_esgotadas_falham_sem_corromper_o_que_ja_existe() {
    let velas: Vec<(i64, &str)> = (0..3).map(|i| (base_ms() + i * 60_000, "63000")).collect();
    let (_d, mut r) = repo();

    // Primeira coleta grava três velas.
    let s1 = ServidorFalso::novo(vec![Resposta::Ok(corpo(&velas)), Resposta::Ok(corpo(&[]))]);
    let c1 = BybitClient::with_base_url(&s1.base_url);
    Collector::new(&c1, &mut r, 2)
        .sem_espera()
        .collect(&btc(), Interval::M1, minuto(0), minuto(3), minuto(120))
        .unwrap();

    // Segunda coleta, período adiante, com a fonte fora do ar.
    let s2 = ServidorFalso::novo(vec![Resposta::Status(503)]);
    let c2 = BybitClient::with_base_url(&s2.base_url);
    let erro = Collector::new(&c2, &mut r, 2).sem_espera().collect(
        &btc(),
        Interval::M1,
        minuto(10),
        minuto(20),
        minuto(120),
    );

    assert!(erro.is_err());
    let cov = r.coverage(&btc(), Interval::M1).unwrap().unwrap();
    assert_eq!(cov.count, 3, "o que já estava gravado não foi tocado");
}

#[test]
fn resposta_de_erro_da_fonte_nao_vira_historico_vazio() {
    let s = ServidorFalso::novo(vec![Resposta::Ok(
        r#"{"retCode":10001,"retMsg":"params error","result":{}}"#.to_string(),
    )]);
    let (_d, mut r) = repo();
    let c = BybitClient::with_base_url(&s.base_url);

    let erro = Collector::new(&c, &mut r, 1).sem_espera().collect(
        &btc(),
        Interval::M1,
        minuto(0),
        minuto(60),
        minuto(120),
    );

    assert!(erro.is_err());
    assert!(r.coverage(&btc(), Interval::M1).unwrap().is_none());
}

#[test]
fn lacunas_sao_detectadas_e_gravadas() {
    // Faltam os minutos 2 a 9.
    let velas: Vec<(i64, &str)> = [0i64, 1, 10, 11]
        .iter()
        .map(|i| (base_ms() + i * 60_000, "63000"))
        .collect();
    let s = ServidorFalso::novo(vec![Resposta::Ok(corpo(&velas)), Resposta::Ok(corpo(&[]))]);
    let (_d, mut r) = repo();

    let c = BybitClient::with_base_url(&s.base_url);
    let out = Collector::new(&c, &mut r, 3)
        .sem_espera()
        .collect(&btc(), Interval::M1, minuto(0), minuto(60), minuto(120))
        .unwrap();

    assert_eq!(out.lacunas.len(), 1);
    let gravadas: i64 = r
        .connection()
        .query_row("SELECT COUNT(*) FROM gap", [], |x| x.get(0))
        .unwrap();
    assert_eq!(gravadas, 1, "a lacuna fica registrada junto do histórico");
}

#[test]
fn a_consulta_envia_category_spot_explicitamente() {
    // A documentação diz que `category` assume `linear` quando omitido, e
    // linear é perpétuo — outro mercado. Omitir traria dados que parecem
    // certos e não são.
    let velas: Vec<(i64, &str)> = (0..2).map(|i| (base_ms() + i * 60_000, "63000")).collect();
    let s = ServidorFalso::novo(vec![Resposta::Ok(corpo(&velas)), Resposta::Ok(corpo(&[]))]);
    let (_d, mut r) = repo();

    let c = BybitClient::with_base_url(&s.base_url);
    Collector::new(&c, &mut r, 3)
        .sem_espera()
        .collect(&btc(), Interval::M1, minuto(0), minuto(60), minuto(120))
        .unwrap();

    let consulta = s.consulta(0);
    assert!(consulta.contains("category=spot"), "consulta: {consulta}");
    assert!(consulta.contains("symbol=BTCUSDT"), "consulta: {consulta}");
    assert!(consulta.contains("interval=1"), "consulta: {consulta}");
}

#[test]
fn nenhuma_credencial_e_enviada() {
    // FR-012: o endpoint é público e a coleta não recebe credencial. A
    // garantia mais simples é não haver nada a enviar.
    let velas: Vec<(i64, &str)> = (0..2).map(|i| (base_ms() + i * 60_000, "63000")).collect();
    let s = ServidorFalso::novo(vec![Resposta::Ok(corpo(&velas)), Resposta::Ok(corpo(&[]))]);
    let (_d, mut r) = repo();

    let c = BybitClient::with_base_url(&s.base_url);
    Collector::new(&c, &mut r, 3)
        .sem_espera()
        .collect(&btc(), Interval::M1, minuto(0), minuto(60), minuto(120))
        .unwrap();

    let consulta = s.consulta(0).to_lowercase();
    for proibido in ["api_key", "apikey", "sign", "timestamp=", "recv_window"] {
        assert!(
            !consulta.contains(proibido),
            "consulta contém '{proibido}': {consulta}"
        );
    }
}
