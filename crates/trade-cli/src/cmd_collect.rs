//! Comando `trade collect`.

use crate::cli::CollectArgs;
use chrono::Utc;
use trade_bybit::{BybitClient, Collector};
use trade_domain::Symbol;
use trade_storage::{SqliteCandleRepository, open_market};

pub fn run(args: &CollectArgs) -> Result<String, (u8, String)> {
    let symbol = Symbol::new(&args.symbol).map_err(|e| (2u8, e.to_string()))?;

    if args.to <= args.from {
        return Err((
            2,
            format!(
                "período vazio: --to ({}) não é posterior a --from ({})",
                args.to.date_naive(),
                args.from.date_naive()
            ),
        ));
    }

    let agora = Utc::now();
    if args.from >= agora {
        return Err((
            2,
            format!(
                "--from ({}) está no futuro: não há histórico a coletar",
                args.from.date_naive()
            ),
        ));
    }

    let conn = open_market(&args.db).map_err(|e| (2u8, e.to_string()))?;
    let mut repo = SqliteCandleRepository::new(conn);
    let client = BybitClient::new();

    println!(
        "Coletando {} {} de {} a {}",
        symbol,
        args.interval,
        args.from.date_naive(),
        args.to.date_naive()
    );

    let out = Collector::new(&client, &mut repo, args.max_retries)
        .collect(&symbol, args.interval, args.from, args.to, agora)
        .map_err(|e| (3u8, e.to_string()))?;

    repo.record_dataset(&symbol, args.interval, "bybit-v5-spot", agora)
        .map_err(|e| (2u8, e.to_string()))?;

    let mut s = format!(
        "  {} páginas · {} velas novas · {} já existentes",
        out.paginas, out.velas_novas, out.velas_ja_existentes
    );
    if out.descartadas_em_formacao > 0 {
        s.push_str(&format!(
            " · {} em formação, descartada(s)",
            out.descartadas_em_formacao
        ));
    }
    if out.tentativas_extras > 0 {
        s.push_str(&format!(" · {} retentativa(s)", out.tentativas_extras));
    }
    if !out.lacunas.is_empty() {
        s.push_str(&format!(
            "\n  {} lacuna(s) na fonte, registradas e não interpoladas",
            out.lacunas.len()
        ));
    }
    s.push_str(&format!("\nGravado em {}", args.db.display()));

    Ok(s)
}
