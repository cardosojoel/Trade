# 25 — Cross-Document Audit — SDD Trading Bot v2

**Status:** registro histórico — **não normativo**  
**Data da auditoria:** 20/09/2026  
**Nota de 2026-09-20:** este documento é registro histórico dos achados e não foi
reescrito. Duas ressalvas para quem o lê hoje: (a) AUD-MATH-001 e AUD-PARAM-002
foram fechados apenas nos documentos numerados e permaneciam abertos nas
especificações matemáticas — foram fechados nelas nesta data; (b) os nomes
usados aqui precedem o [`00_GLOSSARIO.md`](00_GLOSSARIO.md), que hoje é a
autoridade sobre vocabulário: `WorstCaseSessionLoss` lê-se
`WorstCaseSessionExposure`, e "ciclo" lê-se `RecoveryEpisode`; (c) os
identificadores `REQ-…` citados nas seções 5 e 10 eram ilustrativos e **não**
correspondem ao catálogo real, que passou a existir em 2026-09-20 no
[`26_REQUIREMENTS_TRACEABILITY_MATRIX.md`](26_REQUIREMENTS_TRACEABILITY_MATRIX.md).  
**Escopo:** Frentes 01–09 + especificações matemáticas + documentos 10–24  
**Stack:** Rust + SQLite  
**Exchange:** Bybit  
**Ativo inicial:** BTCUSDT

## 0. Resultado executivo

A baseline está **coerente em arquitetura**, mas **ainda não está pronta para ser considerada especificação executável sem ambiguidade**.

Classificação:

| Área | Estado |
|---|---|
| Separação Strategy / Risk / Execution | 🟢 |
| Bybit hot path | 🟢 |
| Market Data | 🟢 |
| Performance architecture | 🟢 |
| Risk/Recovery base | 🟡 |
| Quant model | 🟡 |
| Execution Simulator | 🔴 |
| Model Governance | 🟡 |
| Rust implementation contract | 🔴 |
| Test traceability | 🔴 |

### Resultado geral

**0 bloqueios arquiteturais irreconciliáveis**, porém existem **6 problemas críticos de especificação**, **11 problemas altos**, e diversas lacunas de implementação/traceability.

Os problemas mais importantes são:

1. Recovery cycle após sucesso está semanticamente ambíguo.
2. EV binário e EV ternário coexistem.
3. Parâmetros quantitativos não possuem uma autoridade única.
4. Position sizing não possui contrato completo de instrumento/Bybit.
5. Performance possui SLOs internos, mas não possui budgets por estágio nem orçamento E2E operacional.
6. Não existe ainda uma matriz requisito → código Rust → teste → métrica.

---

# 1. Contradições matemáticas

## AUD-MATH-001 — EV binário vs. EV ternário

**Severidade: CRÍTICA**

Existem dois modelos.

### Modelo ternário

O modelo quantitativo define:

`P(up) + P(down) + P(neutral) = 1`

e:

`ER = P(up)R_up + P(neutral)R_neutral + P(down)R_down`.

### Modelo binário

O documento 14 define:

`EV_net = P(win)*E[profit|win] - (1-P(win))*E[loss|loss] - Costs`.

Isso pressupõe implicitamente apenas dois estados.

### Problema

Se `P(neutral) > 0`, então `1-P(win)` não é necessariamente `P(loss)`.

### Correção obrigatória

Adotar uma única forma canônica:

`EV_net = P_up * E[R_up] + P_neutral * E[R_neutral] + P_down * E[R_down] - Costs`

Para estratégias binárias, declarar explicitamente:

`P_loss = 1 - P_win`

e `P_neutral = 0`.

**Decisão recomendada:** modelo ternário como canonical.

---

## AUD-MATH-002 — Recovery cycle após sucesso

**Severidade: CRÍTICA**

A especificação permite novo ciclo somente enquanto:

`RECOVERY_TARGET_NOT_REACHED`

Mas também define que um novo ciclo utiliza `RECOVERY_BUDGET_REMAINING`.

Após atingir o target:

`RECOVERY_SUCCESS`

o target está atingido e, portanto, a condição de novo ciclo deixa de ser verdadeira.

