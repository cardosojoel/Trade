# 04 — Market Data Quality — Bybit Revision

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

O SLO sub-milissegundo é para processamento interno:

```text
evento recebido
→ decisão
```

Não é uma exigência de que o mercado gere novos eventos a cada microssegundo.

## Order Book

Processar corretamente:

```text
snapshot
delta
reset
u
seq
ts
cts
```

Uma nova mensagem `snapshot` exige reconstrução do book local. [Bybit Orderbook](https://bybit-exchange.github.io/docs/v5/websocket/public/orderbook)

## Trades

O stream de trades é real-time e uma mensagem pode conter até 1024 trades para Futures/Spot. Portanto o parser e o pipeline devem suportar bursts reais. [Bybit Trade](https://bybit-exchange.github.io/docs/v5/websocket/public/trade)

## Data Age

\[
Age = T_{local\_receive} - T_{exchange}
\]

Classificação inicial:

```text
< 50 ms      NORMAL
50–100 ms    WARNING
100–250 ms   DEGRADED
> 250 ms     BLOCK NEW ENTRY
```

Os limites devem ser calibrados para o feed e estratégia.

## Integridade

Detectar:

```text
sequence gap
duplicate
out-of-order
stale
snapshot reset
unexpected timestamp
```

Dados inconsistentes não devem ser corrigidos silenciosamente.

## Conclusão

**Estado: forte e alinhado ao feed da Bybit.**

O benchmark deve utilizar eventos reais/replay e bursts de até 1024 trades, e não apenas uma taxa artificial constante.
