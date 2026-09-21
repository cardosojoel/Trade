//! Medições sobre o registro, para a revisão da Porta 2.
//!
//! O dado que elas medem só existe depois de trinta dias na testnet. O
//! **medidor** não precisa esperar: a aritmética é sobre o extrato e sobre os
//! preenchimentos, cujo esquema já está fixo e é o mesmo que o backtest
//! produz.
//!
//! Escrever antes não é antecipação especulativa — é o contrário. Medir o
//! período mais caro do projeto com código recém-escrito e nunca exercitado é
//! que seria arriscado (decisão 039 do Jev, `fatia_nova_de_medicao`,
//! 1,00 · confiança 1,00).

use crate::Money;
use rust_decimal::Decimal;

/// O slippage observado, resumido.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slippage {
    pub amostras: usize,
    pub mediana: Money,
    pub p95: Money,
    pub minimo: Money,
    pub maximo: Money,
}

/// Por que a medição não pôde ser feita.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MedicaoError {
    #[error(
        "amostra insuficiente: {observadas} preenchimentos contra os {exigidos} que o \
         SC-102 exige. Faltam {faltam}."
    )]
    AmostraInsuficiente {
        observadas: usize,
        exigidos: usize,
        faltam: usize,
    },
}

/// Mediana e p95 do slippage sobre os preenchimentos.
///
/// **Recusa amostra pequena em vez de devolver um número frágil.** A mediana
/// de sete preenchimentos é aritmeticamente válida e estatisticamente inútil,
/// e o `SC-102` fixa cem como o mínimo. Devolver a conta assim mesmo faria o
/// parâmetro ser promovido de `ASSUMED` a `MEASURED` sobre evidência que não
/// sustenta a promoção — que é exatamente o que a Porta 2 existe para impedir.
pub fn slippage(observados: &[Money], minimo: usize) -> Result<Slippage, MedicaoError> {
    if observados.len() < minimo {
        return Err(MedicaoError::AmostraInsuficiente {
            observadas: observados.len(),
            exigidos: minimo,
            faltam: minimo - observados.len(),
        });
    }

    let mut v = observados.to_vec();
    v.sort();
    Ok(Slippage {
        amostras: v.len(),
        mediana: percentil(&v, 50),
        p95: percentil(&v, 95),
        minimo: v[0],
        maximo: v[v.len() - 1],
    })
}

/// O percentil de uma amostra **já ordenada**, pelo método do mais próximo.
///
/// Sem interpolar entre dois vizinhos: interpolar produz um valor que nenhum
/// preenchimento teve, e o que se quer aqui é um slippage que de fato
/// aconteceu. Tudo em `Decimal`, porque slippage é dinheiro.
fn percentil(ordenada: &[Money], p: u32) -> Money {
    if ordenada.is_empty() {
        return Decimal::ZERO;
    }
    // Índice pelo método do mais próximo, arredondando para cima: com 100
    // amostras, o p95 é a 95ª, e não a 94ª.
    let n = Decimal::from(ordenada.len());
    let posicao = (Decimal::from(p) * n / Decimal::from(100)).ceil();
    let i = usize::try_from(posicao.trunc().mantissa().max(1)).unwrap_or(1) - 1;
    ordenada[i.min(ordenada.len() - 1)]
}

/// A distância entre o que o simulador previu e o que a testnet fez.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Divergencia {
    pub resultado_paper: Money,
    pub resultado_backtest: Money,
    /// Paper menos backtest. Negativo significa que o mercado real foi pior.
    pub diferenca: Money,
    /// Quanto da diferença se explica por custo de transação.
    pub por_custo: Money,
    /// Quanto da diferença se explica por slippage.
    pub por_slippage: Money,
    /// O que sobra sem explicação. **É o número que importa.**
    pub inexplicada: Money,
}

