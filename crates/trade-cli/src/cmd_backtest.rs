//! Comando `trade backtest`.
//!
//! Este é o **ponto de composição**: a única parte do sistema que conhece as
//! duas pontas. Aqui o SQLite encontra o motor, a cerca sai do arquivo e vira
//! `RiskLimits`, e a estratégia concreta é escolhida. Nenhuma outra crate faz
//! essa ligação, e é por isso que estratégia e risco continuam sem saber que a
//! Bybit existe.

use crate::cli::BacktestArgs;
use crate::config::{load_fees, load_instrumento, load_limits, validar_perfil};
use crate::report;
use chrono::Utc;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::time::SystemTime;
use trade_backtest::{BacktestConfig, BacktestEngine, RunOutcome};
use trade_domain::{ExecutionMode, FeeModel, Strategy, Symbol};
use trade_ports::AuditRecorder;
use trade_risk::KillSwitch;
use trade_storage::runs_repo::{RunHeader, RunsRepository};
use trade_storage::{SqliteAuditSink, SqliteMarketDataSource, open_market, open_runs};
use trade_strategy::{SmaCross, SmaCrossParams};
use ulid::Ulid;

/// Código de saída do processo.
///
/// As três terminações anômalas têm códigos próprios porque duas delas
/// encerram **sem** completar o período. Um script que só perguntasse
/// "terminou?" trataria as três como sucesso.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Ok = 0,
    Uso = 2,
    HistoricoAusente = 3,
    Interrompido = 4,
    CapitalEsgotado = 5,
}

pub fn run(args: &BacktestArgs) -> (ExitCode, String) {
    match executar(args) {
        Ok((code, saida)) => (code, saida),
        Err((code, msg)) => (code, msg),
    }
}

