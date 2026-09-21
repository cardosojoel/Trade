//! US4 — o registro sustenta a reconstituição.

use chrono::{Duration, TimeZone, Utc};
use rust_decimal::dec;
use tempfile::tempdir;
use trade_domain::{
    AuditKind, ExecutionMode, Fill, Intent, LimitBreach, Order, OrderId, Position, RiskDecision,
    RiskLimits, RiskState, Side, Signal, SignalId, SignalInputs, Verdict,
};
use trade_ports::{AuditRecorder, Recorder};
use trade_storage::audit_sink::SqliteAuditSink;
use trade_storage::open_runs;

fn t(m: i64) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap() + Duration::minutes(m)
}

fn sink(dir: &std::path::Path) -> SqliteAuditSink {
    SqliteAuditSink::new(open_runs(dir.join("runs.db")).unwrap())
}

/// Grava a cadeia completa de uma operação: sinal → ordem → decisão →
/// preenchimento.
fn gravar_cadeia(rec: &mut AuditRecorder<SqliteAuditSink>) {
    rec.record(
        t(0),
        AuditKind::Signal(Signal {
            id: SignalId(41),
            at: t(0),
            intent: Intent::Buy,
            qty: Some(dec!(0.0153)),
            inputs: SignalInputs::default()
                .with_value("sma_fast", dec!(63412.10))
                .with_value("sma_slow", dec!(63180.55))
                .with_param("fast", 9),
        }),
    )
    .unwrap();

    rec.record(
        t(1),
        AuditKind::Order(Order {
            id: OrderId(58),
            signal_ref: SignalId(41),
            side: Side::Buy,
            qty: dec!(0.0153),
            at: t(1),
        }),
    )
    .unwrap();

    rec.record(
        t(1),
        AuditKind::RiskDecision(RiskDecision {
            order_ref: OrderId(58),
            verdict: Verdict::Accepted,
            limits: RiskLimits::default(),
            state: RiskState::default(),
        }),
    )
    .unwrap();

    rec.record(
        t(1),
        AuditKind::Fill(Fill {
            order_ref: OrderId(58),
            price: dec!(63420.00),
            qty: dec!(0.0153),
            fee: dec!(0.97),
            fee_base: rust_decimal::Decimal::ZERO,
            slippage: dec!(0.32),
            at: t(1),
        }),
    )
    .unwrap();
}

