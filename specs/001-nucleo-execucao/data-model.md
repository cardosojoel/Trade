# Phase 1 — Data Model: Núcleo de Execução e Risco

**Feature**: 001-nucleo-execucao | **Date**: 2026-09-20

Todos os valores monetários e de quantidade são `Decimal` (`rust_decimal`). Nenhum
campo financeiro é ponto flutuante, em nenhum ponto do modelo (R-002).

Todos os instantes são UTC. O tipo é `DateTime<Utc>`; na persistência, milissegundos
desde a época como `INTEGER`.

---

## Tipos do domínio (`trade-domain`)

Esta crate não tem nenhuma dependência de infraestrutura — nem banco, nem HTTP, nem
relógio do sistema. É o que permite testá-la inteira sem rede (SC-008).

### `ExecutionMode`

```rust
pub enum ExecutionMode { Backtest }
```

- **Sem `Default`, deliberadamente.** FR-001 proíbe valor padrão; em Rust, a forma de
  proibir é não implementar o trait que o forneceria.
- Apenas `Backtest` existe nesta feature. As strings `paper` e `live` são reconhecidas
  no parse **apenas para serem recusadas com mensagem explícita** (FR-003) — recusar
  com "modo desconhecido" seria pior do que dizer "ainda não implementado".

### `Candle`

| Campo | Tipo | Regra |
|---|---|---|
| `open_time` | `DateTime<Utc>` | início do intervalo; chave junto com símbolo e granularidade |
| `open`, `high`, `low`, `close` | `Decimal` | todos > 0; `low` ≤ `open`, `close` ≤ `high`; `low` ≤ `high` |
| `volume` | `Decimal` | ≥ 0 |
| `turnover` | `Decimal` | ≥ 0 |

A validação das relações entre máxima e mínima não é preciosismo: uma vela que viole
essas relações indica dado corrompido, e alimentar o motor com ela produz P&L sem
significado. É checada na fronteira de leitura.

### `Signal`

| Campo | Tipo | Nota |
|---|---|---|
| `at` | `DateTime<Utc>` | instante da vela que o originou |
| `intent` | `Intent` | `Buy` \| `Sell` \| `Hold` |
| `inputs` | `SignalInputs` | **os dados que produziram o sinal** — FR-034 |

`inputs` é o campo que satisfaz a exigência de reconstituição: registrar apenas "houve
compra" não permite auditar; registrar "média curta 63.412,10 cruzou a longa 63.180,55"
permite. Para a estratégia de referência, contém os valores das duas médias e os
parâmetros vigentes.

### `Order`

| Campo | Tipo | Regra |
|---|---|---|
| `id` | `OrderId` | sequencial dentro da execução; determinístico |
| `signal_ref` | `SignalId` | o sinal que a originou — elo da cadeia de auditoria |
| `side` | `Side` | `Buy` \| `Sell` |
| `qty` | `Decimal` | > 0 |
| `at` | `DateTime<Utc>` | instante da emissão |

Não existe variante de venda a descoberto: `Sell` só é válida contra quantidade detida
(FR-005), e a validação vive na camada de risco, não na estratégia.

### `RiskDecision`

| Campo | Tipo | Nota |
|---|---|---|
| `order_ref` | `OrderId` | |
| `verdict` | `Verdict` | `Accepted` \| `Rejected(LimitBreach)` |
| `limits_snapshot` | `RiskLimits` | **os limites vigentes no momento da avaliação** |
| `state_snapshot` | `RiskState` | perda do dia, exposição, contagem de ordens |

Os dois *snapshots* existem porque auditar uma recusa exige saber contra o que ela foi
avaliada. Sem eles, um registro de recusa em que o limite mudou depois é indecifrável.

`LimitBreach` é enum exaustivo: `MaxPositionSize` | `MaxTotalExposure` |
`DailyLossReached` | `MaxOrdersPerWindow` | `InsufficientBalance` |
`SellExceedsHoldings` | `KillSwitchEngaged`. Exaustivo importa: acrescentar um limite
no futuro quebra a compilação em todo lugar que trate recusas, o que é o
comportamento desejado.

