# 02 — Position Reconciliation — Bybit Revision

## Objetivo

Garantir que o estado local represente o estado real da Bybit após fills, partial fills, cancelamentos, reconexões e falhas.

## Fontes

Usar:

```text
Private Order Stream
Private Execution Stream
execution.fast
Position state
Balance state
REST recovery
```

A Bybit recomenda o WebSocket para informações de execução em tempo real e disponibiliza consulta REST como mecanismo de recuperação. [Bybit Execution](https://bybit-exchange.github.io/docs/v5/websocket/private/execution)

## Regra fundamental

```text
Fast event ≠ complete state
```

`execution.fast` serve para caminho rápido de execução, não como fonte única do lifecycle.

## Reconciliação

```text
Fast Execution
       ↓
Immediate Position Update
       ↓
Full Order/Execution State
       ↓
Reconciliation
```

## Identidade

Para executions:

```text
symbol + seq + execId + orderId
```

A documentação do `execution.fast` informa que `seq` pode ser igual para múltiplas transações simultâneas e que símbolos diferentes podem compartilhar `seq`; portanto `seq` isoladamente não é identificador único. [Bybit Fast Execution](https://bybit-exchange.github.io/docs/v5/websocket/private/fast-execution)

## Startup

```text
Load SQLite snapshot
       ↓
Connect private streams
       ↓
Query remote state
       ↓
Compare
       ↓
Rebuild
       ↓
Persist
       ↓
Enable trading
```

## Divergência

```text
SYNCED
REMOTE_AHEAD
LOCAL_AHEAD
CONFLICT
UNKNOWN
```

Qualquer `CONFLICT` ou `UNKNOWN` relevante:

```text
NEW ENTRIES = BLOCKED
```

## Conclusão

**Estado: forte.**

A arquitetura agora diferencia corretamente o caminho rápido de fills do estado autoritativo completo. O próximo nível é testar recuperação sob falhas reais e fault injection.
