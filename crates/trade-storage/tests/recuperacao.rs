//! T024 e T025 — o estado sobrevive ao reinício do processo (FR-112).
//!
//! O `runs.db` deixa de ser só auditoria e passa a ser fonte de recuperação.
//! Posição, ordens em aberto e contadores de risco vêm do registro, nunca da
//! memória de um processo que já morreu.
//!
//! Como se recupera é a **decisão 030** do Jev: a posição se reconstitui
//! replicando os preenchimentos (`replicar_os_preenchimentos`, 1,00 ·
//! confiança 1,00), e a linha de base do dia se deduz do último retrato
//! (`deduzir_a_linha_de_base`, 0,59 · confiança 0,45 — margem fina, contra
//! 0,23 de retomar do retrato sem deduzir).

use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use rust_decimal::dec;
use tempfile::tempdir;
use trade_domain::{
    AuditKind, ExecutionMode, Fill, Order, OrderId, RiskDecision, RiskLimits, RiskState, Side,
    SignalId, Verdict,
};
use trade_ports::{AuditRecorder, Recorder};
use trade_storage::audit_sink::SqliteAuditSink;
use trade_storage::{open_runs, recuperacao::recuperar};

/// Instante do dia 1, à hora cheia.
fn d1(h: u32, m: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 1, h, m, 0).unwrap()
}

/// Instante do dia 2, à hora cheia.
fn d2(h: u32, m: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 2, h, m, 0).unwrap()
}

fn gravador(dir: &std::path::Path) -> AuditRecorder<SqliteAuditSink> {
    let sink = SqliteAuditSink::new(open_runs(dir.join("runs.db")).unwrap());
    AuditRecorder::new("R1", ExecutionMode::Paper, sink)
}

/// Ordem e preenchimento correspondente, como o motor os grava.
fn negociar(
    rec: &mut AuditRecorder<SqliteAuditSink>,
    id: u64,
    side: Side,
    qty: rust_decimal::Decimal,
    preco: rust_decimal::Decimal,
    at: DateTime<Utc>,
) {
    rec.record(
        at,
        AuditKind::Order(Order {
            id: OrderId(id),
            signal_ref: SignalId(id),
            side,
            qty,
            at,
        }),
    )
    .unwrap();
    rec.record(
        at,
        AuditKind::Fill(Fill {
            order_ref: OrderId(id),
            price: preco,
            qty,
            fee: dec!(0),
            fee_base: dec!(0),
            slippage: dec!(0),
            at,
        }),
    )
    .unwrap();
}

/// Retrato de risco, com o estado que o guard tinha naquele instante.
fn retrato(
    rec: &mut AuditRecorder<SqliteAuditSink>,
    id: u64,
    veredito: Verdict,
    daily_pnl: rust_decimal::Decimal,
    exposure: rust_decimal::Decimal,
    bloqueado: bool,
    at: DateTime<Utc>,
) {
    rec.record(
        at,
        AuditKind::RiskDecision(RiskDecision {
            order_ref: OrderId(id),
            verdict: veredito,
            limits: RiskLimits::default(),
            state: RiskState {
                daily_pnl,
                exposure,
                orders_in_window: 0,
                day: Some(at.date_naive()),
                daily_loss_blocked: bloqueado,
            },
        }),
    )
    .unwrap();
}

fn ler(dir: &std::path::Path, agora: DateTime<Utc>, janela: i64) -> trade_domain::EstadoRetomado {
    let conn = open_runs(dir.join("runs.db")).unwrap();
    recuperar(&conn, "R1", agora, janela)
        .unwrap()
        .expect("a execução tem eventos e deve recuperar")
}

// ---------------------------------------------------------------- T024