### `Fill`

| Campo | Tipo | Regra |
|---|---|---|
| `order_ref` | `OrderId` | |
| `price` | `Decimal` | preço efetivo, **já com slippage aplicado** |
| `qty` | `Decimal` | > 0; pode ser menor que a solicitada (preenchimento parcial) |
| `fee` | `Decimal` | ≥ 0, **discriminada** — FR-027 |
| `slippage` | `Decimal` | diferença entre preço de referência e efetivo, **discriminada** |
| `at` | `DateTime<Utc>` | |

Taxa e slippage são campos próprios, não embutidos no preço. FR-027 exige discriminação,
e há uma razão prática: quando uma estratégia lucrativa no papel dá prejuízo na
simulação, a primeira pergunta é quanto foi custo de transação — e essa pergunta precisa
ser respondível por consulta, não por dedução.

### `Position`

| Campo | Tipo | Invariante |
|---|---|---|
| `qty` | `Decimal` | **≥ 0 sempre** — SC-010. Testada por propriedade, não só por exemplo |
| `avg_price` | `Decimal` | > 0 quando `qty` > 0 |
| `realized_pnl` | `Decimal` | acumulado das operações fechadas |

Transições: `Flat --Buy--> Long`; `Long --Buy--> Long` (preço médio recalculado);
`Long --Sell parcial--> Long`; `Long --Sell total--> Flat` (gera `Trade`).
Não há transição para posição vendida — o tipo não a representa.

### `Trade`

Um ciclo fechado: instante e preço de entrada, instante e preço de saída, quantidade,
taxas totais, resultado realizado. É a unidade do extrato e a base das métricas.

### `RiskLimits`

| Campo | Tipo | Origem |
|---|---|---|
| `max_daily_loss` | `Decimal` | constitution §Restrições — valor inicial 2% do capital |
| `max_position_size` | `Decimal` | 10% do capital |
| `max_total_exposure` | `Decimal` | 20% do capital |
| `max_orders_per_window` | `u32` + `window` | |
| `max_transient_retries` | `u32` | FR-024a |

Todos configuráveis (FR-025), nenhum embutido como constante. Gravados junto ao
resultado de cada execução, porque comparar duas execuções exige saber sob que cerca
cada uma correu.

### `RunMetrics`

`profit_factor`, `max_drawdown`, `trade_count`, `net_result`, `gross_profit`,
`gross_loss`, `total_fees`, `total_slippage`.

Regra de borda: `profit_factor` com `gross_loss` igual a zero é **indefinido**, não
infinito. Representado como `Option<Decimal>` — uma execução sem nenhuma operação
perdedora não tem profit factor, e fingir que tem produziria um número que induz erro
na Porta 1 de promoção.

---

## Esquema de persistência

### `data/market.db` — histórico (cache reconstruível)

```sql
CREATE TABLE dataset (
    symbol        TEXT    NOT NULL,
    interval      TEXT    NOT NULL,
    source        TEXT    NOT NULL,   -- 'bybit-v5-spot'
    first_open_ms INTEGER NOT NULL,
    last_open_ms  INTEGER NOT NULL,
    collected_at  INTEGER NOT NULL,   -- FR-017: procedência
    PRIMARY KEY (symbol, interval)
);

CREATE TABLE candle (
    symbol    TEXT    NOT NULL,
    interval  TEXT    NOT NULL,
    open_ms   INTEGER NOT NULL,
    open      TEXT    NOT NULL,       -- Decimal como texto: exato, sem float
    high      TEXT    NOT NULL,
    low       TEXT    NOT NULL,
    close     TEXT    NOT NULL,
    volume    TEXT    NOT NULL,
    turnover  TEXT    NOT NULL,
    PRIMARY KEY (symbol, interval, open_ms)
) WITHOUT ROWID;

CREATE TABLE gap (                    -- FR-016: lacunas conhecidas
    symbol     TEXT    NOT NULL,
    interval   TEXT    NOT NULL,
    from_ms    INTEGER NOT NULL,
    to_ms      INTEGER NOT NULL,
    detected_at INTEGER NOT NULL,
    PRIMARY KEY (symbol, interval, from_ms)
);
```

