//! Reconciliação entre a posição local e a reportada pela corretora.
//!
//! O Princípio II manda parar diante de "divergência entre posição local e
//! posição reportada pela corretora", e classifica isso como falha de
//! integridade — que exige revisão humana, não retentativa.
//!
//! A pergunta difícil não é "são iguais?". É **quanta diferença é diferença**.
//! Zero absoluto não serve: a taxa cobrada em moeda base deixa resíduo abaixo
//! do passo negociável, e exigir igualdade exata pararia a operação a cada
//! ciclo por um valor que não é vendável. A tolerância é um passo do
//! instrumento — abaixo disso não há ordem possível, e portanto não há
//! divergência que importe.

use rust_decimal::Decimal;
use trade_domain::{Instrumento, Money, Qty};

/// O que a comparação conclui.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Veredito {
    /// Posições coincidem dentro da tolerância.
    Sincronizado,
    /// Divergem além da tolerância. Falha de integridade.
    Divergente {
        local: Qty,
        remota: Qty,
        diferenca: Qty,
    },
    /// Não foi possível saber. **Não é sinônimo de sincronizado**: bloqueia
    /// nova entrada do mesmo jeito que divergência.
    Desconhecido(String),
}

impl Veredito {
    /// Pode abrir posição nova?
    ///
    /// Só quando sincronizado. `Desconhecido` bloqueia porque operar sem saber
    /// a posição é exatamente o que o Princípio II chama de anomalia.
    pub const fn permite_nova_entrada(&self) -> bool {
        matches!(self, Veredito::Sincronizado)
    }

    pub const fn exige_revisao_humana(&self) -> bool {
        matches!(self, Veredito::Divergente { .. })
    }
}

/// Compara quantidade detida local com a reportada.
pub fn comparar(local: Qty, remota: Qty, instrumento: &Instrumento) -> Veredito {
    if local < Decimal::ZERO || remota < Decimal::ZERO {
        return Veredito::Desconhecido(format!(
            "quantidade negativa — local {local}, remota {remota}. Em spot comprado \
             isso não existe, e um número impossível não pode ser base de decisão."
        ));
    }
    let diferenca = (local - remota).abs();
    if diferenca < instrumento.passo_qty() {
        Veredito::Sincronizado
    } else {
        Veredito::Divergente {
            local,
            remota,
            diferenca,
        }
    }
}

/// Compara saldo em caixa, com a mesma lógica e tolerância própria.
///
/// A tolerância aqui é o valor mínimo de ordem: abaixo dele nenhuma ordem
/// existe, e a diferença não muda o que o sistema pode fazer.
pub fn comparar_saldo(local: Money, remoto: Money, instrumento: &Instrumento) -> Veredito {
    let diferenca = (local - remoto).abs();
    if diferenca < instrumento.valor_minimo_ordem() {
        Veredito::Sincronizado
    } else {
        Veredito::Divergente {
            local,
            remota: remoto,
            diferenca,
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

    #[test]
    fn iguais_estao_sincronizados() {
        assert_eq!(
            comparar(dec!(0.5), dec!(0.5), &btcusdt()),
            Veredito::Sincronizado
        );
    }

    #[test]
    fn residuo_abaixo_do_passo_nao_e_divergencia() {
        // Este é o ponto. A taxa em moeda base deixa resíduo abaixo do passo a
        // cada compra; exigir igualdade exata pararia a operação todo ciclo por
        // um valor que não é vendável.
        let v = comparar(dec!(0.000614385), dec!(0.000614), &btcusdt());
        assert_eq!(v, Veredito::Sincronizado);
        assert!(v.permite_nova_entrada());
    }

    #[test]
    fn diferenca_de_um_passo_ja_e_divergencia() {
        // Um passo é a menor quantidade negociável: a partir dele, a diferença
        // é uma ordem que existe de um lado e não do outro.
        let v = comparar(dec!(0.000615), dec!(0.000614), &btcusdt());
        assert!(matches!(v, Veredito::Divergente { .. }), "{v:?}");
        assert!(v.exige_revisao_humana());
        assert!(!v.permite_nova_entrada());
    }

    #[test]
    fn a_divergencia_carrega_os_dois_lados_e_a_diferenca() {
        // Sem os três números, o registro diz que houve divergência e não diz
        // qual — e reconstituir depois vira adivinhação.
        match comparar(dec!(1), dec!(0.5), &btcusdt()) {
            Veredito::Divergente {
                local,
                remota,
                diferenca,
            } => {
                assert_eq!(local, dec!(1));
                assert_eq!(remota, dec!(0.5));
                assert_eq!(diferenca, dec!(0.5));
            }
            outro => panic!("esperava divergência, veio {outro:?}"),
        }
    }

    #[test]
    fn divergir_para_mais_ou_para_menos_e_igualmente_grave() {
        let a = comparar(dec!(1), dec!(0.5), &btcusdt());
        let b = comparar(dec!(0.5), dec!(1), &btcusdt());
        assert!(a.exige_revisao_humana() && b.exige_revisao_humana());
    }

    #[test]
    fn quantidade_negativa_e_desconhecido_e_nao_divergencia() {
        // Em spot comprado posição negativa não existe. Um número impossível
        // não é uma divergência a medir: é um dado que não serve de base.
        let v = comparar(dec!(-1), dec!(0.5), &btcusdt());
        assert!(matches!(v, Veredito::Desconhecido(_)), "{v:?}");
        assert!(!v.permite_nova_entrada());
    }

    #[test]
    fn desconhecido_bloqueia_como_divergencia_mas_nao_exige_revisao() {
        // Bloqueia entrada nova — operar sem saber a posição é anomalia. Mas
        // não é o mesmo que divergência confirmada: a saída é consultar de
        // novo, não parar para revisão.
        let v = Veredito::Desconhecido("sem resposta".into());
        assert!(!v.permite_nova_entrada());
        assert!(!v.exige_revisao_humana());
    }

    #[test]
    fn saldo_tolera_ate_o_valor_minimo_de_ordem() {
        let i = btcusdt();
        assert_eq!(
            comparar_saldo(dec!(100), dec!(99), &i),
            Veredito::Sincronizado
        );
        assert!(comparar_saldo(dec!(100), dec!(94), &i).exige_revisao_humana());
    }
}
