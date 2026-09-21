//! Fatia 2 — leitura do que já rodou (T010 a T015).

use rust_decimal::dec;
use serde_json::Value;
use std::collections::BTreeMap;
use tempfile::tempdir;
use trade_domain::{ExecutionMode, FeeModel, Interval, RiskLimits, RunMetrics, Symbol, Trade};
use trade_serve::respostas;
use trade_storage::open_runs;
use trade_storage::runs_repo::{RunHeader, RunsRepository};

fn t(h: u32) -> chrono::DateTime<chrono::Utc> {
    use chrono::TimeZone;
    chrono::Utc.with_ymd_and_hms(2026, 1, 1, h, 0, 0).unwrap()
}

fn repo() -> (tempfile::TempDir, RunsRepository) {
    let dir = tempdir().unwrap();
    let conn = open_runs(dir.path().join("runs.db")).unwrap();
    (dir, RunsRepository::new(conn))
}

fn gravar(r: &mut RunsRepository, run_id: &str, pnl: rust_decimal::Decimal) {
    let sym = Symbol::new("BTCUSDT").unwrap();
    let lim = RiskLimits {
        max_daily_loss: dec!(200.00),
        ..RiskLimits::default()
    };
    let fees = FeeModel::default();
    let params = BTreeMap::new();
    r.start_run(&RunHeader {
        run_id,
        mode: ExecutionMode::Backtest,
        symbol: &sym,
        interval: Interval::M1,
        from: t(0),
        to: t(12),
        initial_capital: dec!(10000),
        limits: &lim,
        fees: &fees,
        strategy: "sma-cross",
        strategy_params: &params,
        started_at: t(0),
    })
    .unwrap();
    let trades = vec![
        Trade {
            seq: 1,
            entry_at: t(1),
            entry_price: dec!(100),
            exit_at: t(2),
            exit_price: dec!(110),
            qty: dec!(1),
            fees: dec!(0.1),
            pnl,
        },
        Trade {
            seq: 2,
            entry_at: t(3),
            entry_price: dec!(110),
            exit_at: t(4),
            exit_price: dec!(100),
            qty: dec!(1),
            fees: dec!(0.1),
            pnl: dec!(-5),
        },
    ];
    r.save_trades(run_id, &trades).unwrap();
    r.save_metrics(run_id, &RunMetrics::from_trades(&trades, dec!(0.25)))
        .unwrap();
    r.finish_run(run_id, t(12), "completed", None).unwrap();
}

