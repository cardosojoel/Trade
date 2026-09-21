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

use serde_json::{Value, json};
use trade_domain::Money;
use trade_ports::StorageError;
use trade_storage::runs_repo::{ExecucaoLida, MetricasLidas, RunsRepository};

/// Um valor monetário vira texto exato.
pub fn dinheiro(v: Money) -> String {
    v.to_string()
}

/// Por que a comparação entre execuções não se sustenta hoje.
///
/// Enquanto a tabela `run` não gravar a versão do código, duas execuções com a
/// mesma cerca e resultado diferente são indistinguíveis no registro — e foi
/// exatamente o que aconteceu: duas execuções rodaram minutos antes de um
/// commit que mudou o arredondamento da quantidade, e divergiram em 14.299 das
/// 14.308 operações. `FR-007`, `SC-010`.
const POR_QUE_NAO_COMPARAVEL: &str =
    "o registro não guarda a versão do código que produziu a execução";

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
            // Presente e nulo, nunca omitido (FR-006, REQ-UI-044).
            "versao_do_codigo": Value::Null,
            "comparavel": false,
            "por_que_nao_comparavel": POR_QUE_NAO_COMPARAVEL,
        }));
    }
    Ok(json!({ "grupos": saida, "origem": "runs.db" }))
}

/// `GET /runs/{id}` — uma execução, com a decomposição do custo.
pub fn execucao(repo: &RunsRepository, run_id: &str) -> Result<Option<Value>, StorageError> {
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
        "versao_do_codigo": Value::Null,
        "comparavel": false,
        "por_que_nao_comparavel": POR_QUE_NAO_COMPARAVEL,
        "origem": "runs.db",
    })))
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
    Ok(Some(json!({
        "a": lado(repo, &ea)?,
        "b": lado(repo, &eb)?,
        "cercas_diferem_em": diferencas(&ea, &eb),
        // **Sempre falso hoje**, e não por defeito da comparação: sem a
        // versão do código no registro, duas execuções com a mesma cerca e
        // resultado diferente são indistinguíveis. Foi o que aconteceu com
        // duas execuções que rodaram minutos antes e depois de um commit que
        // mudou o arredondamento da quantidade (FR-007, SC-010).
        "confiavel": false,
        "por_que_nao_confiavel": POR_QUE_NAO_COMPARAVEL,
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
        "versao_do_codigo": Value::Null,
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
