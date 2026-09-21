//! Evento de auditoria.
//!
//! Esquema em `specs/001-nucleo-execucao/contracts/audit-event.md`.
//!
//! **Nenhum valor monetário é serializado como número JSON.** Número em JSON é
//! IEEE-754 de 64 bits na maioria dos leitores, e um `Decimal` que passasse por
//! ele perderia exatidão no caminho para o disco — exatamente o erro que
//! `rust_decimal` existe para evitar. Todo dinheiro vira **string**.

use crate::mode::ExecutionMode;
use crate::position::Position;
use crate::risk_types::{Anomaly, RiskDecision};
use crate::types::{CausaDoFechamento, Fill, Order, Signal};
use chrono::{DateTime, Utc};

/// Envelope comum a todo evento (FR-033).
#[derive(Debug, Clone)]
pub struct AuditEvent {
    pub run_id: String,
    /// Ordem **total** dentro da execução, começando em 1.
    ///
    /// É o que permite reconstituir a cadeia sinal → ordem → decisão →
    /// preenchimento mesmo entre eventos do mesmo instante simulado. Sem ele,
    /// dois eventos na mesma vela ficariam sem ordem definida.
    pub seq: u64,
    /// Instante **simulado**, vindo do `Clock` — nunca o relógio da máquina.
    pub at: DateTime<Utc>,
    pub mode: ExecutionMode,
    pub kind: AuditKind,
}

#[derive(Debug, Clone)]
pub enum AuditKind {
    Signal(Signal),
    Order(Order),
    RiskDecision(RiskDecision),
    Fill(Fill),
    Halt {
        reason: String,
        anomaly: Anomaly,
    },
    Resume {
        trigger: String,
        automatic: bool,
    },
    Anomaly {
        anomaly: Anomaly,
        recovered: bool,
    },
    StateTransition {
        from: String,
        to: String,
        position: Position,
        /// O que encerrou o episódio, quando a transição o encerra.
        ///
        /// `None` numa transição que **abre** ou aumenta posição: não há
        /// fechamento a explicar. Presente e nulo, nunca omitido — é a regra
        /// que o `REQ-UI-044` fixou e o `FR-006` levou ao protocolo.
        fechado_por: Option<CausaDoFechamento>,
    },
}

impl AuditKind {
    /// Discriminante usado na coluna `kind` da tabela `audit_event`.
    pub const fn as_str(&self) -> &'static str {
        match self {
            AuditKind::Signal(_) => "signal",
            AuditKind::Order(_) => "order",
            AuditKind::RiskDecision(_) => "risk_decision",
            AuditKind::Fill(_) => "fill",
            AuditKind::Halt { .. } => "halt",
            AuditKind::Resume { .. } => "resume",
            AuditKind::Anomaly { .. } => "anomaly",
            AuditKind::StateTransition { .. } => "state_transition",
        }
    }
}

use serde_json::{Value, json};

fn d(v: crate::Money) -> String {
    v.to_string()
}

