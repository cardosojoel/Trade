//! Os agregados, calculados aqui e em decimal exato (FR-008).
//!
//! Nada disso sai como número JSON: o servidor devolve string pronta. Mandar
//! o extrato bruto e deixar a tela somar seria empurrar aritmética monetária
//! para o navegador, onde todo número é ponto flutuante de 64 bits — e a soma
//! dos dias deixaria de bater com a soma das operações.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde_json::{Value, json};
use trade_domain::{Money, Trade};
use trade_storage::runs_repo::EventoLido;

use crate::erros::{Motivo, Recusa};
use crate::respostas::dinheiro;

/// O resultado de um dia.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dia {
    pub dia: NaiveDate,
    pub resultado: Money,
    /// A soma corrida até este dia, inclusive.
    pub acumulado: Money,
    pub operacoes: usize,
    /// Se o prejuízo do dia **atingiu** o limite diário.
    pub rompeu_o_limite: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PorDia {
    pub dias: Vec<Dia>,
    /// O de maior prejuízo. `None` quando nenhum dia fechou negativo —
    /// inventar um "pior dia" positivo mostraria prejuízo onde não houve.
    pub pior_dia: Option<Dia>,
}

/// Agrupa o extrato por dia de **fechamento**.
///
/// Pelo fechamento, e não pela abertura: uma posição que atravessa a virada
/// abre num dia e realiza noutro, e o resultado pertence ao dia em que foi
/// realizado — é assim que o contador de perda diária o conta.
pub fn por_dia(extrato: &[Trade], max_daily_loss: Money) -> PorDia {
    let mut por_data: std::collections::BTreeMap<NaiveDate, (Money, usize)> =
        std::collections::BTreeMap::new();
    for t in extrato {
        let e = por_data
            .entry(t.exit_at.date_naive())
            .or_insert((Decimal::ZERO, 0));
        e.0 += t.pnl;
        e.1 += 1;
    }

    let mut acumulado = Decimal::ZERO;
    let dias: Vec<Dia> = por_data
        .into_iter()
        .map(|(dia, (resultado, operacoes))| {
            acumulado += resultado;
            Dia {
                dia,
                resultado,
                acumulado,
                operacoes,
                // **Atingir**, não ultrapassar. É a fronteira invertida do
                // FR-019a, e a tela tem de mostrar a mesma que o freio usou.
                rompeu_o_limite: max_daily_loss > Decimal::ZERO && resultado <= -max_daily_loss,
            }
        })
        .collect();

    let pior_dia = dias
        .iter()
        .filter(|d| d.resultado < Decimal::ZERO)
        .min_by_key(|d| d.resultado)
        .cloned();

    PorDia { dias, pior_dia }
}

pub fn por_dia_json(p: &PorDia) -> Value {
    json!({
        "dias": p.dias.iter().map(|d| json!({
            "dia": d.dia.to_string(),
            "resultado": dinheiro(d.resultado),
            "acumulado": dinheiro(d.acumulado),
            "operacoes": d.operacoes,
            "rompeu_o_limite": d.rompeu_o_limite,
        })).collect::<Vec<_>>(),
        // Presente e nulo quando não houve dia negativo, nunca omitido.
        "pior_dia": p.pior_dia.as_ref().map(|d| json!({
            "dia": d.dia.to_string(),
            "resultado": dinheiro(d.resultado),
        })),
        "dias_que_romperam_o_limite": p.dias.iter().filter(|d| d.rompeu_o_limite).count(),
        "origem": "runs.db",
    })
}

/// As cinco exigências da Porta 1 (FR-024).
///
/// **Não há valores embutidos neste tipo, e é de propósito.** A constitution
/// manda tratá-los como valores de partida ajustáveis, e um `Default` com os
/// números de hoje viraria a constante que o requisito proíbe. Quem constrói
/// este tipo leu um arquivo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExigenciasPorta1 {
    pub meses_minimos: u32,
    pub operacoes_minimas: u64,
    pub profit_factor_minimo: Money,
    /// Fração do capital inicial, não valor absoluto.
    pub drawdown_maximo_fracao: Money,
}

