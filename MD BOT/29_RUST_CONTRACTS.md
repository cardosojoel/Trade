# 29 — Rust Contracts

**Status:** normativo · **Versão:** 1.1 · **Atualizado em:** 2026-09-20  
**Domínio de requisitos:** `REQ-RUST-*`  
**Conformidade:** exige correção (ADR-004)  
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
**REQ-RUST-001** Os tipos MUST garantir:

- ordens não autorizadas MUST NOT ser aceitas pelo Execution Adapter;
- `Probability3` deve ser validável e normalizável;
- `AuthorizedOrderIntent` deve ser distinto de `OrderIntent` conceitualmente;
- valores monetários e quantidades devem carregar unidade/contexto suficiente para evitar mistura silenciosa;
- timestamps devem ser explícitos quanto à origem.

## Hot path
**REQ-RUST-002** Interfaces críticas MUST NOT exigir I/O síncrono, lock global ou
alocação não controlada.

## Pendência conhecida

Os exemplos desta página usam `f64` para `tick_size`, `qty_step`, `min_notional`
e para os campos de `ExpectedValue`. Isso conflita com a
`trading_risk_recovery_mathematical_spec.md` §33, que proíbe ponto flutuante em
caminho monetário, e com a constitution do repositório Trade — *Restrições
Operacionais e de Segurança* → **Representação de valores monetários**, desde a
emenda 1.3.0 —, cobrada pelo `tests/no_float.rs`.

A divergência está registrada como [ADR-004](34_ADR_EMENDAS.md#adr-004), e lá
já foi classificada: é **erro de redação desta página**, não escolha de
desenho — dois documentos deste mesmo conjunto se contradizem. **Não há decisão
a tomar aqui e não cabe emenda.** O encaminhamento é corrigir estes contratos
para `Decimal` em todo campo monetário e de quantidade, mantendo `f64` apenas
onde a grandeza é adimensional e não alimenta cálculo de dinheiro —
probabilidade, por exemplo.

## Objetivo
Este documento é contrato arquitetural; tipos concretos, crates e assinaturas finais serão congelados após profiling e revisão do código MVP.
