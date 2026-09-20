# 01 — Execution Engine — Bybit Low-Latency Revision

**Status:** normativo · **Versão:** 1.1 · **Atualizado em:** 2026-09-20  
**Domínio de requisitos:** `REQ-EXEC-*`  
**Vocabulário:** [`00_GLOSSARIO.md`](00_GLOSSARIO.md)

---

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
**REQ-EXEC-001** Operações de `create`, `amend` e `cancel` MUST usar o WebSocket
Order Entry como caminho preferencial:

```text
WebSocket Trade
```

### Recovery / fallback

**REQ-EXEC-002** REST MAY ser usado para:

- reconciliação;
- recuperação;
- consultas;
- operações não críticas;
- contingência quando o WebSocket Order Entry estiver indisponível.

**REQ-EXEC-003** O sistema MUST NOT repetir uma ordem cujo resultado seja
desconhecido antes de reconciliar o estado remoto.

## Idempotência

**REQ-EXEC-004** Cada intenção MUST carregar:

```text
trade_intent_id
client_order_id / orderLinkId
reqId
strategy_version
risk_version
timestamp
```

**REQ-EXEC-005** `reqId` MUST ser único dentro da conexão, para evitar duplicação; a Bybit retorna erro quando um `reqId` é duplicado. [Bybit WebSocket Trade](https://bybit-exchange.github.io/docs/v5/websocket/trade/guideline)

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

**REQ-EXEC-006** Ordem em `UNKNOWN` MUST ser reconciliada antes de qualquer
nova ordem relacionada a ela.

## Stop no mercado à vista

A Bybit aceita stop loss e take profit na criação da ordem à vista, e também
ordens condicionais por preço de gatilho, com saída a mercado ou limitada.
Mas restringe a distância entre o preço de uma ordem **limitada** e o preço
corrente: o instrumento BTCUSDT declara hoje `priceLimitRatioX` 0,005 e
`priceLimitRatioY` 0,01. Ordem limitada fora da banda é **cancelada** por padrão
— ou tem o preço corrigido em silêncio, se a correção automática estiver ativa.

O stop dos perfis fica a 2% do preço de entrada, fora dessa banda.

**REQ-EXEC-010** O stop MUST ser enviado como ordem a mercado no gatilho
(`slOrderType: Market`). Stop enviado como ordem limitada fora da banda de preço
MAY ser recusado, e posição sem stop aceito é posição sem limite de perda.

**REQ-EXEC-011** A confirmação de que o stop foi aceito MUST preceder a
existência da posição. Se o stop for recusado, a posição MUST NOT ser aberta;
se a recusa ocorrer com posição já aberta, o sistema MUST encerrá-la.

## Execution Fast

**REQ-EXEC-007** O sistema MUST assinar `execution.fast` em paralelo ao stream
completo, e MUST NOT tratá-lo como fonte única do lifecycle. A Bybit descreve `execution.fast` como um stream de menor latência, mas ele entrega somente `execType=Trade` e menos campos. [Bybit Fast Execution](https://bybit-exchange.github.io/docs/v5/websocket/private/fast-execution)

Uso:

```text
execution.fast → detecção rápida de fills
execution      → confirmação completa
order stream   → lifecycle da ordem
```

## Métricas

**REQ-EXEC-008** O sistema MUST registrar timestamps monotônicos locais e os
timestamps da Bybit quando disponíveis:

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

O ganho principal desta revisão é separar aceitação de ordem, lifecycle e
execução efetiva.

**REQ-EXEC-009** O sistema MUST NOT interpretar ACK como fill.
