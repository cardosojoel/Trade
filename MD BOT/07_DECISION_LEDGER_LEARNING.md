# 07 — Decision Ledger & Learning

## Objetivo
Criar histórico completo e imutável das decisões para permitir avaliação, aprendizado e evolução controlada do modelo.

## Decision Snapshot
Cada decisão deve registrar:
```text
decision_id
timestamp
market_state
features
regime
strategy_version
feature_version
model_version
risk_version
probability
expected_return
expected_value
decision
reason_codes
risk_state
```

## Outcomes
Após fechamento:
```text
entry
exit
PnL
fees
funding
slippage
MAE
MFE
holding_time
outcome
```

## No Trade
Registrar também:
```text
NO_TRADE
```

Isso evita viés de seleção.

## Aprendizado
Pipeline:
```text
Decision
 ↓
Outcome
 ↓
Dataset
 ↓
Evaluation
 ↓
Candidate Model
 ↓
Walk-forward
 ↓
Validation
 ↓
Promotion
```

## Não permitir
```text
online self-modification
automatic model promotion
training with future information
```

## Drift
Monitorar:
```text
feature drift
probability calibration drift
performance drift
regime drift
```

## Conclusão
**Estado: conceitualmente forte, mas ainda não estado da arte.**

Para nível máximo, adicionar Model Registry, lineage, dataset hashing, experiment tracking, champion/challenger, canary deployment e rollback automático.
