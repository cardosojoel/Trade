//! T014, T016 e T017 — a Porta 1 e o resultado por dia.

use rust_decimal::dec;
use trade_domain::Trade;
use trade_serve::consultas::{self, ExigenciasPorta1};

fn em(dia: u32, h: u32) -> chrono::DateTime<chrono::Utc> {
    use chrono::TimeZone;
    chrono::Utc.with_ymd_and_hms(2026, 1, dia, h, 0, 0).unwrap()
}

fn op(dia: u32, pnl: rust_decimal::Decimal) -> Trade {
    Trade {
        seq: u64::from(dia),
        entry_at: em(dia, 1),
        entry_price: dec!(100),
        exit_at: em(dia, 2),
        exit_price: dec!(110),
        qty: dec!(1),
        fees: dec!(0.1),
        pnl,
    }
}

fn exigencias() -> ExigenciasPorta1 {
    // FR-024: valores de partida, lidos de configuração. Aqui o teste os
    // fornece — é justamente o que o requisito pede que seja possível.
    ExigenciasPorta1 {
        meses_minimos: 12,
        operacoes_minimas: 100,
        profit_factor_minimo: dec!(1.3),
        drawdown_maximo_fracao: dec!(0.15),
    }
}

// ---------------------------------------------------------------- T016

#[test]
fn o_resultado_por_dia_soma_as_operacoes_daquele_dia() {
    let extrato = vec![
        op(1, dec!(10)),
        op(1, dec!(-4)),
        op(2, dec!(7)),
        op(3, dec!(-20)),
    ];
    let d = consultas::por_dia(&extrato, dec!(200));
    assert_eq!(d.dias.len(), 3);
    assert_eq!(d.dias[0].resultado, dec!(6));
    assert_eq!(d.dias[1].resultado, dec!(7));
    assert_eq!(d.dias[2].resultado, dec!(-20));
}

#[test]
fn o_dia_conta_pelo_fechamento_e_nao_pela_abertura() {
    // Uma posição de 72 horas abre num dia e fecha noutro. O resultado
    // pertence ao dia em que foi **realizado** — é assim que o contador de
    // perda diária o conta.
    let mut t = op(1, dec!(10));
    t.exit_at = em(3, 5);
    let d = consultas::por_dia(&[t], dec!(200));
    assert_eq!(d.dias.len(), 1);
    assert_eq!(d.dias[0].dia.to_string(), "2026-01-03");
}

#[test]
fn o_pior_dia_e_o_de_maior_prejuizo() {
    let d = consultas::por_dia(
        &[op(1, dec!(10)), op(2, dec!(-30)), op(3, dec!(-5))],
        dec!(200),
    );
    let pior = d.pior_dia.expect("houve dia negativo");
    assert_eq!(pior.resultado, dec!(-30));
    assert_eq!(pior.dia.to_string(), "2026-01-02");
}

#[test]
fn sem_dia_negativo_nao_ha_pior_dia() {
    // `None`, e não o menos bom: inventar um "pior dia" positivo faria a tela
    // mostrar prejuízo onde não houve.
    let d = consultas::por_dia(&[op(1, dec!(10)), op(2, dec!(3))], dec!(200));
    assert!(d.pior_dia.is_none());
}

#[test]
fn os_dias_que_romperam_o_limite_sao_assinalados() {
    // Atingir, não ultrapassar — é a fronteira invertida do FR-019a, e a
    // tela tem de mostrar a mesma que o freio usou.
    let d = consultas::por_dia(
        &[op(1, dec!(-200)), op(2, dec!(-199.99)), op(3, dec!(-500))],
        dec!(200),
    );
    let romperam: Vec<String> = d
        .dias
        .iter()
        .filter(|x| x.rompeu_o_limite)
        .map(|x| x.dia.to_string())
        .collect();
    assert_eq!(romperam, vec!["2026-01-01", "2026-01-03"]);
}

#[test]
fn a_curva_acumulada_e_a_soma_corrida() {
    let d = consultas::por_dia(
        &[op(1, dec!(10)), op(2, dec!(-4)), op(3, dec!(7))],
        dec!(200),
    );
    let curva: Vec<String> = d.dias.iter().map(|x| x.acumulado.to_string()).collect();
    assert_eq!(curva, vec!["10", "6", "13"]);
}

// ---------------------------------------------------------------- T017