Isso é incompatível com a intenção original de permitir que o mecanismo de recuperação volte a atuar em uma nova deterioração posterior da sessão, caso essa política continue sendo desejada.

### Correção recomendada

Separar:

```text
Recovery Episode
    ↓
Success
    ↓
Return to Protected/Active
    ↓
Future deterioration
    ↓
New Recovery Episode
```

A condição para novo episódio deve ser:

```text
RecoveryTrigger
AND
RecoveryBudgetRemaining > 0
AND
Cycle < MaxCycles
```

e não `RecoveryTargetNotReached`.

O `RecoveryTarget` pertence ao episódio atual, enquanto `RecoveryBudget` pertence ao orçamento global da sessão.

---

## AUD-MATH-003 — “Perda máxima da sessão” não está formalmente definida

**Severidade: ALTA**

O documento 03 pede ao usuário confirmação da “perda máxima absoluta da sessão”, mas não fornece uma fórmula única para esse número.

É necessário separar:

```text
MaxLossFromDeposit
MaxLossFromPeakEquity
MaxRecoveryCapitalAtRisk
MaxSessionDrawdown
WorstCaseSessionLoss
```

### Recomendação

Definir explicitamente:

`WorstCaseSessionLoss = DepositLossLimit + MaximumRecoveryExposure`

e, separadamente, definir se o valor é medido:

- contra depósito inicial;
- contra pico de equity;
- contra equity imediatamente antes do Recovery.

Não misturar essas definições.

---

## AUD-MATH-004 — Recovery Budget vs. Recovery Risk

**Severidade: ALTA**

`Recovery Budget` é orçamento consumível, enquanto `Recovery Trade Risk` é fração do orçamento restante.

Isso é correto, mas a especificação deve declarar se uma perda real reduz o budget pelo:

```text
realized net loss
```

ou pelo:

```text
worst-case stop risk
```

antes da execução.

### Recomendação

Separar:

```text
ReservedRisk
RealizedRecoveryLoss
AvailableRecoveryBudget
```

Assim uma ordem aberta não reduz silenciosamente o orçamento de forma incorreta.

---

# 2. Parâmetros definidos em múltiplos lugares

## AUD-PARAM-001 — Parâmetros quantitativos sem autoridade única

**Severidade: ALTA**

Aparecem em múltiplos documentos:

```text
K
minimum_probability
minimum_ev
minimum_sample
minimum_confidence
similarity_function
minimum_similarity
k / ATR
```

Há `K=100` tanto na base quantitativa quanto na especificação de Historical Pattern Matching.

### Problema

O documento 24 diz que configuração deve ser governada, mas não define um Registry central de parâmetros.

### Correção

Criar uma autoridade única:

```text
Configuration Registry
```

com:

```text
parameter_id
value
unit
type
min
max
scope
version
effective_from
validation_status
```

Nenhum MD deve definir valor operacional definitivo fora desse registry.

---

## AUD-PARAM-002 — Regimes possuem dois vocabulários

**Severidade: ALTA**

`MATHEMATICAL_QUANT_MODEL` utiliza:

```text
BULL_TREND
BEAR_TREND
SIDEWAYS
HIGH_VOLATILITY
EXTREME_VOLATILITY
RECOVERY
UNKNOWN
```

O documento 16 utiliza:

```text
TREND_UP
TREND_DOWN
RANGE
HIGH_VOLATILITY
LOW_VOLATILITY
BREAKOUT
CRASH
UNKNOWN
```

### Correção

Criar enum canônico:

```rust
enum MarketRegime {
    TrendUp,
    TrendDown,
    Range,
    LowVolatility,
    HighVolatility,
    Breakout,
    Crash,
    Unknown,
}
```

`Recovery` não deveria ser MarketRegime se representar estado da sessão/risk.

---

## AUD-PARAM-003 — `P`, `R`, `RT` e `T`

**Severidade: MÉDIA**

A matemática define:

```text
P = % lucro liberado
R = RecoveryMax %
RT = Recovery risk %
T = normal trade risk %
```

Mas o documento 24 não define tipos, unidades e limites.

### Correção

Definir em basis points/decimal exato, com limites:

```text
0 ≤ P ≤ 100%
0 ≤ R ≤ 100%
0 ≤ T ≤ 100%
0 ≤ RT ≤ 100%
```

