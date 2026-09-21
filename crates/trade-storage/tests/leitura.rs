//! T008 e T009 — o lado de **leitura** do registro.
//!
//! Até 2026-09-21 esta crate só escrevia. Quem quis ler, leu por SQL à mão,
//! que é o que o `docs/auditoria.md` documenta para o operador. O servidor da
//! feature 003 precisa ler de dentro do processo, e a consulta fica aqui — não
//! na crate do servidor —, para que o conhecimento do esquema não se divida
//! entre quem escreve e quem lê.

use chrono::{Duration, TimeZone, Utc};
use rust_decimal::dec;
use std::collections::BTreeMap;
use tempfile::tempdir;
use trade_domain::{ExecutionMode, FeeModel, Interval, RiskLimits, RunMetrics, Symbol, Trade};
use trade_storage::open_runs;
use trade_storage::runs_repo::{RunHeader, RunsRepository};

fn t(h: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 1, h, 0, 0).unwrap()
}

fn repo() -> (tempfile::TempDir, RunsRepository) {
    let dir = tempdir().unwrap();
    let conn = open_runs(dir.path().join("runs.db")).unwrap();
    (dir, RunsRepository::new(conn))
}

fn gravar(r: &mut RunsRepository, run_id: &str, hora: u32, capital: rust_decimal::Decimal) {
    let sym = Symbol::new("BTCUSDT").unwrap();
    let lim = RiskLimits {
        max_daily_loss: dec!(200.00),
        ..RiskLimits::default()
    };
    let fees = FeeModel::default();
    let params = BTreeMap::from([("fast".to_string(), "9".to_string())]);
    r.start_run(&RunHeader {
        run_id,
        mode: ExecutionMode::Backtest,
        symbol: &sym,
        interval: Interval::M1,
        from: t(hora),
        to: t(hora) + Duration::days(1),
        initial_capital: capital,
        limits: &lim,
        fees: &fees,
        strategy: "sma-cross",
        strategy_params: &params,
        started_at: t(hora),
    })
    .unwrap();
}

// ---------------------------------------------------------------- T008

#[test]
fn le_a_execucao_com_a_cerca_sob_a_qual_ela_correu() {
    // FR-025 da feature 001: comparar duas execuções exige saber sob que
    // condições cada uma foi feita. De nada serve ler o resultado sem a cerca.
    let (_d, mut r) = repo();
    gravar(&mut r, "R1", 1, dec!(10000));

    let e = r.execucao("R1").unwrap().expect("R1 existe");
    assert_eq!(e.run_id, "R1");
    assert_eq!(e.mode, "backtest");
    assert_eq!(e.symbol, "BTCUSDT");
    assert_eq!(e.interval, "1m");
    assert_eq!(e.initial_capital, dec!(10000));
    assert_eq!(e.strategy, "sma-cross");
    assert!(e.limits_json.contains("\"max_daily_loss\":\"200.00\""));
    assert!(e.fees_json.contains("taker_fee_rate"));
    assert_eq!(e.ended_at, None, "não encerrada ainda");
    assert_eq!(e.outcome, None);
}

#[test]
fn execucao_que_nao_existe_e_none_e_nao_erro() {
    // "Não existe" e "não deu para ler" decidem coisas diferentes: a primeira
    // é 404, a segunda é 503 (FR-023).
    let (_d, r) = repo();
    assert!(r.execucao("nao-existe").unwrap().is_none());
}

#[test]
fn o_desfecho_aparece_depois_de_encerrada() {
    let (_d, mut r) = repo();
    gravar(&mut r, "R1", 1, dec!(10000));
    r.finish_run("R1", t(5), "halted", Some("divergência"))
        .unwrap();

    let e = r.execucao("R1").unwrap().unwrap();
    assert_eq!(e.outcome.as_deref(), Some("halted"));
    assert_eq!(e.halt_reason.as_deref(), Some("divergência"));
    assert_eq!(e.ended_at, Some(t(5)));
}

