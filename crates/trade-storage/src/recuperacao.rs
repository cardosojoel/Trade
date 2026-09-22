//! O `runs.db` como fonte de recuperação, e não só de auditoria (FR-112).
//!
//! Reiniciar não recomeça do zero: o estado volta do registro. Como ele volta
//! é a **decisão 030** do Jev — a posição se reconstitui replicando os
//! preenchimentos (`replicar_os_preenchimentos`, 1,00 · confiança 1,00), e a
//! linha de base do dia se deduz do último retrato de risco
//! (`deduzir_a_linha_de_base`, 0,59 · confiança 0,45, contra 0,23 de retomar
//! do retrato sem deduzir).
//!
//! **Por que deduzir.** O evento `risk_decision` grava o resultado do dia já
//! somado, e não as três parcelas que o compõem. A remarcação a mercado não
//! gera evento, e a virada do dia só gera evento quando havia bloqueio a
//! soltar. Retomar o total e passar a medir a variação a partir do reinício
//! deixaria a perda ocorrida durante a parada fora do limite daquele dia — um
//! limite contornado por acidente, que é o que o Princípio II não admite.

use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use rusqlite::Connection;
use rust_decimal::Decimal;
use serde_json::Value;
use std::collections::BTreeMap;
use std::str::FromStr;
use trade_domain::{EstadoRetomado, Fill, Money, OrderId, Position, Side};
use trade_ports::StorageError;

/// Lê um valor monetário gravado como string. Número em JSON é IEEE-754 na
/// maioria dos leitores, e é por isso que nada monetário foi gravado assim.
fn dinheiro(v: &Value, campo: &str) -> Result<Money, StorageError> {
    let s = v
        .get(campo)
        .and_then(Value::as_str)
        .ok_or_else(|| StorageError::Query(format!("campo `{campo}` ausente ou não textual")))?;
    Decimal::from_str(s).map_err(|e| StorageError::Query(format!("`{campo}` = {s:?}: {e}")))
}

fn inteiro(v: &Value, campo: &str) -> Result<u64, StorageError> {
    v.get(campo)
        .and_then(Value::as_u64)
        .ok_or_else(|| StorageError::Query(format!("campo `{campo}` ausente ou não numérico")))
}

fn instante(ms: i64) -> Result<DateTime<Utc>, StorageError> {
    Utc.timestamp_millis_opt(ms)
        .single()
        .ok_or_else(|| StorageError::Query(format!("instante inválido: {ms}")))
}

/// O último retrato de risco e o que a posição era naquele ponto do fluxo.
struct Retrato {
    daily_pnl: Money,
    exposure: Money,
    daily_loss_blocked: bool,
    day: NaiveDate,
    /// A posição como estava **no instante do retrato** — o fluxo continua
    /// depois dele, porque o preenchimento é registrado após a decisão que o
    /// autorizou.
    position: Position,
    /// O realizado do dia acumulado até o retrato, também anterior ao que
    /// venha depois.
    realizado_no_dia: Money,
}