/// O que a execução de fato mostrou.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservadoPorta1 {
    pub meses_de_historico: u32,
    pub operacoes: u64,
    /// Taxas e slippage presentes e discriminados.
    pub custo_modelado: bool,
    /// `None` quando indefinido — sem operação perdedora a razão não tem
    /// denominador.
    pub profit_factor: Option<Money>,
    pub max_drawdown: Money,
    pub capital_inicial: Money,
}

/// Confere as cinco, com o observado ao lado do exigido e o veredito.
///
/// O veredito diz que passou; o observado diz o quanto faltava (`REQ-UI-006`,
/// mesma regra do `REQ-UI-025`). Uma resposta que só dissesse "reprovado"
/// obrigaria quem lê a ir buscar por quê.
pub fn porta1(e: &ExigenciasPorta1, o: ObservadoPorta1) -> Value {
    let teto_drawdown = o.capital_inicial * e.drawdown_maximo_fracao;

    let exigencias = vec![
        exigencia(
            "cobertura de histórico",
            format!("≥ {} meses", e.meses_minimos),
            Value::String(format!("{} meses", o.meses_de_historico)),
            o.meses_de_historico >= e.meses_minimos,
        ),
        exigencia(
            "número de operações",
            format!("≥ {}", e.operacoes_minimas),
            json!(o.operacoes),
            o.operacoes >= e.operacoes_minimas,
        ),
        exigencia(
            "taxas e slippage modelados",
            "presentes e discriminados".to_string(),
            Value::Bool(o.custo_modelado),
            o.custo_modelado,
        ),
        exigencia(
            "profit factor",
            format!("≥ {}", dinheiro(e.profit_factor_minimo)),
            // Indefinido vem nulo. Não é aprovado por omissão: sem operação
            // perdedora não há evidência de que a estratégia sobrevive a uma.
            o.profit_factor
                .map_or(Value::Null, |v| Value::String(dinheiro(v))),
            o.profit_factor.is_some_and(|v| v >= e.profit_factor_minimo),
        ),
        exigencia(
            "drawdown máximo",
            format!(
                "≤ {} ({} do capital)",
                dinheiro(teto_drawdown),
                dinheiro(e.drawdown_maximo_fracao)
            ),
            Value::String(dinheiro(o.max_drawdown)),
            o.max_drawdown <= teto_drawdown,
        ),
    ];

    let passou = exigencias.iter().all(|x| x["passou"] == Value::Bool(true));
    json!({ "exigencias": exigencias, "passou": passou })
}

fn exigencia(nome: &str, exigido: String, observado: Value, passou: bool) -> Value {
    json!({ "exigencia": nome, "exigido": exigido, "observado": observado, "passou": passou })
}

// ------------------------------------------------------------ episódios

/// Um episódio de posição: abre uma vez, pode sair muitas.
///
/// A distinção não é acadêmica. Numa das execuções gravadas, 146 episódios
/// produziram 28.327 saídas — quem lê "28.327 operações" conclui atividade
/// duzentas vezes maior do que houve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Episodio {
    pub entrada: chrono::DateTime<chrono::Utc>,
    pub fechamento: chrono::DateTime<chrono::Utc>,
    pub duracao_horas: Money,
    /// `sinal`, `prazo`, ou `None` quando o registro não distingue — é o caso
    /// de tudo que foi gravado antes de a emenda 2.0.0 criar o prazo.
    pub fechado_por: Option<String>,
    pub saidas: usize,
    pub resultado: Money,
    pub taxas: Money,
}

