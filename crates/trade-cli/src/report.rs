//! Saída do backtest.

use rust_decimal::Decimal;
use trade_backtest::{BacktestOutcome, RunOutcome};
use trade_domain::Money;

fn dinheiro(v: Money) -> String {
    format!("{:>14}", v.round_dp(2).to_string())
}

fn percentual(parte: Money, todo: Money) -> String {
    if todo.is_zero() {
        return "     —".into();
    }
    let p = (parte / todo) * Decimal::from(100);
    format!("{:>+6.1}%", p.round_dp(1))
}

/// Monta o relatório de uma execução.
pub fn render(run_id: &str, capital: Money, r: &BacktestOutcome) -> String {
    let m = &r.metrics;
    let mut s = String::new();

    s.push_str(&format!("\nExecução {run_id}  ·  modo=backtest\n\n"));
    s.push_str(&format!("  Capital inicial   {}\n", dinheiro(capital)));
    s.push_str(&format!(
        "  Resultado líquido {}   {}\n",
        dinheiro(m.net_result),
        percentual(m.net_result, capital)
    ));
    s.push_str(&format!("  Taxas             {}\n", dinheiro(m.total_fees)));
    s.push_str(&format!(
        "  Slippage          {}\n\n",
        dinheiro(m.total_slippage)
    ));

    s.push_str(&format!("  Operações         {:>14}\n", m.trade_count));
    s.push_str(&format!(
        "  Profit factor     {:>14}\n",
        // Indefinido, nunca ∞: sem operação perdedora não há razão entre
        // lucro e prejuízo, e inventar uma induziria erro na Porta 1.
        match m.profit_factor {
            Some(pf) => pf.round_dp(2).to_string(),
            None => "indefinido".to_string(),
        }
    ));
    s.push_str(&format!(
        "  Drawdown máximo   {}   {}\n\n",
        dinheiro(m.max_drawdown),
        percentual(m.max_drawdown, capital)
    ));

    let total_recusas = r.rejections_total();
    s.push_str(&format!("  Ordens recusadas  {total_recusas:>14}"));
    if total_recusas > 0 {
        let detalhe: Vec<String> = r
            .rejections
            .iter()
            .map(|(b, n)| format!("{} {}", b.as_str(), n))
            .collect();
        s.push_str(&format!("   ({})", detalhe.join(" · ")));
    }
    s.push('\n');

    if !r.gaps.is_empty() {
        s.push_str(&format!(
            "  Lacunas           {:>14}   (reportadas, não interpoladas)\n",
            r.gaps.len()
        ));
    }

    s.push_str(&format!("  Velas percorridas {:>14}\n\n", r.candles_seen));

    match &r.outcome {
        RunOutcome::Completed => {
            s.push_str("  Período percorrido por completo.\n");
        }
        RunOutcome::Halted { reason, at } => {
            s.push_str(&format!(
                "  INTERRUPÇÃO em {at}: {reason}\n  \
                 Falha de integridade exige revisão humana antes de retomar.\n"
            ));
        }
        RunOutcome::CapitalExhausted { at } => {
            s.push_str(&format!(
                "  CAPITAL ESGOTADO em {at}.\n  \
                 O patrimônio caiu abaixo do piso configurado; a execução parou ali.\n"
            ));
        }
    }

    if !r.final_position.is_flat() {
        s.push_str(&format!(
            "\n  Posição em aberto: {} ao preço médio {}\n",
            r.final_position.qty(),
            r.final_position.avg_price().round_dp(2)
        ));
    }

    s
}
