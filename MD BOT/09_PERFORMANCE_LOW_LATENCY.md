# 09 — Performance & Low-Latency — Bybit-Calibrated

**Status:** normativo · **Versão:** 1.1 · **Atualizado em:** 2026-09-20  
**Domínio de requisitos:** `REQ-PERF-*`  
**Vocabulário:** [`00_GLOSSARIO.md`](00_GLOSSARIO.md)

---

## Objetivo

Minimizar a latência ponta a ponta e, principalmente, tail latency e jitter, sem confundir otimização interna do Rust com latência efetiva de mercado/exchange.

## Princípio

```text
latência interna
+
rede
+
infraestrutura Bybit
+
matching/execution
```

O objetivo não é simplesmente atingir o menor número de microssegundos dentro do processo.

## Arquitetura de baixa latência

```text
Public Market Data
        ↓
RAM Market State
        ↓
Feature Update
        ↓
Pattern Matching
        ↓
Probability
        ↓
Expected Value
        ↓
Risk
        ↓
Order Builder
        ↓
WebSocket Order Entry
        ↓
Bybit
```

A Bybit disponibiliza WebSocket Order Entry em `/v5/trade` para create/amend/cancel. [Bybit WebSocket Trade](https://bybit-exchange.github.io/docs/v5/websocket/trade/guideline)

## Emergency Path

```text
Position/Risk Event
       ↓
Hard Risk Check
       ↓
Immediate Exit
       ↓
WebSocket Order Entry
       ↓
execution.fast
```

**REQ-PERF-001** O caminho de emergência MUST NOT aguardar:

```text
SQLite
learning
analytics
blocking logs
pattern retraining
```

## SLO 1 — Internal Decision Path

```text
Market Event Received
        ↓
Decision Completed
```

**REQ-PERF-002** Meta do caminho interno de decisão:

| Métrica | SLO |
|---|---:|
| p50 | ≤ 100 µs |
| p95 | ≤ 250 µs |
| p99 | ≤ 500 µs |
| p99.9 | ≤ 1 ms |

Esses valores são metas de engenharia, não garantias da Bybit.

## SLO 2 — Emergency Internal Path

```text
Hard Risk Trigger
        ↓
Order Ready
```

**REQ-PERF-003** Meta do caminho interno de emergência:

| Métrica | SLO |
|---|---:|
| p50 | ≤ 50 µs |
| p95 | ≤ 100 µs |
| p99 | ≤ 250 µs |
| p99.9 | ≤ 500 µs |

## SLO 3 — Market Data

Não impor uma meta fixa de microssegundos ao feed.

Medir:

\[
L_{market}=T_{receive}-T_{exchange}
\]

e também:

```text
sequence integrity
data age
jitter
burst handling
```

A frequência de atualização do orderbook depende da profundidade assinada; para linear/inverse, a documentação atual informa 10 ms para Level 1, 20 ms para Level 50 e 100 ms para Level 200. O trade stream é real-time. [Bybit Orderbook](https://bybit-exchange.github.io/docs/v5/websocket/public/orderbook) [Bybit Trade](https://bybit-exchange.github.io/docs/v5/websocket/public/trade)

## SLO 4 — Execution

Separar:

```text
decision → submit
submit → ACK
ACK → order event
submit → execution.fast
submit → confirmed execution
```

Não definir inicialmente um SLO absoluto de `fill < X ms`.

Primeiro estabelecer baseline real em Testnet e, posteriormente, ambiente controlado de produção.

## Execution Fast

A Bybit informa que `execution.fast` reduz significativamente a latência em relação ao stream de execution original, porém entrega somente `execType=Trade`. [Bybit Fast Execution](https://bybit-exchange.github.io/docs/v5/websocket/private/fast-execution)

Arquitetura:

```text
execution.fast → low-latency fill signal
execution      → complete execution state
order stream   → order lifecycle
```

## SLO 5 — Rate Limit

Não assumir um valor universal.

**REQ-PERF-004** O rate limit MUST ser monitorado dinamicamente pelos headers,
nunca presumido:

```text
X-Bapi-Limit
X-Bapi-Limit-Status
X-Bapi-Limit-Reset-Timestamp
```

A documentação do WebSocket Trade informa também limite IP de 3000 requests/s para o serviço WS Trade e códigos específicos de proteção/frequência. [Bybit WebSocket Trade](https://bybit-exchange.github.io/docs/v5/websocket/trade/guideline)

Política interna:

```text
headroom ≥ 30% → NORMAL
headroom 10–30% → WARNING
headroom < 10% → THROTTLE
limit violation → CIRCUIT BREAKER
```

## SLO 6 — Memory

**REQ-PERF-005** Memória:

```text
unbounded growth = FAIL
OOM = FAIL
queue overflow = FAIL
```

Durante teste de 24h:

```text
RSS growth < 1%
```

como meta inicial.

## SLO 7 — CPU

**REQ-PERF-006** CPU em operação normal:

```text
< 50%
```

Stress:

```text
< 80%
```

O objetivo é preservar margem para bursts.

## Hot Path

**REQ-PERF-007** O hot path MUST NOT conter:

```text
SQLite
blocking I/O
synchronous logging
unnecessary allocations
large locks
full-history scans
```

Preferir:

```text
RAM
preallocation
incremental calculations
ring buffers
bounded queues
minimal cloning
```

## Latency Budget

Dividir:

```text
Market Data
Feature Engine
Pattern Matching
Probability
EV
Risk
Order Construction
Network OUT
Bybit
Network IN
Execution
```

**REQ-PERF-008** Os budgets por estágio MUST ser obtidos por profiling e MUST
NOT ser presumidos.

## Fórmulas

\[
L_{decision}=T_{decision}-T_{market\_received}
\]

\[
L_{submit}=T_{ack}-T_{submit}
\]

\[
L_{fastfill}=T_{execution.fast}-T_{submit}
\]

\[
L_{fill}=T_{confirmed\_fill}-T_{submit}
\]

\[
L_{E2E}=T_{confirmed\_fill}-T_{market\_event}
\]

## Benchmark Protocol

### 1. Microbenchmark

Testar:

```text
feature calculations
pattern matching
probability
EV
risk
serialization
```

Medir:

```text
p50/p95/p99/p99.9/max
```

### 2. Pipeline

Executar:

```text
market → features → pattern → probability → EV → risk → order
```

sem rede.

### 3. Stress

Executar bursts progressivos até saturação.

### 4. Realistic Burst

Incluir:

```text
single message
→ up to 1024 trades
```

conforme comportamento documentado do trade stream da Bybit. [Bybit Trade](https://bybit-exchange.github.io/docs/v5/websocket/public/trade)

### 5. Fault Injection

Testar:

```text
disconnect
latency
packet loss
duplicate
reorder
missing sequence
exchange delay
CPU pressure
memory pressure
SQLite contention
```

### 6. Exchange Benchmark

Testar em Testnet:

```text
WS Trade
ACK
order stream
execution.fast
execution
```

### 7. Shadow Trading

Usar mercado real sem enviar ordens reais.

## Critério de aprovação

**REQ-PERF-009** Nenhuma promoção sem que todos estes critérios passem:

```text
Internal p99       ≤ 500 µs
Internal p99.9     ≤ 1 ms
Emergency p99      ≤ 250 µs
No lost events
No duplicate orders
No unbounded queues
No memory leak
No risk bypass
Rate limit headroom
Reconciliation consistent
```

## Conclusão

O ponto fundamental é separar:

```text
performance do Rust
        ≠
latência do feed
        ≠
latência de rede
        ≠
latência da exchange
        ≠
latência de execução
```

O objetivo operacional é minimizar e estabilizar a latência ponta a ponta, mantendo o caminho interno sub-milissegundo e o Emergency Exit independente do caminho analítico.
