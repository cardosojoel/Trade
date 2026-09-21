//! Fatia 5 — o laço contínuo (T026 a T031).
//!
//! Tudo aqui roda com duplo: fonte de velas roteirizada, executor roteirizado
//! e relógio roteirizado. Nenhum teste espera, nenhum alcança a rede — é o que
//! o SC-008 exige e o que permite que a fatia 5 exista antes da credencial.

use chrono::{DateTime, Duration, TimeZone, Utc};
use rust_decimal::{Decimal, dec};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use trade_domain::{
    Candle, ExecutionMode, FeeModel, Instrumento, Intent, MarketContext, Money, Position,
    RiskLimits, Signal, SignalId, SignalInputs, Strategy, Symbol,
};
use trade_ports::testing::{
    InMemoryAuditSink, PrecoIgnorado, StubOrderExecutor, VecLiveCandleSource,
};
use trade_ports::{AuditRecorder, Clock};
use trade_risk::{KillSwitch, RiskGuard};
use trade_session::{Ambiente, FimDaSessao, Sessao, SessaoConfig, SessaoInicio};

fn t(h: i64) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap() + Duration::hours(h)
}

fn vela(at: DateTime<Utc>, preco: Money) -> Candle {
    Candle {
        open_time: at,
        open: preco,
        high: preco,
        low: preco,
        close: preco,
        volume: dec!(1),
        turnover: preco,
    }
}

/// Relógio que devolve instantes roteirizados, um por volta do laço.
///
/// O laço consulta o relógio **uma vez por vela** e reusa o instante em todos
/// os eventos daquela volta — é o que faz a cadeia sinal → ordem → decisão →
/// preenchimento compartilhar o mesmo instante, como no backtest. Se o laço
/// consultar duas vezes, este duplo desalinha e o teste acusa.
struct RelogioRoteirizado {
    instantes: RefCell<std::collections::VecDeque<DateTime<Utc>>>,
    ultimo: RefCell<DateTime<Utc>>,
}

impl RelogioRoteirizado {
    fn novo(v: Vec<DateTime<Utc>>) -> Self {
        RelogioRoteirizado {
            instantes: RefCell::new(v.into()),
            ultimo: RefCell::new(t(0)),
        }
    }
}

impl Clock for RelogioRoteirizado {
    fn now(&self) -> DateTime<Utc> {
        if let Some(x) = self.instantes.borrow_mut().pop_front() {
            *self.ultimo.borrow_mut() = x;
        }
        *self.ultimo.borrow()
    }
}

/// Estratégia que devolve intenções roteirizadas, uma por vela.
struct EstrategiaRoteirizada {
    intencoes: std::collections::VecDeque<Intent>,
    proximo: u64,
}

impl EstrategiaRoteirizada {
    fn nova(v: Vec<Intent>) -> Self {
        EstrategiaRoteirizada {
            intencoes: v.into(),
            proximo: 0,
        }
    }
    fn muda() -> Self {
        EstrategiaRoteirizada::nova(Vec::new())
    }
}

impl Strategy for EstrategiaRoteirizada {
    fn name(&self) -> &str {
        "roteirizada"
    }
    fn params(&self) -> BTreeMap<String, String> {
        BTreeMap::new()
    }
    fn on_candle(&mut self, _ctx: &MarketContext<'_>) -> Option<Signal> {
        let intent = self.intencoes.pop_front()?;
        if intent == Intent::Hold {
            return None;
        }
        self.proximo += 1;
        Some(Signal {
            id: SignalId(self.proximo),
            at: t(0),
            intent,
            qty: Some(dec!(1)),
            inputs: SignalInputs::default(),
        })
    }
}

fn limites() -> RiskLimits {
    RiskLimits {
        max_daily_loss: dec!(200),
        max_position_size: dec!(100000),
        max_total_exposure: dec!(200000),
        max_orders_per_window: 100,
        window_minutes: 60,
        max_transient_retries: 3,
        max_price_deviation_ratio: dec!(0.50),
        max_position_hours: 72,
    }
}

