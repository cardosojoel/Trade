//! As respostas, e a regra que vale para todas: **dinheiro é string**.
//!
//! O `FR-002` proíbe número JSON em campo que alcance saldo, posição, P&L,
//! risco ou ordem, e o `SC-002` cobra isso varrendo toda resposta. Contagem e
//! identificador seguem número — transformar tudo em string faria a tela
//! converter de volta, e é na conversão de volta que o erro entra.
//!
//! A garantia não é de disciplina. O workspace declara `rust_decimal` **sem**
//! a feature `serde`, de modo que um `Decimal` não tem `Serialize`: esquecer a
//! conversão **não compila**. Ligar a feature o faria serializar como número,
//! que é exatamente o que se quer proibir.

use crate::consultas::{ExigenciasPorta1, ObservadoPorta1, porta1};
use serde_json::{Value, json};
use trade_domain::Money;
use trade_ports::StorageError;
use trade_storage::runs_repo::{ExecucaoLida, MetricasLidas, RunsRepository};

/// Um valor monetário vira texto exato.
pub fn dinheiro(v: Money) -> String {
    v.to_string()
}

/// Se a comparação entre duas execuções se sustenta, e por que não quando não.
///
/// O `FR-007` sempre disse "não confiável **enquanto** a versão do código não
/// estiver no registro". Desde a implementação de P6/P9 ela está — e a
/// resposta passou a poder ser sim.
///
/// **A marca de árvore suja não se compara com nada, nem consigo mesma.** Duas
/// execuções marcadas `abc123-sujo` podem ter rodado códigos diferentes: a
/// marca existe justamente porque o commit não identifica o que estava na
/// árvore. Texto igual não é código igual, e tratar como igual seria repetir o
/// defeito original com uma camada a mais de confiança.
fn comparabilidade(a: Option<&str>, b: Option<&str>) -> (bool, Option<String>) {
    match (a, b) {
        (None, _) | (_, None) => (
            false,
            Some("o registro não guarda a versão do código de ao menos uma das execuções".into()),
        ),
        (Some(x), Some(y)) if x.ends_with(SUJO) || y.ends_with(SUJO) => (
            false,
            Some(
                "ao menos uma execução rodou a partir de árvore com alteração não \
                 commitada: o identificador não descreve o código que rodou, e duas \
                 marcas iguais podem ser códigos diferentes"
                    .into(),
            ),
        ),
        (Some(x), Some(y)) if x != y => (
            false,
            Some(format!(
                "as execuções rodaram versões diferentes do código: {x} e {y}"
            )),
        ),
        (Some(_), Some(_)) => (true, None),
    }
}

/// Sufixo que o `build.rs` acrescenta quando a árvore não estava limpa.
const SUJO: &str = "-sujo";

/// Uma execução sozinha é comparável quando o registro sabe qual código a
/// produziu — e árvore suja não sabe.
fn comparavel_sozinha(v: Option<&str>) -> (bool, Option<String>) {
    comparabilidade(v, v)
}

/// `GET /runs` — os grupos de execuções.
///
/// Agrupa o que é **idêntico**. Nas nove execuções gravadas até hoje, oito
/// eram a mesma coisa; listá-las lado a lado esconde isso de quem lê, e faz
/// parecer que há mais evidência do que há.
pub fn execucoes(repo: &RunsRepository) -> Result<Value, StorageError> {
    let todas = repo.listar_execucoes()?;

    // Chave de agrupamento **estável**: tudo que define a execução, mais as
    // métricas. A cerca entra porque duas execuções sob limites diferentes
    // não são a mesma coisa, ainda que deem o mesmo número.
    let mut ordem: Vec<String> = Vec::new();
    let mut grupos: std::collections::BTreeMap<String, Vec<ExecucaoLida>> =
        std::collections::BTreeMap::new();

    for e in todas {
        let m = repo.metricas(&e.run_id)?;
        let chave = format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}",
            e.mode,
            e.symbol,
            e.interval,
            e.from.timestamp_millis(),
            e.to.timestamp_millis(),
            e.outcome.as_deref().unwrap_or(""),
            e.limits_json,
            e.fees_json,
            m.as_ref().map_or_else(String::new, resumo_de_metricas),
        );
        if !grupos.contains_key(&chave) {
            ordem.push(chave.clone());
        }
        grupos.entry(chave).or_default().push(e);
    }

    let mut saida = Vec::new();
    for chave in ordem {
        let membros = &grupos[&chave];
        let primeira = &membros[0];
        let metricas = repo.metricas(&primeira.run_id)?;
        saida.push(json!({
            "identicas": membros.len(),
            "execucoes": membros.iter().map(|e| e.run_id.clone()).collect::<Vec<_>>(),
            "modo": primeira.mode,
            "simbolo": primeira.symbol,
            "intervalo": primeira.interval,
            "periodo": { "de_ms": primeira.from.timestamp_millis(),
                         "ate_ms": primeira.to.timestamp_millis() },
            "desfecho": primeira.outcome,
            "periodo_completado": primeira.outcome.as_deref() == Some("completed"),
            "metricas": metricas.as_ref().map(metricas_json),
            "cerca": { "limits": cru(&primeira.limits_json),
                       "fees": cru(&primeira.fees_json) },
            // Presente e nulo quando o registro não sabe, nunca omitido
            // (FR-006, REQ-UI-044).
            "versao_do_codigo": primeira.code_version,
            "comparavel": comparavel_sozinha(primeira.code_version.as_deref()).0,
            "por_que_nao_comparavel": comparavel_sozinha(primeira.code_version.as_deref()).1,
        }));
    }
    Ok(json!({ "grupos": saida, "origem": "runs.db" }))
}

