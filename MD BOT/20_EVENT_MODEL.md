# 20 — Event Model

**Status:** normativo · **Versão:** 1.1 · **Atualizado em:** 2026-09-20  
**Domínio de requisitos:** `REQ-EVENT-*`  
**Conformidade:** conforme  
**Vocabulário:** [`00_GLOSSARIO.md`](00_GLOSSARIO.md)

---

Tipos:
`MarketEvent`, `SignalEvent`, `RiskEvent`, `OrderIntent`, `OrderEvent`, `ExecutionEvent`, `PositionEvent`, `SessionEvent`, `ModelEvent`, `RecoveryEvent`.

**REQ-EVENT-001** Todo evento MUST carregar o envelope completo:
`event_id`, `event_type`, `schema_version`, `event_time`, `receive_time`, `source`, `sequence` quando aplicável, `correlation_id`, `causation_id`, `payload`.

**REQ-EVENT-002** Eventos MUST ser append-only; correção MUST ser evento novo,
nunca alteração do anterior.

**REQ-EVENT-003** Causalidade MUST NOT ser inferida apenas de timestamp:
sequência, `correlation_id` e `causation_id` MUST ser usados quando disponíveis.

O envelope deve permitir replay dentro dos limites especificados.
