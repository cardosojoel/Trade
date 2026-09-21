//! Encerramento limpo por sinal do sistema (T031).
//!
//! A decisão 033 do Jev pôs o tratador de sinal **aqui**, e não na crate do
//! laço: o laço consulta uma bandeira a cada volta e não precisa saber o que a
//! levantou. Assim ele fica testável sem sinal, e a crate isolada não ganha
//! dependência de sistema operacional.
//!
//! Por que cooperativo, e não morrer no sinal: um processo que sai dentro do
//! tratador não roda destrutor nenhum, e o lote de eventos que ainda estava em
//! memória some. A bandeira deixa a volta corrente terminar, gravar o que
//! aconteceu nela e só então sair — que é o que "sem perder evento" quer
//! dizer.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// Instala o tratador de SIGINT e SIGTERM e devolve a bandeira que ele
/// levanta.
///
/// Só pode ser chamado uma vez por processo; a segunda chamada devolve erro,
/// e devolver erro é melhor que substituir em silêncio um tratador que alguém
/// instalou por um motivo.
pub fn instalar() -> Result<Arc<AtomicBool>, ctrlc::Error> {
    let bandeira = Arc::new(AtomicBool::new(false));
    let para_o_tratador = Arc::clone(&bandeira);
    ctrlc::set_handler(move || {
        para_o_tratador.store(true, std::sync::atomic::Ordering::SeqCst);
    })?;
    Ok(bandeira)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn a_bandeira_nasce_abaixada_e_o_tratador_instala_uma_vez_so() {
        let b = instalar().expect("primeira instalação");
        assert!(
            !b.load(Ordering::SeqCst),
            "nada foi sinalizado ainda: a sessão tem de começar podendo rodar"
        );
        assert!(
            instalar().is_err(),
            "a segunda instalação falha em vez de substituir a primeira em silêncio"
        );
    }
}
