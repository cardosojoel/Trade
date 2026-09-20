//! Binário `trade`.

use clap::Parser;
use std::process::ExitCode;
use trade_cli::cli::{Cli, Command};
use trade_cli::{cmd_backtest, cmd_collect, cmd_paper, cmd_perfil};
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

        Command::Paper(args) => {
            let r = match args.acao {
                trade_cli::cli::PaperAcao::Verificar => cmd_paper::run(),
            };
            match r {
                Ok(saida) => {
                    println!("{saida}");
                    ExitCode::SUCCESS
                }
                Err((codigo, msg)) => {
                    eprintln!("{msg}");
                    ExitCode::from(codigo)
                }
            }
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
