# 20 — Event Model

Tipos:
`MarketEvent`, `SignalEvent`, `RiskEvent`, `OrderIntent`, `OrderEvent`, `ExecutionEvent`, `PositionEvent`, `SessionEvent`, `ModelEvent`, `RecoveryEvent`.

Envelope:
`event_id`, `event_type`, `schema_version`, `event_time`, `receive_time`, `source`, `sequence` quando aplicável, `correlation_id`, `causation_id`, `payload`.

Eventos são append-only. Correções são novos eventos.

Não assumir causalidade apenas por timestamp; usar sequência/correlation/causation quando disponíveis.

O envelope deve permitir replay dentro dos limites especificados.