fn config() -> SessaoConfig {
    SessaoConfig {
        symbol: Symbol::new("BTCUSDT").unwrap(),
        fees: FeeModel {
            taker_fee_rate: Decimal::ZERO,
            slippage_rate: Decimal::ZERO,
        },
        instrumento: Instrumento::novo(dec!(0.000001), dec!(5)).unwrap(),
        // O prazo da emenda 2.0.0. Vem da configuração, nunca embutido.
        prazo_maximo: Duration::hours(72),
    }
}

fn inicio(position: Position, balance: Money) -> SessaoInicio {
    SessaoInicio {
        position,
        balance,
        proxima_ordem: 0,
    }
}

fn recorder() -> AuditRecorder<InMemoryAuditSink> {
    AuditRecorder::new(
        "sessao-de-teste",
        ExecutionMode::Paper,
        InMemoryAuditSink::new(),
    )
}

/// Posição comprada com instante de abertura conhecido.
fn comprado_em(qty: Money, preco: Money, quando: DateTime<Utc>) -> Position {
    let mut p = Position::default();
    p.apply_fill(
        trade_domain::Side::Buy,
        &trade_domain::Fill {
            order_ref: trade_domain::OrderId(0),
            price: preco,
            qty,
            fee: Decimal::ZERO,
            fee_base: Decimal::ZERO,
            slippage: Decimal::ZERO,
            at: quando,
        },
    )
    .unwrap();
    p
}

// ---------------------------------------------------------------- T029

#[test]
fn o_laco_encaminha_a_ordem_pelo_guard() {
    // O sinal da vela i é executado na vela i+1 — a mesma disciplina do
    // backtest, e pelo mesmo motivo: no instante do preenchimento, a vela que
    // originou o sinal já passou.
    let mut fonte = VecLiveCandleSource::new(vec![
        vela(t(0), dec!(100)),
        vela(t(1), dec!(100)),
        vela(t(2), dec!(100)),
    ]);
    let mut guard = RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        limites(),
        KillSwitch::disarmed(),
    );
    let relogio = RelogioRoteirizado::novo(vec![t(0), t(1), t(2)]);
    let mut est = EstrategiaRoteirizada::nova(vec![Intent::Buy, Intent::Hold, Intent::Hold]);
    let mut rec = recorder();
    let parar = Arc::new(AtomicBool::new(false));

    let r = Sessao::new(config())
        .executar(
            &mut est,
            &mut fonte,
            &mut guard,
            &Ambiente {
                clock: &relogio,
                parar: &parar,
                preco: &PrecoIgnorado,
            },
            inicio(Position::default(), dec!(10000)),
            &mut rec,
        )
        .unwrap();

    assert_eq!(r.velas, 3);
    assert_eq!(r.position.qty(), dec!(1), "a compra abriu posição");
    assert!(matches!(r.fim, FimDaSessao::FonteEncerrada));
    assert_eq!(rec.sink().count("order"), 1);
    assert_eq!(
        rec.sink().count("risk_decision"),
        1,
        "SC-002: uma decisão de risco para cada ordem"
    );
}

#[test]
fn nenhuma_ordem_alcanca_o_executor_sem_decisao_registrada() {
    let mut fonte = VecLiveCandleSource::new((0..6).map(|i| vela(t(i), dec!(100))).collect());
    let mut guard = RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        limites(),
        KillSwitch::disarmed(),
    );
    let relogio = RelogioRoteirizado::novo((0..6).map(t).collect());
    let mut est = EstrategiaRoteirizada::nova(vec![
        Intent::Buy,
        Intent::Sell,
        Intent::Buy,
        Intent::Sell,
        Intent::Hold,
        Intent::Hold,
    ]);
    let mut rec = recorder();
    let parar = Arc::new(AtomicBool::new(false));

    Sessao::new(config())
        .executar(
            &mut est,
            &mut fonte,
            &mut guard,
            &Ambiente {
                clock: &relogio,
                parar: &parar,
                preco: &PrecoIgnorado,
            },
            inicio(Position::default(), dec!(10000)),
            &mut rec,
        )
        .unwrap();

    assert_eq!(
        rec.sink().count("order"),
        rec.sink().count("risk_decision"),
        "toda ordem atravessou a camada de risco"
    );
}

// ---------------------------------------------------------------- T027