Duas decisões de esquema merecem justificativa:

**Preços como `TEXT`, não `REAL`.** O tipo `REAL` do SQLite é IEEE-754 de 64 bits —
gravar `Decimal` nele desfaz toda a garantia de R-002 na primeira ida ao disco. Texto
preserva o valor exatamente como veio da Bybit, que também o envia como string. Custa
espaço e custa comparação numérica em SQL; ambos aceitáveis diante de correção
monetária.

**`PRIMARY KEY (symbol, interval, open_ms)` com `WITHOUT ROWID`.** É o que torna a
coleta idempotente: `INSERT OR IGNORE` de uma página já gravada é operação vazia, e a
retomada de FR-014 sai de graça, sem código de deduplicação. `WITHOUT ROWID` evita o
índice secundário redundante numa tabela que só é acessada por essa chave.

### `data/runs.db` — execuções e auditoria (insubstituível)

```sql
CREATE TABLE run (
    run_id        TEXT    PRIMARY KEY,   -- ULID: ordenável por tempo
    mode          TEXT    NOT NULL,      -- 'backtest'
    symbol        TEXT    NOT NULL,
    interval      TEXT    NOT NULL,
    from_ms       INTEGER NOT NULL,
    to_ms         INTEGER NOT NULL,
    initial_capital TEXT  NOT NULL,
    limits_json   TEXT    NOT NULL,      -- FR-025: cerca sob a qual correu
    fees_json     TEXT    NOT NULL,      -- taxa e slippage configurados
    strategy      TEXT    NOT NULL,
    strategy_params_json TEXT NOT NULL,
    started_at    INTEGER NOT NULL,
    ended_at      INTEGER,
    outcome       TEXT,                  -- 'completed' | 'halted' | 'capital_exhausted'
    halt_reason   TEXT                   -- FR-024c
);

CREATE TABLE audit_event (
    run_id     TEXT    NOT NULL,
    seq        INTEGER NOT NULL,         -- ordem total dentro da execução
    at_ms      INTEGER NOT NULL,         -- instante simulado, UTC
    kind       TEXT    NOT NULL,         -- signal|order|risk_decision|fill|
                                         -- halt|resume|anomaly|state_transition
    payload_json TEXT  NOT NULL,
    PRIMARY KEY (run_id, seq)
) WITHOUT ROWID;

CREATE TABLE trade (
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

CREATE TABLE metrics (
    run_id        TEXT PRIMARY KEY,
    profit_factor TEXT,                  -- NULL quando indefinido
    max_drawdown  TEXT NOT NULL,
    trade_count   INTEGER NOT NULL,
    net_result    TEXT NOT NULL,
    gross_profit  TEXT NOT NULL,
    gross_loss    TEXT NOT NULL,
    total_fees    TEXT NOT NULL,
    total_slippage TEXT NOT NULL
);
```

O campo `seq` em `audit_event` é o que torna a reconstituição possível: ele dá **ordem
total** aos eventos dentro de uma execução, inclusive entre eventos do mesmo
milissegundo simulado. Sem ele, dois eventos na mesma vela ficariam sem ordem definida,
e a cadeia sinal → ordem → decisão → preenchimento não seria reconstituível.

`profit_factor` é a única coluna de métrica que aceita `NULL`, pelo motivo dado acima.

### Consulta de reconstituição (US4, SC-005)

A cadeia que prova o Princípio IV, em uma consulta:

```sql
SELECT seq, at_ms, kind, payload_json
FROM audit_event
WHERE run_id = ?
ORDER BY seq;
```

Partindo de uma operação do extrato, o `payload_json` do `fill` traz o `order_ref`; o
do `order` traz o `signal_ref`; o do `signal` traz os `inputs`. Quatro saltos, tudo em
SQL, sem reexecutar nada — abrível no DBeaver.