impl AuditKind {
    /// Carga do evento, conforme `contracts/audit-event.md`.
    pub fn payload(&self) -> Value {
        match self {
            AuditKind::Signal(s) => json!({
                "signal_id": s.id.0,
                "intent": match s.intent {
                    crate::Intent::Buy => "Buy",
                    crate::Intent::Sell => "Sell",
                    crate::Intent::Hold => "Hold",
                },
                "qty": s.qty.map(d),
                // Os dados que produziram o sinal. Sem eles o registro diz o
                // que o robô fez e não por quê — e "por quê" é a pergunta que
                // se faz quando algo dá errado (FR-034).
                "inputs": {
                    "values": s.inputs.values,
                    "params": s.inputs.params,
                },
            }),

            AuditKind::Order(o) => json!({
                "order_id": o.id.0,
                "signal_ref": o.signal_ref.0,
                "side": o.side.as_str(),
                "qty": d(o.qty),
            }),

            AuditKind::RiskDecision(dec) => {
                let l = &dec.limits;
                let st = &dec.state;
                json!({
                    "order_ref": dec.order_ref.0,
                    "verdict": dec.verdict.as_str(),
                    "breach": match dec.verdict {
                        crate::Verdict::Rejected(b) => Some(b.as_str()),
                        crate::Verdict::Accepted => None,
                    },
                    // Retratos do momento da avaliação. Sem eles, uma recusa
                    // cujo limite mudou depois vira registro indecifrável:
                    // sabe-se que houve recusa, não contra o quê.
                    "limits": {
                        "max_daily_loss": d(l.max_daily_loss),
                        "max_position_size": d(l.max_position_size),
                        "max_total_exposure": d(l.max_total_exposure),
                        "max_orders_per_window": l.max_orders_per_window,
                        "window_minutes": l.window_minutes,
                        "max_transient_retries": l.max_transient_retries,
                        "max_price_deviation_ratio": d(l.max_price_deviation_ratio),
                    },
                    "state": {
                        "daily_pnl": d(st.daily_pnl),
                        "exposure": d(st.exposure),
                        "orders_in_window": st.orders_in_window,
                        "daily_loss_blocked": st.daily_loss_blocked,
                    },
                })
            }

            AuditKind::Fill(f) => json!({
                "order_ref": f.order_ref.0,
                "price": d(f.price),
                "qty": d(f.qty),
                // Discriminados, nunca embutidos no preço (FR-027). A taxa
                // aparece nas duas moedas porque no spot a compra paga em
                // moeda base e a venda em caixa: somar as duas num campo só
                // tornaria o extrato irreconciliável com a corretora.
                "fee": d(f.fee),
                "fee_base": d(f.fee_base),
                "slippage": d(f.slippage),
            }),

            AuditKind::Halt { reason, anomaly } => json!({
                "reason": reason,
                "classification": anomaly.classification(),
                "cause": match anomaly {
                    Anomaly::Integrity(c) => Some(c.as_str()),
                    Anomaly::Transient { .. } => None,
                },
                "attempts": match anomaly {
                    Anomaly::Transient { attempt, .. } => Some(*attempt),
                    Anomaly::Integrity(_) => None,
                },
                // É este campo que permite conferir SC-015 por consulta:
                // nenhum resume automático pode suceder um halt que o exige.
                "requires_human": anomaly.requires_human(),
            }),

            AuditKind::Resume { trigger, automatic } => json!({
                "trigger": trigger,
                "automatic": automatic,
            }),

            AuditKind::Anomaly { anomaly, recovered } => json!({
                "classification": anomaly.classification(),
                "cause": match anomaly {
                    Anomaly::Integrity(c) => Some(c.as_str()),
                    Anomaly::Transient { .. } => None,
                },
                "attempt": match anomaly {
                    Anomaly::Transient { attempt, .. } => Some(*attempt),
                    Anomaly::Integrity(_) => None,
                },
                "max_retries": match anomaly {
                    Anomaly::Transient { max_retries, .. } => Some(*max_retries),
                    Anomaly::Integrity(_) => None,
                },
                "recovered": recovered,
                "requires_human": anomaly.requires_human(),
            }),

            AuditKind::StateTransition {
                from,
                to,
                position,
                fechado_por,
            } => json!({
                "from": from,
                "to": to,
                "qty": d(position.qty()),
                "avg_price": d(position.avg_price()),
                "fechado_por": fechado_por.map(CausaDoFechamento::as_str),
            }),
        }
    }

    pub fn payload_json(&self) -> String {
        self.payload().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Fill, Intent, OrderId, Side, Signal, SignalId, SignalInputs};
    use chrono::{TimeZone, Utc};
    use rust_decimal::dec;

