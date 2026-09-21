//! `trade paper rodar` — a sessão contínua na testnet (T041, T044, T045).
//!
//! O segundo ponto de composição do sistema, e o mais carregado: aqui a
//! credencial vira cliente autenticado, o cliente vira `OrderExecutor`, o
//! executor é **movido** para dentro do `RiskGuard` e some de vista, o estado
//! sai do `runs.db` e vira contador de risco, e o laço recebe tudo já montado
//! sem saber que a Bybit existe.
//!
//! A ordem das verificações não é arbitrária. Cada uma só faz sentido se a
//! anterior passou, e todas vêm **antes** da primeira vela:
//!
//! 1. O modo é `paper`. `live` é recusado aqui, não na décima ordem.
//! 2. A cerca e o instrumento são lidos do arquivo, e o perfil é validado.
//! 3. A credencial existe, não convive com a de produção, e não saca.
//! 4. A sessão retoma a execução anterior, se houver uma em aberto.
//! 5. **A posição local é reconciliada com a da corretora.** Divergente ou
//!    desconhecido, a sessão não começa.

use crate::cli::RodarArgs;
use crate::cmd_backtest::ExitCode;
use crate::config::{load_fees, load_instrumento, load_limits, validar_perfil};
use crate::parada;
use chrono::{Duration, Utc};
use rust_decimal::Decimal;
use std::time::SystemTime;
use trade_bybit::cliente_autenticado::ClienteAutenticado;
use trade_bybit::credencial::Credencial;
use trade_bybit::{BybitClient, FonteAoVivo};
use trade_domain::{
    AuditKind, EstadoRetomado, ExecutionMode, FeeModel, Position, Symbol, Veredito,
};
use trade_paper::executor::PrecoCorrente;
use trade_paper::{PaperExecutor, comparar, saldo_de, verificar_sem_saque};
use trade_ports::{AuditRecorder, Recorder};
use trade_risk::{KillSwitch, RiskGuard};
use trade_session::{Ambiente, FimDaSessao, RelogioReal, Sessao, SessaoConfig, SessaoInicio};
use trade_storage::runs_repo::{RunHeader, RunsRepository};
use trade_storage::{SqliteAuditSink, open_runs, recuperar};
use ulid::Ulid;

type Saida = Result<(ExitCode, String), (ExitCode, String)>;

pub fn run(args: &RodarArgs) -> (ExitCode, String) {
    match executar(args) {
        Ok(v) | Err(v) => v,
    }
}

