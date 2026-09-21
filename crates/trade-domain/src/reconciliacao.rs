//! O veredito de uma reconciliação.
//!
//! O tipo vive aqui, e não na crate que o calcula, porque o registro de
//! auditoria precisa nomeá-lo e `trade-domain` não pode depender de
//! `trade-paper`. A **lógica** da comparação continua onde está: é ela que
//! conhece a tolerância e o instrumento.

use crate::Qty;

/// O que a comparação entre o local e o reportado conclui.
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

    /// Palavra escrita, para o registro e para quem lê.
    ///
    /// Os três casos têm nome próprio de propósito: `REQ-UI-049` exige que a
    /// tela os distinga **com palavra escrita**, e um booleano os colapsaria
    /// em dois.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Veredito::Sincronizado => "sincronizado",
            Veredito::Divergente { .. } => "divergente",
            Veredito::Desconhecido(_) => "desconhecido",
        }
    }
}