/// Agrupa o extrato em episódios.
///
/// A chave é o **instante de entrada**: a posição guarda o instante em que
/// abriu e o mantém até ficar zerada, de modo que todas as saídas de um mesmo
/// episódio carregam a mesma entrada. Não é coincidência a ser explorada — é
/// o que `Position::apply_fill` faz, e o que torna o episódio reconstituível
/// sem replicar o fluxo inteiro.
pub fn episodios(extrato: &[Trade], fechamentos: &[EventoLido]) -> Vec<Episodio> {
    let mut por_entrada: std::collections::BTreeMap<
        chrono::DateTime<chrono::Utc>,
        (chrono::DateTime<chrono::Utc>, usize, Money, Money),
    > = std::collections::BTreeMap::new();

    for t in extrato {
        let e =
            por_entrada
                .entry(t.entry_at)
                .or_insert((t.exit_at, 0, Decimal::ZERO, Decimal::ZERO));
        e.0 = e.0.max(t.exit_at);
        e.1 += 1;
        e.2 += t.pnl;
        e.3 += t.fees;
    }

    por_entrada
        .into_iter()
        .map(
            |(entrada, (fechamento, saidas, resultado, taxas))| Episodio {
                entrada,
                fechamento,
                duracao_horas: horas(fechamento - entrada),
                fechado_por: causa_em(fechamentos, fechamento),
                saidas,
                resultado,
                taxas,
            },
        )
        .collect()
}

/// A causa gravada na transição que fechou a posição naquele instante.
fn causa_em(fechamentos: &[EventoLido], quando: chrono::DateTime<chrono::Utc>) -> Option<String> {
    fechamentos
        .iter()
        .filter(|e| e.kind == "state_transition" && e.payload["to"] == "Flat")
        .find(|e| e.at == quando)
        .and_then(|e| e.payload["fechado_por"].as_str())
        .map(str::to_string)
}

/// Duração em horas, exata.
///
/// Dos milissegundos, e não das horas inteiras: um episódio de 59,7 horas
/// arredondado para 59 ou 60 esconderia justamente o que interessa perto do
/// teto de 72.
fn horas(d: chrono::TimeDelta) -> Money {
    Decimal::from(d.num_milliseconds()) / Decimal::from(3_600_000)
}

pub fn episodios_json(run_id: &str, eps: &[Episodio]) -> Value {
    // O aviso existe porque nulo sozinho não distingue "ninguém sabe" de "o
    // registro é anterior ao campo". A segunda é dizível, e dizer é melhor.
    let algum_nulo = eps.iter().any(|e| e.fechado_por.is_none());
    json!({
        "execucao": run_id,
        "episodios": eps.iter().map(|e| json!({
            "entrada_ms": e.entrada.timestamp_millis(),
            "fechamento_ms": e.fechamento.timestamp_millis(),
            "duracao_horas": dinheiro(e.duracao_horas),
            "fechado_por": e.fechado_por,
            "saidas": e.saidas,
            "resultado": dinheiro(e.resultado),
            "taxas": dinheiro(e.taxas),
        })).collect::<Vec<_>>(),
        "total_de_episodios": eps.len(),
        "aviso": if algum_nulo {
            Value::String(
                "`fechado_por` é nulo onde o registro não distingue fechamento por sinal \
                 de fechamento por prazo — é o caso de tudo gravado antes da emenda 2.0.0"
                    .into(),
            )
        } else {
            Value::Null
        },
        "origem": "runs.db",
    })
}

/// As saídas de um episódio, com a taxa ao lado do resultado (FR-011).
pub fn saidas_json(
    run_id: &str,
    entrada: chrono::DateTime<chrono::Utc>,
    extrato: &[Trade],
    teto: usize,
) -> Value {
    let todas: Vec<&Trade> = extrato.iter().filter(|t| t.entry_at == entrada).collect();
    let cortou = todas.len() > teto;
    let mostradas = todas.iter().take(teto);
    json!({
        "execucao": run_id,
        "entrada_ms": entrada.timestamp_millis(),
        "saidas": mostradas.map(|t| json!({
            "seq": t.seq,
            "saida_ms": t.exit_at.timestamp_millis(),
            "preco_de_entrada": dinheiro(t.entry_price),
            "preco_de_saida": dinheiro(t.exit_price),
            "quantidade": dinheiro(t.qty),
            "resultado": dinheiro(t.pnl),
            // Ao lado, sempre: parte do resultado é custo, e sem a taxa a
            // linha não se explica (REQ-UI-030).
            "taxas": dinheiro(t.fees),
        })).collect::<Vec<_>>(),
        // Declarados sempre, e não só quando cortou: uma resposta que só
        // avisasse ao cortar faria a tela concluir, no silêncio, que não há
        // teto nenhum (FR-025).
        "teto": teto,
        "cortou": cortou,
        "total": todas.len(),
        "origem": "runs.db",
    })
}

