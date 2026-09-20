# 01 — Execution Engine — Bybit Low-Latency Revision

## Objetivo

Executar ordens com baixa latência, idempotência e segurança, usando o WebSocket Order Entry da Bybit como caminho preferencial para operações críticas.

A Bybit disponibiliza `wss://stream.bybit.com/v5/trade` para WebSocket Order Entry, com `order.create`, `order.amend` e `order.cancel`. O ACK confirma aceitação da requisição, não execução; o estado efetivo deve ser confirmado pelo stream privado de ordens/executions. [Bybit WebSocket Trade](https://bybit-exchange.github.io/docs/v5/websocket/trade/guideline)

## Arquitetura

```text
Risk Authorization
       ↓
OrderIntent
       ↓
Hot-path Validation
       ↓
WebSocket Order Entry
       ↓
ACK
       ↓
Private Order Stream
       +
execution.fast
       ↓
Execution State
       ↓
Reconciliation
```

## Caminhos

### Hot path
Preferir:

```text
WebSocket Trade
```

para `create/amend/cancel`.

### Recovery / fallback

REST pode ser usado para:

- reconciliação;
- recuperação;
- consultas;
- operações não críticas;
- contingência quando o WebSocket Order Entry estiver indisponível.

Não fazer retry cego de uma ordem cujo resultado seja desconhecido.

## Idempotência

Cada intenção:

```text
trade_intent_id
client_order_id / orderLinkId
reqId
strategy_version
risk_version
timestamp
```

`reqId` deve ser único dentro da conexão para evitar duplicação; a Bybit retorna erro quando um `reqId` é duplicado. [Bybit WebSocket Trade](https://bybit-exchange.github.io/docs/v5/websocket/trade/guideline)

## Estados

```text
CREATED
VALIDATING
SUBMITTING
ACKNOWLEDGED
PARTIALLY_FILLED
FILLED
CANCEL_PENDING
CANCELLED
REJECTED
UNKNOWN
```

`UNKNOWN` exige reconciliação antes de qualquer nova ordem relacionada.

## Execution Fast

Assinar `execution.fast` em paralelo ao stream completo. A Bybit descreve `execution.fast` como um stream de menor latência, mas ele entrega somente `execType=Trade` e menos campos. [Bybit Fast Execution](https://bybit-exchange.github.io/docs/v5/websocket/private/fast-execution)

Uso:

```text
execution.fast → detecção rápida de fills
execution      → confirmação completa
order stream   → lifecycle da ordem
```

## Métricas

Registrar timestamps monotônicos locais e timestamps da Bybit quando disponíveis:

```text
market_received
decision_started
decision_completed
submit_started
ack_received
order_event_received
fast_execution_received
execution_received
fill_confirmed
```

Métricas:

```text
decision_to_submit
submit_to_ack
ack_to_order_event
submit_to_execution_fast
market_to_fill
```

Sempre reportar:

```text
p50
p95
p99
p99.9
max
jitter
```

## Conclusão

**Estado: arquitetura de produção forte.**

O ganho principal desta revisão é separar aceitação de ordem, lifecycle e execução efetiva. O sistema não pode interpretar ACK como fill.
