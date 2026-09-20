# 04 — Market Data Quality — Bybit Revision

**Status:** normativo · **Versão:** 1.1 · **Atualizado em:** 2026-09-20  
**Domínio de requisitos:** `REQ-DATA-*`  
**Vocabulário:** [`00_GLOSSARIO.md`](00_GLOSSARIO.md)

---

## Objetivo

Garantir que decisões sejam tomadas somente sobre dados atuais, ordenados e consistentes.

## Feeds

Para BTCUSDT, avaliar:

```text
publicTrade.BTCUSDT
orderbook.1.BTCUSDT
orderbook.50.BTCUSDT
orderbook.200.BTCUSDT
```

A Bybit informa atualmente frequências de 10 ms para Level 1, 20 ms para Level 50 e 100 ms para Level 200 em linear/inverse. O stream de trades é real-time. [Bybit Orderbook](https://bybit-exchange.github.io/docs/v5/websocket/public/orderbook) [Bybit Trade](https://bybit-exchange.github.io/docs/v5/websocket/public/trade)

## Não confundir frequência do feed com latência do bot

**REQ-DATA-001** O SLO sub-milissegundo aplica-se ao processamento interno, não
à frequência do feed:

```text
evento recebido
→ decisão
```

Não é uma exigência de que o mercado gere novos eventos a cada microssegundo.

## Order Book

**REQ-DATA-002** O processamento do book MUST tratar corretamente:

```text
snapshot
delta
reset
u
seq
ts
cts
```

**REQ-DATA-003** Uma nova mensagem `snapshot` MUST provocar reconstrução do
book local. [Bybit Orderbook](https://bybit-exchange.github.io/docs/v5/websocket/public/orderbook)

## Trades

O stream de trades é real-time e uma mensagem pode conter até 1024 trades para Futures/Spot. Portanto o parser e o pipeline MUST suportar bursts reais de até 1024 trades
por mensagem (**REQ-DATA-004**). [Bybit Trade](https://bybit-exchange.github.io/docs/v5/websocket/public/trade)

## Data Age

\[
Age = T_{local\_receive} - T_{exchange}
\]

**REQ-DATA-005** A idade do dado MUST ser classificada e MUST bloquear nova
entrada acima do limiar. Classificação inicial:

```text
< 50 ms      NORMAL
50–100 ms    WARNING
100–250 ms   DEGRADED
> 250 ms     BLOCK NEW ENTRY
```

Os limites devem ser calibrados para o feed e estratégia.

## Integridade

**REQ-DATA-006** O sistema MUST detectar:

```text
sequence gap
duplicate
out-of-order
stale
snapshot reset
unexpected timestamp
```

**REQ-DATA-007** Dados inconsistentes MUST NOT ser corrigidos silenciosamente.

## Conclusão

**REQ-DATA-008** O benchmark MUST utilizar eventos reais ou replay e bursts de
até 1024 trades, não apenas uma taxa artificial constante.
