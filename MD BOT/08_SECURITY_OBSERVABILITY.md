# 08 — Security, Reliability & Observability

## Objetivo
Proteger credenciais, impedir operações indevidas e tornar cada componente observável.

## Secrets
API credentials:
```text
fora do SQLite
fora do Git
fora de logs
withdrawal disabled
```

Separar:
```text
TESTNET
PAPER
LIVE
```

## Kill Switch
Manual:
```text
NEW ORDERS = DISABLED
```

Automático em:
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
Logs estruturados:
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
Eventos:
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
**Estado: bom, mas não estado da arte.**

A base é adequada para produção. Para excelência, acrescentar threat modeling, least privilege, secret rotation, tamper-evident audit trail, alerting e testes de recuperação.