fn executar(args: &BacktestArgs) -> Result<(ExitCode, String), (ExitCode, String)> {
    // Redundante com o parser, e deliberadamente: se um dia alguém tornar o
    // argumento opcional, esta verificação continua barrando.
    if args.mode != ExecutionMode::Backtest {
        return Err((
            ExitCode::Uso,
            "apenas o modo 'backtest' está disponível nesta versão".into(),
        ));
    }

    let symbol = Symbol::new(&args.symbol).map_err(|e| (ExitCode::Uso, e.to_string()))?;
    let limits = load_limits(&args.limits).map_err(|e| (ExitCode::Uso, e.to_string()))?;
    let fees_cfg = load_fees(&args.fees).map_err(|e| (ExitCode::Uso, e.to_string()))?;
    let instrumento =
        load_instrumento(&args.instrumento).map_err(|e| (ExitCode::Uso, e.to_string()))?;

    // REQ-BYBIT-007 — a releitura pode invalidar o perfil, e aí a sessão não
    // começa. É deliberado que isto venha antes de qualquer vela ser lida:
    // descobrir no meio da execução que nenhuma ordem era negociável custa o
    // tempo inteiro da execução para produzir um resultado vazio.
    validar_perfil(&instrumento, limits.max_position_size, args.capital)
        .map_err(|e| (ExitCode::Uso, e))?;

    if args.to <= args.from {
        return Err((
            ExitCode::Uso,
            format!(
                "período vazio: --to ({}) não é posterior a --from ({})",
                args.to, args.from
            ),
        ));
    }

    let market =
        open_market(&args.market_db).map_err(|e| (ExitCode::HistoricoAusente, e.to_string()))?;
    let source = SqliteMarketDataSource::new(market);

    let mut strategy =
        montar_estrategia(&args.strategy, &args.strategy_params).map_err(|m| (ExitCode::Uso, m))?;

    let config = BacktestConfig {
        symbol: symbol.clone(),
        interval: args.interval,
        from: args.from,
        to: args.to,
        initial_capital: args.capital,
        fees: FeeModel {
            taker_fee_rate: fees_cfg.taker_fee_rate,
            slippage_rate: fees_cfg.slippage_rate,
        },
        limits: limits.clone(),
        instrumento,
        // Piso de 1% do capital. Em spot comprado o patrimônio nunca chega a
        // zero, então "capital esgotado" precisa de um piso para significar
        // alguma coisa.
        min_equity: args.capital / Decimal::from(100),
    };

    // ULID e não UUID v4: ordenável por tempo, de modo que listar
    // execuções em ordem cronológica seja ORDER BY run_id.
    let run_id = Ulid::from_datetime(SystemTime::now()).to_string();
    let iniciado_em = Utc::now();

    let runs = open_runs(&args.runs_db).map_err(|e| (ExitCode::Uso, e.to_string()))?;
    let mut repo = RunsRepository::new(runs);

    let params = strategy.params();
    repo.start_run(&RunHeader {
        run_id: &run_id,
        mode: ExecutionMode::Backtest,
        symbol: &symbol,
        interval: args.interval,
        from: args.from,
        to: args.to,
        initial_capital: args.capital,
        limits: &limits,
        fees: &config.fees,
        strategy: strategy.name(),
        strategy_params: &params,
        started_at: iniciado_em,
    })
    .map_err(|e| (ExitCode::Uso, e.to_string()))?;

    // A auditoria vai para o mesmo banco da execução. Não existe configuração
    // que a desligue: o motor não é construível sem um destino de registro, e
    // o Princípio IV não admite execução sem ele.
    let audit_conn = open_runs(&args.runs_db).map_err(|e| (ExitCode::Uso, e.to_string()))?;
    let mut audit = AuditRecorder::new(
        run_id.clone(),
        ExecutionMode::Backtest,
        SqliteAuditSink::new(audit_conn),
    );

    let resultado = BacktestEngine::new(config.clone())
        .run(
            strategy.as_mut(),
            &source,
            KillSwitch::sentinel(&args.kill_file),
            &mut audit,
        )
        .map_err(|e| (ExitCode::HistoricoAusente, e.to_string()))?;

    audit
        .flush()
        .map_err(|e| (ExitCode::Uso, format!("falha ao gravar a auditoria: {e}")))?;
    let eventos = audit.sink().gravados();

    if resultado.candles_seen == 0 {
        return Err((
            ExitCode::HistoricoAusente,
            format!(
                "nenhuma vela de {} {} entre {} e {} em {}.\n\
                 Colete o histórico antes de executar o backtest.",
                symbol,
                args.interval,
                args.from.date_naive(),
                args.to.date_naive(),
                args.market_db.display()
            ),
        ));
    }

    repo.save_trades(&run_id, &resultado.trades)
        .map_err(|e| (ExitCode::Uso, e.to_string()))?;
    repo.save_metrics(&run_id, &resultado.metrics)
        .map_err(|e| (ExitCode::Uso, e.to_string()))?;

    let motivo = match &resultado.outcome {
        RunOutcome::Halted { reason, .. } => Some(reason.clone()),
        _ => None,
    };
    repo.finish_run(
        &run_id,
        Utc::now(),
        resultado.outcome.as_str(),
        motivo.as_deref(),
    )
    .map_err(|e| (ExitCode::Uso, e.to_string()))?;

    let code = match resultado.outcome {
        RunOutcome::Completed => ExitCode::Ok,
        RunOutcome::Halted { .. } => ExitCode::Interrompido,
        RunOutcome::CapitalExhausted { .. } => ExitCode::CapitalEsgotado,
    };

    let mut saida = report::render(&run_id, args.capital, &resultado);
    saida.push_str(&format!(
        "\nRegistro: {} · run_id {} · {} eventos de auditoria\n",
        args.runs_db.display(),
        run_id,
        eventos
    ));

    Ok((code, saida))
}

fn montar_estrategia(nome: &str, params: &str) -> Result<Box<dyn Strategy>, String> {
    let mapa = parse_params(params)?;
    match nome {
        "sma-cross" => {
            let mut p = SmaCrossParams::default();
            if let Some(v) = mapa.get("fast") {
                p.fast = v.parse().map_err(|_| format!("fast '{v}' inválido"))?;
            }
            if let Some(v) = mapa.get("slow") {
                p.slow = v.parse().map_err(|_| format!("slow '{v}' inválido"))?;
            }
            if let Some(v) = mapa.get("position_fraction") {
                p.position_fraction = v
                    .parse()
                    .map_err(|_| format!("position_fraction '{v}' inválido"))?;
            }
            if p.fast >= p.slow {
                return Err(format!(
                    "fast ({}) precisa ser menor que slow ({}) — senão não há cruzamento a observar",
                    p.fast, p.slow
                ));
            }
            Ok(Box::new(SmaCross::new(p)))
        }
        outro => Err(format!(
            "estratégia '{outro}' desconhecida — disponível: sma-cross"
        )),
    }
}

fn parse_params(s: &str) -> Result<BTreeMap<String, String>, String> {
    let mut m = BTreeMap::new();
    for par in s.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let (k, v) = par
            .split_once('=')
            .ok_or_else(|| format!("parâmetro '{par}' malformado — use chave=valor"))?;
        m.insert(k.trim().to_string(), v.trim().to_string());
    }
    Ok(m)
}
