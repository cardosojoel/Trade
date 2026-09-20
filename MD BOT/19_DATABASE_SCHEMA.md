# 19 — Database Schema — v3

Stack: Rust + SQLite.

## Entidades
`sessions`, `market_events`, `features`, `decisions`, `orders`, `executions`, `positions`, `risk_events`, `recovery_episodes`, `model_versions`, `model_predictions`, `decision_outcomes`, `recovery_events`, `performance_metrics`, `configuration_versions`, `configuration_parameters`, `instrument_specs`, `audit_events`.

## Princípio
```text
Hot State → RAM
Durable State → SQLite
```

## SQLite
- WAL;
- foreign keys;
- migrations versionadas;
- índices orientados ao workload;
- transações curtas;
- persistência fora do hot path.

## Integridade
Falha de persistência não pode alterar silenciosamente o Risk State em RAM. Deve gerar evento e aplicar política de fail-safe.

## Retenção
Dados necessários à auditoria, replay e aprendizagem não podem ser apagados antes da política de retenção correspondente.