/// `GET /runs/{id}` — uma execução, a decomposição do custo e o estado da
/// Porta 1.
///
/// As exigências chegam de fora porque são **configuração**, não constante: a
/// constitution manda tratá-las como valores de partida ajustáveis, e o
/// `FR-024` proíbe o servidor lê-las de dentro do código.
pub fn execucao(
    repo: &RunsRepository,
    run_id: &str,
    exigencias: &ExigenciasPorta1,
) -> Result<Option<Value>, StorageError> {
    let Some(e) = repo.execucao(run_id)? else {
        return Ok(None);
    };
    let m = repo.metricas(run_id)?;
    Ok(Some(json!({
        "run_id": e.run_id,
        "modo": e.mode,
        "simbolo": e.symbol,
        "intervalo": e.interval,
        "periodo": { "de_ms": e.from.timestamp_millis(), "ate_ms": e.to.timestamp_millis() },
        "capital_inicial": dinheiro(e.initial_capital),
        "estrategia": e.strategy,
        "desfecho": e.outcome,
        "motivo_da_parada": e.halt_reason,
        "cerca": { "limits": cru(&e.limits_json), "fees": cru(&e.fees_json) },
        "metricas": m.as_ref().map(metricas_json),
        // Discriminado, nunca embutido no resultado: é o que torna
        // respondível quanto do prejuízo foi custo de transação.
        "custo": m.as_ref().map(|x| json!({
            "total_fees": dinheiro(x.total_fees),
            "total_slippage": dinheiro(x.total_slippage),
        })),
        // FR-024: devolvidas **calculadas**, com o observado ao lado do
        // exigido e o veredito de cada uma.
        "porta_1": m.as_ref().map(|x| porta1(exigencias, ObservadoPorta1 {
            meses_de_historico: meses(e.from, e.to),
            operacoes: x.trade_count,
            // Taxa e slippage discriminados na cerca gravada: é o que a
            // terceira exigência pede, e está no `fees_json` da execução.
            custo_modelado: cru(&e.fees_json).get("taker_fee_rate").is_some()
                && cru(&e.fees_json).get("slippage_rate").is_some(),
            profit_factor: x.profit_factor,
            max_drawdown: x.max_drawdown,
            capital_inicial: e.initial_capital,
        })),
        "versao_do_codigo": e.code_version,
        "comparavel": comparavel_sozinha(e.code_version.as_deref()).0,
        "por_que_nao_comparavel": comparavel_sozinha(e.code_version.as_deref()).1,
        "origem": "runs.db",
    })))
}

/// Meses corridos de cobertura, arredondados para baixo.
///
/// Para baixo de propósito: onze meses e vinte e nove dias **não são** doze
/// meses, e a primeira exigência da Porta 1 é cobertura mínima.
fn meses(de: chrono::DateTime<chrono::Utc>, ate: chrono::DateTime<chrono::Utc>) -> u32 {
    let dias = ate.signed_duration_since(de).num_days().max(0);
    u32::try_from(dias / 30).unwrap_or(0)
}

fn metricas_json(m: &MetricasLidas) -> Value {
    json!({
        // Nulo com o motivo ao lado, nunca zero nem infinito (FR-021). Sem
        // operação perdedora o fator **é** indefinido, e inventar um número
        // induziria erro na Porta 1 de promoção.
        "profit_factor": m.profit_factor.map(dinheiro),
        "profit_factor_indefinido_porque": if m.profit_factor.is_none() {
            Value::String("não houve operação perdedora: a razão não tem denominador".into())
        } else {
            Value::Null
        },
        "max_drawdown": dinheiro(m.max_drawdown),
        "trade_count": m.trade_count,
        "net_result": dinheiro(m.net_result),
        "gross_profit": dinheiro(m.gross_profit),
        "gross_loss": dinheiro(m.gross_loss),
        "total_fees": dinheiro(m.total_fees),
        "total_slippage": dinheiro(m.total_slippage),
    })
}

