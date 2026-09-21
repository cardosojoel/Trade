//! Binário `trade`.

use clap::Parser;
use std::process::ExitCode;
use trade_cli::cli::{Cli, Command};
use trade_cli::{cmd_backtest, cmd_collect, cmd_paper, cmd_paper_rodar, cmd_perfil, cmd_serve};
use trade_risk::KillSwitch;

fn main() -> ExitCode {
    let cli = Cli::parse();

    match &cli.command {
        Command::Collect(args) => match cmd_collect::run(args) {
            Ok(saida) => {
                println!("{saida}");
                ExitCode::SUCCESS
            }
            Err((codigo, msg)) => {
                eprintln!("{msg}");
                ExitCode::from(codigo)
            }
        },

        Command::Backtest(args) => {
            let (code, saida) = cmd_backtest::run(args);
            if code == cmd_backtest::ExitCode::Ok
                || matches!(
                    code,
                    cmd_backtest::ExitCode::Interrompido | cmd_backtest::ExitCode::CapitalEsgotado
                )
            {
                print!("{saida}");
            } else {
                eprintln!("{saida}");
            }
            ExitCode::from(code as u8)
        }

        Command::Paper(args) => match &args.acao {
            trade_cli::cli::PaperAcao::Verificar => match cmd_paper::run() {
                Ok(saida) => {
                    println!("{saida}");
                    ExitCode::SUCCESS
                }
                Err((codigo, msg)) => {
                    eprintln!("{msg}");
                    ExitCode::from(codigo)
                }
            },
            trade_cli::cli::PaperAcao::Rodar(rodar) => {
                let (codigo, saida) = cmd_paper_rodar::run(rodar);
                if codigo == trade_cli::cmd_backtest::ExitCode::Ok {
                    println!("{saida}");
                } else {
                    eprintln!("{saida}");
                }
                ExitCode::from(codigo as u8)
            }
        },

        Command::Serve(args) => {
            let (codigo, saida) = cmd_serve::run(args);
            if !saida.is_empty() {
                eprintln!("{saida}");
            }
            ExitCode::from(codigo as u8)
        }

        Command::Perfil(args) => match cmd_perfil::run(args) {
            Ok(saida) => {
                println!("{saida}");
                ExitCode::SUCCESS
            }
            Err((codigo, msg)) => {
                eprintln!("{msg}");
                ExitCode::from(codigo)
            }
        },

        Command::Kill(args) => {
            let ks = KillSwitch::sentinel(&args.kill_file);
            let r = if args.release {
                ks.release()
            } else {
                ks.engage()
            };
            match r {
                Ok(()) if args.release => {
                    println!("Kill switch liberado. A operação volta a aceitar ordens.");
                    ExitCode::SUCCESS
                }
                Ok(()) => {
                    println!(
                        "Kill switch acionado em {}.\n\
                         Nenhuma ordem nova será aceita. A posição em aberto é mantida e \
                         reportada — liquidar é decisão sua, não do robô.",
                        args.kill_file.display()
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("falha ao operar o kill switch: {e}");
                    ExitCode::from(2)
                }
            }
        }
    }
}