#[test]
fn lista_as_execucoes_da_mais_recente_para_a_mais_antiga() {
    // O identificador é ULID, ordenável por tempo. Ordenar por ele não
    // depende de o relógio da máquina ter andado para a frente.
    let (_d, mut r) = repo();
    for (id, h) in [("01A", 1), ("01B", 2), ("01C", 3)] {
        gravar(&mut r, id, h, dec!(10000));
    }
    let ids: Vec<String> = r
        .listar_execucoes()
        .unwrap()
        .into_iter()
        .map(|e| e.run_id)
        .collect();
    assert_eq!(ids, vec!["01C", "01B", "01A"]);
}

#[test]
fn o_capital_volta_com_todos_os_digitos() {
    // Gravado como TEXT justamente para isto. Um float no caminho perderia
    // dígito entre a escrita e a leitura, e o extrato deixaria de fechar.
    let (_d, mut r) = repo();
    gravar(&mut r, "R1", 1, dec!(10000.12345678));
    let e = r.execucao("R1").unwrap().unwrap();
    assert_eq!(e.initial_capital.to_string(), "10000.12345678");
}

// ---------------------------------------------------------------- T009

#[test]
fn le_as_metricas_de_uma_execucao() {
    let (_d, mut r) = repo();
    gravar(&mut r, "R1", 1, dec!(10000));
    let trades = vec![operacao(1, dec!(10)), operacao(2, dec!(-4))];
    r.save_trades("R1", &trades).unwrap();
    r.save_metrics("R1", &RunMetrics::from_trades(&trades, dec!(0.5)))
        .unwrap();

    let m = r.metricas("R1").unwrap().expect("gravadas");
    assert_eq!(m.trade_count, 2);
    assert_eq!(m.net_result, dec!(6));
    assert_eq!(m.total_slippage, dec!(0.5));
}

#[test]
fn profit_factor_indefinido_volta_como_none() {
    // Sem operação perdedora ele é indefinido. Gravado NULL, lido `None` — e
    // nunca convertido em zero, infinito ou texto (FR-021).
    let (_d, mut r) = repo();
    gravar(&mut r, "R1", 1, dec!(10000));
    let trades = vec![operacao(1, dec!(10))];
    r.save_metrics("R1", &RunMetrics::from_trades(&trades, dec!(0)))
        .unwrap();
    assert_eq!(r.metricas("R1").unwrap().unwrap().profit_factor, None);
}

#[test]
fn execucao_sem_metricas_gravadas_e_none() {
    let (_d, mut r) = repo();
    gravar(&mut r, "R1", 1, dec!(10000));
    assert!(r.metricas("R1").unwrap().is_none());
}

#[test]
fn le_o_extrato_em_ordem() {
    let (_d, mut r) = repo();
    gravar(&mut r, "R1", 1, dec!(10000));
    let trades: Vec<Trade> = (1..=5).map(|i| operacao(i, dec!(1))).collect();
    r.save_trades("R1", &trades).unwrap();

    let lidos = r.extrato("R1").unwrap();
    assert_eq!(lidos.len(), 5);
    let seqs: Vec<u64> = lidos.iter().map(|t| t.seq).collect();
    assert_eq!(seqs, vec![1, 2, 3, 4, 5]);
}

#[test]
fn o_extrato_volta_com_os_digitos_que_foi_gravado() {
    let (_d, mut r) = repo();
    gravar(&mut r, "R1", 1, dec!(10000));
    let mut t1 = operacao(1, dec!(0.0000000008));
    t1.entry_price = dec!(63420.12345678);
    t1.qty = dec!(0.00000001);
    r.save_trades("R1", &[t1]).unwrap();

    let lido = &r.extrato("R1").unwrap()[0];
    assert_eq!(lido.entry_price.to_string(), "63420.12345678");
    assert_eq!(lido.qty.to_string(), "0.00000001");
    assert_eq!(lido.pnl.to_string(), "0.0000000008");
}