#[test]
fn a_posicao_volta_do_registro() {
    let dir = tempdir().unwrap();
    {
        let mut rec = gravador(dir.path());
        negociar(&mut rec, 1, Side::Buy, dec!(2), dec!(100), d1(9, 0));
        negociar(&mut rec, 2, Side::Buy, dec!(2), dec!(200), d1(9, 30));
        rec.flush().unwrap();
    }
    let e = ler(dir.path(), d1(10, 0), 15);
    assert_eq!(e.position.qty(), dec!(4));
    assert_eq!(
        e.position.avg_price(),
        dec!(150),
        "o preço médio sai do replay, não de um campo gravado"
    );
}

#[test]
fn a_venda_replicada_devolve_o_resultado_realizado() {
    let dir = tempdir().unwrap();
    {
        let mut rec = gravador(dir.path());
        negociar(&mut rec, 1, Side::Buy, dec!(2), dec!(100), d1(9, 0));
        negociar(&mut rec, 2, Side::Sell, dec!(1), dec!(90), d1(9, 30));
        rec.flush().unwrap();
    }
    let e = ler(dir.path(), d1(10, 0), 15);
    assert_eq!(e.position.qty(), dec!(1));
    assert_eq!(e.position.realized_pnl(), dec!(-10));
}

#[test]
fn o_instante_de_abertura_da_posicao_volta() {
    // É o que o prazo de 72 horas da T027 vai consultar. Ler o último
    // state_transition daria quantidade e preço médio e perderia isto.
    let dir = tempdir().unwrap();
    {
        let mut rec = gravador(dir.path());
        negociar(&mut rec, 1, Side::Buy, dec!(1), dec!(100), d1(9, 0));
        negociar(&mut rec, 2, Side::Buy, dec!(1), dec!(100), d1(23, 0));
        rec.flush().unwrap();
    }
    let e = ler(dir.path(), d2(10, 0), 15);
    let idade = d2(10, 0) - e.position.opened_at().expect("posição aberta");
    assert_eq!(
        idade,
        Duration::hours(25),
        "a idade conta da primeira compra, não da última"
    );
}

#[test]
fn o_seq_continua_sem_buraco() {
    let dir = tempdir().unwrap();
    {
        let mut rec = gravador(dir.path());
        negociar(&mut rec, 1, Side::Buy, dec!(1), dec!(100), d1(9, 0));
        retrato(
            &mut rec,
            1,
            Verdict::Accepted,
            dec!(0),
            dec!(100),
            false,
            d1(9, 0),
        );
        rec.flush().unwrap();
    }
    let e = ler(dir.path(), d1(10, 0), 15);
    assert_eq!(e.proximo_seq, 4, "três eventos gravados; o próximo é o 4");
}

#[test]
fn execucao_sem_evento_nenhum_nao_recupera_nada() {
    let dir = tempdir().unwrap();
    let conn = open_runs(dir.path().join("runs.db")).unwrap();
    assert!(
        recuperar(&conn, "R1", d1(10, 0), 15).unwrap().is_none(),
        "sem registro não há o que retomar, e inventar zero seria pior"
    );
}

// ---------------------------------------------------------------- T025

#[test]
fn a_janela_de_ordens_volta_e_so_conta_as_aceitas() {
    // O limite conta ordens **aceitas**: recusada nunca chegou ao mercado.
    let dir = tempdir().unwrap();
    {
        let mut rec = gravador(dir.path());
        retrato(
            &mut rec,
            1,
            Verdict::Accepted,
            dec!(0),
            dec!(0),
            false,
            d1(13, 50),
        );
        retrato(
            &mut rec,
            2,
            Verdict::Rejected(trade_domain::LimitBreach::MaxPositionSize),
            dec!(0),
            dec!(0),
            false,
            d1(13, 55),
        );
        retrato(
            &mut rec,
            3,
            Verdict::Accepted,
            dec!(0),
            dec!(0),
            false,
            d1(13, 58),
        );
        rec.flush().unwrap();
    }
    let e = ler(dir.path(), d1(14, 0), 15);
    assert_eq!(e.aceitas, vec![d1(13, 50), d1(13, 58)]);
}