#[test]
fn todo_evento_carrega_o_envelope_completo() {
    // FR-033: run_id, seq, instante em UTC e tipo, sem exceção.
    let dir = tempdir().unwrap();
    let mut rec = AuditRecorder::new("R1", ExecutionMode::Backtest, sink(dir.path()));
    gravar_cadeia(&mut rec);
    rec.flush().unwrap();

    let conn = rec.sink().connection();
    let mut stmt = conn
        .prepare("SELECT run_id, seq, at_ms, kind FROM audit_event ORDER BY seq")
        .unwrap();
    let linhas: Vec<(String, i64, i64, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();

    assert_eq!(linhas.len(), 4);
    for (run_id, seq, at_ms, kind) in &linhas {
        assert_eq!(run_id, "R1");
        assert!(*seq > 0 && *at_ms > 0 && !kind.is_empty());
    }
}

#[test]
fn seq_e_ordem_total_sem_buraco_nem_repeticao() {
    // Dois eventos no mesmo instante simulado continuam ordenados entre si.
    // Sem isso, a cadeia teria um elo ambíguo dentro da mesma vela.
    let dir = tempdir().unwrap();
    let mut rec = AuditRecorder::new("R1", ExecutionMode::Backtest, sink(dir.path()));
    for _ in 0..3 {
        gravar_cadeia(&mut rec);
    }
    rec.flush().unwrap();

    let seqs: Vec<i64> = rec
        .sink()
        .connection()
        .prepare("SELECT seq FROM audit_event ORDER BY seq")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();

    assert_eq!(seqs, (1..=12).collect::<Vec<i64>>());
}

#[test]
fn a_operacao_e_reconstituida_ate_os_dados_de_entrada_so_por_sql() {
    // SC-005: partindo do preenchimento, chegar ao sinal e aos dados que o
    // produziram, sem reexecutar nada.
    let dir = tempdir().unwrap();
    let mut rec = AuditRecorder::new("R1", ExecutionMode::Backtest, sink(dir.path()));
    gravar_cadeia(&mut rec);
    rec.flush().unwrap();
    let conn = rec.sink().connection();

    let fill: String = conn
        .query_row(
            "SELECT payload_json FROM audit_event WHERE run_id='R1' AND kind='fill'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let fill: serde_json::Value = serde_json::from_str(&fill).unwrap();
    let order_ref = fill["order_ref"].as_i64().unwrap();
    assert_eq!(fill["fee"], "0.97", "taxa discriminada, e como string");

    let ordem: String = conn
        .query_row(
            "SELECT payload_json FROM audit_event WHERE run_id='R1' AND kind='order' \
             AND json_extract(payload_json,'$.order_id') = ?1",
            [order_ref],
            |r| r.get(0),
        )
        .unwrap();
    let ordem: serde_json::Value = serde_json::from_str(&ordem).unwrap();
    let signal_ref = ordem["signal_ref"].as_i64().unwrap();

    let sinal: String = conn
        .query_row(
            "SELECT payload_json FROM audit_event WHERE run_id='R1' AND kind='signal' \
             AND json_extract(payload_json,'$.signal_id') = ?1",
            [signal_ref],
            |r| r.get(0),
        )
        .unwrap();
    let sinal: serde_json::Value = serde_json::from_str(&sinal).unwrap();

    assert_eq!(sinal["inputs"]["values"]["sma_fast"], "63412.10");
    assert_eq!(sinal["inputs"]["values"]["sma_slow"], "63180.55");
    assert_eq!(sinal["inputs"]["params"]["fast"], "9");

    let decisoes: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM audit_event WHERE kind='risk_decision' \
             AND json_extract(payload_json,'$.order_ref') = ?1",
            [order_ref],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        decisoes, 1,
        "a decisão que autorizou está ligada à mesma ordem"
    );
}

#[test]
fn toda_ordem_tem_exatamente_uma_decisao() {
    // SC-002 verificado por contagem, em SQL.
    let dir = tempdir().unwrap();
    let mut rec = AuditRecorder::new("R1", ExecutionMode::Backtest, sink(dir.path()));
    for _ in 0..25 {
        gravar_cadeia(&mut rec);
    }
    rec.flush().unwrap();

    let conn = rec.sink().connection();
    let ordens: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM audit_event WHERE kind='order'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let decisoes: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM audit_event WHERE kind='risk_decision'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(ordens, decisoes);
    assert_eq!(ordens, 25);
}

#[test]
fn nenhum_resume_automatico_sucede_uma_parada_que_exige_humano() {
    // SC-015, verificável por consulta.
    let dir = tempdir().unwrap();
    let mut rec = AuditRecorder::new("R1", ExecutionMode::Backtest, sink(dir.path()));
    rec.record(
        t(0),
        AuditKind::Halt {
            reason: "posição divergente".into(),
            anomaly: trade_domain::Anomaly::Integrity(
                trade_domain::IntegrityCause::PositionDivergence,
            ),
        },
    )
    .unwrap();
    rec.flush().unwrap();

    let n: i64 = rec
        .sink()
        .connection()
        .query_row(
            "SELECT COUNT(*) FROM audit_event r \
             WHERE r.kind='resume' AND json_extract(r.payload_json,'$.automatic') = 1 \
               AND EXISTS (SELECT 1 FROM audit_event h WHERE h.kind='halt' AND h.seq < r.seq \
                           AND json_extract(h.payload_json,'$.requires_human') = 1)",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 0);
}

#[test]
fn o_lote_e_descarregado_mesmo_sem_flush_explicito() {
    // Eventos presos em memória seriam auditoria perdida em silêncio.
    let dir = tempdir().unwrap();
    {
        let mut rec = AuditRecorder::new("R1", ExecutionMode::Backtest, sink(dir.path()));
        gravar_cadeia(&mut rec);
    }

    let conn = open_runs(dir.path().join("runs.db")).unwrap();
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM audit_event", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 4);
}

#[test]
fn a_recusa_registra_qual_limite_foi_violado() {
    let dir = tempdir().unwrap();
    let mut rec = AuditRecorder::new("R1", ExecutionMode::Backtest, sink(dir.path()));
    rec.record(
        t(0),
        AuditKind::RiskDecision(RiskDecision {
            order_ref: OrderId(1),
            verdict: Verdict::Rejected(LimitBreach::DailyLossReached),
            limits: RiskLimits::default(),
            state: RiskState::default(),
        }),
    )
    .unwrap();
    rec.flush().unwrap();

    let breach: String = rec
        .sink()
        .connection()
        .query_row(
            "SELECT json_extract(payload_json,'$.breach') FROM audit_event \
             WHERE kind='risk_decision'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(breach, "DailyLossReached");
}

#[test]
fn a_transicao_de_estado_grava_valores_como_texto() {
    let dir = tempdir().unwrap();
    let mut rec = AuditRecorder::new("R1", ExecutionMode::Backtest, sink(dir.path()));
    rec.record(
        t(0),
        AuditKind::StateTransition {
            from: "Flat".into(),
            to: "Long".into(),
            position: Position::default(),
            fechado_por: None,
        },
    )
    .unwrap();
    rec.flush().unwrap();

    let tipo: String = rec
        .sink()
        .connection()
        .query_row(
            "SELECT json_type(payload_json,'$.qty') FROM audit_event \
             WHERE kind='state_transition'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(tipo, "text", "quantidade como string, nunca número JSON");
}
