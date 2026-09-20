-- Histórico coletado. É cache: pode ser reconstruído da fonte a qualquer
-- momento, e por isso vive em arquivo separado da auditoria, que não pode.

CREATE TABLE IF NOT EXISTS dataset (
    symbol        TEXT    NOT NULL,
    interval      TEXT    NOT NULL,
    source        TEXT    NOT NULL,   -- procedência: FR-017
    first_open_ms INTEGER NOT NULL,
    last_open_ms  INTEGER NOT NULL,
    collected_at  INTEGER NOT NULL,
    PRIMARY KEY (symbol, interval)
);

-- Preços em TEXT, não REAL: REAL é IEEE-754 e desfaria a exatidão no disco.
-- A chave primária é o que torna a coleta idempotente — um INSERT OR IGNORE de
-- página já gravada é operação vazia, e a retomada de FR-014 sai de graça, sem
-- nenhuma lógica de deduplicação escrita à mão.
CREATE TABLE IF NOT EXISTS candle (
    symbol    TEXT    NOT NULL,
    interval  TEXT    NOT NULL,
    open_ms   INTEGER NOT NULL,
    open      TEXT    NOT NULL,
    high      TEXT    NOT NULL,
    low       TEXT    NOT NULL,
    close     TEXT    NOT NULL,
    volume    TEXT    NOT NULL,
    turnover  TEXT    NOT NULL,
    PRIMARY KEY (symbol, interval, open_ms)
) WITHOUT ROWID;

-- Lacunas conhecidas (FR-016). Registradas em vez de interpoladas: interpolar
-- inventa preço que não existiu, e o backtest não teria como saber.
CREATE TABLE IF NOT EXISTS gap (
    symbol      TEXT    NOT NULL,
    interval    TEXT    NOT NULL,
    from_ms     INTEGER NOT NULL,
    to_ms       INTEGER NOT NULL,
    detected_at INTEGER NOT NULL,
    PRIMARY KEY (symbol, interval, from_ms)
);