#[test]
fn posicao_que_atinge_o_prazo_e_encerrada() {
    // A posição abriu em t(0). O laço vê a vela de t(72): o prazo foi
    // atingido, e a posição é encerrada mesmo sem sinal da estratégia.
    let mut fonte = VecLiveCandleSource::new(vec![vela(t(72), dec!(100))]);
    let mut guard = RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        limites(),
        KillSwitch::disarmed(),
    );
    let relogio = RelogioRoteirizado::novo(vec![t(72)]);
    let mut est = EstrategiaRoteirizada::muda();
    let mut rec = recorder();
    let parar = Arc::new(AtomicBool::new(false));

    let r = Sessao::new(config())
        .executar(
            &mut est,
            &mut fonte,
            &mut guard,
            &Ambiente {
                clock: &relogio,
                parar: &parar,
                preco: &PrecoIgnorado,
            },
            inicio(comprado_em(dec!(1), dec!(100), t(0)), dec!(0)),
            &mut rec,
        )
        .unwrap();

    assert!(
        r.position.is_flat(),
        "72 horas atingidas: a posição foi encerrada (FR-114)"
    );
    assert_eq!(r.trades.len(), 1);
}

#[test]
fn abaixo_do_prazo_a_posicao_continua() {
    let mut fonte = VecLiveCandleSource::new(vec![vela(t(71), dec!(100))]);
    let mut guard = RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        limites(),
        KillSwitch::disarmed(),
    );
    let relogio = RelogioRoteirizado::novo(vec![t(71)]);
    let mut est = EstrategiaRoteirizada::muda();
    let mut rec = recorder();
    let parar = Arc::new(AtomicBool::new(false));

    let r = Sessao::new(config())
        .executar(
            &mut est,
            &mut fonte,
            &mut guard,
            &Ambiente {
                clock: &relogio,
                parar: &parar,
                preco: &PrecoIgnorado,
            },
            inicio(comprado_em(dec!(1), dec!(100), t(0)), dec!(0)),
            &mut rec,
        )
        .unwrap();

    assert_eq!(r.position.qty(), dec!(1), "71 horas não é 72");
    assert_eq!(rec.sink().count("order"), 0);
}

#[test]
fn o_registro_diz_que_foi_o_prazo_que_fechou() {
    // Decisão 033 do Jev e pendência P11: a partir do prazo a posição fecha
    // por dois motivos, e o registro tem de dizer qual foi.
    let mut fonte = VecLiveCandleSource::new(vec![vela(t(72), dec!(100))]);
    let mut guard = RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        limites(),
        KillSwitch::disarmed(),
    );
    let relogio = RelogioRoteirizado::novo(vec![t(72)]);
    let mut est = EstrategiaRoteirizada::muda();
    let mut rec = recorder();
    let parar = Arc::new(AtomicBool::new(false));

    Sessao::new(config())
        .executar(
            &mut est,
            &mut fonte,
            &mut guard,
            &Ambiente {
                clock: &relogio,
                parar: &parar,
                preco: &PrecoIgnorado,
            },
            inicio(comprado_em(dec!(1), dec!(100), t(0)), dec!(0)),
            &mut rec,
        )
        .unwrap();

    let causas = rec.sink().causas_de_fechamento();
    assert_eq!(causas, vec!["prazo".to_string()]);
}

#[test]
fn o_registro_diz_que_foi_o_sinal_que_fechou() {
    let mut fonte = VecLiveCandleSource::new(vec![
        vela(t(0), dec!(100)),
        vela(t(1), dec!(100)),
        vela(t(2), dec!(100)),
    ]);
    let mut guard = RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        limites(),
        KillSwitch::disarmed(),
    );
    let relogio = RelogioRoteirizado::novo(vec![t(0), t(1), t(2)]);
    let mut est = EstrategiaRoteirizada::nova(vec![Intent::Sell, Intent::Hold, Intent::Hold]);
    let mut rec = recorder();
    let parar = Arc::new(AtomicBool::new(false));

    Sessao::new(config())
        .executar(
            &mut est,
            &mut fonte,
            &mut guard,
            &Ambiente {
                clock: &relogio,
                parar: &parar,
                preco: &PrecoIgnorado,
            },
            inicio(comprado_em(dec!(1), dec!(100), t(0)), dec!(0)),
            &mut rec,
        )
        .unwrap();

    assert_eq!(rec.sink().causas_de_fechamento(), vec!["sinal".to_string()]);
}

