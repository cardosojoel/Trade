# 27 — Configuration Registry

## Objetivo
Ser a única fonte operacional dos valores configuráveis do sistema.

## Contrato
Cada parâmetro possui:
```text
parameter_id
value
type
unit
scope
min
max
default
version
effective_from
status
source
```

## Escopos
`GLOBAL`, `SESSION`, `SYMBOL`, `MODEL`, `ENVIRONMENT`.

## Parâmetros mínimos
```text
risk.max_loss_pct
risk.recovery_profit_pct
risk.recovery_max_pct
risk.max_risk_per_trade_pct
risk.max_recovery_risk_pct
risk.max_recovery_episodes
risk.max_recovery_attempts
strategy.min_probability
strategy.min_ev
strategy.min_ev_conservative
pattern.k
pattern.min_samples
pattern.min_similarity
performance.data_max_age_us
performance.decision_p99_us
performance.decision_p999_us
performance.emergency_p99_us
```

Os valores concretos devem ser preenchidos por ambiente e validação; este documento define nomes, tipos e invariantes, não números arbitrários.

## Validação
Startup deve rejeitar configuração fora dos limites ou inconsistente.

## Precedência
Runtime lê o Registry versionado. Código contém apenas invariantes que não podem ser configurados.
