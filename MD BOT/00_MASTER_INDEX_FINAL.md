# Trading Bot SDD — Master Index Final v3

**Status:** SDD v3 — P0 remediation applied
**Stack:** Rust + SQLite
**Exchange:** Bybit
**Ativo inicial:** BTCUSDT

## 1. Regra de autoridade
A especificação é a fonte de verdade. Valores operacionais concretos vivem no `27_CONFIGURATION_REGISTRY.md`; regras do instrumento vivem no `28_BYBIT_INSTRUMENT_REGISTRY.md`; contratos de software vivem no `29_RUST_CONTRACTS.md`; rastreabilidade vive no `26_REQUIREMENTS_TRACEABILITY_MATRIX.md`.

## 2. Baseline 01–09
01 Execution Engine
02 Position Reconciliation
03 Session State Machine / Risk & Recovery
04 Market Data Quality
05 Backtest Engine
06 Execution Simulator
07 Decision Ledger / Learning
08 Security / Observability
09 Performance / Low Latency

## 3. Estratégia e quant 10–18
10 Trading Strategy Model
11 Feature Engineering
12 Pattern Matching
13 Probability Model
14 Expected Value Model — EV ternário canônico
15 Position Sizing
16 Market Regime
17 Model Validation
18 Model Promotion

## 4. Plataforma 19–24
19 Database Schema
20 Event Model
21 Replay Engine
22 Failure Recovery
23 Testing Strategy
24 Configuration & Governance

## 5. Governança P0 25–29
25 Cross-Document Audit
26 Requirements Traceability Matrix
27 Configuration Registry
28 Bybit Instrument Registry
29 Rust Contracts

## 6. Matemática canônica
```text
Market Data
→ Features
→ Regime
→ Pattern Matching
→ Probability3
→ Expected Value
→ Position Sizing
→ Risk
→ Execution
→ Reconciliation
→ Ledger
→ Validation
→ Promotion
```

### Probability3
```text
P_up + P_neutral + P_down = 1
```

### EV
```text
EV_gross = P_up*R_up + P_neutral*R_neutral + P_down*R_down
EV_net = EV_gross - Costs
```

EV binário é apenas uma projeção especial do modelo ternário.

## 7. Risk / Recovery
```text
MaxLossDeposit = D0 * L
CapitalFloor = D0 - MaxLossDeposit
RecoveryMaxSession = D0 * R
RecoveryBudgetSession <= RecoveryMaxSession
WorstCaseSessionExposure = MaxLossDeposit + RecoveryMaxSession
```

`Recovery Episode` e `Recovery Budget` são conceitos distintos. Sucesso de um episódio não cria budget novo.

## 8. Bybit
- WebSocket Order Entry é preferencial para hot path.
- `execution.fast` é aceleração de detecção de trade execution.
- `order`/`execution` continuam necessários para lifecycle completo.
- ACK não significa fill.
- REST é usado para recovery/reconciliation/fallback conforme política.
- Instrument constraints são centralizados no `28_BYBIT_INSTRUMENT_REGISTRY.md`.

## 9. Performance
O sistema mede separadamente:
- latência interna do hot path;
- latência de rede/feed;
- latência E2E observável.

SLOs internos existentes na Frente 09 permanecem autoridade. Budgets por estágio devem ser derivados por profiling antes de serem congelados.

## 10. Traceability
Todo requisito crítico deve seguir:
```text
Requirement
→ Specification
→ Rust Module
→ Test
→ Benchmark/Backtest
→ Production Metric
→ Verification
```

## 11. Critério de maturidade
```text
Specification
→ Implementation
→ Automated Tests
→ Benchmarks / Backtests
→ Paper Trading
→ Real-world Evidence
```

## 12. P0 status
Aplicado:
- EV ternário;
- Recovery Episode/Budget;
- WorstCaseSessionExposure;
- Configuration Registry;
- Bybit Instrument Registry;
- Rust Contracts;
- Requirements Traceability Matrix;
- reconnect/reconcile protocol;
- testes das novas invariantes.

## 13. Próximo gate
Não adicionar novas frentes antes de preencher a matriz 26 com referências reais aos módulos Rust e testes do MVP. O próximo ciclo é **Implementation Traceability & Evidence**, não criação indiscriminada de novos MDs.