e validações de coerência.

---

# 3. Regras sem implementação definida

## AUD-IMPL-001 — Pattern index

**Severidade: CRÍTICA**

O documento 12 exige busca histórica em estrutura de memória, mas não define:

- índice;
- estrutura;
- atualização;
- eviction;
- persistência;
- dimensionalidade;
- algoritmo de busca;
- concorrência;
- rebuild.

### Para Rust

É necessário especificar pelo menos:

```text
PatternStore
PatternIndex
PatternRecord
SimilarityMetric
SearchResult
```

e o contrato de concorrência.

---

## AUD-IMPL-002 — Probability calibration

**Severidade: CRÍTICA**

O documento 13 exige calibração, Brier Score, log loss e reliability curve, mas não define:

- método de calibração;
- dataset de calibração;
- janela;
- atualização;
- versão;
- fallback.

Definir um método baseline, por exemplo:

```text
Platt / isotonic / beta calibration
```

e deixar o método versionado.

---

## AUD-IMPL-003 — Regime detector

**Severidade: ALTA**

O documento 16 define estados, mas não define o algoritmo de classificação.

O Rust precisa saber:

```text
inputs
thresholds
transition rules
hysteresis
minimum dwell time
confidence
UNKNOWN condition
```

---

## AUD-IMPL-004 — Model promotion

**Severidade: ALTA**

O documento 18 exige promoção e rollback, mas não define os gates.

Faltam:

```text
minimum improvement
maximum drawdown regression
calibration degradation
minimum sample
minimum OOS period
rollback threshold
canary duration
```

---

## AUD-IMPL-005 — Replay clock

**Severidade: ALTA**

O documento 21 exige relógio lógico, mas não define a interface Rust.

Recomendação:

```rust
trait Clock {
    fn now(&self) -> Timestamp;
}
```

com:

```text
RealClock
ReplayClock
```

---

# 4. Decisões Bybit que não chegaram a todas as especificações

## AUD-BYBIT-001 — Instrument specification

**Severidade: ALTA**

O documento 15 diz que sizing deve respeitar tick/lot size e regras do instrumento, mas não existe uma especificação formal de:

```text
tick_size
qty_step
min_qty
max_qty
min_notional
price_precision
qty_precision
contract_multiplier
leverage limits
```

### Correção

Criar:

```text
Exchange Instrument Registry
```

e adapter Bybit.

---

## AUD-BYBIT-002 — Rate-limit policy fora da Frente 09

**Severidade: MÉDIA**

A Frente 09 possui monitoramento de rate limit, mas Execution e Failure Recovery não especificam o comportamento quando o limite é atingido.

Definir:

```text
WARNING
THROTTLE
REJECT
CIRCUIT_BREAKER
RECOVERY
```

e quais operações continuam permitidas.

---

## AUD-BYBIT-003 — Order state mapping

**Severidade: ALTA**

Existe estado interno:

```text
ACKNOWLEDGED
PARTIALLY_FILLED
FILLED
...
```

mas não existe tabela formal:

```text
Bybit status
→ Internal status
→ Allowed transitions
```

Isso deve ser obrigatório.

---

## AUD-BYBIT-004 — Reconnect protocol

**Severidade: ALTA**

Há recovery genérico, mas falta protocolo completo:

```text
disconnect
→ reconnect
→ authentication
→ subscription
→ snapshot/state recovery
→ sequence validation
→ reconciliation
→ resume
```

---

# 5. Requisitos sem teste correspondente

**Severidade geral: ALTA**

O documento 23 define categorias de testes, mas não existe matriz requisito → teste.

Exemplos:

| Requisito | Teste explícito? |
|---|---|
| ACK ≠ Fill | Sim, conceitualmente |
| Recovery invariants | Sim |
| Recovery cycles | Parcial |
| Pattern index | Não |
| Probability calibration | Não |
| Regime hysteresis | Não |
| Model rollback | Não |
| Bybit status mapping | Não |
| Rate-limit circuit breaker | Não |
| Instrument rounding | Não |
| Replay determinism | Sim, conceitualmente |
| 500 µs p99 | Sim, conceitualmente |
| 1 ms p99.9 | Sim, conceitualmente |

