# 19 — Database Schema — v3

**Status:** normativo · **Versão:** 1.1 · **Atualizado em:** 2026-09-20  
**Domínio de requisitos:** `REQ-DB-*`  
**Conformidade:** conforme  
**Vocabulário:** [`00_GLOSSARIO.md`](00_GLOSSARIO.md)

---

Stack: Rust + SQLite.

## Entidades
`sessions`, `market_events`, `features`, `decisions`, `orders`, `executions`, `positions`, `risk_events`, `recovery_episodes`, `model_versions`, `model_predictions`, `decision_outcomes`, `recovery_events`, `performance_metrics`, `configuration_versions`, `configuration_parameters`, `instrument_specs`, `audit_events`.

## Princípio
**REQ-DB-001** A separação MUST ser respeitada:

```text
Hot State → RAM
Durable State → SQLite
```

## SQLite
**REQ-DB-002** A persistência MUST usar:

- WAL;
- foreign keys;
- migrations versionadas;
- índices orientados ao workload;
- transações curtas;
- persistência fora do hot path.

## Integridade
**REQ-DB-003** Falha de persistência MUST NOT alterar silenciosamente o Risk
State em RAM: MUST gerar evento e aplicar política de fail-safe.

## Retenção
**REQ-DB-004** Dados necessários a auditoria, replay e aprendizado MUST NOT ser
apagados antes da política de retenção correspondente.
