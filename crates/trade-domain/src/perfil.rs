//! Derivação do perfil de operação a partir do depósito.
//!
//! Quem opera informa **apenas o depósito**. Teto de posição, stop, risco por
//! operação, limite diário e chão de operação saem daqui — não são escolhidos,
//! e por isso não podem ser afrouxados por conveniência.
//!
//! As identidades estão em `MD BOT/27_CONFIGURATION_REGISTRY.md`, e os testes
//! deste módulo usam a tabela daquele documento como oráculo: se o código e o
//! documento divergirem, o teste quebra. É o que impede a folha de parâmetros
//! de virar ficção mantida à parte.

use crate::{Instrumento, Money, RiskLimits};
use rust_decimal::Decimal;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PerfilError {
    #[error(
        "depósito de {capital} não sustenta o perfil: a ordem mínima é {minimo} e o \
         drawdown de parada é {drawdown}, então o mínimo viável é {viavel}"
    )]
    CapitalInsuficiente {
        capital: String,
        minimo: String,
        drawdown: String,
        viavel: String,
    },
    #[error("parâmetro `{campo}` precisa ser positivo, veio {valor}")]
    ParametroNaoPositivo { campo: &'static str, valor: String },
}

/// Insumos da derivação. Vivem em arquivo, nunca no código: a constitution
/// exige que limiar de risco seja configuração externa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParametrosDerivacao {
    /// Fração do capital em que a operação para para revisão. `CONSTITUTION`.
    pub drawdown_de_parada: Money,
    /// Quantas perdas cheias o drawdown deve tolerar. É o que fixa o risco.
    pub perdas_toleradas: u32,
    /// Stop que minimiza o acerto necessário, medido. `MEASURED`.
    pub stop_otimo: Money,
    /// Prazo máximo de posição, em minutos. `CONSTITUTION` — emenda 2.0.0.
    pub prazo_maximo_min: i64,
    /// Frequência natural: 1440 ÷ tempo médio até resolver. `MEASURED`.
    pub operacoes_por_dia: Money,
}

/// O perfil derivado. Frações do capital, exceto onde dito.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Perfil {
    pub risco_por_operacao: Money,
    pub stop: Money,
    pub teto_de_posicao: Money,
    pub limite_diario: Money,
    /// Em moeda de cotação: abaixo disso a conta não emite ordem.
    pub chao_de_operacao: Money,
    pub prazo_maximo_min: i64,
}

impl Perfil {
    pub fn derivar(
        capital: Money,
        instrumento: &Instrumento,
        p: &ParametrosDerivacao,
    ) -> Result<Self, PerfilError> {
        if p.drawdown_de_parada <= Decimal::ZERO {
            return Err(PerfilError::ParametroNaoPositivo {
                campo: "drawdown_de_parada",
                valor: p.drawdown_de_parada.to_string(),
            });
        }
        if p.perdas_toleradas == 0 {
            return Err(PerfilError::ParametroNaoPositivo {
                campo: "perdas_toleradas",
                valor: "0".into(),
            });
        }
        if p.stop_otimo <= Decimal::ZERO {
            return Err(PerfilError::ParametroNaoPositivo {
                campo: "stop_otimo",
                valor: p.stop_otimo.to_string(),
            });
        }

        let minimo = instrumento.valor_minimo_ordem();
        let sobra = Decimal::ONE - p.drawdown_de_parada;

        // Viabilidade: a posição precisa continuar emitível depois do drawdown
        // máximo tolerado. Abaixo disso não há perfil a derivar — e reduzir
        // algo para caber seria derivação que ninguém confirmou.
        let viavel = minimo / sobra;
        if capital < viavel {
            return Err(PerfilError::CapitalInsuficiente {
                capital: capital.to_string(),
                minimo: minimo.to_string(),
                drawdown: p.drawdown_de_parada.to_string(),
                viavel: viavel.round_dp(2).to_string(),
            });
        }

        let risco_por_operacao = p.drawdown_de_parada / Decimal::from(p.perdas_toleradas);
        let teto_minimo = minimo / (capital * sobra);
        let stop = p.stop_otimo.min(risco_por_operacao / teto_minimo);
        let teto_de_posicao = risco_por_operacao / stop;

        Ok(Self {
            risco_por_operacao,
            stop,
            teto_de_posicao,
            limite_diario: p.operacoes_por_dia * risco_por_operacao,
            chao_de_operacao: minimo / teto_de_posicao,
            prazo_maximo_min: p.prazo_maximo_min,
        })
    }