### Correção

Criar matriz:

```text
REQ-ID
SPEC
RUST MODULE
TEST-ID
BENCH-ID
METRIC
PASS CRITERIA
```

---

# 6. Riscos sem mecanismo de proteção suficiente

## AUD-RISK-001 — Model uncertainty

Existe Confidence, mas não existe uma regra explícita de como Confidence reduz ou bloqueia exposição.

**Status:** GAP.

---

## AUD-RISK-002 — Stale model

Existe model drift, mas não existe regra:

```text
drift threshold
→ disable model
→ fallback
→ rollback
```

---

## AUD-RISK-003 — Execution simulator mismatch

A própria baseline reconhece que o simulador ainda está em evolução e precisa de fills reais/microestrutura. Isso significa que backtest positivo ainda não é evidência suficiente para live. Fonte: `06_EXECUTION_SIMULATOR.md`, Conclusão; `05_BACKTEST_ENGINE.md`, Conclusão.

---

## AUD-RISK-004 — Persistence failure

A regra de fail-safe existe, mas falta definir:

```text
quanto tempo pode operar sem persistência;
```

e se:

```text
trading continua
ou
new entries bloqueadas
```

durante indisponibilidade do SQLite.

O princípio de manter Risk State em RAM é correto. Fonte: `19_DATABASE_SCHEMA.md`, Princípio e Integridade.

---

# 7. Ambiguidades para Rust

## AUD-RUST-001 — Tipos monetários

A especificação matemática determina que dinheiro não use floating point, mas não existe um contrato único de tipo Rust.

Recomendação:

```rust
type Money = i64;      // unidade mínima
type Bps = i32;
type Price = i64;
type Quantity = i64;
```

ou um tipo decimal fixo formalizado.

---

## AUD-RUST-002 — Timestamps

Definir:

```text
ExchangeTimestamp
ReceiveTimestamp
MonotonicTimestamp
LogicalTimestamp
```

e impedir mistura entre relógio de parede e monotônico.

---

## AUD-RUST-003 — Errors

Ainda não existe taxonomia de erro.

Criar:

```rust
enum TradingError {
    MarketDataStale,
    SequenceGap,
    RiskDenied,
    UnknownOrder,
    ReconciliationRequired,
    ExchangeRejected,
    RateLimited,
    PersistenceUnavailable,
    InvalidConfiguration,
    ...
}
```

---

## AUD-RUST-004 — State transitions

As máquinas de estado estão documentadas, mas não existe contrato formal de:

```text
allowed transition
guard
side effect
event emitted
rollback behavior
```

---

# 8. Dependências circulares

Não existe um ciclo arquitetural fatal.

A cadeia principal é saudável:

```text
Market
→ Features
→ Regime
→ Pattern
→ Probability
→ EV
→ Sizing
→ Risk
→ Execution
→ Reconciliation
```

Isso está alinhado com a autoridade registrada no Master Index. Fonte: `00_MASTER_INDEX_FINAL.md` §6.

### Entretanto existe um risco lógico

```text
Learning
→ Candidate Model
→ Validation
→ Promotion
→ Decision
→ Learning
```

Isso é um ciclo intencional.

O ciclo só é seguro porque o documento 18 impede Challenger de executar e exige promoção controlada. Fonte: `DECISION_LEARNING.md` §24 e §37.

Portanto:

**não é um bug arquitetural**, mas precisa de gates de promoção formalizados.

---

# 9. Performance incompatível ou subespecificada

## AUD-PERF-001 — SLO interno vs. Pattern Matching

A meta de p99 ≤ 500 µs inclui a cadeia:

```text
features
→ pattern
→ probability
→ EV
→ risk
```

A Frente 09 define essa cadeia como hot path, mas não define budgets individuais. Fonte: `09_PERFORMANCE_LOW_LATENCY.md`, Hot Path e Latency Budget.

### Problema

Sem budgets:

```text
Pattern Matching = 400 µs
Probability = 50 µs
EV = 10 µs
Risk = 20 µs
```

pode passar em alguns testes e falhar sob burst.

### Correção

Definir budget:

```text
Feature Engine       ≤ X µs
Pattern Matching     ≤ X µs
Probability          ≤ X µs
EV                   ≤ X µs
Risk                 ≤ X µs
Order Build          ≤ X µs
```

