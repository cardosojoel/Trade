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
