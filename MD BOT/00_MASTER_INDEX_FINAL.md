# Trading Bot SDD — Master Index Final v3

**Status:** índice do conjunto · **Versão:** 3.2 · **Atualizado em:** 2026-09-20  
**Conteúdo:** SDD com camada normativa única, vocabulário canônico e requisitos
identificados
**Stack:** Rust + SQLite
**Exchange:** Bybit
**Ativo inicial:** BTCUSDT

## 1. Regra de autoridade
A especificação é a fonte de verdade. Valores operacionais concretos vivem no `27_CONFIGURATION_REGISTRY.md`; regras do instrumento vivem no `28_BYBIT_INSTRUMENT_REGISTRY.md`; contratos de software vivem no `29_RUST_CONTRACTS.md`; rastreabilidade vive no `26_REQUIREMENTS_TRACEABILITY_MATRIX.md`; **nomes** vivem no `00_GLOSSARIO.md`.

## 1.1 Camadas: normativo e índice
Cada assunto tem **um** documento normativo. Documentos de índice localizam o assunto e não enunciam regra própria — um assunto reenunciado em dois lugares foi, na prática, corrigido em um só.

| Assunto | Normativo | Índices |
|---|---|---|
| Risco e Recovery da sessão | `trading_risk_recovery_mathematical_spec.md` | `03` |
| Features, regime, EV, sizing | `MATHEMATICAL_QUANT_MODEL.md` | `10`, `11`, `15`, `16` |
| Pattern matching, probabilidade, EV | `HISTORICAL_PATTERN_PROBABILITY_EV.md` | `12`, `13`, `14` |
| Ledger, aprendizado, validação, promoção | `DECISION_LEARNING.md` | `07`, `17`, `18` |

Os documentos `01`, `02`, `04`, `05`, `06`, `08`, `09`, `19`–`24`, `26`–`29` e `31`–`33` são normativos nos seus próprios assuntos e não têm par. O `25` e o `30` são registro histórico.

Vocabulário: `00_GLOSSARIO.md` é normativo sobre nomes e símbolos. Nenhum documento introduz sinônimo para termo já definido, nem reusa símbolo já atribuído.

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

## 5.1 Escopo, exemplo e decisões 31–33
31 Exemplo numérico fim a fim — atravessa a cadeia com os mesmos números
32 Não-objetivos — o contorno do sistema e os quatro conflitos com a constitution
33 Registro de decisões — inclui os valores que hoje governam sem racional registrado

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
MaxLossDeposit = D * L
CapitalFloor = D - MaxLossDeposit
RecoveryMaxSession = D * R
RecoveryBudgetSession <= RecoveryMaxSession
WorstCaseSessionExposure = MaxLossDeposit + RecoveryMaxSession
```

`RecoveryEpisode` e `RecoveryBudgetSession` são conceitos distintos. Sucesso de um episódio não cria budget novo. Símbolos conforme `00_GLOSSARIO.md`.

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
Cada requisito tem identificador estável `REQ-<DOMÍNIO>-<NNN>`, escrito ao lado da regra na especificação. Os domínios estão no `00_GLOSSARIO.md` §10 e o catálogo no `26_REQUIREMENTS_TRACEABILITY_MATRIX.md`.

Todo requisito crítico MUST seguir:
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
Aplicado — e, desde 2026-09-20, aplicado também às especificações matemáticas, onde EV binário e vocabulário duplicado de regime haviam sobrevivido:
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

Pendências conhecidas, registradas no `CURRENT_STATE.md` do repositório: os quatro conflitos com a constitution do projeto (mercado spot, Recovery depois do freio, ciclo de vida da sessão, `f64` no `29_RUST_CONTRACTS.md`) continuam abertos e só se resolvem por emenda formal. Nenhum deles foi decidido aqui.
