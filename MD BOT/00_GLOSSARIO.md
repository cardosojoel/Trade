# 00 — Glossário Canônico

**Status:** normativo · **Versão:** 1.1 · **Atualizado em:** 2026-09-20

## Regra de autoridade

Este documento define o vocabulário do conjunto. Nenhum outro documento pode
introduzir sinônimo para um termo aqui definido, nem reusar um símbolo já
atribuído.

Quando um termo desta tabela aparecer em qualquer documento, é este o
significado. Um documento que precise de um conceito novo declara o termo aqui
primeiro.

A coluna `parameter_id` aponta para o identificador no
[`27_CONFIGURATION_REGISTRY.md`](27_CONFIGURATION_REGISTRY.md), que continua
sendo a única autoridade sobre **valores**. Este glossário governa **nomes**.

---

## 1. Sessão e capital

| Termo canônico | Símbolo | Definição | `parameter_id` |
|---|---|---|---|
| Depósito inicial | `D` | Capital da sessão no instante da confirmação. Imutável até o encerramento. | — |
| Limite de perda do depósito | `L` | Fração de `D` que a sessão pode perder do capital original. | `risk.max_loss_pct` |
| MaxLossDeposit | — | `D × L`. Perda máxima autorizada sobre o capital original. | — |
| CapitalFloor | — | `D − MaxLossDeposit`. Menor equity permitida antes de o limite ser atingido. | — |
| PeakEquity | — | Maior equity registrada na sessão. Nunca diminui. | — |
| RealizedProfit | — | `max(0, RealizedEquity − D)`. Somente lucro realizado. | — |

## 2. Recovery

| Termo canônico | Símbolo | Definição | `parameter_id` |
|---|---|---|---|
| Fração de lucro elegível | `P` | Fração do `RealizedProfit` que pode ser recolocada em risco. | `risk.recovery_profit_pct` |
| RecoveryEligibleProfit | — | `RealizedProfit × P`. | — |
| ProtectedProfit | — | `RealizedProfit − RecoveryEligibleProfit`. Nunca volta a risco. | — |
| Teto estrutural de Recovery | `R` | Fração de `D` que limita o Recovery independentemente do lucro. | `risk.recovery_max_pct` |
| RecoveryMaxSession | — | `D × R`. | — |
| RecoveryBudgetSession | — | `min(RecoveryEligibleProfit, RecoveryMaxSession)` no instante da autorização. Criado **uma única vez** por sessão. | — |
| RecoveryBudgetRemaining | — | Saldo consumível do orçamento. Perdas reduzem; ganhos **não** repõem. | — |
| RecoveryEpisode | — | Unidade operacional de recuperação dentro da sessão. Possui alvo e tentativas próprios. | — |
| RecoveryTarget | — | Perda que o **episódio corrente** se propõe a recuperar. Pertence ao episódio, não à sessão. | — |
| RecoveredAmount | — | Soma líquida dos resultados das operações do episódio. | — |
| RecoveryProgress | — | `RecoveredAmount / RecoveryTarget`, limitado a `[0, 1]`. | — |
| Máximo de episódios | `Emax` | Número máximo de `RecoveryEpisode` por sessão. | `risk.max_recovery_episodes` |
| Máximo de tentativas | `Amax` | Número máximo de tentativas dentro de um episódio. | `risk.max_recovery_attempts` |

**Budget e episódio são grandezas diferentes.** O budget é global à sessão e
consumível; o episódio é uma unidade operacional. Concluir um episódio com
sucesso **não** cria orçamento novo.

## 3. Risco por operação

| Termo canônico | Símbolo | Definição | `parameter_id` |
|---|---|---|---|
| Risco normal por operação | `T` | Fração de `D` arriscável em uma operação fora de Recovery. | `risk.max_risk_per_trade_pct` |
| NormalTradeRisk | — | `D × T`. | — |
| Risco de Recovery por operação | `RT` | Fração do `RecoveryBudgetRemaining` arriscável em uma operação de Recovery. | `risk.max_recovery_risk_pct` |
| RecoveryTradeRisk | — | `RecoveryBudgetRemaining × RT`. Diminui a cada perda. | — |
| AllowedTradeRisk | — | Menor valor entre todos os limites aplicáveis. É o único valor que autoriza tamanho. | — |
| WorstCaseSessionExposure | — | `MaxLossDeposit + RecoveryMaxSession`. Exposição máxima **teórica** autorizada pela política — não é previsão de perda. | — |