// ---------------------------------------------------------------- cadeia

/// Os cinco elos de uma decisão, a partir de qualquer `seq` da cadeia.
///
/// Ordenados por `seq`, nunca por instante: os cinco acontecem dentro da
/// mesma vela, e o instante não os separa (FR-012).
pub fn cadeia(eventos: &[EventoLido], seq: u64) -> Option<Value> {
    const ELOS: [&str; 5] = [
        "signal",
        "order",
        "risk_decision",
        "fill",
        "state_transition",
    ];
    let alvo = eventos.iter().find(|e| e.seq == seq)?;

    // A cadeia é a vizinhança do alvo: o elo mais próximo de cada tipo, sem
    // pular para outra cadeia. Procura-se para trás a partir do alvo e para
    // frente a partir dele, o que mantém a cadeia inteira dentro da mesma
    // sequência de eventos.
    let mais_proximo = |tipo: &str| {
        eventos
            .iter()
            .filter(|e| e.kind == tipo)
            .min_by_key(|e| e.seq.abs_diff(alvo.seq))
    };

    // A quantidade **pedida** vive no elo `order`, não no `fill`: o
    // preenchimento diz o que veio, e só a ordem diz o que se pediu. Sem
    // cruzar os dois não há como assinalar divergência (FR-014).
    let pedido = mais_proximo("order").map(|o| o.payload["qty"].clone());

    let elos: Vec<Value> = ELOS
        .iter()
        .filter_map(|tipo| Some(elo(mais_proximo(tipo)?, pedido.as_ref())))
        .collect();

    Some(json!({
        "seq_pedido": seq,
        "elos": elos,
        // Cinco é fixo, e por isso esta rota não tem teto (FR-025).
        "elos_esperados": ELOS.len(),
        "origem": "runs.db",
    }))
}

fn elo(e: &EventoLido, pedido: Option<&Value>) -> Value {
    let mut v = json!({
        "tipo": e.kind,
        "seq": e.seq,
        "instante_ms": e.at.timestamp_millis(),
    });
    match e.kind.as_str() {
        "risk_decision" => {
            v["veredito"] = e.payload["verdict"].clone();
            v["limite_violado"] = e.payload["breach"].clone();
            v["limites"] = limites_com_observado(&e.payload);
        }
        "fill" => {
            let (divergente, proporcao) = divergencia(pedido, &e.payload["qty"]);
            v["pedido"] = pedido.cloned().unwrap_or(Value::Null);
            v["obtido"] = e.payload["qty"].clone();
            v["preco"] = e.payload["price"].clone();
            v["taxa"] = e.payload["fee"].clone();
            v["divergente"] = Value::Bool(divergente);
            v["proporcao"] = proporcao;
        }
        _ => {}
    }
    v["carga"] = e.payload.clone();
    v
}

/// Cada limite com o valor observado ao lado do teto (FR-013, `REQ-UI-025`).
fn limites_com_observado(p: &Value) -> Value {
    let l = &p["limits"];
    let s = &p["state"];
    json!([
        par(
            "max_daily_loss",
            l["max_daily_loss"].clone(),
            s["daily_pnl"].clone()
        ),
        par(
            "max_total_exposure",
            l["max_total_exposure"].clone(),
            s["exposure"].clone()
        ),
        par(
            "max_position_size",
            l["max_position_size"].clone(),
            s["exposure"].clone()
        ),
        par(
            "max_orders_per_window",
            l["max_orders_per_window"].clone(),
            s["orders_in_window"].clone()
        ),
    ])
}

fn par(limite: &str, teto: Value, observado: Value) -> Value {
    json!({ "limite": limite, "teto": teto, "observado": observado })
}

