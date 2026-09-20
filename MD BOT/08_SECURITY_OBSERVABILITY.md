# 08 — Security, Reliability & Observability

**Status:** normativo · **Versão:** 1.1 · **Atualizado em:** 2026-09-20  
**Domínio de requisitos:** `REQ-SEC-*`  
**Conformidade:** conforme  
**Vocabulário:** [`00_GLOSSARIO.md`](00_GLOSSARIO.md)

---

## Objetivo
Proteger credenciais, impedir operações indevidas e tornar cada componente observável.

## Secrets
**REQ-SEC-001** As credenciais de API MUST permanecer:
```text
fora do SQLite
fora do Git
fora de logs
withdrawal disabled
```

**REQ-SEC-002** Os ambientes MUST ser separados e MUST NOT coexistir na mesma
configuração carregada:

```text
TESTNET
PAPER
LIVE
```

## Kill Switch
**REQ-SEC-003** MUST existir kill switch manual, acionável sem acesso ao código:
```text
NEW ORDERS = DISABLED
```

**REQ-SEC-004** O kill switch MUST ser acionado automaticamente em:
```text
reconciliation failure
stale data
unexpected position
API failure
risk inconsistency
model invalid
latency breach
```

## Observabilidade
**REQ-SEC-005** Os logs MUST ser estruturados e MUST NOT conter segredo:
```text
event_id
timestamp
component
severity
trade_id
decision_id
latency
error_code
```

Métricas:
```text
PnL
latency
order reject
fill rate
API errors
WebSocket health
CPU
memory
database latency
```

## Auditoria
**REQ-SEC-006** MUST ser registrado, no mínimo:
```text
SESSION_CREATED
RISK_CONFIRMED
SIGNAL_GENERATED
ORDER_SUBMITTED
ORDER_FILLED
POSITION_CLOSED
RECOVERY_STARTED
RECOVERY_FAILED
KILL_SWITCH
MODEL_CHANGED
```

## Conclusão
A base é adequada para produção. Para excelência, acrescentar threat modeling, least privilege, secret rotation, tamper-evident audit trail, alerting e testes de recuperação.