/// Reconstitui o estado de uma execução a partir dos eventos gravados.
///
/// Devolve `None` quando a execução não tem evento nenhum: sem registro não há
/// o que retomar, e devolver um estado zerado seria afirmar que a sessão
/// começou do nada — que é exatamente o que não se sabe.
///
/// `agora` e `window_minutes` recortam a janela deslizante de ordens: só os
/// instantes que ainda estão dentro dela voltam.
pub fn recuperar(
    conn: &Connection,
    run_id: &str,
    agora: DateTime<Utc>,
    window_minutes: i64,
) -> Result<Option<EstadoRetomado>, StorageError> {
    let mut stmt = conn
        .prepare(
            "SELECT seq, at_ms, kind, payload_json FROM audit_event \
             WHERE run_id = ?1 ORDER BY seq",
        )
        .map_err(|e| StorageError::Query(e.to_string()))?;

    let linhas = stmt
        .query_map([run_id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| StorageError::Query(e.to_string()))?;

    let mut position = Position::default();
    let mut lados: BTreeMap<u64, Side> = BTreeMap::new();
    let mut realizado_por_dia: BTreeMap<NaiveDate, Money> = BTreeMap::new();
    let mut aceitas: Vec<DateTime<Utc>> = Vec::new();
    let mut ultimo_retrato: Option<Retrato> = None;
    let mut ultimo_preco: Option<Money> = None;
    let mut maior_seq = 0i64;
    let mut houve_evento = false;

    let corte = agora - Duration::minutes(window_minutes);

    for linha in linhas {
        let (seq, at_ms, kind, payload) = linha.map_err(|e| StorageError::Query(e.to_string()))?;
        houve_evento = true;
        maior_seq = maior_seq.max(seq);
        let at = instante(at_ms)?;
        let v: Value = serde_json::from_str(&payload)
            .map_err(|e| StorageError::Query(format!("payload do seq {seq}: {e}")))?;

        match kind.as_str() {
            // O lado não viaja no preenchimento: é a ordem que o carrega.
            "order" => {
                let id = inteiro(&v, "order_id")?;
                let lado = match v.get("side").and_then(Value::as_str) {
                    Some("Buy") => Side::Buy,
                    Some("Sell") => Side::Sell,
                    outro => {
                        return Err(StorageError::Query(format!(
                            "lado desconhecido no seq {seq}: {outro:?}"
                        )));
                    }
                };
                lados.insert(id, lado);
            }

            "fill" => {
                let referencia = inteiro(&v, "order_ref")?;
                let lado = *lados.get(&referencia).ok_or_else(|| {
                    StorageError::Query(format!(
                        "preenchimento do seq {seq} referencia a ordem {referencia}, que não está no registro"
                    ))
                })?;
                let fill = Fill {
                    order_ref: OrderId(referencia),
                    price: dinheiro(&v, "price")?,
                    qty: dinheiro(&v, "qty")?,
                    fee: dinheiro(&v, "fee")?,
                    fee_base: dinheiro(&v, "fee_base")?,
                    slippage: dinheiro(&v, "slippage")?,
                    at,
                    causa_parcial: None,
                };
                ultimo_preco = Some(fill.price);
                // Pelo mesmo `apply_fill` que os produziu: a posição retomada
                // é a mesma que a original, e não uma segunda aritmética que
                // pudesse divergir dela.
                let operacao = position
                    .apply_fill(lado, &fill)
                    .map_err(|e| StorageError::Query(format!("replay do seq {seq}: {e}")))?;
                if let Some(t) = operacao {
                    *realizado_por_dia
                        .entry(t.exit_at.date_naive())
                        .or_insert(Decimal::ZERO) += t.pnl;
                }
            }

            "risk_decision" => {
                let dia = at.date_naive();
                if v.get("verdict").and_then(Value::as_str) == Some("Accepted") {
                    // Aceitas, não submetidas: o limite delimita atividade no
                    // mercado, e ordem recusada nunca chegou lá.
                    if at > corte {
                        aceitas.push(at);
                    }
                }
                let estado = v
                    .get("state")
                    .ok_or_else(|| StorageError::Query(format!("seq {seq} sem `state`")))?;
                let exposure = dinheiro(estado, "exposure")?;
                // A exposição gravada dividida pela quantidade de então
                // devolve o preço de referência daquele instante — é o único
                // lugar onde ele sobreviveu, e é mais recente que o preço do
                // último preenchimento.
                if !position.qty().is_zero() {
                    ultimo_preco = Some(exposure / position.qty());
                }
                ultimo_retrato = Some(Retrato {
                    daily_pnl: dinheiro(estado, "daily_pnl")?,
                    exposure,
                    daily_loss_blocked: estado
                        .get("daily_loss_blocked")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    day: dia,
                    position: position.clone(),
                    realizado_no_dia: realizado_por_dia
                        .get(&dia)
                        .copied()
                        .unwrap_or(Decimal::ZERO),
                });
            }

            _ => {}
        }
    }

    if !houve_evento {
        return Ok(None);
    }

    let proximo_seq = u64::try_from(maior_seq).unwrap_or(0) + 1;

    let Some(r) = ultimo_retrato else {
        // Eventos sem retrato de risco nenhum: há posição a retomar, e
        // contador de dia não há. SC-002 torna o caso improvável — toda ordem
        // atravessa uma decisão registrada —, e mesmo assim ele não pode
        // inventar um dia corrente.
        return Ok(Some(EstadoRetomado {
            position,
            proximo_seq,
            realized_today: Decimal::ZERO,
            unrealized: Decimal::ZERO,
            unrealized_at_day_start: Decimal::ZERO,
            daily_loss_blocked: false,
            day: None,
            aceitas,
        }));
    };

    // O aberto no instante do retrato, ao preço que a exposição gravada
    // revela.
    let aberto_no_retrato = if r.position.qty().is_zero() {
        Decimal::ZERO
    } else {
        r.position.unrealized_at(r.exposure / r.position.qty())
    };

    // A linha de base por diferença: das três parcelas do resultado do dia,
    // duas se reconstituem e a terceira sai da identidade que as liga.
    //
    //   daily_pnl = realizado_no_dia + (aberto − linha_de_base)
    let unrealized_at_day_start = r.realizado_no_dia + aberto_no_retrato - r.daily_pnl;

    // O aberto corrente é o da posição **ao fim do fluxo**, ao preço mais
    // recente que o registro conhece — o do último retrato, ou o do último
    // preenchimento quando ele vier depois. O laço remarca na primeira vela;
    // até lá, é o melhor que o registro sustenta.
    let unrealized = match ultimo_preco {
        Some(p) if !position.qty().is_zero() => position.unrealized_at(p),
        _ => Decimal::ZERO,
    };

    Ok(Some(EstadoRetomado {
        position,
        proximo_seq,
        realized_today: realizado_por_dia
            .get(&r.day)
            .copied()
            .unwrap_or(Decimal::ZERO),
        unrealized,
        unrealized_at_day_start,
        daily_loss_blocked: r.daily_loss_blocked,
        day: Some(r.day),
        aceitas,
    }))
}