    /// Converte as frações em limites absolutos, que é o que a cerca cobra.
    pub fn limites(&self, capital: Money, base: &RiskLimits) -> RiskLimits {
        RiskLimits {
            max_daily_loss: (capital * self.limite_diario).round_dp(2),
            max_position_size: (capital * self.teto_de_posicao).round_dp(2),
            // Ativo único e uma posição por vez: a exposição total é o teto de
            // posição. Declarar um número maior seria decorativo.
            max_total_exposure: (capital * self.teto_de_posicao).round_dp(2),
            ..base.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::dec;

    fn btcusdt() -> Instrumento {
        Instrumento::novo(dec!(0.000001), dec!(5)).unwrap()
    }

    /// Os insumos registrados no 27_CONFIGURATION_REGISTRY.md.
    fn parametros() -> ParametrosDerivacao {
        ParametrosDerivacao {
            drawdown_de_parada: dec!(0.15),
            perdas_toleradas: 15,
            stop_otimo: dec!(0.02),
            prazo_maximo_min: 4320,
            operacoes_por_dia: dec!(0.95),
        }
    }

    #[test]
    fn capital_abaixo_do_minimo_viavel_nao_deriva() {
        // 5 ÷ 0,85 = 5,88 — abaixo disso a primeira posição não sobrevive ao
        // drawdown e ainda continua emitível.
        let e = Perfil::derivar(dec!(5.87), &btcusdt(), &parametros()).unwrap_err();
        assert!(matches!(e, PerfilError::CapitalInsuficiente { .. }), "{e}");
        assert!(Perfil::derivar(dec!(5.89), &btcusdt(), &parametros()).is_ok());
    }

    #[test]
    fn a_tabela_do_registry_e_reproduzida() {
        // Oráculo: 27_CONFIGURATION_REGISTRY.md, seção "Família derivada".
        for (capital, teto, chao) in [
            (dec!(200), dec!(0.5), dec!(10)),
            (dec!(500), dec!(0.5), dec!(10)),
            (dec!(900), dec!(0.5), dec!(10)),
        ] {
            let p = Perfil::derivar(capital, &btcusdt(), &parametros()).unwrap();
            assert_eq!(p.risco_por_operacao, dec!(0.01), "risco em {capital}");
            assert_eq!(p.stop, dec!(0.02), "stop em {capital}");
            assert_eq!(p.teto_de_posicao, teto, "teto em {capital}");
            assert_eq!(p.chao_de_operacao, chao, "chão em {capital}");
            assert_eq!(p.prazo_maximo_min, 4320);
        }
    }

    #[test]
    fn banca_pequena_aperta_o_stop_e_levanta_o_teto() {
        // Com US$ 10 o teto mínimo é 5 ÷ 8,50 = 58,8%, e o stop tem de caber
        // nele: 1% ÷ 58,8% = 1,7%. É a mesma linha da folha de limites.
        let p = Perfil::derivar(dec!(10), &btcusdt(), &parametros()).unwrap();
        assert_eq!(p.stop.round_dp(4), dec!(0.0170));
        assert_eq!(p.teto_de_posicao.round_dp(3), dec!(0.588));
        assert!(
            p.teto_de_posicao < Decimal::ONE,
            "teto acima de 100% exigiria caixa que a conta não tem"
        );
    }

    #[test]
    fn o_teto_nunca_passa_de_cem_por_cento() {
        // Em spot o risco por operação nunca excede a distância do stop, e é
        // essa identidade que impede o teto de ultrapassar o caixa.
        for capital in [dec!(6), dec!(10), dec!(50), dec!(1000), dec!(100000)] {
            let p = Perfil::derivar(capital, &btcusdt(), &parametros()).unwrap();
            assert!(
                p.teto_de_posicao <= Decimal::ONE,
                "capital {capital} derivou teto de {}",
                p.teto_de_posicao
            );
            assert!(p.risco_por_operacao <= p.stop, "capital {capital}");
        }
    }

    #[test]
    fn limites_absolutos_saem_das_fracoes() {
        let p = Perfil::derivar(dec!(200), &btcusdt(), &parametros()).unwrap();
        let base = RiskLimits {
            max_daily_loss: dec!(0),
            max_position_size: dec!(0),
            max_total_exposure: dec!(0),
            max_orders_per_window: 10,
            window_minutes: 60,
            max_transient_retries: 5,
            max_price_deviation_ratio: dec!(0.1),
            max_position_hours: 72,
        };
        let l = p.limites(dec!(200), &base);
        assert_eq!(l.max_position_size, dec!(100), "50% de 200");
        assert_eq!(l.max_daily_loss, dec!(1.90), "0,95% de 200");
        assert_eq!(
            l.max_total_exposure, l.max_position_size,
            "ativo único e uma posição por vez"
        );
        assert_eq!(
            l.max_orders_per_window, 10,
            "o que não se deriva se preserva"
        );
    }
}
