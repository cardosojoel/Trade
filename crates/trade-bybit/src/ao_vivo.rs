//! A fonte de velas de uma sessão ao vivo (T040).
//!
//! O coletor percorre um período fechado e termina. Esta acompanha o mercado
//! enquanto ele anda: cada chamada devolve a **próxima vela fechada ainda não
//! entregue**, esperando por ela se preciso.
//!
//! Duas regras que não são detalhe:
//!
//! 1. **Só vela fechada.** No meio de uma vela o preço de fechamento é apenas
//!    o último negócio, e vai mudar. Decidir sobre ele é decidir sobre um
//!    número que ainda não existe — é o mesmo erro que o coletor evita
//!    descartando a vela em formação.
//! 2. **A mais antiga primeiro, nunca a mais recente.** Se o processo ficou
//!    para trás, devolver a vela de agora pularia as do meio, e a estratégia
//!    decidiria sem ter visto o que aconteceu. Atrasado é recuperável; cego
//!    não.

use crate::client::{BybitClient, LIMITE_MAXIMO};
use chrono::{DateTime, Duration, Utc};
use std::time::Duration as EsperaReal;
use trade_domain::{Candle, Interval, Symbol};
use trade_ports::{Clock, LiveCandleSource, MarketError};

pub struct FonteAoVivo<'a> {
    client: &'a BybitClient,
    clock: &'a dyn Clock,
    symbol: Symbol,
    interval: Interval,
    /// Abertura da última vela entregue.
    ultima: Option<DateTime<Utc>>,
    /// Quanto dormir entre sondagens que não trouxeram nada.
    espera: EsperaReal,
    /// Quantas sondagens vazias seguidas antes de declarar a fonte encerrada.
    ///
    /// `None` é o caso real: a sessão acompanha o mercado e não desiste. O
    /// limite existe para que um teste termine, e para que uma corretora muda
    /// por tempo demais não deixe o processo pendurado em silêncio.
    limite_de_sondagens_vazias: Option<u32>,
}

impl<'a> FonteAoVivo<'a> {
    pub fn nova(
        client: &'a BybitClient,
        clock: &'a dyn Clock,
        symbol: Symbol,
        interval: Interval,
    ) -> Self {
        FonteAoVivo {
            client,
            clock,
            symbol,
            interval,
            ultima: None,
            espera: EsperaReal::from_secs(5),
            limite_de_sondagens_vazias: None,
        }
    }

    /// Começa depois desta vela, em vez de da mais recente.
    ///
    /// É por aqui que uma sessão retomada não repete o que já processou.
    pub fn depois_de(mut self, abertura: DateTime<Utc>) -> Self {
        self.ultima = Some(abertura);
        self
    }

    pub fn com_espera(mut self, espera: EsperaReal) -> Self {
        self.espera = espera;
        self
    }

    pub fn desistindo_apos(mut self, sondagens_vazias: u32) -> Self {
        self.limite_de_sondagens_vazias = Some(sondagens_vazias);
        self
    }

    fn passo(&self) -> Duration {
        Duration::minutes(self.interval.minutes())
    }
}

impl LiveCandleSource for FonteAoVivo<'_> {
    fn proxima(&mut self) -> Result<Option<Candle>, MarketError> {
        let passo = self.passo();
        let mut vazias = 0u32;

        loop {
            let agora = self.clock.now();
            // Sem última entregue, busca só o passado imediato: uma sessão
            // que começa agora não tem por que reprocessar o dia inteiro.
            let inicio = self.ultima.map_or(agora - passo * 2, |u| u + passo);

            let pagina =
                self.client
                    .klines(&self.symbol, self.interval, inicio, agora, LIMITE_MAXIMO)?;

            let proxima = pagina
                .candles
                .into_iter()
                .filter(|c| c.open_time + passo <= agora)
                .filter(|c| self.ultima.is_none_or(|u| c.open_time > u))
                .min_by_key(|c| c.open_time);

            if let Some(c) = proxima {
                self.ultima = Some(c.open_time);
                return Ok(Some(c));
            }

            vazias += 1;
            if self
                .limite_de_sondagens_vazias
                .is_some_and(|max| vazias >= max)
            {
                return Ok(None);
            }
            std::thread::sleep(self.espera);
        }
    }
}
