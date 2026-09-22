//! Fatia 4 — comparação e histórico (T026 a T031).

use rust_decimal::dec;
use std::collections::BTreeMap;
use tempfile::tempdir;
use trade_domain::{ExecutionMode, FeeModel, Interval, RiskLimits, RunMetrics, Symbol, Trade};
use trade_serve::{consultas, respostas};
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

fn exigencias() -> consultas::ExigenciasPorta1 {
    consultas::ExigenciasPorta1 {
        meses_minimos: 12,
        operacoes_minimas: 100,
        profit_factor_minimo: dec!(1.3),
        drawdown_maximo_fracao: dec!(0.15),
    }
}

fn gravar(r: &mut RunsRepository, run_id: &str, perda_diaria: &str, pnl: rust_decimal::Decimal) {
    use std::str::FromStr;
    let sym = Symbol::new("BTCUSDT").unwrap();
    let lim = RiskLimits {
        max_daily_loss: rust_decimal::Decimal::from_str(perda_diaria).unwrap(),
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
        code_version: Some("v1"),
    })
    .unwrap();
    let trades = vec![Trade {
        seq: 1,
        entry_at: t(1),
        entry_price: dec!(100),
        exit_at: t(2),
        exit_price: dec!(110),
        qty: dec!(1),
        fees: dec!(0.1),
        pnl,
    }];
    r.save_trades(run_id, &trades).unwrap();
    r.save_metrics(run_id, &RunMetrics::from_trades(&trades, dec!(0)))
        .unwrap();
    r.finish_run(run_id, t(12), "completed", None).unwrap();
}

// ---------------------------------------------------------------- T026

#[test]
fn a_comparacao_poe_as_duas_cercas_lado_a_lado() {
    let (_d, mut r) = repo();
    gravar(&mut r, "01A", "200.00", dec!(10));
    gravar(&mut r, "01B", "500.00", dec!(10));

    let c = respostas::comparar(&r, "01A", "01B")
        .unwrap()
        .expect("as duas existem");
    assert_eq!(c["a"]["run_id"], "01A");
    assert_eq!(c["b"]["run_id"], "01B");
    assert_eq!(c["a"]["cerca"]["limits"]["max_daily_loss"], "200.00");
    assert_eq!(c["b"]["cerca"]["limits"]["max_daily_loss"], "500.00");
}

#[test]
fn a_comparacao_assinala_em_que_as_cercas_diferem() {
    // Mostrar as duas lado a lado sem dizer onde diferem deixa o trabalho de
    // comparar para quem lê — que é justamente o que a rota existe para fazer.
    let (_d, mut r) = repo();
    gravar(&mut r, "01A", "200.00", dec!(10));
    gravar(&mut r, "01B", "500.00", dec!(10));
    let c = respostas::comparar(&r, "01A", "01B").unwrap().unwrap();
    let difs = c["cercas_diferem_em"].as_array().unwrap();
    assert!(
        difs.iter().any(|d| d == "max_daily_loss"),
        "a diferença de limite foi assinalada: {difs:?}"
    );
}

#[test]
fn comparar_com_execucao_inexistente_devolve_nada() {
    let (_d, mut r) = repo();
    gravar(&mut r, "01A", "200.00", dec!(10));
    assert!(
        respostas::comparar(&r, "01A", "nao-existe")
            .unwrap()
            .is_none()
    );
}

// ---------------------------------------------------------------- T027

#[test]
fn cercas_identicas_nao_bastam_sem_a_versao_do_codigo() {
    // FR-007 e SC-010, e o caso concreto que originou a regra: duas
    // execuções com a mesma cerca e as mesmas 14.308 operações deram
    // resultados diferentes, porque rodaram minutos antes e depois de um
    // commit que mudou o arredondamento.
    //
    // **Este teste mudou de forma quando P6/P9 foi implementada.** Antes, a
    // comparação era sempre não confiável, porque a versão nunca existia.
    // Agora o que a torna não confiável é a versão **faltar** — e o teste
    // afirma isso, que é o que sempre significou.
    let (_d, mut r) = repo();
    gravar_com_versao(&mut r, "01A", None);
    gravar_com_versao(&mut r, "01B", None);

    let c = respostas::comparar(&r, "01A", "01B").unwrap().unwrap();
    assert!(
        c["cercas_diferem_em"].as_array().unwrap().is_empty(),
        "as cercas são idênticas"
    );
    assert_eq!(
        c["confiavel"], false,
        "e ainda assim a comparação não se sustenta"
    );
    assert!(
        c["por_que_nao_confiavel"]
            .as_str()
            .unwrap()
            .contains("versão do código"),
        "a resposta diz por quê"
    );
}

// ---------------------------------------------------------------- T029