fn executar(args: &RodarArgs) -> Saida {
    if args.mode != ExecutionMode::Paper {
        return Err((
            ExitCode::Uso,
            format!(
                "`paper rodar` só opera em modo 'paper', e veio '{}'. A promoção para \
                 capital real é ato humano registrado, e não um argumento de linha de \
                 comando.",
                args.mode
            ),
        ));
    }

    let symbol = Symbol::new(&args.symbol).map_err(|e| (ExitCode::Uso, e.to_string()))?;
    let limits = load_limits(&args.limits).map_err(|e| (ExitCode::Uso, e.to_string()))?;
    let fees_cfg = load_fees(&args.fees).map_err(|e| (ExitCode::Uso, e.to_string()))?;
    let instrumento =
        load_instrumento(&args.instrumento).map_err(|e| (ExitCode::Uso, e.to_string()))?;
    validar_perfil(&instrumento, limits.max_position_size, args.capital)
        .map_err(|e| (ExitCode::Uso, e))?;

    let credencial = Credencial::do_ambiente().map_err(|e| (ExitCode::Uso, e.to_string()))?;
    let cliente = ClienteAutenticado::novo(credencial);
    verificar_sem_saque(&cliente).map_err(|e| (ExitCode::Uso, e.to_string()))?;

    let mut strategy =
        crate::cmd_backtest::montar_estrategia(&args.strategy, &args.strategy_params)
            .map_err(|m| (ExitCode::Uso, m))?;

    let runs = open_runs(&args.runs_db).map_err(|e| (ExitCode::Uso, e.to_string()))?;
    let mut repo = RunsRepository::new(runs);

    // Retomar é o padrão, e não uma opção: um processo que cai e sobe de novo
    // continua a mesma sessão. Começar uma execução nova com posição aberta
    // partiria o registro dela em dois (decisão 035).
    let em_aberto = repo
        .ultima_paper_em_aberto()
        .map_err(|e| (ExitCode::Uso, e.to_string()))?;

    let agora = Utc::now();
    let (run_id, retomado, retomando) = match em_aberto {
        Some(e) => {
            let conn = open_runs(&args.runs_db).map_err(|x| (ExitCode::Uso, x.to_string()))?;
            let estado = recuperar(&conn, &e.run_id, agora, limits.window_minutes)
                .map_err(|x| (ExitCode::Uso, x.to_string()))?;
            (e.run_id, estado, true)
        }
        None => {
            // ULID e não UUID v4: ordenável por tempo, de modo que listar
            // execuções em ordem cronológica seja ORDER BY run_id.
            let id = Ulid::from_datetime(SystemTime::now()).to_string();
            let params = strategy.params();
            let fees = FeeModel {
                taker_fee_rate: fees_cfg.taker_fee_rate,
                slippage_rate: fees_cfg.slippage_rate,
            };
            repo.start_run(&RunHeader {
                run_id: &id,
                mode: ExecutionMode::Paper,
                symbol: &symbol,
                interval: args.interval,
                from: agora,
                // Uma sessão não tem fim conhecido ao começar. O campo é
                // `NOT NULL` no esquema, e gravar o início nos dois é dizer
                // "ainda não terminou" sem inventar uma data futura.
                to: agora,
                initial_capital: args.capital,
                limits: &limits,
                fees: &fees,
                strategy: strategy.name(),
                strategy_params: &params,
                started_at: agora,
            })
            .map_err(|e| (ExitCode::Uso, e.to_string()))?;
            (id, None, false)
        }
    };

    let position = retomado
        .as_ref()
        .map_or_else(Position::default, |e| e.position.clone());
    let balance = args.capital;

    // O registro retoma a numeração de onde parou. Sem isto, a chave
    // (run_id, seq) colidiria e o `INSERT OR REPLACE` sobrescreveria, um a
    // um, os eventos da sessão anterior — sem erro e sem aviso.
    let audit_conn = open_runs(&args.runs_db).map_err(|e| (ExitCode::Uso, e.to_string()))?;
    let ultimo_seq = retomado.as_ref().map_or(0, |e| e.proximo_seq - 1);
    let mut audit = AuditRecorder::retomando(
        run_id.clone(),
        ExecutionMode::Paper,
        SqliteAuditSink::new(audit_conn),
        ultimo_seq,
    );

    // A reconciliação, antes de qualquer ordem (T041, FR-108).
    let veredito = match saldo_de(&cliente, moeda_base(&symbol)) {
        Ok(remota) => comparar(position.qty(), remota, &instrumento),
        Err(e) => Veredito::Desconhecido(e.to_string()),
    };
    let _ = audit.record(
        agora,
        AuditKind::Reconciliation {
            veredito: veredito.clone(),
            local: position.qty(),
        },
    );

    if !veredito.permite_nova_entrada() {
        let _ = audit.flush();
        repo.finish_run(&run_id, Utc::now(), "halted", Some(veredito.as_str()))
            .map_err(|e| (ExitCode::Uso, e.to_string()))?;
        return Err((
            ExitCode::Interrompido,
            format!(
                "a sessão não começa: reconciliação **{}**.\n\
                 A posição local e a reportada pela corretora não conferem, ou não foi \
                 possível saber. Operar sem saber quanto se tem é o que o Princípio II \
                 chama de anomalia.\n\
                 run_id {run_id} · o veredito está no registro, em {}.",
                veredito.as_str(),
                args.runs_db.display()
            ),
        ));
    }

    // Daqui em diante o executor deixa de ser alcançável: ele é movido para
    // dentro do guard, e não existe caminho de volta.
    let preco = PrecoCorrente::default();
    let executor = PaperExecutor::novo(
        cliente,
        symbol.as_str(),
        run_id.clone(),
        args.stop_fracao,
        preco.clone(),
    );
    let mut guard = match &retomado {
        Some(e) => RiskGuard::retomar(
            executor,
            limits.clone(),
            KillSwitch::sentinel(&args.kill_file),
            e,
        ),
        None => RiskGuard::new(
            executor,
            limits.clone(),
            KillSwitch::sentinel(&args.kill_file),
        ),
    };

    let relogio = RelogioReal;
    let publico = BybitClient::new();
    let mut fonte = FonteAoVivo::nova(&publico, &relogio, symbol.clone(), args.interval);
    if let Some(ultima) = ultima_vela(&retomado) {
        fonte = fonte.depois_de(ultima);
    }

    let bandeira = parada::instalar().map_err(|e| {
        (
            ExitCode::Uso,
            format!("não foi possível instalar o tratador de sinal: {e}"),
        )
    })?;

    let sessao = Sessao::new(SessaoConfig {
        symbol: symbol.clone(),
        fees: FeeModel {
            taker_fee_rate: fees_cfg.taker_fee_rate,
            slippage_rate: fees_cfg.slippage_rate,
        },
        instrumento,
        // Da cerca, não do argumento: é limiar de risco, e vai gravado com
        // a execução (decisão 040 do Jev, 1,00 · confiança 1,00).
        prazo_maximo: Duration::hours(limits.max_position_hours),
    });

    let resultado = sessao
        .executar(
            strategy.as_mut(),
            &mut fonte,
            &mut guard,
            &Ambiente {
                clock: &relogio,
                parar: &bandeira,
                preco: &preco,
            },
            SessaoInicio {
                position,
                balance,
                proxima_ordem: 0,
            },
            &mut audit,
        )
        .map_err(|e| (ExitCode::Uso, e.to_string()))?;

    audit
        .flush()
        .map_err(|e| (ExitCode::Uso, format!("falha ao gravar a auditoria: {e}")))?;

    repo.save_trades(&run_id, &resultado.trades)
        .map_err(|e| (ExitCode::Uso, e.to_string()))?;
    repo.save_metrics(
        &run_id,
        &trade_domain::RunMetrics::from_trades(&resultado.trades, Decimal::ZERO),
    )
    .map_err(|e| (ExitCode::Uso, e.to_string()))?;

    let (desfecho, motivo, code) = match &resultado.fim {
        FimDaSessao::FonteEncerrada => ("completed", None, ExitCode::Ok),
        FimDaSessao::Interrompida => ("completed", None, ExitCode::Ok),
        FimDaSessao::Parada { motivo } => ("halted", Some(motivo.clone()), ExitCode::Interrompido),
    };
    repo.finish_run(&run_id, Utc::now(), desfecho, motivo.as_deref())
        .map_err(|e| (ExitCode::Uso, e.to_string()))?;

    Ok((
        code,
        format!(
            "\n  Sessão {}\n  run_id                   {run_id}\n  \
             Velas processadas        {}\n  Operações                {}\n  \
             Posição ao encerrar      {}\n  Fim                      {}\n  \
             Registro                 {}\n",
            if retomando { "retomada" } else { "nova" },
            resultado.velas,
            resultado.trades.len(),
            resultado.position.qty(),
            match &resultado.fim {
                FimDaSessao::FonteEncerrada => "a fonte de velas se encerrou".to_string(),
                FimDaSessao::Interrompida => "encerrada por sinal do sistema".to_string(),
                FimDaSessao::Parada { motivo } => format!("parada — {motivo}"),
            },
            args.runs_db.display()
        ),
    ))
}

/// A moeda que a posição detém.
///
/// `BTCUSDT` detém BTC. A convenção da Bybit no spot é base seguida de cotação,
/// e as cotações que este projeto usa terminam em USDT.
fn moeda_base(symbol: &Symbol) -> &str {
    symbol
        .as_str()
        .strip_suffix("USDT")
        .unwrap_or(symbol.as_str())
}

/// Depois de qual vela a fonte deve retomar.
///
/// A abertura da posição não serve: uma sessão pode ter processado centenas de
/// velas sem abrir nada. O que serve é não ter nada — e aí a fonte começa do
/// presente.
fn ultima_vela(retomado: &Option<EstadoRetomado>) -> Option<chrono::DateTime<Utc>> {
    retomado.as_ref()?.aceitas.last().copied()
}