com margem de segurança.

---

## AUD-PERF-002 — Emergency Path

O Emergency SLO mede até `Order Ready`, não até fill. Isso é correto e evita atribuir à exchange uma garantia que o bot não controla. A documentação já separa explicitamente latência interna de feed, rede, exchange e execução. Fonte: `09_PERFORMANCE_LOW_LATENCY.md`, SLO 2 e Conclusão.

### Porém

O caminho:

```text
Hard Risk
→ Immediate Exit
→ WebSocket
→ execution.fast
```

precisa de uma especificação de comportamento quando o WebSocket está indisponível.

Esse caso deve estar em `22_FAILURE_RECOVERY`.

---

## AUD-PERF-003 — Memory SLO

`RSS growth < 1% em 24h` é mensurável, mas falta definir:

```text
baseline RSS
workload
warm-up
GC/allocator behavior
allowed cache growth
```

O requisito é, portanto, incompleto.

---

# 10. Ausência de rastreabilidade

**Severidade: CRÍTICA**

Este é o maior gap estrutural encontrado.

O Master Index define que a maturidade final deve evoluir:

```text
Specification
→ Implementation
→ Unit Tests
→ Integration Tests
→ Failure Tests
→ Benchmark / Backtest
→ Paper Trading
→ Real-world Evidence
```

mas ainda não existe o nível intermediário de rastreabilidade por requisito. Fonte: `00_MASTER_INDEX_FINAL.md` §11.

## Deve ser criado

`26_REQUIREMENTS_TRACEABILITY_MATRIX.md`

Formato:

| ID | Requisito | Fonte | Rust Module | Test | Benchmark | Métrica | Gate |
|---|---|---|---|---|---|---|---|
| REQ-RISK-001 | Max loss | 03 | `risk/session.rs` | T-RISK-001 | B-RISK-001 | max_loss | PASS |
| REQ-RISK-002 | Recovery budget | 03 | `risk/recovery.rs` | T-REC-001 | — | budget | PASS |
| REQ-EXEC-001 | ACK ≠ fill | 01 | `execution/bybit.rs` | T-EXEC-001 | B-EXEC-001 | state correctness | PASS |
| REQ-PERF-001 | p99 ≤500µs | 09 | `engine/hot_path.rs` | T-PERF-001 | B-PERF-001 | p99 | PASS |
| REQ-DATA-001 | sequence integrity | 04 | `market/orderbook.rs` | T-DATA-001 | B-DATA-001 | gap rate | PASS |

---

# 11. Matriz de severidade

| ID | Severidade | Status |
|---|---|---|
| AUD-MATH-001 | CRÍTICA | Corrigir |
| AUD-MATH-002 | CRÍTICA | Corrigir |
| AUD-MATH-003 | ALTA | Corrigir |
| AUD-MATH-004 | ALTA | Corrigir |
| AUD-PARAM-001 | ALTA | Corrigir |
| AUD-PARAM-002 | ALTA | Corrigir |
| AUD-IMPL-001 | CRÍTICA | Corrigir |
| AUD-IMPL-002 | CRÍTICA | Corrigir |
| AUD-IMPL-003 | ALTA | Corrigir |
| AUD-IMPL-004 | ALTA | Corrigir |
| AUD-IMPL-005 | ALTA | Corrigir |
| AUD-BYBIT-001 | ALTA | Corrigir |
| AUD-BYBIT-002 | MÉDIA | Corrigir |
| AUD-BYBIT-003 | ALTA | Corrigir |
| AUD-BYBIT-004 | ALTA | Corrigir |
| AUD-RISK-001 | ALTA | Corrigir |
| AUD-RISK-002 | ALTA | Corrigir |
| AUD-RISK-003 | CRÍTICA | Corrigir |
| AUD-RISK-004 | MÉDIA | Corrigir |
| AUD-RUST-001 | ALTA | Corrigir |
| AUD-RUST-002 | ALTA | Corrigir |
| AUD-RUST-003 | MÉDIA | Corrigir |
| AUD-RUST-004 | ALTA | Corrigir |
| AUD-PERF-001 | ALTA | Corrigir |
| AUD-PERF-002 | ALTA | Corrigir |
| AUD-PERF-003 | MÉDIA | Corrigir |
| AUD-TRACE-001 | CRÍTICA | Corrigir |

