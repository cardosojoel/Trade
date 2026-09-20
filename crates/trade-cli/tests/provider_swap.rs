//! US5 e SC-006 — a mesma estratégia contra provedores diferentes.
//!
//! Este teste mora em `trade-cli` por necessidade estrutural: é a única crate
//! autorizada a conhecer as duas pontas. `trade-backtest` não pode depender de
//! `trade-storage`, e a trava em `tests/architecture.rs` falha o build se
//! alguém tentar — de modo que nem seria possível escrevê-lo lá.

use chrono::{Duration, TimeZone, Utc};
use rust_decimal::{Decimal, dec};
use tempfile::tempdir;
use trade_backtest::{BacktestConfig, BacktestEngine};
use trade_domain::{Candle, ExecutionMode, FeeModel, Interval, Money, RiskLimits, Symbol};
use trade_ports::CandleRepository;
use trade_ports::testing::{FailingMarketDataSource, InMemoryAuditSink, VecMarketDataSource};
use trade_ports::{AuditRecorder, MarketDataSource, MarketError};
use trade_risk::KillSwitch;
use trade_storage::{SqliteCandleRepository, SqliteMarketDataSource, open_market};
use trade_strategy::{SmaCross, SmaCrossParams};

fn t(m: i64) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap() + Duration::minutes(m)
}

/// Série com oscilação suficiente para a estratégia cruzar as médias.
fn serie() -> Vec<Candle> {
    (0..200i64)
        .map(|i| {
            let onda = Decimal::from((i % 40) * 30);
            let p = dec!(60000) + onda;
            Candle {
                open_time: t(i),
                open: p,
                high: p,
                low: p,
                close: p,
                volume: dec!(100),
                turnover: p * dec!(100),
            }
        })
        .collect()
}

fn config(capital: Money) -> BacktestConfig {
    BacktestConfig {
        symbol: Symbol::new("BTCUSDT").unwrap(),
        interval: Interval::M1,
        from: t(0),
        to: t(300),
        initial_capital: capital,
        fees: FeeModel {
            taker_fee_rate: dec!(0.001),
            slippage_rate: dec!(0.0005),
        },
        limits: RiskLimits {
            max_daily_loss: dec!(1_000_000),
            max_position_size: dec!(1_000_000),
            max_total_exposure: dec!(1_000_000),
            max_orders_per_window: 100_000,
            window_minutes: 60,
            max_transient_retries: 3,
            max_price_deviation_ratio: dec!(0.50),
        },
        min_equity: capital / Decimal::from(100),
    }
}

fn executar(fonte: &dyn MarketDataSource) -> trade_backtest::BacktestOutcome {
    let mut estrategia = SmaCross::new(SmaCrossParams {
        fast: 5,
        slow: 13,
        position_fraction: dec!(0.1),
    });
    let mut rec = AuditRecorder::new("R", ExecutionMode::Backtest, InMemoryAuditSink::new());
    BacktestEngine::new(config(dec!(10_000)))
        .run(&mut estrategia, fonte, KillSwitch::disarmed(), &mut rec)
        .unwrap()
}

#[test]
fn a_mesma_estrategia_roda_contra_sqlite_e_contra_memoria() {
    let velas = serie();

    // Provedor A: SQLite, com o histórico gravado.
    let dir = tempdir().unwrap();
    let conn = open_market(dir.path().join("m.db")).unwrap();
    let mut repo = SqliteCandleRepository::new(conn);
    repo.upsert_page(&Symbol::new("BTCUSDT").unwrap(), Interval::M1, &velas)
        .unwrap();
    let sqlite = SqliteMarketDataSource::new(open_market(dir.path().join("m.db")).unwrap());

    // Provedor B: vetor em memória.
    let memoria = VecMarketDataSource::new(velas);

    let a = executar(&sqlite);
    let b = executar(&memoria);

    // Nenhuma linha da estratégia mudou entre as duas execuções.
    assert_eq!(a.candles_seen, b.candles_seen);
    assert_eq!(
        a.metrics, b.metrics,
        "provedor diferente, resultado idêntico"
    );
    assert_eq!(a.trades, b.trades);
    assert!(
        a.metrics.trade_count > 0,
        "o cenário precisa produzir operações"
    );
}

#[test]
fn a_falha_do_provedor_chega_pela_mesma_forma() {
    // FR-010: quem está acima vê `MarketError`, não o erro de quem falhou.
    let falho = FailingMarketDataSource("fonte inventada fora do ar".into());
    let mut estrategia = SmaCross::new(SmaCrossParams::default());
    let mut rec = AuditRecorder::new("R", ExecutionMode::Backtest, InMemoryAuditSink::new());

    let erro = BacktestEngine::new(config(dec!(10_000)))
        .run(&mut estrategia, &falho, KillSwitch::disarmed(), &mut rec)
        .unwrap_err();

    assert!(matches!(erro, MarketError::Unavailable(_)));
}

#[test]
fn um_provedor_vazio_nao_e_confundido_com_erro() {
    let vazio = VecMarketDataSource::new(Vec::new());
    let r = executar(&vazio);
    assert_eq!(r.candles_seen, 0);
    assert_eq!(r.metrics.trade_count, 0);
}