/// Mede e **decompõe** a divergência (SC-106).
///
/// Decompor é o ponto. Saber que o paper deu menos que o backtest não diz
/// nada: o simulador modela taxa e slippage de propósito, então parte da
/// diferença é esperada e já está contabilizada. O que a Porta 3 precisa ver é
/// o que **sobra** depois de descontar o que era previsível — porque é isso
/// que o modelo não captura.
pub fn divergencia(
    resultado_paper: Money,
    resultado_backtest: Money,
    custo_paper: Money,
    custo_backtest: Money,
    slippage_paper: Money,
    slippage_backtest: Money,
) -> Divergencia {
    let diferenca = resultado_paper - resultado_backtest;
    let por_custo = -(custo_paper - custo_backtest);
    let por_slippage = -(slippage_paper - slippage_backtest);
    Divergencia {
        resultado_paper,
        resultado_backtest,
        diferenca,
        por_custo,
        por_slippage,
        inexplicada: diferenca - por_custo - por_slippage,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::dec;

    fn amostra(n: usize) -> Vec<Money> {
        (1..=n).map(|i| Decimal::from(i as i64)).collect()
    }

    #[test]
    fn a_mediana_e_o_p95_saem_da_amostra_ordenada() {
        let s = slippage(&amostra(100), 100).unwrap();
        assert_eq!(s.amostras, 100);
        assert_eq!(s.mediana, dec!(50), "a 50ª de cem");
        assert_eq!(s.p95, dec!(95), "a 95ª de cem");
        assert_eq!(s.minimo, dec!(1));
        assert_eq!(s.maximo, dec!(100));
    }

    #[test]
    fn a_amostra_fora_de_ordem_da_o_mesmo_resultado() {
        let mut baguncada = amostra(100);
        baguncada.reverse();
        assert_eq!(
            slippage(&baguncada, 100).unwrap(),
            slippage(&amostra(100), 100).unwrap()
        );
    }

    #[test]
    fn o_percentil_nao_inventa_valor_que_ninguem_teve() {
        // Sem interpolação: o p95 é um slippage que de fato aconteceu.
        let v = vec![dec!(1), dec!(2), dec!(3), dec!(100)];
        let s = slippage(&v, 4).unwrap();
        assert!(v.contains(&s.p95), "p95 {} não está na amostra", s.p95);
        assert!(v.contains(&s.mediana));
    }

    #[test]
    fn amostra_pequena_nao_vira_medida_e_diz_quantas_faltam() {
        // SC-102 exige cem. A mediana de sete é válida na aritmética e inútil
        // na evidência — e promoveria `ASSUMED` a `MEASURED` sem base.
        let e = slippage(&amostra(7), 100).unwrap_err();
        assert_eq!(
            e,
            MedicaoError::AmostraInsuficiente {
                observadas: 7,
                exigidos: 100,
                faltam: 93
            }
        );
        assert!(e.to_string().contains("Faltam 93"), "veio {e}");
    }

    #[test]
    fn exatamente_o_minimo_passa() {
        assert!(slippage(&amostra(100), 100).is_ok());
        assert!(slippage(&amostra(99), 100).is_err());
    }

    #[test]
    fn nenhum_digito_se_perde_na_medicao() {
        let v = vec![dec!(0.00000001), dec!(0.12345678), dec!(0.87654321)];
        let s = slippage(&v, 3).unwrap();
        assert_eq!(s.mediana.to_string(), "0.12345678");
        assert_eq!(s.maximo.to_string(), "0.87654321");
    }

    #[test]
    fn a_divergencia_desconta_o_que_o_simulador_ja_previa() {
        // O paper deu 100 a menos. Custo e slippage explicam 90; sobram 10
        // sem explicação — e são os 10 que a Porta 3 precisa ver.
        let d = divergencia(
            dec!(-500),
            dec!(-400),
            dec!(150),
            dec!(80),
            dec!(40),
            dec!(20),
        );
        assert_eq!(d.diferenca, dec!(-100));
        assert_eq!(d.por_custo, dec!(-70), "custo 70 maior no real");
        assert_eq!(d.por_slippage, dec!(-20));
        assert_eq!(d.inexplicada, dec!(-10));
    }

    #[test]
    fn sem_divergencia_nada_fica_inexplicado() {
        let d = divergencia(
            dec!(-400),
            dec!(-400),
            dec!(80),
            dec!(80),
            dec!(20),
            dec!(20),
        );
        assert_eq!(d.diferenca, Decimal::ZERO);
        assert_eq!(d.inexplicada, Decimal::ZERO);
    }

    #[test]
    fn a_decomposicao_fecha_sempre() {
        // A soma das partes é a diferença, por construção — não por
        // coincidência de arredondamento.
        let d = divergencia(
            dec!(-573.12345678),
            dec!(-400.87654321),
            dec!(150.11111111),
            dec!(80.22222222),
            dec!(40.33333333),
            dec!(20.44444444),
        );
        assert_eq!(d.por_custo + d.por_slippage + d.inexplicada, d.diferenca);
    }
}

// ------------------------------------------------------------ Porta 2

/// O que se observou no período, para a revisão da Porta 2.
///
/// Os seis campos correspondem a `SC-101` a `SC-106` da feature 002. Quem
/// preenche este tipo leu o registro; quem o avalia não precisa saber de
/// SQLite.
#[derive(Debug, Clone)]
pub struct ObservadoPorta2 {
    /// Dias corridos de operação **ininterrupta**.
    pub dias_corridos: Money,
    /// O `seq` do registro não tem buraco.
    pub seq_sem_buraco: bool,
    /// A medição do slippage, ou por que não foi possível.
    pub slippage: Result<Slippage, MedicaoError>,
    /// Divergências de posição que ocorreram **sem** aparecer classificadas no
    /// registro. Qualquer número acima de zero reprova.
    pub divergencias_nao_detectadas: usize,
    /// Ordens com identificador de cliente repetido.
    pub ordens_duplicadas: usize,
    /// Reinícios com posição aberta, e quantos foram reconciliados.
    pub reinicios: usize,
    pub reinicios_reconciliados: usize,
    /// A divergência entre paper e backtest, quando houve o que comparar.
    pub divergencia: Option<Divergencia>,
}

/// Um critério, com o veredito e o que se observou.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Criterio {
    pub codigo: &'static str,
    pub exigencia: &'static str,
    pub observado: String,
    pub passou: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatorioPorta2 {
    pub criterios: Vec<Criterio>,
    pub passou: bool,
}

/// Avalia os seis critérios da Porta 2.
///
/// **A Porta 2 não é aprovada por maioria.** Os seis são conjuntivos: um só
/// reprovado reprova o conjunto, porque cada um cobre uma forma diferente de
/// o sistema estar errado, e passar em cinco não compensa falhar no sexto.
///
/// O relatório não promove nada. Promover é ato humano registrado (Princípio
/// I) — o que este tipo faz é pôr diante de quem decide o que o registro diz,
/// com o observado ao lado do exigido.
pub fn relatorio_porta2(
    o: &ObservadoPorta2,
    dias_exigidos: Money,
    amostra_minima: usize,
) -> RelatorioPorta2 {
    let criterios = vec![
        Criterio {
            codigo: "SC-101",
            exigencia: "30 dias corridos ininterruptos, sem intervenção, com seq sem buraco",
            observado: format!(
                "{} dias · seq {}",
                o.dias_corridos,
                if o.seq_sem_buraco {
                    "sem buraco"
                } else {
                    "COM BURACO"
                }
            ),
            passou: o.dias_corridos >= dias_exigidos && o.seq_sem_buraco,
        },
        Criterio {
            codigo: "SC-102",
            exigencia: "slippage real sobre ≥ 100 preenchimentos, com mediana e p95",
            observado: match &o.slippage {
                Ok(s) => format!(
                    "{} amostras · mediana {} · p95 {}",
                    s.amostras, s.mediana, s.p95
                ),
                Err(e) => e.to_string(),
            },
            passou: o
                .slippage
                .as_ref()
                .is_ok_and(|s| s.amostras >= amostra_minima),
        },
        Criterio {
            codigo: "SC-103",
            exigencia: "zero divergências de posição não detectadas",
            observado: format!("{} não detectadas", o.divergencias_nao_detectadas),
            passou: o.divergencias_nao_detectadas == 0,
        },
        Criterio {
            codigo: "SC-104",
            exigencia: "nenhuma ordem duplicada, por identificador de cliente",
            observado: format!("{} duplicadas", o.ordens_duplicadas),
            passou: o.ordens_duplicadas == 0,
        },
        Criterio {
            codigo: "SC-105",
            exigencia: "todo reinício com posição aberta recuperou e reconciliou",
            observado: format!(
                "{} de {} reinícios reconciliados",
                o.reinicios_reconciliados, o.reinicios
            ),
            passou: o.reinicios_reconciliados == o.reinicios,
        },
        Criterio {
            codigo: "SC-106",
            exigencia: "divergência paper × backtest medida, explicada e registrada",
            observado: match &o.divergencia {
                Some(d) => format!(
                    "diferença {} · custo {} · slippage {} · inexplicada {}",
                    d.diferenca, d.por_custo, d.por_slippage, d.inexplicada
                ),
                None => "não medida".to_string(),
            },
            passou: o.divergencia.is_some(),
        },
    ];

    let passou = criterios.iter().all(|c| c.passou);
    RelatorioPorta2 { criterios, passou }
}

#[cfg(test)]
mod testes_porta2 {
    use super::*;
    use rust_decimal::dec;

    fn tudo_certo() -> ObservadoPorta2 {
        ObservadoPorta2 {
            dias_corridos: dec!(30),
            seq_sem_buraco: true,
            slippage: slippage(&(1..=100).map(Decimal::from).collect::<Vec<_>>(), 100),
            divergencias_nao_detectadas: 0,
            ordens_duplicadas: 0,
            reinicios: 3,
            reinicios_reconciliados: 3,
            divergencia: Some(divergencia(
                dec!(-500),
                dec!(-400),
                dec!(150),
                dec!(80),
                dec!(40),
                dec!(20),
            )),
        }
    }

    #[test]
    fn com_tudo_em_ordem_a_porta_passa() {
        let r = relatorio_porta2(&tudo_certo(), dec!(30), 100);
        assert_eq!(r.criterios.len(), 6);
        assert!(r.passou);
        assert!(r.criterios.iter().all(|c| c.passou));
    }

    #[test]
    fn um_criterio_reprovado_reprova_a_porta() {
        // Conjuntivos: passar em cinco não compensa falhar no sexto, porque
        // cada um cobre uma forma diferente de o sistema estar errado.
        for (rotulo, o) in [
            (
                "dias",
                ObservadoPorta2 {
                    dias_corridos: dec!(29),
                    ..tudo_certo()
                },
            ),
            (
                "seq",
                ObservadoPorta2 {
                    seq_sem_buraco: false,
                    ..tudo_certo()
                },
            ),
            (
                "divergência",
                ObservadoPorta2 {
                    divergencias_nao_detectadas: 1,
                    ..tudo_certo()
                },
            ),
            (
                "duplicada",
                ObservadoPorta2 {
                    ordens_duplicadas: 1,
                    ..tudo_certo()
                },
            ),
            (
                "reinício",
                ObservadoPorta2 {
                    reinicios_reconciliados: 2,
                    ..tudo_certo()
                },
            ),
            (
                "sem medir",
                ObservadoPorta2 {
                    divergencia: None,
                    ..tudo_certo()
                },
            ),
        ] {
            assert!(
                !relatorio_porta2(&o, dec!(30), 100).passou,
                "{rotulo} devia reprovar"
            );
        }
    }

    #[test]
    fn a_amostra_insuficiente_reprova_e_diz_quantas_faltam() {
        let o = ObservadoPorta2 {
            slippage: slippage(&(1..=42).map(Decimal::from).collect::<Vec<_>>(), 100),
            ..tudo_certo()
        };
        let r = relatorio_porta2(&o, dec!(30), 100);
        let c = r.criterios.iter().find(|c| c.codigo == "SC-102").unwrap();
        assert!(!c.passou);
        assert!(c.observado.contains("Faltam 58"), "veio {}", c.observado);
    }

    #[test]
    fn um_seq_com_buraco_reprova_mesmo_com_os_trinta_dias() {
        // Trinta dias de operação cujo registro tem buraco não são trinta
        // dias de registro: o que falta pode ser justamente o que importa.
        let o = ObservadoPorta2 {
            seq_sem_buraco: false,
            ..tudo_certo()
        };
        let c = relatorio_porta2(&o, dec!(30), 100).criterios[0].clone();
        assert!(!c.passou);
        assert!(c.observado.contains("COM BURACO"));
    }

    #[test]
    fn o_relatorio_nao_promove_nada() {
        // Promover é ato humano registrado (Princípio I). O relatório põe o
        // observado ao lado do exigido e para aí.
        let r = relatorio_porta2(&tudo_certo(), dec!(30), 100);
        assert!(r.passou, "os seis critérios passam");
        // E mesmo assim nada no tipo diz que promoveu: `passou` é constatação
        // sobre o registro, não autorização.
        assert_eq!(r.criterios.iter().filter(|c| c.passou).count(), 6);
    }
}
