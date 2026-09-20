-- Execuções e auditoria. Insubstituível: ao contrário do histórico, não pode
-- ser reconstruído de fonte nenhuma.

CREATE TABLE IF NOT EXISTS run (
    run_id               TEXT    PRIMARY KEY,
    mode                 TEXT    NOT NULL,
    symbol               TEXT    NOT NULL,
    interval             TEXT    NOT NULL,
    from_ms              INTEGER NOT NULL,
    to_ms                INTEGER NOT NULL,
    initial_capital      TEXT    NOT NULL,
    limits_json          TEXT    NOT NULL,   -- a cerca sob a qual correu: FR-025
    fees_json            TEXT    NOT NULL,
    strategy             TEXT    NOT NULL,
    strategy_params_json TEXT    NOT NULL,
    started_at           INTEGER NOT NULL,
    ended_at             INTEGER,
    outcome              TEXT,               -- completed | halted | capital_exhausted
    halt_reason          TEXT                -- FR-024c
);

-- seq dá ordem TOTAL aos eventos dentro de uma execução, inclusive entre dois
-- do mesmo instante simulado. Sem ele, a cadeia sinal → ordem → decisão →
-- preenchimento teria um elo ambíguo dentro da mesma vela.
CREATE TABLE IF NOT EXISTS audit_event (
    run_id       TEXT    NOT NULL,
    seq          INTEGER NOT NULL,
    at_ms        INTEGER NOT NULL,
    kind         TEXT    NOT NULL,
    payload_json TEXT    NOT NULL,
    PRIMARY KEY (run_id, seq)
) WITHOUT ROWID;

CREATE INDEX IF NOT EXISTS idx_audit_kind ON audit_event (run_id, kind);

CREATE TABLE IF NOT EXISTS trade (
    run_id      TEXT    NOT NULL,
    trade_seq   INTEGER NOT NULL,
    entry_ms    INTEGER NOT NULL,
    entry_price TEXT    NOT NULL,
    exit_ms     INTEGER NOT NULL,
    exit_price  TEXT    NOT NULL,
    qty         TEXT    NOT NULL,
    fees        TEXT    NOT NULL,
    pnl         TEXT    NOT NULL,
    PRIMARY KEY (run_id, trade_seq)
);

-- profit_factor é a única métrica que aceita NULL: sem operação perdedora ele é
-- indefinido, e gravar infinito produziria um número que induz erro na Porta 1.
CREATE TABLE IF NOT EXISTS metrics (
    run_id         TEXT PRIMARY KEY,
    profit_factor  TEXT,
    max_drawdown   TEXT    NOT NULL,
    trade_count    INTEGER NOT NULL,
    net_result     TEXT    NOT NULL,
    gross_profit   TEXT    NOT NULL,
    gross_loss     TEXT    NOT NULL,
    total_fees     TEXT    NOT NULL,
    total_slippage TEXT    NOT NULL
);
