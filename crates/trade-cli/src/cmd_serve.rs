//! `trade serve` — o servidor local de leitura do registro.
//!
//! O terceiro ponto de composição, e o mais estreito: o servidor não sabe
//! montar execução nenhuma. Ele recebe, na construção, algo que sabe — e quem
//! sabe é esta função, que chama a **mesma** composição do `trade backtest`
//! (FR-001).
//!
//! É o que faz o `FR-001` e o `SC-008` valerem juntos. Sem esta ligação,
//! `trade-serve` precisaria declarar `trade-cli` e herdaria, pelo grafo, tudo
//! que a CLI alcança — inclusive a corretora.

use crate::cli::{BacktestArgs, ServeArgs};
use crate::cmd_backtest::ExitCode;
use crate::config::load_porta1;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use trade_domain::{ExecutionMode, Interval};
use trade_serve::Servidor;
use trade_serve::consultas::ExigenciasPorta1;
use trade_serve::escrita::{IniciarExecucao, Parametros, Quadro, TrabalhoId};
use ulid::Ulid;

/// O ponto de ligação. Sabe montar um backtest; não sabe mais nada.
struct Composicao {
    base: ServeArgs,
    quadros: Arc<Mutex<std::collections::BTreeMap<String, Quadro>>>,
}

impl IniciarExecucao for Composicao {
    fn iniciar(&self, p: &Parametros) -> Result<TrabalhoId, String> {
        // `live` já foi recusado pelo servidor; esta é a segunda barreira, e
        // é deliberada. Se um dia alguém afrouxar a primeira, esta continua.
        let mode = ExecutionMode::from_str(&p.modo).map_err(|e| e.to_string())?;
        if mode != ExecutionMode::Backtest {
            return Err(format!(
                "o servidor só inicia `backtest`. `{}` opera continuamente e tem comando \
                 próprio, que não passa por aqui.",
                p.modo
            ));
        }

        let args = BacktestArgs {
            mode,
            symbol: p.simbolo.clone(),
            interval: Interval::from_str(&p.intervalo).map_err(|e| e.to_string())?,
            from: instante(p.de_ms)?,
            to: instante(p.ate_ms)?,
            capital: Decimal::from_str(&p.capital_inicial)
                .map_err(|e| format!("capital inválido: {e}"))?,
            strategy: p.estrategia.clone(),
            strategy_params: p
                .parametros
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(","),
            limits: self.base.limits.clone(),
            fees: self.base.fees.clone(),
            instrumento: self.base.instrumento.clone(),
            market_db: self.base.market_db.clone(),
            runs_db: self.base.runs_db.clone(),
            kill_file: self.base.kill_file.clone(),
        };

        let trabalho = format!("job_{}", Ulid::from_datetime(std::time::SystemTime::now()));
        let execucao = String::new();
        self.quadros
            .lock()
            .map_err(|_| "quadro de trabalhos indisponível".to_string())?
            .insert(trabalho.clone(), Quadro::novo(&trabalho, &execucao));

        // Numa thread, e devolvendo já: o FR-017 exige resposta imediata, e
        // um backtest de doze meses leva cerca de um segundo — mas "cerca de"
        // não é contrato.
        let quadros = Arc::clone(&self.quadros);
        let id = trabalho.clone();
        std::thread::spawn(move || {
            let (codigo, saida) = crate::cmd_backtest::run(&args);
            let mut q = quadros.lock().expect("quadro");
            if let Some(quadro) = q.get_mut(&id) {
                quadro.registrar(saida);
                if codigo == ExitCode::Ok {
                    quadro.concluir();
                } else {
                    quadro.falhar(format!("terminou com código {}", codigo as u8));
                }
            }
        });

        Ok(TrabalhoId {
            trabalho,
            execucao: "(atribuído ao iniciar)".into(),
        })
    }
}

fn instante(ms: i64) -> Result<DateTime<Utc>, String> {
    DateTime::from_timestamp_millis(ms).ok_or_else(|| format!("instante inválido: {ms}"))
}

pub fn run(args: &ServeArgs) -> (ExitCode, String) {
    match executar(args) {
        Ok(v) | Err(v) => v,
    }
}

fn executar(args: &ServeArgs) -> Result<(ExitCode, String), (ExitCode, String)> {
    // As exigências da Porta 1 vêm do arquivo, e sem ele o servidor **não
    // sobe**. Subir julgando por números que ninguém escolheu seria pior que
    // não subir (FR-024).
    let p1 = load_porta1(&args.porta1).map_err(|e| (ExitCode::Uso, e.to_string()))?;
    let exigencias = ExigenciasPorta1 {
        meses_minimos: p1.meses_minimos,
        operacoes_minimas: p1.operacoes_minimas,
        profit_factor_minimo: Decimal::from_str(&p1.profit_factor_minimo)
            .map_err(|e| (ExitCode::Uso, format!("profit_factor_minimo: {e}")))?,
        drawdown_maximo_fracao: Decimal::from_str(&p1.drawdown_maximo_fracao)
            .map_err(|e| (ExitCode::Uso, format!("drawdown_maximo_fracao: {e}")))?,
    };

    let servidor = Servidor::subir(args.porta).map_err(|e| {
        (
            ExitCode::Uso,
            format!("não foi possível escutar em 127.0.0.1:{}: {e}", args.porta),
        )
    })?;

    let endereco = servidor
        .endereco()
        .map(|a| a.to_string())
        .unwrap_or_else(|| format!("127.0.0.1:{}", args.porta));

    let composicao = Composicao {
        base: args.clone(),
        quadros: Arc::new(Mutex::new(std::collections::BTreeMap::new())),
    };

    // **Uma vez, aqui, e em lugar nenhum mais.** O token não vai para o
    // registro, para log, para mensagem de erro nem para resposta de rota
    // alguma (FR-019, FR-020). Quem não copiou agora reinicia o servidor.
    println!(
        "\n  Servidor em               http://{endereco}\n  \
         Token de escrita          {}\n  \
         Registro                  {}\n  \
         Histórico                 {}\n\n  \
         O token morre com o processo: reiniciar sorteia outro. Ele não é gravado\n  \
         em lugar nenhum — se perder, reinicie.\n\n  \
         Só `POST /runs` o exige. As dez rotas de leitura não.\n",
        servidor.guarda().token(),
        args.runs_db.display(),
        args.market_db.display(),
    );

    let app = trade_serve::Aplicacao {
        runs_db: args.runs_db.clone(),
        market_db: args.market_db.clone(),
        exigencias,
        ligacao: &composicao,
        quadros: Mutex::new(std::collections::BTreeMap::new()),
    };

    servidor.atender(&app);
    Ok((ExitCode::Ok, String::new()))
}
