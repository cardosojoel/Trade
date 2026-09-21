//! T040 — a fonte de velas de uma sessão ao vivo.
//!
//! Contra o mesmo servidor local que os demais testes desta crate usam.
//! Nenhum toca a rede de verdade.

mod common;

use chrono::{DateTime, Duration, TimeZone, Utc};
use common::{Resposta, ServidorFalso, corpo};
use trade_bybit::{BybitClient, FonteAoVivo};
use trade_domain::{Interval, Symbol};
use trade_ports::LiveCandleSource;
use trade_ports::testing::FakeClock;

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
}

fn minuto(n: i64) -> DateTime<Utc> {
    t0() + Duration::minutes(n)
}

fn btc() -> Symbol {
    Symbol::new("BTCUSDT").unwrap()
}

/// Cinco velas de um minuto, da 0 à 4.
fn cinco() -> String {
    let v: Vec<(i64, &str)> = (0..5)
        .map(|i| (t0().timestamp_millis() + i * 60_000, "63000"))
        .collect();
    corpo(&v)
}

#[test]
fn devolve_a_mais_antiga_ainda_nao_entregue_e_nao_a_mais_recente() {
    // Ficar para trás é recuperável; pular vela deixa a estratégia cega sobre
    // o que aconteceu no meio.
    let s = ServidorFalso::novo(vec![Resposta::Ok(cinco())]);
    let c = BybitClient::with_base_url(&s.base_url);
    // Agora são 00:10: as cinco velas estão todas fechadas.
    let relogio = FakeClock::at(minuto(10));

    let mut f = FonteAoVivo::nova(&c, &relogio, btc(), Interval::M1).desistindo_apos(1);

    assert_eq!(f.proxima().unwrap().unwrap().open_time, minuto(0));
    assert_eq!(f.proxima().unwrap().unwrap().open_time, minuto(1));
    assert_eq!(f.proxima().unwrap().unwrap().open_time, minuto(2));
}

#[test]
fn a_vela_em_formacao_nao_e_entregue() {
    // Agora são 00:04:30. A vela das 00:04 ainda está aberta, e o preço de
    // fechamento dela é só o último negócio — vai mudar.
    let s = ServidorFalso::novo(vec![Resposta::Ok(cinco())]);
    let c = BybitClient::with_base_url(&s.base_url);
    let relogio = FakeClock::at(minuto(4) + Duration::seconds(30));

    let mut f = FonteAoVivo::nova(&c, &relogio, btc(), Interval::M1)
        .desistindo_apos(1)
        .depois_de(minuto(2));

    assert_eq!(f.proxima().unwrap().unwrap().open_time, minuto(3));
    assert!(
        f.proxima().unwrap().is_none(),
        "a vela das 00:04 ainda não fechou: a fonte espera, não a entrega"
    );
}

#[test]
fn a_sessao_retomada_nao_repete_o_que_ja_processou() {
    let s = ServidorFalso::novo(vec![Resposta::Ok(cinco())]);
    let c = BybitClient::with_base_url(&s.base_url);
    let relogio = FakeClock::at(minuto(10));

    let mut f = FonteAoVivo::nova(&c, &relogio, btc(), Interval::M1)
        .desistindo_apos(1)
        .depois_de(minuto(3));

    assert_eq!(f.proxima().unwrap().unwrap().open_time, minuto(4));
    assert!(f.proxima().unwrap().is_none(), "não há mais nada fechado");
}

#[test]
fn sem_novidade_a_fonte_desiste_em_vez_de_pendurar_o_processo() {
    let s = ServidorFalso::novo(vec![Resposta::Ok(corpo(&[]))]);
    let c = BybitClient::with_base_url(&s.base_url);
    let relogio = FakeClock::at(minuto(10));

    let mut f = FonteAoVivo::nova(&c, &relogio, btc(), Interval::M1)
        .desistindo_apos(3)
        .com_espera(std::time::Duration::ZERO);

    assert!(f.proxima().unwrap().is_none());
    assert_eq!(
        s.requisicoes.load(std::sync::atomic::Ordering::SeqCst),
        3,
        "sondou três vezes antes de desistir"
    );
}

#[test]
fn a_consulta_declara_spot_explicitamente() {
    // `category` assume `linear` quando omitido, e linear é perpétuo — outro
    // mercado. O teste de arquitetura já cobra isso; aqui é o caminho real.
    let s = ServidorFalso::novo(vec![Resposta::Ok(cinco())]);
    let c = BybitClient::with_base_url(&s.base_url);
    let relogio = FakeClock::at(minuto(10));

    let mut f = FonteAoVivo::nova(&c, &relogio, btc(), Interval::M1).desistindo_apos(1);
    f.proxima().unwrap();

    let consultas = s.consultas.lock().unwrap();
    assert!(
        consultas[0].contains("category=spot"),
        "veio {}",
        consultas[0]
    );
}