#[test]
fn a_soma_por_dia_bate_com_a_soma_do_extrato_ate_o_ultimo_digito() {
    // FR-008: agregado calculado no servidor, em decimal exato. Um float no
    // caminho deixaria a soma dos dias diferente da soma das operações, e a
    // tela mostraria dois números que deviam ser um.
    let extrato: Vec<Trade> = (1..=9)
        .map(|i| op(u32::try_from(i).unwrap(), dec!(0.1234567890123456789)))
        .collect();
    let d = consultas::por_dia(&extrato, dec!(200));
    let soma_dos_dias: rust_decimal::Decimal = d.dias.iter().map(|x| x.resultado).sum();
    let soma_do_extrato: rust_decimal::Decimal = extrato.iter().map(|t| t.pnl).sum();
    assert_eq!(soma_dos_dias, soma_do_extrato);
    assert_eq!(d.dias.last().unwrap().acumulado, soma_do_extrato);
}

// ---------------------------------------------------------------- T014

#[test]
fn a_porta_1_traz_o_observado_ao_lado_do_exigido_e_o_veredito() {
    // REQ-UI-006: o veredito diz que passou, a folga diz o quanto faltava.
    let p = consultas::porta1(
        &exigencias(),
        consultas::ObservadoPorta1 {
            meses_de_historico: 12,
            operacoes: 140,
            custo_modelado: true,
            profit_factor: Some(dec!(1.45)),
            max_drawdown: dec!(1200),
            capital_inicial: dec!(10000),
        },
    );
    assert_eq!(p["exigencias"].as_array().unwrap().len(), 5);
    let pf = &p["exigencias"][3];
    assert_eq!(
        pf["exigido"], "≥ 1.3",
        "o exigido vem legível, como a tela o mostra"
    );
    assert_eq!(pf["observado"], "1.45");
    assert_eq!(pf["passou"], true);
    assert_eq!(p["passou"], true);
}

#[test]
fn uma_exigencia_reprovada_reprova_a_porta() {
    let p = consultas::porta1(
        &exigencias(),
        consultas::ObservadoPorta1 {
            meses_de_historico: 12,
            operacoes: 140,
            custo_modelado: true,
            profit_factor: Some(dec!(0.07)),
            max_drawdown: dec!(1200),
            capital_inicial: dec!(10000),
        },
    );
    assert_eq!(p["passou"], false);
    assert_eq!(p["exigencias"][3]["passou"], false);
}

#[test]
fn profit_factor_indefinido_nao_passa_na_porta() {
    // Indefinido não é aprovado por omissão: sem operação perdedora não há
    // evidência de que a estratégia sobrevive a uma.
    let p = consultas::porta1(
        &exigencias(),
        consultas::ObservadoPorta1 {
            meses_de_historico: 12,
            operacoes: 140,
            custo_modelado: true,
            profit_factor: None,
            max_drawdown: dec!(0),
            capital_inicial: dec!(10000),
        },
    );
    assert_eq!(p["exigencias"][3]["passou"], false);
    assert!(p["exigencias"][3]["observado"].is_null());
}

#[test]
fn o_drawdown_e_comparado_como_fracao_do_capital() {
    // 15% de 10.000 é 1.500. 1.500 exatos passam; 1.500,01 não.
    for (dd, esperado) in [(dec!(1500), true), (dec!(1500.01), false)] {
        let p = consultas::porta1(
            &exigencias(),
            consultas::ObservadoPorta1 {
                meses_de_historico: 12,
                operacoes: 140,
                custo_modelado: true,
                profit_factor: Some(dec!(2)),
                max_drawdown: dd,
                capital_inicial: dec!(10000),
            },
        );
        assert_eq!(p["exigencias"][4]["passou"], esperado, "drawdown {dd}");
    }
}

#[test]
fn os_limiares_nao_estao_embutidos_no_codigo() {
    // FR-024: lidos de configuração. Trocar o limiar muda o veredito — se
    // estivesse embutido, este teste não teria como mudá-lo.
    let frouxa = ExigenciasPorta1 {
        profit_factor_minimo: dec!(0.05),
        ..exigencias()
    };
    let obs = consultas::ObservadoPorta1 {
        meses_de_historico: 12,
        operacoes: 140,
        custo_modelado: true,
        profit_factor: Some(dec!(0.07)),
        max_drawdown: dec!(0),
        capital_inicial: dec!(10000),
    };
    assert_eq!(
        consultas::porta1(&exigencias(), obs.clone())["passou"],
        false
    );
    assert_eq!(consultas::porta1(&frouxa, obs)["passou"], true);
}
