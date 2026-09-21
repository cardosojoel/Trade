//! O corpo único de recusa (FR-023).
//!
//! Toda recusa sai no mesmo formato, com código próprio. É o que permite à
//! tela distinguir uma da outra **sem interpretar texto** — e interpretar
//! texto é o que quebra no dia em que alguém melhora a redação de uma
//! mensagem.

use serde_json::{Value, json};

/// Por que a requisição foi recusada.
///
/// Enum exaustivo de propósito, como o `LimitBreach` da camada de risco:
/// acrescentar um motivo quebra a compilação em todo lugar que trate recusas.
/// Um motivo novo que passe despercebido num `match` é uma recusa sem código.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motivo {
    /// Início de execução sem o modo declarado. O Princípio I não admite que
    /// a ausência de configuração resulte em algo que toque dinheiro.
    ModoAusente,
    /// Pediu `live`. A promoção para capital real é ato humano registrado, e
    /// não cabe numa requisição HTTP.
    ModoRecusado,
    TokenAusente,
    TokenInvalido,
    /// `Origin` ou `Referer` apontando para outro lugar.
    OrigemRecusada,
    HistoricoInsuficiente,
    ExecucaoDesconhecida,
    /// O `runs.db` está ilegível. É indisponibilidade, não ausência: dizer
    /// "não existe" faria a tela concluir que nunca houve execução.
    RegistroIndisponivel,
}

impl Motivo {
    pub const fn codigo(self) -> &'static str {
        match self {
            Motivo::ModoAusente => "modo_ausente",
            Motivo::ModoRecusado => "modo_recusado",
            Motivo::TokenAusente => "token_ausente",
            Motivo::TokenInvalido => "token_invalido",
            Motivo::OrigemRecusada => "origem_recusada",
            Motivo::HistoricoInsuficiente => "historico_insuficiente",
            Motivo::ExecucaoDesconhecida => "execucao_desconhecida",
            Motivo::RegistroIndisponivel => "registro_indisponivel",
        }
    }

    /// Status HTTP, conforme a tabela do FR-023.
    pub const fn status(self) -> u16 {
        match self {
            Motivo::ModoAusente => 400,
            Motivo::ModoRecusado | Motivo::OrigemRecusada => 403,
            Motivo::TokenAusente | Motivo::TokenInvalido => 401,
            Motivo::HistoricoInsuficiente => 409,
            Motivo::ExecucaoDesconhecida => 404,
            Motivo::RegistroIndisponivel => 503,
        }
    }

    /// Todos, para que um teste possa varrer o conjunto inteiro.
    pub const fn todos() -> [Motivo; 8] {
        [
            Motivo::ModoAusente,
            Motivo::ModoRecusado,
            Motivo::TokenAusente,
            Motivo::TokenInvalido,
            Motivo::OrigemRecusada,
            Motivo::HistoricoInsuficiente,
            Motivo::ExecucaoDesconhecida,
            Motivo::RegistroIndisponivel,
        ]
    }
}

/// Uma recusa pronta para virar resposta.
#[derive(Debug, Clone)]
pub struct Recusa {
    motivo: Motivo,
    mensagem: String,
    comando: Option<String>,
    o_que_falta: Option<Value>,
}

impl Recusa {
    pub fn nova(motivo: Motivo, mensagem: impl Into<String>) -> Self {
        Recusa {
            motivo,
            mensagem: mensagem.into(),
            comando: None,
            o_que_falta: None,
        }
    }

    /// O comando que resolve, quando existe um (FR-022).
    pub fn com_comando(mut self, comando: impl Into<String>) -> Self {
        self.comando = Some(comando.into());
        self
    }

    pub fn com_o_que_falta(mut self, v: Value) -> Self {
        self.o_que_falta = Some(v);
        self
    }

    pub const fn status(&self) -> u16 {
        self.motivo.status()
    }

    /// O corpo, sempre com os mesmos campos.
    ///
    /// `comando` e `o_que_falta` vêm **presentes e nulos** quando não há o que
    /// dizer — mesma regra do FR-006. Omitir faria a tela não distinguir "não
    /// há comando que resolva" de "esqueceram de mandar".
    pub fn corpo(&self) -> Value {
        json!({
            "erro": {
                "codigo": self.motivo.codigo(),
                "mensagem": self.mensagem,
                "o_que_falta": self.o_que_falta,
                "comando": self.comando,
            }
        })
    }
}
