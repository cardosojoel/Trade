# 02 — Position Reconciliation — Bybit Revision

**Status:** normativo · **Versão:** 1.1 · **Atualizado em:** 2026-09-20  
**Domínio de requisitos:** `REQ-RECON-*`  
**Vocabulário:** [`00_GLOSSARIO.md`](00_GLOSSARIO.md)

---

## Objetivo

Garantir que o estado local represente o estado real da Bybit após fills, partial fills, cancelamentos, reconexões e falhas.

## Fontes

**REQ-RECON-001** A reconciliação MUST considerar todas estas fontes:

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

**REQ-RECON-002** `execution.fast` serve ao caminho rápido de execução e MUST
NOT ser fonte única do lifecycle.

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

**REQ-RECON-003** A identidade de uma execution MUST ser composta:

```text
symbol + seq + execId + orderId
```

A documentação do `execution.fast` informa que `seq` pode ser igual para múltiplas transações simultâneas e que símbolos diferentes podem compartilhar `seq`; portanto `seq` isoladamente não é identificador único. [Bybit Fast Execution](https://bybit-exchange.github.io/docs/v5/websocket/private/fast-execution)

## Startup

**REQ-RECON-004** A habilitação de trading MUST ser o último passo da sequência
de startup:

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

**REQ-RECON-005** Qualquer `CONFLICT` ou `UNKNOWN` relevante MUST bloquear
novas entradas:

```text
NEW ENTRIES = BLOCKED
```

## Conclusão

A arquitetura agora diferencia corretamente o caminho rápido de fills do estado autoritativo completo. O próximo nível é testar recuperação sob falhas reais e fault injection.