`MaxLossDeposit` e `RecoveryMaxSession` MUST ser exibidos separadamente antes da
soma: são fontes de capital diferentes.

## 4. Estados de sessão

`CREATED`, `CONFIRMING`, `ACTIVE`, `PROFIT_PROTECTED`, `LOSS_LIMIT_REACHED`,
`RECOVERY`, `RECOVERY_SUCCESS`, `RECOVERY_FAILED`, `STOPPED`, `ERROR`,
`RECONCILIATION_REQUIRED`, `EMERGENCY_STOP`.

## 5. Decisão quantitativa

| Termo canônico | Símbolo | Definição |
|---|---|---|
| Probabilidade de estado | `P_up`, `P_neutral`, `P_down` | Estimativa ternária para um horizonte explícito. `P_up + P_neutral + P_down = 1`. Em schema persistido: `probability_up`, `probability_neutral`, `probability_down`. |
| Expected Return | `ER` | Retorno esperado em unidade de **retorno**, antes de custos. |
| EV_gross | — | Valor esperado em unidade **monetária**, antes de custos. |
| Custos totais | `C_total` | `fees + spread + slippage + funding + execution cost`. |
| EV_net | — | `EV_gross − C_total`. Único valor que o Decision Engine pode usar. |
| EV_conservative | — | `EV_net` recalculado sob degradação versionada de probabilidade, retorno favorável, slippage, fees e fill probability. |
| Break-even probability | `P_BE` | `(L_médio + C_total) / (G_médio + L_médio)`. |
| Vizinhos | `K` | Número de padrões históricos selecionados. |
| Sensibilidade da distância | `λ` | Parâmetro do peso exponencial de similaridade. |
| Limiares de classificação | `θ_up`, `θ_down` | Fronteiras de retorno que separam `UP`, `NEUTRAL` e `DOWN`. |
| Effective Sample Size | `ESS` | `1 / Σ w_i²`. Tamanho efetivo da amostra ponderada. |
| Confidence | — | Confiabilidade da estimativa. **Não** é `P_up`. |
| Ações registráveis | — | `BUY`, `SELL`, `HOLD`, `NO_TRADE`, `DENY_BY_RISK`. |

`C` **não** é um símbolo válido: foi usado para custos e para ciclos de Recovery
em documentos diferentes. Use `C_total` para custos e `Emax` para episódios.

## 6. Regime de mercado

Enum canônico — é o único vocabulário de regime admitido:

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

Regime é hipótese estatística sobre o **mercado**. `RECOVERY` não é regime: é
estado de sessão (seção 4).

A granulação de volatilidade usada em análise de desempenho — `LOW`, `NORMAL`,
`HIGH`, `EXTREME` — é `VolatilityBucket`, grandeza distinta de `MarketRegime`.

## 7. Modelo e aprendizado

| Termo canônico | Definição |
|---|---|
| `LIVE_MODEL` | Modelo autorizado a decidir. Imutável após aprovação. |
| `CANDIDATE_MODEL` | Modelo em avaliação. Não possui autoridade de execução. |
| `ModelRegistry` | Registro com status `CANDIDATE`, `VALIDATING`, `PAPER`, `APPROVED`, `LIVE`, `REJECTED`, `RETIRED`. |
| `DecisionSnapshot` | Estado imutável conhecido no instante da decisão. |
| `DecisionOutcome` | Resultado observado, entidade separada do snapshot. |

## 8. Termos aposentados

Cada linha existia em pelo menos um documento e não deve reaparecer.