    fn instante() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
    }

    /// Percorre o JSON procurando qualquer número onde deveria haver string.
    fn numeros_em(v: &Value, caminho: &str, achados: &mut Vec<String>) {
        const MONETARIOS: &[&str] = &[
            "qty",
            "price",
            "fee",
            "slippage",
            "avg_price",
            "daily_pnl",
            "exposure",
            "max_daily_loss",
            "max_position_size",
            "max_total_exposure",
            "max_price_deviation_ratio",
        ];
        match v {
            Value::Object(m) => {
                for (k, val) in m {
                    let c = format!("{caminho}.{k}");
                    if MONETARIOS.contains(&k.as_str()) && val.is_number() {
                        achados.push(c.clone());
                    }
                    numeros_em(val, &c, achados);
                }
            }
            Value::Array(a) => {
                for (i, val) in a.iter().enumerate() {
                    numeros_em(val, &format!("{caminho}[{i}]"), achados);
                }
            }
            _ => {}
        }
    }

    fn todos_os_eventos() -> Vec<AuditKind> {
        let pos = crate::Position::default();
        vec![
            AuditKind::Signal(Signal {
                id: SignalId(1),
                at: instante(),
                intent: Intent::Buy,
                qty: Some(dec!(0.015)),
                inputs: SignalInputs::default().with_value("sma_fast", dec!(63412.10)),
            }),
            AuditKind::Order(crate::Order {
                id: OrderId(1),
                signal_ref: SignalId(1),
                side: Side::Buy,
                qty: dec!(0.015),
                at: instante(),
            }),
            AuditKind::RiskDecision(crate::RiskDecision {
                order_ref: OrderId(1),
                verdict: crate::Verdict::Rejected(crate::LimitBreach::MaxPositionSize),
                limits: crate::RiskLimits::default(),
                state: crate::RiskState::default(),
            }),
            AuditKind::Fill(Fill {
                order_ref: OrderId(1),
                price: dec!(63420.00),
                qty: dec!(0.015),
                fee: dec!(0.97),
                fee_base: dec!(0),
                slippage: dec!(0.32),
                at: instante(),
            }),
            AuditKind::Halt {
                reason: "divergência".into(),
                anomaly: Anomaly::Integrity(crate::IntegrityCause::PositionDivergence),
            },
            AuditKind::Resume {
                trigger: "DayBoundary".into(),
                automatic: true,
            },
            AuditKind::Anomaly {
                anomaly: Anomaly::Transient {
                    attempt: 2,
                    max_retries: 5,
                },
                recovered: true,
            },
            AuditKind::StateTransition {
                from: "Long".into(),
                to: "Flat".into(),
                position: pos,
                fechado_por: Some(CausaDoFechamento::Prazo),
            },
        ]
    }

    #[test]
    fn nenhum_valor_monetario_e_numero_json() {
        // Número em JSON é IEEE-754 na maioria dos leitores: serializar
        // Decimal como número desfaria a exatidão na saída para o disco.
        for e in todos_os_eventos() {
            let mut achados = Vec::new();
            numeros_em(&e.payload(), e.as_str(), &mut achados);
            assert!(
                achados.is_empty(),
                "valores monetários como número: {achados:?}"
            );
        }
    }

    #[test]
    fn todo_evento_produz_json_valido() {
        for e in todos_os_eventos() {
            let s = e.payload_json();
            serde_json::from_str::<Value>(&s)
                .unwrap_or_else(|_| panic!("JSON inválido em {}", e.as_str()));
        }
    }

    #[test]
    fn o_sinal_carrega_os_dados_que_o_produziram() {
        let e = &todos_os_eventos()[0];
        let p = e.payload();
        assert_eq!(p["inputs"]["values"]["sma_fast"], "63412.10");
    }

    #[test]
    fn a_decisao_carrega_o_retrato_dos_limites_e_o_limite_violado() {
        let e = &todos_os_eventos()[2];
        let p = e.payload();
        assert_eq!(p["verdict"], "Rejected");
        assert_eq!(p["breach"], "MaxPositionSize");
        assert!(p["limits"]["max_position_size"].is_string());
        assert!(p["state"]["exposure"].is_string());
    }

    #[test]
    fn a_parada_diz_se_exige_ato_humano() {
        let halt = &todos_os_eventos()[4];
        assert_eq!(halt.payload()["requires_human"], true);
        assert_eq!(halt.payload()["classification"], "Integrity");

        let anomalia = &todos_os_eventos()[6];
        assert_eq!(anomalia.payload()["requires_human"], false);
        assert_eq!(anomalia.payload()["classification"], "Transient");
    }

    #[test]
    fn a_transicao_diz_o_que_fechou_o_episodio() {
        // REQ-UI-042: a partir da emenda 2.0.0 a posição fecha por dois
        // motivos, e o registro tem de dizer qual.
        let p = todos_os_eventos()[7].payload();
        assert_eq!(p["fechado_por"], "prazo");
    }

    #[test]
    fn a_transicao_que_abre_traz_o_campo_presente_e_nulo() {
        // Presente e nulo, nunca omitido: é a regra do REQ-UI-044, e é o que
        // permite a quem lê distinguir "não fechou" de "não sei".
        let abre = AuditKind::StateTransition {
            from: "Flat".into(),
            to: "Long".into(),
            position: crate::Position::default(),
            fechado_por: None,
        };
        let p = abre.payload();
        assert!(p.get("fechado_por").is_some(), "o campo existe");
        assert!(p["fechado_por"].is_null(), "e vem nulo");
    }

    #[test]
    fn taxa_e_slippage_aparecem_separados_do_preco() {
        let p = todos_os_eventos()[3].payload();
        assert_eq!(p["price"], "63420.00");
        assert_eq!(p["fee"], "0.97");
        assert_eq!(p["slippage"], "0.32");
    }

    fn chaves(v: &Value, saida: &mut Vec<String>) {
        match v {
            Value::Object(m) => {
                for (k, val) in m {
                    saida.push(k.to_lowercase());
                    chaves(val, saida);
                }
            }
            Value::Array(a) => a.iter().for_each(|x| chaves(x, saida)),
            _ => {}
        }
    }

    #[test]
    fn nenhum_evento_carrega_credencial() {
        // Nesta feature a garantia é trivial — o sistema não possui
        // credencial alguma. O teste existe porque paper trading e live virão
        // e o esquema não muda.
        //
        // A verificação é por **nome de campo**, não por substring: procurar
        // "sign" no texto inteiro acusa `signal_id`, que é justamente o campo
        // que sustenta a reconstituição.
        const PROIBIDOS: &[&str] = &[
            "api_key",
            "apikey",
            "secret",
            "token",
            "password",
            "passphrase",
            "signature",
            "credential",
            "private_key",
            "authorization",
        ];
        for e in todos_os_eventos() {
            let mut ks = Vec::new();
            chaves(&e.payload(), &mut ks);
            for k in &ks {
                assert!(
                    !PROIBIDOS.iter().any(|p| k.contains(p)),
                    "evento {} tem o campo '{k}'",
                    e.as_str()
                );
            }
        }
    }
}