#[test]
fn toda_resposta_declara_de_qual_banco_veio() {
    // REQ-UI-028. `market.db` é cache reconstruível da fonte; `runs.db` é
    // auditoria insubstituível. Quem lê precisa saber o que pode ser
    // regenerado e o que não pode.
    let (_d, mut r) = repo();
    gravar(&mut r, "01A", "200.00", dec!(10));

    assert_eq!(respostas::execucoes(&r).unwrap()["origem"], "runs.db");
    assert_eq!(
        respostas::execucao(&r, "01A", &exigencias())
            .unwrap()
            .unwrap()["origem"],
        "runs.db"
    );
    let c = respostas::comparar(&r, "01A", "01A").unwrap().unwrap();
    assert_eq!(c["origem"], "runs.db");
}

#[test]
fn a_resposta_de_historico_diz_que_o_cache_e_reconstruivel() {
    let corpo = consultas::datasets_json(&[]);
    assert_eq!(corpo["origem"], "market.db");
    assert_eq!(
        corpo["reconstruivel"], true,
        "o histórico pode ser recoletado da fonte; a auditoria não"
    );
}

// ---------------------------------------------------------------- T028

#[test]
fn o_historico_traz_procedencia_cobertura_e_as_lacunas_como_lacunas() {
    // FR-015: jamais interpoladas. Interpolar inventa preço que não existiu,
    // e a estratégia decidiria sobre um mercado imaginário.
    let corpo = consultas::datasets_json(&[consultas::Dataset {
        simbolo: "BTCUSDT".into(),
        intervalo: "1m".into(),
        procedencia: "bybit-v5-spot".into(),
        primeira_ms: t(0).timestamp_millis(),
        ultima_ms: t(12).timestamp_millis(),
        velas: 720,
        coletado_em_ms: t(12).timestamp_millis(),
        lacunas: vec![(t(3).timestamp_millis(), t(4).timestamp_millis())],
    }]);
    let d = &corpo["datasets"][0];
    assert_eq!(d["procedencia"], "bybit-v5-spot");
    assert_eq!(d["velas"], 720);
    let lac = d["lacunas"].as_array().unwrap();
    assert_eq!(lac.len(), 1);
    assert_eq!(lac[0]["de_ms"], t(3).timestamp_millis());
    assert_eq!(d["tem_lacunas"], true);
}

