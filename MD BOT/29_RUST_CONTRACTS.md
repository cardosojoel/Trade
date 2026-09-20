# 29 — Rust Contracts

## Objetivo
Transformar as regras críticas em contratos de tipos/interfaces antes da implementação.

## Módulos
```text
market_data
features
regime
pattern
probability
expected_value
position_sizing
risk
execution
reconciliation
recovery
persistence
configuration
bybit_adapter
observability
replay
```

## Contratos conceituais
```rust
struct Probability3 { up: f64, neutral: f64, down: f64 }
struct ExpectedValue { gross: f64, costs: CostBreakdown, net: f64, conservative: f64 }
struct PositionSize { raw: f64, final_qty: f64, constraints: ConstraintReport }
struct InstrumentSpec { tick_size: f64, qty_step: f64, min_qty: f64, max_qty: f64, min_notional: f64 }
```

```rust
trait PatternIndex {
    fn search(&self, query: PatternQuery) -> PatternSearchResult;
}

trait ProbabilityModel {
    fn predict(&self, input: ProbabilityInput) -> Probability3;
}

trait RiskEngine {
    fn authorize(&self, intent: OrderIntent) -> RiskDecision;
}

trait ExecutionAdapter {
    fn submit(&self, intent: AuthorizedOrderIntent) -> ExecutionRequestResult;
}
```

## Regras de segurança de tipos
- ordens não autorizadas não devem ser aceitas pelo Execution Adapter;
- `Probability3` deve ser validável e normalizável;
- `AuthorizedOrderIntent` deve ser distinto de `OrderIntent` conceitualmente;
- valores monetários e quantidades devem carregar unidade/contexto suficiente para evitar mistura silenciosa;
- timestamps devem ser explícitos quanto à origem.

## Hot path
Interfaces críticas não devem exigir I/O síncrono, lock global ou alocação não controlada.

## Objetivo
Este documento é contrato arquitetural; tipos concretos, crates e assinaturas finais serão congelados após profiling e revisão do código MVP.