| Termo aposentado | Canônico | Onde aparecia |
|---|---|---|
| `D0` | `D` | `00_MASTER_INDEX`, `03` |
| `LOSS_LIMIT`, `MAX_DEPOSIT_LOSS`, `DepositLossLimit` | `MaxLossDeposit` | spec de risco, `25` |
| `RECOVERY_MAX`, `MaximumRecoveryExposure` | `RecoveryMaxSession` | spec de risco, `25` |
| `INITIAL_RECOVERY_BUDGET` | `RecoveryBudgetSession` | spec de risco |
| `MAX_PROFIT_AT_RISK` | `RecoveryMaxSession` | spec de risco §31 |
| `WorstCaseSessionLoss` | `WorstCaseSessionExposure` | `25` §AUD-MATH-003 |
| ciclo, `cycle`, `RECOVERY_CYCLE`, `MAX_RECOVERY_CYCLES`, `C`, `Cmax` | `RecoveryEpisode`, `Emax` | spec de risco, `03` |
| `A`, `MAX_RECOVERY_ATTEMPTS` | `Amax` | spec de risco |
| `R` como risco monetário por operação | `AllowedTradeRisk` | spec de risco §24, quant model §22 |
| `BULL_TREND` | `TrendUp` | quant model, pattern/EV, learning |
| `BEAR_TREND` | `TrendDown` | quant model, pattern/EV, learning |
| `SIDEWAYS` | `Range` | quant model, pattern/EV, learning |
| `EXTREME_VOLATILITY` (como regime) | `HighVolatility` + `VolatilityBucket::Extreme` | quant model |
| `RECOVERY` (como regime) | estado de sessão | quant model §11 |
| champion / challenger | `LIVE_MODEL` / `CANDIDATE_MODEL` | `18` |
| `EV = P(win)·AvgWin − P(loss)·AvgLoss − Costs` como fórmula independente | projeção binária do EV ternário | quant model §15 |

## 9. Notação

Um mesmo termo pode aparecer em duas grafias: `MaxLossDeposit` em prosa e
`MAX_LOSS_DEPOSIT` dentro de bloco de pseudocódigo. São o mesmo termo. O que
este glossário proíbe é palavra diferente para a mesma coisa, não capitalização
diferente da mesma palavra.

## 10. Linguagem normativa e identificação de requisito

`MUST`, `MUST NOT`, `SHOULD`, `MAY` no sentido da RFC 2119. Texto que não usa
uma dessas formas é explicação, não requisito, e não gera teste.

Todo requisito recebe identificador estável no formato
`REQ-<DOMÍNIO>-<NNN>`, escrito **onde a regra está** — nunca em lista à parte,
que viraria uma segunda camada a manter. O identificador é imutável: um
requisito que deixa de valer é marcado como retirado, nunca renumerado nem
reaproveitado.

Domínios em uso, um por documento normativo:

| Domínio | Documento |
|---|---|
| `EXEC` | `01_EXECUTION_ENGINE.md` |
| `RECON` | `02_POSITION_RECONCILIATION.md` |
| `DATA` | `04_MARKET_DATA_QUALITY.md` |
| `BACKTEST` | `05_BACKTEST_ENGINE.md` |
| `SIM` | `06_EXECUTION_SIMULATOR.md` |
| `SEC` | `08_SECURITY_OBSERVABILITY.md` |
| `PERF` | `09_PERFORMANCE_LOW_LATENCY.md` |
| `DB` | `19_DATABASE_SCHEMA.md` |
| `EVENT` | `20_EVENT_MODEL.md` |
| `REPLAY` | `21_REPLAY_ENGINE.md` |
| `FAIL` | `22_FAILURE_RECOVERY.md` |
| `TEST` | `23_TESTING_STRATEGY.md` |
| `GOV` | `24_CONFIGURATION_AND_GOVERNANCE.md` |
| `CFG` | `27_CONFIGURATION_REGISTRY.md` |
| `BYBIT` | `28_BYBIT_INSTRUMENT_REGISTRY.md` |
| `RUST` | `29_RUST_CONTRACTS.md` |
| `SCOPE` | `32_NAO_OBJETIVOS.md` |
| `RISK`, `RECOVERY` | `trading_risk_recovery_mathematical_spec.md` |
| `FEATURE`, `REGIME`, `EV`, `SIZING`, `STRATEGY` | `MATHEMATICAL_QUANT_MODEL.md` |
| `PATTERN`, `PROB` | `HISTORICAL_PATTERN_PROBABILITY_EV.md` |
| `LEARN` | `DECISION_LEARNING.md` |

A rastreabilidade de cada ID até módulo, teste e métrica vive no
[`26_REQUIREMENTS_TRACEABILITY_MATRIX.md`](26_REQUIREMENTS_TRACEABILITY_MATRIX.md).