#[test]
fn a_ordem_fora_da_janela_nao_volta() {
    let dir = tempdir().unwrap();
    {
        let mut rec = gravador(dir.path());
        retrato(
            &mut rec,
            1,
            Verdict::Accepted,
            dec!(0),
            dec!(0),
            false,
            d1(13, 0),
        );
        retrato(
            &mut rec,
            2,
            Verdict::Accepted,
            dec!(0),
            dec!(0),
            false,
            d1(13, 50),
        );
        rec.flush().unwrap();
    }
    let e = ler(dir.path(), d1(14, 0), 15);
    assert_eq!(e.aceitas, vec![d1(13, 50)], "a das 13h já saiu da janela");
}

#[test]
fn o_bloqueio_de_perda_diaria_volta_travado() {
    // O freio trava até a virada do dia. Reiniciar o processo não é virada
    // de dia, e não pode ser a forma barata de destravá-lo.
    let dir = tempdir().unwrap();
    {
        let mut rec = gravador(dir.path());
        retrato(
            &mut rec,
            1,
            Verdict::Accepted,
            dec!(-500),
            dec!(0),
            true,
            d1(13, 50),
        );
        rec.flush().unwrap();
    }
    let e = ler(dir.path(), d1(14, 0), 15);
    assert!(e.daily_loss_blocked);
    assert_eq!(e.day, Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()));
}

#[test]
fn a_linha_de_base_do_dia_e_deduzida_do_ultimo_retrato() {
    // Decisão 030. A posição atravessou a virada: a linha de base do dia 2
    // é o aberto que ela tinha na virada, e esse número **não é gravado**.
    //
    // Compra 2 @ 100 no dia 1. No dia 2, às 10h, o retrato diz resultado do
    // dia −10 com exposição 170 — isto é, preço 85 e aberto −30. Das três
    // parcelas, duas são conhecidas, e a linha de base sai por diferença:
    // −30 − (−10) = −20, o aberto que a posição tinha na virada.
    let dir = tempdir().unwrap();
    {
        let mut rec = gravador(dir.path());
        negociar(&mut rec, 1, Side::Buy, dec!(2), dec!(100), d1(9, 0));
        retrato(
            &mut rec,
            2,
            Verdict::Accepted,
            dec!(-10),
            dec!(170),
            false,
            d2(10, 0),
        );
        rec.flush().unwrap();
    }
    let e = ler(dir.path(), d2(14, 0), 15);
    assert_eq!(e.unrealized, dec!(-30), "aberto no instante do retrato");
    assert_eq!(e.realized_today, dec!(0), "nenhuma venda no dia 2");
    assert_eq!(
        e.unrealized_at_day_start,
        dec!(-20),
        "a linha de base sai por diferença, e nunca foi gravada"
    );
    assert_eq!(
        e.realized_today + (e.unrealized - e.unrealized_at_day_start),
        dec!(-10),
        "as três parcelas recompõem exatamente o resultado do retrato"
    );
}

#[test]
fn o_realizado_do_dia_conta_so_as_vendas_do_dia() {
    let dir = tempdir().unwrap();
    {
        let mut rec = gravador(dir.path());
        negociar(&mut rec, 1, Side::Buy, dec!(4), dec!(100), d1(9, 0));
        negociar(&mut rec, 2, Side::Sell, dec!(1), dec!(90), d1(10, 0));
        negociar(&mut rec, 3, Side::Sell, dec!(1), dec!(80), d2(10, 0));
        retrato(
            &mut rec,
            4,
            Verdict::Accepted,
            dec!(-20),
            dec!(160),
            false,
            d2(11, 0),
        );
        rec.flush().unwrap();
    }
    let e = ler(dir.path(), d2(14, 0), 15);
    assert_eq!(
        e.realized_today,
        dec!(-20),
        "a venda do dia 1 não conta contra o limite do dia 2"
    );
    assert_eq!(
        e.position.realized_pnl(),
        dec!(-30),
        "o acumulado é dos dois dias"
    );
}
