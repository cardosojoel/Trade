# 29 — Rust Contracts

**Status:** normativo · **Versão:** 1.1 · **Atualizado em:** 2026-09-20  
**Domínio de requisitos:** `REQ-RUST-*`  
**Conformidade:** conforme  
**Vocabulário:** [`00_GLOSSARIO.md`](00_GLOSSARIO.md)

---

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
// Probabilidade é adimensional e não alimenta cálculo de dinheiro: f64 serve.
struct Probability3 { up: f64, neutral: f64, down: f64 }

// Tudo abaixo é dinheiro ou quantidade, e MUST ser Decimal — constitution,
// Restrições Operacionais → Representação de valores monetários.
struct ExpectedValue { gross: Decimal, costs: CostBreakdown, net: Decimal, conservative: Decimal }
struct PositionSize { raw: Decimal, final_qty: Decimal, constraints: ConstraintReport }
struct InstrumentSpec { tick_size: Decimal, qty_step: Decimal, min_qty: Decimal, max_qty: Decimal, min_notional: Decimal }
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
**REQ-RUST-001** Os tipos MUST garantir:

- ordens não autorizadas MUST NOT ser aceitas pelo Execution Adapter;
- `Probability3` deve ser validável e normalizável;
- `AuthorizedOrderIntent` deve ser distinto de `OrderIntent` conceitualmente;
- valores monetários e quantidades devem carregar unidade/contexto suficiente para evitar mistura silenciosa;
- timestamps devem ser explícitos quanto à origem.

## Hot path
**REQ-RUST-002** Interfaces críticas MUST NOT exigir I/O síncrono, lock global ou
alocação não controlada.

## Corrigido em 2026-09-20

Esta página usava `f64` para `tick_size`, `qty_step`, `min_notional` e para os
campos de `ExpectedValue`, contradizendo a `trading_risk_recovery_mathematical_spec.md`
§33 e a constitution do Trade. Era erro de redação, não escolha de desenho —
[ADR-004](34_ADR_EMENDAS.md#adr-004), encerrada sem ir à assinatura.

## Objetivo
Este documento é contrato arquitetural; tipos concretos, crates e assinaturas finais serão congelados após profiling e revisão do código MVP.