// ---------------------------------------------------------------- T030

#[test]
fn a_virada_de_dia_solta_o_freio_com_a_posicao_atravessando() {
    // FR-019b: o freio diário zera na virada, e a posição de 72 horas
    // atravessa — não é encerrada por ter mudado o dia.
    let mut guard = RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        limites(),
        KillSwitch::disarmed(),
    );
    // Prejuízo realizado que atinge o limite de 200 e trava o freio.
    guard.record_realized(dec!(-200), dec!(0));
    assert!(guard.daily_loss_blocked());

    let mut fonte = VecLiveCandleSource::new(vec![vela(t(23), dec!(100)), vela(t(25), dec!(100))]);
    let relogio = RelogioRoteirizado::novo(vec![t(23), t(25)]);
    let mut est = EstrategiaRoteirizada::muda();
    let mut rec = recorder();
    let parar = Arc::new(AtomicBool::new(false));

    let r = Sessao::new(config())
        .executar(
            &mut est,
            &mut fonte,
            &mut guard,
            &Ambiente {
                clock: &relogio,
                parar: &parar,
                preco: &PrecoIgnorado,
            },
            inicio(comprado_em(dec!(1), dec!(100), t(20)), dec!(0)),
            &mut rec,
        )
        .unwrap();

    assert!(
        !guard.daily_loss_blocked(),
        "a virada soltou o freio sozinha"
    );
    assert_eq!(
        rec.sink().count("resume"),
        1,
        "e a retomada ficou registrada"
    );
    assert_eq!(
        r.position.qty(),
        dec!(1),
        "a posição atravessou a virada: o dia mudou, o prazo dela não"
    );
}

// ---------------------------------------------------------------- T028

#[test]
fn falha_transitoria_retenta_e_a_sessao_continua() {
    // Duas falhas transitórias contra um limite de três: retenta sozinha,
    // sem parar e sem pedir permissão a ninguém (FR-111, Princípio II).
    let mut fonte = VecLiveCandleSource::new(vec![
        vela(t(0), dec!(100)),
        vela(t(1), dec!(100)),
        vela(t(2), dec!(100)),
    ]);
    let mut guard = RiskGuard::new(
        StubOrderExecutor::fails_then_fills(2, dec!(100)),
        limites(),
        KillSwitch::disarmed(),
    );
    let relogio = RelogioRoteirizado::novo(vec![t(0), t(1), t(2)]);
    let mut est = EstrategiaRoteirizada::nova(vec![Intent::Buy, Intent::Hold, Intent::Hold]);
    let mut rec = recorder();
    let parar = Arc::new(AtomicBool::new(false));

    let r = Sessao::new(config())
        .executar(
            &mut est,
            &mut fonte,
            &mut guard,
            &Ambiente {
                clock: &relogio,
                parar: &parar,
                preco: &PrecoIgnorado,
            },
            inicio(Position::default(), dec!(10000)),
            &mut rec,
        )
        .unwrap();

    assert_eq!(r.position.qty(), dec!(1), "a terceira tentativa preencheu");
    assert!(matches!(r.fim, FimDaSessao::FonteEncerrada));
    assert_eq!(
        rec.sink().count("anomaly"),
        1,
        "a retentativa ficou registrada"
    );
}

#[test]
fn retentativas_esgotadas_param_a_sessao() {
    // Acima do limite, a falha transitória vira falha de integridade — e
    // falha de integridade não se retenta, para.
    let mut fonte = VecLiveCandleSource::new(vec![
        vela(t(0), dec!(100)),
        vela(t(1), dec!(100)),
        vela(t(2), dec!(100)),
    ]);
    let mut guard = RiskGuard::new(
        StubOrderExecutor::fails_then_fills(9, dec!(100)),
        limites(),
        KillSwitch::disarmed(),
    );
    let relogio = RelogioRoteirizado::novo(vec![t(0), t(1), t(2)]);
    let mut est = EstrategiaRoteirizada::nova(vec![Intent::Buy, Intent::Hold, Intent::Hold]);
    let mut rec = recorder();
    let parar = Arc::new(AtomicBool::new(false));

    let r = Sessao::new(config())
        .executar(
            &mut est,
            &mut fonte,
            &mut guard,
            &Ambiente {
                clock: &relogio,
                parar: &parar,
                preco: &PrecoIgnorado,
            },
            inicio(Position::default(), dec!(10000)),
            &mut rec,
        )
        .unwrap();

    assert!(
        matches!(r.fim, FimDaSessao::Parada { .. }),
        "esgotadas as tentativas, a sessão para: veio {:?}",
        r.fim
    );
    assert_eq!(rec.sink().count("halt"), 1, "e a parada ficou registrada");
}