fn resumo_de_metricas(m: &MetricasLidas) -> String {
    format!(
        "{}|{}|{}",
        dinheiro(m.net_result),
        dinheiro(m.max_drawdown),
        m.trade_count
    )
}

/// O JSON da cerca vem cru do registro.
///
/// Reinterpretá-lo aqui obrigaria a conhecer a forma de `RiskLimits` em toda
/// versão que já foi gravada — e o registro é insubstituível: tem linha de
/// ontem, e vai ter de amanhã. Se não for JSON válido, vai como texto, porque
/// dizer o que está lá é mais útil que esconder.
fn cru(s: &str) -> Value {
    serde_json::from_str(s).unwrap_or_else(|_| Value::String(s.to_string()))
}

/// `GET /runs/compare` — as duas cercas lado a lado.
///
/// A rota existe para **comparar**, não para mostrar duas coisas: além dos
/// dois lados, ela diz em que diferem, e se a comparação se sustenta.
pub fn comparar(repo: &RunsRepository, a: &str, b: &str) -> Result<Option<Value>, StorageError> {
    let (Some(ea), Some(eb)) = (repo.execucao(a)?, repo.execucao(b)?) else {
        return Ok(None);
    };
    let (confiavel, motivo) =
        comparabilidade(ea.code_version.as_deref(), eb.code_version.as_deref());
    Ok(Some(json!({
        "a": lado(repo, &ea)?,
        "b": lado(repo, &eb)?,
        "cercas_diferem_em": diferencas(&ea, &eb),
        // FR-007 e SC-010. Duas execuções com a mesma cerca e resultado
        // diferente foram indistinguíveis por não gravarem a versão — e o que
        // as separava era um commit que mudou o arredondamento da quantidade.
        "confiavel": confiavel,
        "por_que_nao_confiavel": motivo,
        "origem": "runs.db",
    })))
}

fn lado(repo: &RunsRepository, e: &ExecucaoLida) -> Result<Value, StorageError> {
    let m = repo.metricas(&e.run_id)?;
    Ok(json!({
        "run_id": e.run_id,
        "periodo": { "de_ms": e.from.timestamp_millis(), "ate_ms": e.to.timestamp_millis() },
        "estrategia": e.strategy,
        "cerca": { "limits": cru(&e.limits_json), "fees": cru(&e.fees_json) },
        "metricas": m.as_ref().map(metricas_json),
        "versao_do_codigo": e.code_version,
    }))
}

/// Em que as duas cercas diferem, campo a campo.
fn diferencas(a: &ExecucaoLida, b: &ExecucaoLida) -> Vec<String> {
    let mut out = Vec::new();
    for (rotulo, x, y) in [
        ("limits", &a.limits_json, &b.limits_json),
        ("fees", &a.fees_json, &b.fees_json),
    ] {
        let (va, vb) = (cru(x), cru(y));
        match (va.as_object(), vb.as_object()) {
            (Some(oa), Some(ob)) => {
                for (k, v) in oa {
                    if ob.get(k) != Some(v) {
                        out.push(k.clone());
                    }
                }
                for k in ob.keys() {
                    if !oa.contains_key(k) {
                        out.push(k.clone());
                    }
                }
            }
            // Cerca ilegível de um dos lados: dizer o rótulo é mais honesto
            // que afirmar que são iguais.
            _ if va != vb => out.push(rotulo.to_string()),
            _ => {}
        }
    }
    out.sort();
    out.dedup();
    out
}

/// O que se quer iniciar, para o aviso de repetição.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pedido {
    pub modo: String,
    pub simbolo: String,
    pub intervalo: String,
    pub de_ms: i64,
    pub ate_ms: i64,
}

/// `GET /runs/match` — já rodou isso antes?
///
/// Antes de iniciar, e não depois: refazer o que já foi feito gasta tempo e
/// enche o registro de linhas que não acrescentam evidência.
pub fn repetida(repo: &RunsRepository, p: &Pedido) -> Result<Value, StorageError> {
    let iguais: Vec<String> = repo
        .listar_execucoes()?
        .into_iter()
        .filter(|e| {
            e.mode == p.modo
                && e.symbol == p.simbolo
                && e.interval == p.intervalo
                && e.from.timestamp_millis() == p.de_ms
                && e.to.timestamp_millis() == p.ate_ms
        })
        .map(|e| e.run_id)
        .collect();

    Ok(json!({
        "ja_existe": !iguais.is_empty(),
        "execucoes": iguais,
        "origem": "runs.db",
    }))
}
