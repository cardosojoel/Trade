//! Definição da linha de comando.

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use clap::{Parser, Subcommand};
use rust_decimal::Decimal;
use std::path::PathBuf;
use trade_domain::{ExecutionMode, Interval};

#[derive(Parser, Debug)]
#[command(
    name = "trade",
    version,
    about = "Robô de day trade automatizado de Bitcoin",
    long_about = "Robô de day trade de Bitcoin, mercado spot e apenas comprado.\n\n\
                  Nesta versão só existe o modo `backtest`. Paper trading e execução com \
                  capital real são features futuras, e a constitution do projeto exige que \
                  a promoção entre elas seja um ato humano registrado."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

// A variante Backtest é bem maior que Kill. Boxear resolveria o aviso e
// acrescentaria uma indireção a um enum construído uma única vez, na
// inicialização do processo — custo sem benefício.
#[allow(clippy::large_enum_variant)]
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Coleta o histórico público da Bybit e grava localmente.
    Collect(CollectArgs),
    /// Executa um backtest sobre o histórico local.
    Backtest(BacktestArgs),
    /// Aciona ou libera o kill switch.
    Kill(KillArgs),
}

#[derive(clap::Args, Debug)]
pub struct CollectArgs {
    #[arg(long, default_value = "BTCUSDT")]
    pub symbol: String,

    #[arg(long, default_value = "1m", value_parser = parse_interval)]
    pub interval: Interval,

    /// Início do período, em UTC (AAAA-MM-DD).
    #[arg(long, value_parser = parse_date)]
    pub from: DateTime<Utc>,

    /// Fim do período, exclusivo, em UTC (AAAA-MM-DD).
    #[arg(long, value_parser = parse_date)]
    pub to: DateTime<Utc>,

    #[arg(long, default_value = "data/market.db")]
    pub db: PathBuf,

    /// Tentativas antes de desistir de uma falha transitória.
    #[arg(long, default_value_t = 5)]
    pub max_retries: u32,
}

// Repare no que NÃO existe acima: nenhum parâmetro de credencial. O endpoint
// de histórico da Bybit é público, e a forma mais simples de garantir FR-012 é
// não haver nada a passar.

#[derive(clap::Args, Debug)]
pub struct BacktestArgs {
    /// Modo de execução. Obrigatório e sem valor padrão.
    ///
    /// Não há padrão de propósito: o Princípio I da constitution proíbe que a
    /// ausência de configuração resulte em qualquer coisa que toque dinheiro.
    #[arg(long, required = true, value_parser = parse_mode)]
    pub mode: ExecutionMode,

    #[arg(long, default_value = "BTCUSDT")]
    pub symbol: String,

    #[arg(long, default_value = "1m", value_parser = parse_interval)]
    pub interval: Interval,

    /// Início do período, em UTC (AAAA-MM-DD).
    #[arg(long, value_parser = parse_date)]
    pub from: DateTime<Utc>,

    /// Fim do período, exclusivo, em UTC (AAAA-MM-DD).
    #[arg(long, value_parser = parse_date)]
    pub to: DateTime<Utc>,

    /// Capital inicial simulado.
    #[arg(long, value_parser = parse_decimal)]
    pub capital: Decimal,

    #[arg(long, default_value = "sma-cross")]
    pub strategy: String,

    /// Parâmetros da estratégia, ex.: fast=9,slow=21,position_fraction=0.1
    #[arg(long, default_value = "")]
    pub strategy_params: String,

    /// Arquivo com a cerca. Sem ele não há execução: limite ausente não pode
    /// virar "tudo passa".
    #[arg(long, default_value = "limits.toml")]
    pub limits: PathBuf,

    #[arg(long, default_value = "fees.toml")]
    pub fees: PathBuf,

    /// Regras de quantidade do instrumento, relidas a cada execução.
    ///
    /// A Bybit revisa passo e valor mínimo nos dias 3 e 17 de cada mês. Herdar
    /// de uma execução anterior é operar com regra que já não vale.
    #[arg(long, default_value = "instrumento.toml")]
    pub instrumento: PathBuf,

    #[arg(long, default_value = "data/market.db")]
    pub market_db: PathBuf,

    #[arg(long, default_value = "data/runs.db")]
    pub runs_db: PathBuf,

    /// Arquivo sentinela do kill switch.
    #[arg(long, default_value = "data/KILL")]
    pub kill_file: PathBuf,
}

#[derive(clap::Args, Debug)]
pub struct KillArgs {
    /// Libera o kill switch. Ato humano explícito — nada no sistema faz isto.
    #[arg(long)]
    pub release: bool,

    #[arg(long, default_value = "data/KILL")]
    pub kill_file: PathBuf,
}

fn parse_mode(s: &str) -> Result<ExecutionMode, String> {
    s.parse::<ExecutionMode>().map_err(|e| e.to_string())
}

fn parse_interval(s: &str) -> Result<Interval, String> {
    s.parse::<Interval>().map_err(|e| e.to_string())
}

fn parse_date(s: &str) -> Result<DateTime<Utc>, String> {
    let d = NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d")
        .map_err(|_| format!("data '{s}' inválida — use AAAA-MM-DD, em UTC"))?;
    Utc.from_local_datetime(&d.and_hms_opt(0, 0, 0).unwrap())
        .single()
        .ok_or_else(|| format!("data '{s}' ambígua em UTC"))
}

fn parse_decimal(s: &str) -> Result<Decimal, String> {
    s.trim()
        .parse::<Decimal>()
        .map_err(|_| format!("'{s}' não é um número decimal válido"))
}