---

# 12. Conclusão da auditoria

## O que está correto

A arquitetura fundamental está boa:

```text
Market
→ Quant
→ Risk
→ Execution
→ Reconciliation
→ Learning
```

A separação de autoridade está correta e evita que o modelo quantitativo contorne o Risk Engine. Fonte: `00_MASTER_INDEX_FINAL.md` §6; `MATHEMATICAL_QUANT_MODEL.md` §2.

A integração Bybit também está conceitualmente correta:

```text
WebSocket Order Entry
+
execution.fast
+
order/execution
+
REST recovery
```

e ACK não é tratado como fill. Fonte: `01_EXECUTION_ENGINE.md`, Conclusão.

O hot path também está corretamente separado do SQLite e do aprendizado. Fonte: `09_PERFORMANCE_LOW_LATENCY.md`, Hot Path.

## O que impede considerar o SDD “fechado”

Não é falta de arquitetura.

É falta de **contratos executáveis e rastreáveis**.

Precisamos transformar:

```text
conceito
```

em:

```text
fórmula
→ parâmetro
→ tipo Rust
→ módulo
→ estado
→ evento
→ teste
→ benchmark
→ métrica
→ gate
```

## Ordem de correção recomendada

### P0 — antes de continuar implementando

1. Corrigir EV ternário/binário.
2. Corrigir semântica de Recovery Episode/Cycle.
3. Definir `WorstCaseSessionLoss`.
4. Criar Configuration Registry.
5. Criar Instrument Registry Bybit.
6. Definir contratos Rust de tipos, estados e erros.
7. Criar Requirements Traceability Matrix.
8. Definir Pattern Index.
9. Definir Probability Calibration.
10. Definir Execution Simulator.

### P1

11. Definir regime detector.
12. Definir model promotion gates.
13. Definir Bybit reconnect/state mapping.
14. Definir performance budgets por estágio.
15. Definir replay clock e determinismo.
16. Definir persistence failure policy.

### P2

17. Threat model.
18. Tamper-evident audit.
19. Dataset lineage.
20. Experiment registry.
21. Canary/rollback operacional.

## Veredito

**O SDD v2 não deve ser congelado ainda.**

Mas a arquitetura está suficientemente madura para a próxima fase.

O próximo documento que eu criaria **não é uma nova frente de trading**.

É:

> **`26_REQUIREMENTS_TRACEABILITY_MATRIX.md`**

Porque ele vai conectar os 24 documentos ao **código Rust, testes, benchmarks e SLOs**.

Depois dele, eu criaria uma pequena camada de correções P0 nos documentos existentes, em vez de simplesmente continuar adicionando MDs.



---

# 13. Avaliação de maturidade por frente

Transferida em 2026-09-20 das próprias especificações, onde vivia como
autoavaliação no fim de cada documento. Uma especificação não se dá nota: quem
avalia é este documento.

| Frente | Avaliação registrada | O que faltava |
|---|---|---|
| 01 Execution Engine | arquitetura de produção forte | separar aceitação, lifecycle e execução já está feito |
| 02 Position Reconciliation | forte | testar recuperação sob falhas reais e fault injection |
| 04 Market Data Quality | forte, alinhada ao feed da Bybit | benchmark com eventos reais e bursts de até 1024 trades |
| 05 Backtest Engine | forte, mas não estado da arte | latência, slippage por liquidez, partial fill, funding, replay determinístico |
| 06 Execution Simulator | precisa de evolução significativa | calibração com fills reais e replay de microestrutura |
| 07 Decision Ledger / Learning | conceitualmente forte, não estado da arte | model registry, lineage, dataset hashing, canary, rollback automático |
| 08 Security / Observability | bom, não estado da arte | threat modeling, least privilege, rotação de segredo, trilha à prova de adulteração |
| 09 Performance | calibrada para a infraestrutura atual da Bybit | budgets por estágio obtidos por profiling |

A avaliação de 07 foi recuperada do histórico do repositório: aquele documento
virou índice em 2026-09-20 e a nota teria se perdido com ele.