#[test]
fn o_extrato_de_uma_execucao_nao_traz_o_de_outra() {
    let (_d, mut r) = repo();
    gravar(&mut r, "R1", 1, dec!(10000));
    gravar(&mut r, "R2", 2, dec!(10000));
    r.save_trades("R1", &[operacao(1, dec!(1))]).unwrap();
    r.save_trades("R2", &[operacao(1, dec!(2)), operacao(2, dec!(3))])
        .unwrap();

    assert_eq!(r.extrato("R1").unwrap().len(), 1);
    assert_eq!(r.extrato("R2").unwrap().len(), 2);
}

fn operacao(seq: u64, pnl: rust_decimal::Decimal) -> Trade {
    Trade {
        seq,
        entry_at: t(1),
        entry_price: dec!(100),
        exit_at: t(2),
        exit_price: dec!(110),
        qty: dec!(1),
        fees: dec!(0.1),
        pnl,
    }
}

// ---------------------------------------------------------------- T018

/// Grava eventos de auditoria fora de ordem de instante, mas em ordem de seq.
fn gravar_eventos(dir: &std::path::Path) {
    use trade_domain::{AuditKind, CausaDoFechamento, OrderId, Position, Side, SignalId};
    use trade_ports::{AuditRecorder, Recorder};
    use trade_storage::SqliteAuditSink;

    let sink = SqliteAuditSink::new(open_runs(dir.join("runs.db")).unwrap());
    let mut rec = AuditRecorder::new("R1", ExecutionMode::Backtest, sink);

    // Dois eventos no **mesmo instante simulado**. É exatamente o caso que o
    // `seq` existe para desempatar: ordenar por `at_ms` os devolveria em
    // ordem indefinida, e a cadeia sinal → ordem teria um elo ambíguo.
    rec.record(
        t(5),
        AuditKind::Order(trade_domain::Order {
            id: OrderId(1),
            signal_ref: SignalId(1),
            side: Side::Buy,
            qty: dec!(1),
            at: t(5),
        }),
    )
    .unwrap();
    rec.record(
        t(5),
        AuditKind::StateTransition {
            from: "Flat".into(),
            to: "Long".into(),
            position: Position::default(),
            fechado_por: None,
        },
    )
    .unwrap();
    rec.record(
        t(9),
        AuditKind::StateTransition {
            from: "Long".into(),
            to: "Flat".into(),
            position: Position::default(),
            fechado_por: Some(CausaDoFechamento::Prazo),
        },
    )
    .unwrap();
    rec.flush().unwrap();
}

#[test]
fn a_linha_do_tempo_vem_ordenada_por_seq_e_nunca_por_instante() {
    // FR-012. Dois eventos do mesmo instante têm ordem definida só pelo
    // `seq`; ordenar por `at_ms` deixaria a cadeia ambígua dentro da vela.
    let dir = tempdir().unwrap();
    gravar_eventos(dir.path());
    let r = RunsRepository::new(open_runs(dir.path().join("runs.db")).unwrap());

    let eventos = r.linha_do_tempo("R1").unwrap();
    let seqs: Vec<u64> = eventos.iter().map(|e| e.seq).collect();
    assert_eq!(seqs, vec![1, 2, 3]);
    assert_eq!(eventos[0].kind, "order");
    assert_eq!(eventos[1].kind, "state_transition");
}

#[test]
fn a_linha_do_tempo_traz_a_carga_de_cada_evento() {
    let dir = tempdir().unwrap();
    gravar_eventos(dir.path());
    let r = RunsRepository::new(open_runs(dir.path().join("runs.db")).unwrap());

    let eventos = r.linha_do_tempo("R1").unwrap();
    assert_eq!(eventos[2].payload["fechado_por"], "prazo");
    assert_eq!(eventos[0].payload["side"], "Buy");
}

#[test]
fn a_linha_do_tempo_de_uma_execucao_nao_traz_a_de_outra() {
    let dir = tempdir().unwrap();
    gravar_eventos(dir.path());
    let r = RunsRepository::new(open_runs(dir.path().join("runs.db")).unwrap());
    assert!(r.linha_do_tempo("R2").unwrap().is_empty());
}