// ---------------------------------------------------------------- T031

#[test]
fn a_bandeira_de_parada_encerra_limpo_sem_perder_evento() {
    // A bandeira sobe antes da segunda vela. O laço termina a volta que
    // começou, registra o que aconteceu nela e sai — nada fica no ar.
    let mut fonte = VecLiveCandleSource::new(vec![
        vela(t(0), dec!(100)),
        vela(t(1), dec!(100)),
        vela(t(2), dec!(100)),
    ]);
    let mut guard = RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        limites(),
        KillSwitch::disarmed(),
    );
    let relogio = RelogioRoteirizado::novo(vec![t(0), t(1), t(2)]);
    let mut est = EstrategiaRoteirizada::nova(vec![Intent::Buy, Intent::Hold, Intent::Hold]);
    let mut rec = recorder();
    let parar = Arc::new(AtomicBool::new(true));

    let r = Sessao::new(config())
        .executar(
            &mut est,
            &mut fonte,
            &mut guard,
            &Ambiente {
                clock: &relogio,
                parar: &parar,
                preco: &PrecoIgnorado,
            },
            inicio(Position::default(), dec!(10000)),
            &mut rec,
        )
        .unwrap();

    assert!(matches!(r.fim, FimDaSessao::Interrompida));
    assert_eq!(
        r.velas, 0,
        "a bandeira já estava de pé: nenhuma volta correu"
    );
    assert!(parar.load(Ordering::SeqCst), "a bandeira continua de pé");
}

#[test]
fn a_bandeira_no_meio_encerra_depois_da_volta_corrente() {
    let mut fonte = VecLiveCandleSource::new(vec![
        vela(t(0), dec!(100)),
        vela(t(1), dec!(100)),
        vela(t(2), dec!(100)),
    ]);
    let mut guard = RiskGuard::new(
        StubOrderExecutor::always_fills_at(dec!(100)),
        limites(),
        KillSwitch::disarmed(),
    );
    let relogio = RelogioRoteirizado::novo(vec![t(0), t(1), t(2)]);
    let mut est = EstrategiaRoteirizada::nova(vec![Intent::Buy, Intent::Hold, Intent::Hold]);
    let mut rec = recorder();
    let parar = Arc::new(AtomicBool::new(false));

    // Sobe a bandeira depois da primeira vela, de dentro da estratégia.
    struct Sobe<'a>(&'a AtomicBool, EstrategiaRoteirizada);
    impl Strategy for Sobe<'_> {
        fn name(&self) -> &str {
            "sobe"
        }
        fn params(&self) -> BTreeMap<String, String> {
            BTreeMap::new()
        }
        fn on_candle(&mut self, ctx: &MarketContext<'_>) -> Option<Signal> {
            self.0.store(true, Ordering::SeqCst);
            self.1.on_candle(ctx)
        }
    }
    let mut est = Sobe(
        &parar,
        std::mem::replace(&mut est, EstrategiaRoteirizada::muda()),
    );

    let r = Sessao::new(config())
        .executar(
            &mut est,
            &mut fonte,
            &mut guard,
            &Ambiente {
                clock: &relogio,
                parar: &parar,
                preco: &PrecoIgnorado,
            },
            inicio(Position::default(), dec!(10000)),
            &mut rec,
        )
        .unwrap();

    assert!(matches!(r.fim, FimDaSessao::Interrompida));
    assert_eq!(r.velas, 1, "a volta que começou terminou antes de sair");
    assert_eq!(
        rec.sink().count("signal"),
        1,
        "e o evento dela não se perdeu"
    );
}
