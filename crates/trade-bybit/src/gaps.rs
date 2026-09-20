//! Detecção de lacunas no histórico.

use chrono::{DateTime, Duration, Utc};
use trade_domain::{Candle, Gap, Interval};

/// Encontra os intervalos em que a fonte não entregou vela.
///
/// Registradas, nunca interpoladas (FR-016, FR-031): interpolar inventa preço
/// que não existiu, e uma estratégia de tendência decidiria sobre um mercado
/// imaginário sem que nada no sistema soubesse.
pub fn detectar(velas: &[Candle], interval: Interval) -> Vec<Gap> {
    let passo = Duration::minutes(interval.minutes());
    let mut lacunas = Vec::new();

    for par in velas.windows(2) {
        let (a, b) = (&par[0], &par[1]);
        if b.open_time > a.open_time + passo {
            lacunas.push(Gap {
                from: a.open_time,
                to: b.open_time,
            });
        }
    }

    lacunas
}

/// Lacuna entre o fim do que já existe e o começo do que chegou.
pub fn entre(
    anterior: Option<DateTime<Utc>>,
    primeira: DateTime<Utc>,
    interval: Interval,
) -> Option<Gap> {
    let anterior = anterior?;
    let passo = Duration::minutes(interval.minutes());
    (primeira > anterior + passo).then_some(Gap {
        from: anterior,
        to: primeira,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use rust_decimal::dec;

    fn vela(minuto: i64) -> Candle {
        let t = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap() + Duration::minutes(minuto);
        Candle {
            open_time: t,
            open: dec!(100),
            high: dec!(100),
            low: dec!(100),
            close: dec!(100),
            volume: dec!(1),
            turnover: dec!(100),
        }
    }

    #[test]
    fn historico_continuo_nao_tem_lacuna() {
        let v: Vec<_> = (0..5).map(vela).collect();
        assert!(detectar(&v, Interval::M1).is_empty());
    }

    #[test]
    fn salto_vira_lacuna_com_as_duas_pontas() {
        let v = vec![vela(0), vela(1), vela(10), vela(11)];
        let l = detectar(&v, Interval::M1);
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].from, vela(1).open_time);
        assert_eq!(l[0].to, vela(10).open_time);
    }

    #[test]
    fn varias_lacunas_sao_todas_reportadas() {
        let v = vec![vela(0), vela(5), vela(6), vela(20)];
        assert_eq!(detectar(&v, Interval::M1).len(), 2);
    }

    #[test]
    fn a_granularidade_muda_o_que_e_lacuna() {
        // De cinco em cinco minutos, um salto de 5 é continuidade.
        let v = vec![vela(0), vela(5), vela(10)];
        assert!(detectar(&v, Interval::M5).is_empty());
        assert_eq!(detectar(&v, Interval::M1).len(), 2);
    }

    #[test]
    fn lacuna_entre_paginas_e_detectada() {
        let g = entre(Some(vela(1).open_time), vela(9).open_time, Interval::M1);
        assert!(g.is_some());
        assert!(entre(Some(vela(1).open_time), vela(2).open_time, Interval::M1).is_none());
        assert!(entre(None, vela(9).open_time, Interval::M1).is_none());
    }
}
