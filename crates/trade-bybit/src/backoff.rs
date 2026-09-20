//! Recuo entre tentativas.
//!
//! A documentação indica **600 requisições em janela de 5 segundos por IP** —
//! e uma coleta de doze meses de velas de um minuto cabe em ~526 requisições.
//! A restrição real, portanto, não é vazão: é educação. O coletor não precisa
//! de paralelismo nem de otimização, precisa de não ser rude e saber recuar.

use std::time::Duration;

/// Espera antes da tentativa `n` (contada a partir de 1), com teto.
///
/// Dobra a cada tentativa: 200ms, 400ms, 800ms… até o teto de 30 segundos.
/// Sem teto, uma indisponibilidade longa produziria esperas absurdas; sem
/// crescimento, insistir no mesmo ritmo é o que mantém a fonte irritada.
pub fn espera(tentativa: u32) -> Duration {
    const BASE_MS: u64 = 200;
    const TETO: Duration = Duration::from_secs(30);

    let expoente = tentativa.saturating_sub(1).min(10);
    let ms = BASE_MS.saturating_mul(1u64 << expoente);
    Duration::from_millis(ms).min(TETO)
}

/// Quota restante informada pela fonte, se presente.
///
/// A documentação lista `X-Bapi-Limit`, `X-Bapi-Limit-Status` e
/// `X-Bapi-Limit-Reset-Timestamp`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quota {
    pub limite: Option<u32>,
    pub restante: Option<u32>,
}

impl Quota {
    /// Se convém pausar antes da próxima requisição.
    pub fn apertada(&self) -> bool {
        matches!((self.restante, self.limite), (Some(r), Some(l)) if l > 0 && r * 10 < l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_espera_dobra_a_cada_tentativa() {
        assert_eq!(espera(1), Duration::from_millis(200));
        assert_eq!(espera(2), Duration::from_millis(400));
        assert_eq!(espera(3), Duration::from_millis(800));
    }

    #[test]
    fn a_espera_tem_teto() {
        assert_eq!(espera(50), Duration::from_secs(30));
    }

    #[test]
    fn quota_apertada_abaixo_de_dez_por_cento() {
        assert!(
            Quota {
                limite: Some(600),
                restante: Some(50)
            }
            .apertada()
        );
        assert!(
            !Quota {
                limite: Some(600),
                restante: Some(200)
            }
            .apertada()
        );
        assert!(
            !Quota {
                limite: None,
                restante: None
            }
            .apertada()
        );
    }
}
