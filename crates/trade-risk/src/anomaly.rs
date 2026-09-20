//! Classificação de anomalia e política de retentativa.
//!
//! A distinção entre falha **transitória** e falha **de integridade** é o que
//! permite ao robô operar sozinho sem ser imprudente. Parar por três segundos
//! de rede custa oportunidade sem comprar segurança; seguir operando com a
//! posição divergindo da corretora custa dinheiro.

use rust_decimal::Decimal;
use trade_domain::{Anomaly, IntegrityCause, Money, Position};
use trade_ports::ExecError;

/// Classifica uma falha de execução.
///
/// `attempt` é a tentativa que acabou de falhar, começando em 1.
pub fn classify(err: &ExecError, attempt: u32, max_retries: u32) -> Option<Anomaly> {
    match err {
        // Divergência reportada pelo executor nunca é transitória.
        ExecError::Integrity(_) => Some(Anomaly::Integrity(IntegrityCause::PositionDivergence)),

        // Indisponibilidade se resolve sozinha — até deixar de se resolver.
        e if e.is_transient_candidate() => {
            if attempt >= max_retries {
                Some(Anomaly::Integrity(IntegrityCause::RetriesExhausted))
            } else {
                Some(Anomaly::Transient {
                    attempt,
                    max_retries,
                })
            }
        }

        // Recusa da corretora e saldo insuficiente são ordens que falharam,
        // não anomalias: retentar não muda o resultado, e parar o robô por
        // causa disso seria parar por algo que a própria cerca já trata.
        ExecError::Rejected(_) | ExecError::Insufficient(_) => None,

        _ => None,
    }
}

/// Detecta preço implausível em relação à referência (FR-024).
pub fn implausible_price(
    observed: Money,
    reference: Money,
    max_deviation_ratio: Money,
) -> Option<Anomaly> {
    if reference <= Decimal::ZERO || max_deviation_ratio <= Decimal::ZERO {
        return None;
    }
    let desvio = (observed - reference).abs() / reference;
    (desvio > max_deviation_ratio).then_some(Anomaly::Integrity(IntegrityCause::ImplausiblePrice))
}

/// Detecta divergência entre a posição calculada e a reportada pela fonte.
///
/// É a anomalia mais cara de ignorar: significa que o sistema e o mercado
/// discordam sobre quanto se tem, e toda decisão seguinte parte de premissa
/// errada.
pub fn position_divergence(local: &Position, reported_qty: Money) -> Option<Anomaly> {
    (local.qty() != reported_qty).then_some(Anomaly::Integrity(IntegrityCause::PositionDivergence))
}