/// Percorre o JSON e devolve os caminhos onde há número em campo monetário.
fn numeros_em_campo_monetario(v: &Value) -> Vec<String> {
    const MONETARIOS: &[&str] = &[
        "net_result",
        "profit_factor",
        "max_drawdown",
        "gross_profit",
        "gross_loss",
        "total_fees",
        "total_slippage",
        "initial_capital",
        "max_daily_loss",
        "max_position_size",
        "max_total_exposure",
        "qty",
        "pnl",
        "fees",
        "entry_price",
        "exit_price",
        "taker_fee_rate",
        "slippage_rate",
    ];
    fn anda(v: &Value, c: &str, out: &mut Vec<String>, m: &[&str]) {
        match v {
            Value::Object(o) => {
                for (k, x) in o {
                    let cc = format!("{c}.{k}");
                    if m.contains(&k.as_str()) && x.is_number() {
                        out.push(cc.clone());
                    }
                    anda(x, &cc, out, m);
                }
            }
            Value::Array(a) => {
                for (i, x) in a.iter().enumerate() {
                    anda(x, &format!("{c}[{i}]"), out, m);
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    anda(v, "", &mut out, MONETARIOS);
    out
}

// ---------------------------------------------------------------- T010

#[test]
fn nenhum_valor_monetario_sai_como_numero_json() {
    // SC-002, varrendo toda resposta. Contagem e identificador seguem número;
    // dinheiro, quantidade e preço são string.
    let (_d, mut r) = repo();
    gravar(&mut r, "01A", dec!(10));
    let corpo = respostas::execucoes(&r).unwrap();
    let achados = numeros_em_campo_monetario(&corpo);
    assert!(
        achados.is_empty(),
        "números onde devia haver string: {achados:?}"
    );
}

#[test]
fn contagem_continua_sendo_numero() {
    // O inverso também importa: transformar tudo em string faria a tela
    // converter de volta, e converter de volta é onde o erro entra.
    let (_d, mut r) = repo();
    gravar(&mut r, "01A", dec!(10));
    let corpo = respostas::execucoes(&r).unwrap();
    assert!(corpo["grupos"][0]["metricas"]["trade_count"].is_number());
    assert!(corpo["grupos"][0]["identicas"].is_number());
}

// ---------------------------------------------------------------- T011

#[test]
fn a_versao_do_codigo_vem_presente_e_nula() {
    // FR-006 e SC-009: presente e nulo, **nunca omitido**. É o que permite à
    // tela distinguir "o registro não guarda" de "esqueceram de mandar".
    let (_d, mut r) = repo();
    gravar(&mut r, "01A", dec!(10));
    let g = &respostas::execucoes(&r).unwrap()["grupos"][0];
    assert!(g.get("versao_do_codigo").is_some(), "o campo existe");
    assert!(g["versao_do_codigo"].is_null(), "e vem nulo");
}

#[test]
fn sem_versao_do_codigo_a_execucao_se_declara_nao_comparavel() {
    // FR-007: não confiável ainda que cerca e taxas sejam idênticas.
    let (_d, mut r) = repo();
    gravar(&mut r, "01A", dec!(10));
    let g = &respostas::execucoes(&r).unwrap()["grupos"][0];
    assert_eq!(g["comparavel"], false);
    assert!(
        g["por_que_nao_comparavel"]
            .as_str()
            .unwrap()
            .contains("versão do código"),
        "a resposta diz por quê, em vez de só negar"
    );
}

// ---------------------------------------------------------------- T012

#[test]
fn execucoes_de_resultado_identico_viram_um_grupo_so() {
    // Nove execuções, das quais oito idênticas, foi o que o registro real
    // mostrou. Listar as nove lado a lado esconde que oito são a mesma coisa.
    let (_d, mut r) = repo();
    for id in ["01A", "01B", "01C"] {
        gravar(&mut r, id, dec!(10));
    }
    let corpo = respostas::execucoes(&r).unwrap();
    let grupos = corpo["grupos"].as_array().unwrap();
    assert_eq!(grupos.len(), 1, "as três são idênticas");
    assert_eq!(grupos[0]["identicas"], 3);
    assert_eq!(grupos[0]["execucoes"].as_array().unwrap().len(), 3);
}

#[test]
fn resultados_diferentes_ficam_em_grupos_diferentes() {
    let (_d, mut r) = repo();
    gravar(&mut r, "01A", dec!(10));
    gravar(&mut r, "01B", dec!(20));
    let corpo = respostas::execucoes(&r).unwrap();
    assert_eq!(corpo["grupos"].as_array().unwrap().len(), 2);
}

#[test]
fn o_grupo_traz_a_cerca_sob_a_qual_correu() {
    let (_d, mut r) = repo();
    gravar(&mut r, "01A", dec!(10));
    let g = &respostas::execucoes(&r).unwrap()["grupos"][0];
    assert_eq!(g["cerca"]["limits"]["max_daily_loss"], "200.00");
    assert!(g["cerca"]["fees"]["taker_fee_rate"].is_string());
}

// ---------------------------------------------------------------- T013

#[test]
fn a_execucao_traz_metricas_e_decomposicao_do_custo() {
    let (_d, mut r) = repo();
    gravar(&mut r, "01A", dec!(10));
    let c = respostas::execucao(&r, "01A").unwrap().expect("existe");
    assert_eq!(c["run_id"], "01A");
    assert!(c["metricas"]["net_result"].is_string());
    assert!(c["custo"]["total_fees"].is_string());
    assert!(c["custo"]["total_slippage"].is_string());
}

#[test]
fn execucao_inexistente_devolve_nada() {
    let (_d, r) = repo();
    assert!(respostas::execucao(&r, "nao-existe").unwrap().is_none());
}

// ---------------------------------------------------------------- T015

#[test]
fn profit_factor_indefinido_vem_nulo_com_o_motivo() {
    // FR-021: nunca zero, infinito ou texto. Gravar infinito produziria um
    // número que induz erro na Porta 1.
    let (_d, mut r) = repo();
    let sym = Symbol::new("BTCUSDT").unwrap();
    let lim = RiskLimits::default();
    let fees = FeeModel::default();
    let params = BTreeMap::new();
    r.start_run(&RunHeader {
        run_id: "01Z",
        mode: ExecutionMode::Backtest,
        symbol: &sym,
        interval: Interval::M1,
        from: t(0),
        to: t(1),
        initial_capital: dec!(10000),
        limits: &lim,
        fees: &fees,
        strategy: "s",
        strategy_params: &params,
        started_at: t(0),
    })
    .unwrap();
    // Só operação vencedora: o fator fica indefinido.
    let trades = vec![Trade {
        seq: 1,
        entry_at: t(1),
        entry_price: dec!(100),
        exit_at: t(2),
        exit_price: dec!(110),
        qty: dec!(1),
        fees: dec!(0),
        pnl: dec!(10),
    }];
    r.save_trades("01Z", &trades).unwrap();
    r.save_metrics("01Z", &RunMetrics::from_trades(&trades, dec!(0)))
        .unwrap();

    let c = respostas::execucao(&r, "01Z").unwrap().unwrap();
    assert!(c["metricas"]["profit_factor"].is_null());
    assert!(
        c["metricas"]["profit_factor_indefinido_porque"]
            .as_str()
            .is_some(),
        "o motivo vem junto, em vez de a tela ter que adivinhar"
    );
}
