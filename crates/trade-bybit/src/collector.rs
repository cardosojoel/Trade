//! Coleta paginada do histórico.

use crate::backoff::espera;
use crate::client::{BybitClient, LIMITE_MAXIMO};
use crate::errors::is_transient;
use crate::gaps;
use chrono::{DateTime, Duration, Utc};
use trade_domain::{Candle, Gap, Interval, Symbol};
use trade_ports::{CandleRepository, MarketError};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct CollectOutcome {
    pub paginas: u32,
    pub velas_novas: usize,
    pub velas_ja_existentes: usize,
    pub descartadas_em_formacao: usize,
    pub lacunas: Vec<Gap>,
    pub tentativas_extras: u32,
}

impl CollectOutcome {
    pub fn velas_totais(&self) -> usize {
        self.velas_novas + self.velas_ja_existentes
    }
}

pub struct Collector<'a, R: CandleRepository> {
    client: &'a BybitClient,
    repo: &'a mut R,
    max_retries: u32,
    dormir: bool,
}

impl<'a, R: CandleRepository> Collector<'a, R> {
    pub fn new(client: &'a BybitClient, repo: &'a mut R, max_retries: u32) -> Self {
        Collector {
            client,
            repo,
            max_retries,
            dormir: true,
        }
    }

    /// Desliga a espera entre tentativas. Só para teste — sem isto, verificar o
    /// caminho de retentativa custaria segundos de relógio real.
    pub fn sem_espera(mut self) -> Self {
        self.dormir = false;
        self
    }

    /// Coleta o período, retomando de onde parou.
    ///
    /// A retomada não precisa de lógica de deduplicação: a chave primária
    /// `(symbol, interval, open_ms)` com `INSERT OR IGNORE` torna regravar uma
    /// página uma operação vazia. FR-014 sai do esquema, não do código.
    pub fn collect(
        &mut self,
        symbol: &Symbol,
        interval: Interval,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        agora: DateTime<Utc>,
    ) -> Result<CollectOutcome, MarketError> {
        let passo = Duration::minutes(interval.minutes());
        let mut out = CollectOutcome::default();

        // Retoma do fim do que já existe, quando existe e está dentro do pedido.
        let mut cursor = match self.repo.coverage(symbol, interval) {
            Ok(Some(c)) if c.last >= from && c.last < to => c.last + passo,
            _ => from,
        };

        let mut ultima_gravada: Option<DateTime<Utc>> = None;

        // `end` da fonte é inclusivo; o `to` deste método é exclusivo, como no
        // resto do sistema. Um milissegundo antes reconcilia os dois sem
        // depender de as velas estarem alinhadas a um múltiplo do passo.
        let fim_inclusivo = to - Duration::milliseconds(1);

        while cursor < to {
            // A janela de cada página é limitada ao que cabe em uma
            // requisição. A fonte devolve as `limit` velas **mais recentes**
            // da janela pedida, não as mais antigas: pedir o período inteiro
            // com limite de mil traria o fim dele e deixaria o começo para
            // trás — silenciosamente, porque a resposta parece completa.
            // `LIMITE_MAXIMO - 1` passos, e não `LIMITE_MAXIMO`: com `end`
            // inclusivo, uma janela de mil passos contém 1001 velas, e o
            // limite descartaria a mais antiga — a que estávamos buscando.
            let largura =
                passo * i32::try_from(LIMITE_MAXIMO.saturating_sub(1)).unwrap_or(i32::MAX);
            let fim_da_pagina = fim_inclusivo.min(cursor + largura);

            let pagina =
                self.buscar_com_retentativa(symbol, interval, cursor, fim_da_pagina, &mut out)?;

            if pagina.is_empty() {
                // Janela vazia não significa fim do período: pode ser um
                // trecho sem negócio. Avança para a próxima janela.
                if fim_da_pagina >= fim_inclusivo {
                    break;
                }
                cursor = fim_da_pagina + passo;
                continue;
            }

            // A vela em formação é descartada. A documentação diz que
            // `closePrice` é "the last traded price when the candle is not
            // closed": gravá-la significa gravar um fechamento que ainda vai
            // mudar. Duas coletas do mesmo período produziriam históricos
            // diferentes, quebrando o determinismo — sem erro visível.
            let antes = pagina.len();
            let fechadas: Vec<Candle> = pagina
                .into_iter()
                .filter(|c| c.open_time + passo <= agora)
                .collect();
            out.descartadas_em_formacao += antes - fechadas.len();

            if fechadas.is_empty() {
                // Só restavam velas em formação: não há mais histórico fechado.
                break;
            }

            if let Some(g) = gaps::entre(ultima_gravada, fechadas[0].open_time, interval) {
                out.lacunas.push(g);
            }
            out.lacunas.extend(gaps::detectar(&fechadas, interval));

            let novas = self
                .repo
                .upsert_page(symbol, interval, &fechadas)
                .map_err(|e| MarketError::Storage(e.to_string()))?;

            out.paginas += 1;
            out.velas_novas += novas;
            out.velas_ja_existentes += fechadas.len() - novas;

            let ultima = fechadas[fechadas.len() - 1].open_time;
            ultima_gravada = Some(ultima);
            cursor = (ultima + passo).max(cursor + passo);
        }

        if !out.lacunas.is_empty() {
            self.repo
                .record_gaps(symbol, interval, &out.lacunas)
                .map_err(|e| MarketError::Storage(e.to_string()))?;
        }

        Ok(out)
    }

    fn buscar_com_retentativa(
        &self,
        symbol: &Symbol,
        interval: Interval,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        out: &mut CollectOutcome,
    ) -> Result<Vec<Candle>, MarketError> {
        let mut tentativa = 0u32;

        loop {
            tentativa += 1;
            match self
                .client
                .klines(symbol, interval, start, end, LIMITE_MAXIMO)
            {
                Ok(p) => return Ok(p.candles),

                // Transitória: retenta sozinha, sem interromper nada e sem
                // pedir permissão a ninguém (FR-024a). Esgotado o teto, o erro
                // sobe — e quem está acima o trata como falha de integridade.
                Err(e) if is_transient(&e) && tentativa <= self.max_retries => {
                    out.tentativas_extras += 1;
                    if self.dormir {
                        std::thread::sleep(espera(tentativa));
                    }
                }

                Err(e) => return Err(e),
            }
        }
    }
}