#[test]
fn sem_lacuna_a_resposta_diz_que_nao_ha() {
    let corpo = consultas::datasets_json(&[consultas::Dataset {
        simbolo: "BTCUSDT".into(),
        intervalo: "1m".into(),
        procedencia: "bybit-v5-spot".into(),
        primeira_ms: 0,
        ultima_ms: 1,
        velas: 525_600,
        coletado_em_ms: 1,
        lacunas: vec![],
    }]);
    assert_eq!(corpo["datasets"][0]["tem_lacunas"], false);
    assert!(
        corpo["datasets"][0]["lacunas"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

// ---------------------------------------------------------------- T030

#[test]
fn o_aviso_de_repeticao_encontra_execucao_igual() {
    // Antes de iniciar: rodar de novo o que já rodou gasta tempo e enche o
    // registro de linhas que não acrescentam evidência.
    let (_d, mut r) = repo();
    gravar(&mut r, "01A", "200.00", dec!(10));

    let c = respostas::repetida(
        &r,
        &respostas::Pedido {
            modo: "backtest".into(),
            simbolo: "BTCUSDT".into(),
            intervalo: "1m".into(),
            de_ms: t(0).timestamp_millis(),
            ate_ms: t(12).timestamp_millis(),
        },
    )
    .unwrap();
    assert_eq!(c["ja_existe"], true);
    assert_eq!(c["execucoes"].as_array().unwrap().len(), 1);
}

#[test]
fn periodo_diferente_nao_e_repeticao() {
    let (_d, mut r) = repo();
    gravar(&mut r, "01A", "200.00", dec!(10));
    let c = respostas::repetida(
        &r,
        &respostas::Pedido {
            modo: "backtest".into(),
            simbolo: "BTCUSDT".into(),
            intervalo: "1m".into(),
            de_ms: t(0).timestamp_millis(),
            ate_ms: t(1).timestamp_millis(),
        },
    )
    .unwrap();
    assert_eq!(c["ja_existe"], false);
}

// ---------------------------------------------------------------- T031

#[test]
fn o_erro_de_historico_traz_o_trecho_e_o_comando_ja_preenchido() {
    // FR-022 e REQ-UI-032. Um erro que diz "histórico insuficiente" e para aí
    // obriga quem lê a descobrir sozinho o que coletar.
    let r = consultas::historico_insuficiente(
        "BTCUSDT",
        "1m",
        t(3).timestamp_millis(),
        t(9).timestamp_millis(),
    );
    let c = r.corpo();
    assert_eq!(c["erro"]["codigo"], "historico_insuficiente");
    assert_eq!(c["erro"]["o_que_falta"]["de_ms"], t(3).timestamp_millis());
    let cmd = c["erro"]["comando"].as_str().unwrap();
    assert!(cmd.starts_with("trade collect"), "veio {cmd}");
    assert!(cmd.contains("--symbol BTCUSDT"), "veio {cmd}");
    assert!(
        cmd.contains("--from 2026-01-01"),
        "as datas já preenchidas: {cmd}"
    );
    assert_eq!(r.status(), 409);
}

// ------------------ P6/P9: a comparação passa a poder se sustentar

/// Grava com uma versão de código declarada.
fn gravar_com_versao(r: &mut RunsRepository, run_id: &str, versao: Option<&str>) {
    use trade_domain::Trade;
    let sym = Symbol::new("BTCUSDT").unwrap();
    let lim = RiskLimits::default();
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
        code_version: versao,
    })
    .unwrap();
    let trades = vec![Trade {
        seq: 1,
        entry_at: t(1),
        entry_price: dec!(100),
        exit_at: t(2),
        exit_price: dec!(110),
        qty: dec!(1),
        fees: dec!(0.1),
        pnl: dec!(10),
    }];
    r.save_trades(run_id, &trades).unwrap();
    r.save_metrics(run_id, &RunMetrics::from_trades(&trades, dec!(0)))
        .unwrap();
    r.finish_run(run_id, t(12), "completed", None).unwrap();
}

#[test]
fn com_a_versao_no_registro_a_comparacao_passa_a_se_sustentar() {
    // É o que o FR-007 sempre previu: não confiável **enquanto** a versão não
    // estiver no registro. Ela está.
    let (_d, mut r) = repo();
    gravar_com_versao(&mut r, "01A", Some("abc123def456"));
    gravar_com_versao(&mut r, "01B", Some("abc123def456"));

    let c = respostas::comparar(&r, "01A", "01B").unwrap().unwrap();
    assert_eq!(c["a"]["versao_do_codigo"], "abc123def456");
    assert_eq!(c["confiavel"], true);
    assert!(c["por_que_nao_confiavel"].is_null());
}

#[test]
fn versoes_diferentes_nao_se_comparam() {
    // O caso real: duas execuções com a mesma cerca e resultado diferente,
    // separadas por um commit que mudou o arredondamento.
    let (_d, mut r) = repo();
    gravar_com_versao(&mut r, "01A", Some("abc123def456"));
    gravar_com_versao(&mut r, "01B", Some("999888777666"));

    let c = respostas::comparar(&r, "01A", "01B").unwrap().unwrap();
    assert_eq!(c["confiavel"], false);
    assert!(
        c["por_que_nao_confiavel"]
            .as_str()
            .unwrap()
            .contains("versões diferentes"),
        "veio {}",
        c["por_que_nao_confiavel"]
    );
}

#[test]
fn versao_de_arvore_suja_nunca_sustenta_comparacao() {
    // **O ponto que não é óbvio.** Duas execuções marcadas `abc-sujo` podem
    // ter rodado códigos diferentes: a marca existe justamente porque o
    // commit não identifica o que estava na árvore. Iguais no texto, e ainda
    // assim não comparáveis.
    let (_d, mut r) = repo();
    gravar_com_versao(&mut r, "01A", Some("abc123def456-sujo"));
    gravar_com_versao(&mut r, "01B", Some("abc123def456-sujo"));

    let c = respostas::comparar(&r, "01A", "01B").unwrap().unwrap();
    assert_eq!(c["confiavel"], false, "texto igual não é código igual");
    assert!(
        c["por_que_nao_confiavel"]
            .as_str()
            .unwrap()
            .contains("não commitada"),
        "veio {}",
        c["por_que_nao_confiavel"]
    );
}

#[test]
fn execucao_anterior_a_coluna_continua_nao_comparavel() {
    let (_d, mut r) = repo();
    gravar_com_versao(&mut r, "01A", None);
    gravar_com_versao(&mut r, "01B", Some("abc123def456"));

    let c = respostas::comparar(&r, "01A", "01B").unwrap().unwrap();
    assert_eq!(c["confiavel"], false);
    assert!(c["a"]["versao_do_codigo"].is_null());
    assert!(
        c["por_que_nao_confiavel"]
            .as_str()
            .unwrap()
            .contains("não guarda"),
        "veio {}",
        c["por_que_nao_confiavel"]
    );
}

#[test]
fn a_execucao_devolve_a_versao_que_a_produziu() {
    let (_d, mut r) = repo();
    gravar_com_versao(&mut r, "01A", Some("abc123def456"));
    let c = respostas::execucao(&r, "01A", &exigencias())
        .unwrap()
        .unwrap();
    assert_eq!(c["versao_do_codigo"], "abc123def456");
    assert_eq!(c["comparavel"], true);
}
