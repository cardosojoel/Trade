//! A janela de cada página precisa caber no limite da fonte.
//!
//! Defeito encontrado rodando uma coleta real: pedido o dia 2026-09-18
//! inteiro, a fonte devolveu de 07:21 em diante — as mil velas **mais
//! recentes** da janela, não as mil mais antigas. A paginação para frente
//! encerrou achando que tinha terminado, e 441 minutos nunca foram buscados.

mod common;

use chrono::{Duration, TimeZone, Utc};
use common::ServidorRealista;
use tempfile::tempdir;
use trade_bybit::{BybitClient, Collector};
use trade_domain::{Interval, Symbol};
use trade_ports::CandleRepository;
use trade_storage::{SqliteCandleRepository, open_market};

fn minuto(n: i64) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap() + Duration::minutes(n)
}

#[test]
fn um_periodo_maior_que_uma_pagina_e_coletado_por_inteiro() {
    // 1440 velas — um dia de velas de um minuto — contra uma fonte que só
    // entrega 1000 por requisição, pelas mais recentes da janela.
    let total = 1440;
    let s = ServidorRealista::novo(minuto(0).timestamp_millis(), total);
    let dir = tempdir().unwrap();
    let conn = open_market(dir.path().join("m.db")).unwrap();
    let mut repo = SqliteCandleRepository::new(conn);
    let btc = Symbol::new("BTCUSDT").unwrap();

    let client = BybitClient::with_base_url(&s.base_url);
    let out = Collector::new(&client, &mut repo, 3)
        .sem_espera()
        .collect(
            &btc,
            Interval::M1,
            minuto(0),
            minuto(total),
            minuto(total + 60),
        )
        .unwrap();

    let cov = repo.coverage(&btc, Interval::M1).unwrap().unwrap();

    assert_eq!(
        cov.count, total as u64,
        "faltaram velas: coletadas {} de {}",
        cov.count, total
    );
    assert_eq!(cov.first, minuto(0), "o começo do período precisa estar lá");
    assert_eq!(cov.last, minuto(total - 1));
    assert!(
        out.paginas >= 2,
        "um dia de velas de um minuto não cabe em uma página"
    );
}

#[test]
fn a_retomada_continua_de_onde_parou_em_periodo_longo() {
    let total = 2500;
    let s = ServidorRealista::novo(minuto(0).timestamp_millis(), total);
    let dir = tempdir().unwrap();
    let conn = open_market(dir.path().join("m.db")).unwrap();
    let mut repo = SqliteCandleRepository::new(conn);
    let btc = Symbol::new("BTCUSDT").unwrap();
    let client = BybitClient::with_base_url(&s.base_url);

    Collector::new(&client, &mut repo, 3)
        .sem_espera()
        .collect(
            &btc,
            Interval::M1,
            minuto(0),
            minuto(total),
            minuto(total + 60),
        )
        .unwrap();

    let segunda = Collector::new(&client, &mut repo, 3)
        .sem_espera()
        .collect(
            &btc,
            Interval::M1,
            minuto(0),
            minuto(total),
            minuto(total + 60),
        )
        .unwrap();

    assert_eq!(segunda.velas_novas, 0, "nada novo a buscar");
    assert_eq!(
        repo.coverage(&btc, Interval::M1).unwrap().unwrap().count,
        total as u64
    );
}