/// Quanto da ordem foi preenchido, e se divergiu (FR-014).
///
/// Na execução `01M2ZG2N88…` foram **quatro ordens em 28.618** preenchidas em
/// parte, e nenhum evento registra a causa. A interface mostra a divergência;
/// mostrar a causa ela não pode, porque a causa não está gravada — é o achado
/// 2 do DsTrade, pendência P3.
fn divergencia(pedido: Option<&Value>, obtido: &Value) -> (bool, Value) {
    use std::str::FromStr;
    let ler = |v: Option<&Value>| {
        v.and_then(Value::as_str)
            .and_then(|s| Decimal::from_str(s).ok())
            .filter(|d| *d > Decimal::ZERO)
    };
    match (ler(pedido), ler(Some(obtido))) {
        (Some(pedido), Some(obtido)) => {
            let prop = obtido / pedido;
            (obtido != pedido, Value::String(dinheiro(prop)))
        }
        // Sem o pedido no registro não dá para afirmar divergência. Dizer
        // `false` sem base seria afirmar que conferiu.
        _ => (false, Value::Null),
    }
}

// ------------------------------------------------------------- histórico

/// A cobertura de um par e granularidade no `market.db`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dataset {
    pub simbolo: String,
    pub intervalo: String,
    /// De onde veio. `FR-017` da feature 001: o histórico traz procedência.
    pub procedencia: String,
    pub primeira_ms: i64,
    pub ultima_ms: i64,
    pub velas: u64,
    pub coletado_em_ms: i64,
    /// Pares `(de_ms, ate_ms)`.
    pub lacunas: Vec<(i64, i64)>,
}

/// `GET /datasets` — cobertura, procedência e lacunas.
///
/// **As lacunas vão como lacunas, jamais interpoladas** (FR-015). Interpolar
/// inventa preço que não existiu, e a estratégia decidiria sobre um mercado
/// imaginário — é a mesma regra que o coletor segue ao gravar.
pub fn datasets_json(ds: &[Dataset]) -> Value {
    json!({
        "datasets": ds.iter().map(|d| json!({
            "simbolo": d.simbolo,
            "intervalo": d.intervalo,
            "procedencia": d.procedencia,
            "cobertura": { "de_ms": d.primeira_ms, "ate_ms": d.ultima_ms },
            "velas": d.velas,
            "coletado_em_ms": d.coletado_em_ms,
            "lacunas": d.lacunas.iter().map(|(de, ate)| json!({
                "de_ms": de, "ate_ms": ate,
            })).collect::<Vec<_>>(),
            "tem_lacunas": !d.lacunas.is_empty(),
        })).collect::<Vec<_>>(),
        "origem": "market.db",
        // REQ-UI-028: o cache se reconstrói da fonte, a auditoria não. Quem
        // lê precisa saber o que pode ser regenerado antes de decidir apagar.
        "reconstruivel": true,
    })
}

/// A recusa por histórico insuficiente, com o comando que resolve.
///
/// FR-022 e `REQ-UI-032`. Um erro que diz "histórico insuficiente" e para aí
/// obriga quem lê a descobrir sozinho o que coletar — e quem está na tela não
/// tem o `runs.db` aberto ao lado para ir ver.
pub fn historico_insuficiente(simbolo: &str, intervalo: &str, de_ms: i64, ate_ms: i64) -> Recusa {
    let dia = |ms: i64| {
        chrono::DateTime::from_timestamp_millis(ms)
            .map_or_else(|| "?".to_string(), |d| d.date_naive().to_string())
    };
    Recusa::nova(
        Motivo::HistoricoInsuficiente,
        format!(
            "faltam velas de {simbolo} {intervalo} entre {} e {}",
            dia(de_ms),
            dia(ate_ms)
        ),
    )
    .com_o_que_falta(json!({ "de_ms": de_ms, "ate_ms": ate_ms }))
    .com_comando(format!(
        "trade collect --symbol {simbolo} --interval {intervalo} --from {} --to {}",
        dia(de_ms),
        dia(ate_ms)
    ))
}
