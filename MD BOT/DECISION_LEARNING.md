# Trading Bot — Decision Learning & Historical Learning System

**Versão:** 1.1  
**Status:** **normativo** — esta é a fonte de verdade sobre ledger de decisões,
aprendizado, validação e promoção de modelo. Os documentos `07`, `17` e `18` são
índices para cá.  
**Domínio de requisitos:** `REQ-LEARN-*`  
**Vocabulário:** [`00_GLOSSARIO.md`](00_GLOSSARIO.md)  
**Escopo:** Histórico de decisões, resultados, avaliação, aprendizado e evolução controlada do modelo

---

# 1. Objetivo

Construir uma base de aprendizado que permita ao bot melhorar sua capacidade de decisão usando suas próprias decisões históricas sem criar:

- look-ahead bias;
- data leakage;
- overfitting;
- feedback loops perigosos;
- alteração retroativa de decisões;
- aumento automático de risco.

O histórico deve ser tratado como um **banco de experimentos quantitativos**.

---

# 2. Princípio fundamental

Cada decisão deve responder:

```text
O que o bot sabia?
O que o bot previu?
Qual decisão tomou?
Quanto confiava?
Quanto arriscou?
O que aconteceu depois?
Quanto custou?
Em qual regime ocorreu?
Qual versão do modelo tomou a decisão?
```

---

# 3. Decision Snapshot

Criar uma entidade imutável:

```text
DecisionSnapshot
```

Ela representa o estado completo conhecido pelo bot no momento da decisão.

**REQ-LEARN-001** Depois de criado:

```text
DecisionSnapshot = IMMUTABLE
```

Nunca recalcular o snapshot utilizando dados futuros.

---

# 4. Registro de todas as decisões

Registrar:

```text
BUY
SELL
HOLD
NO_TRADE
DENY_BY_RISK
```

**REQ-LEARN-002** O ledger MUST registrar toda decisão, não apenas as
operações executadas.

Isso permite medir:

```text
oportunidades aceitas
oportunidades recusadas
oportunidades bloqueadas pelo risco
```

---

# 5. Schema conceitual

```text
DecisionSnapshot {
    decision_id
    timestamp
    symbol

    market_snapshot

    feature_vector

    model_version

    probability_up
    probability_down
    probability_neutral

    expected_return
    expected_value
    confidence

    proposed_action

    risk_snapshot

    final_action
}
```

---

# 6. Market Snapshot

Deve armazenar:

```text
price
open
high
low
close
volume
trade_count
timestamp
timeframe
```

O snapshot deve representar exclusivamente dados disponíveis até o timestamp da decisão.

---

# 7. Feature Snapshot

Armazenar exatamente as features utilizadas:

```text
returns
momentum
volatility
ATR
EMA
trend_score
volume_zscore
drawdown
regime
```

Não substituir posteriormente valores antigos por versões recalculadas.

---

# 8. Model Version

Toda decisão deve registrar:

```text
model_version
```

Exemplo:

```text
model_v001
model_v002
model_v003
```

O modelo utilizado deve ser recuperável de forma determinística.

Registrar também:

```text
feature_schema_version
configuration_version
risk_configuration_version
```

---

# 9. Outcome

Após o horizonte definido, gerar:

```text
DecisionOutcome
```

Exemplo:

```text
DecisionOutcome {
    decision_id

    return_5m
    return_15m
    return_30m
    return_1h
    return_4h
    return_24h

    maximum_favorable_excursion
    maximum_adverse_excursion

    gross_pnl
    fees
    slippage
    net_pnl

    result
}
```

---

# 10. Horizontes independentes

Uma decisão deve poder ser avaliada em múltiplos horizontes:

```text
5m
15m
30m
1h
4h
24h
```

Isso permite descobrir o horizonte em que o sinal possui maior vantagem estatística.

---

# 11. MFE

Maximum Favorable Excursion:

```text
maior movimento favorável após a entrada
```

Deve ser calculado independentemente do resultado final.

---

# 12. MAE

Maximum Adverse Excursion:

```text
maior movimento contrário após a entrada
```

MAE é fundamental para avaliar:

- stop loss;
- volatilidade;
- sizing;
- qualidade do sinal.

---

# 13. No-Trade Outcomes

Decisões `NO_TRADE` também devem receber outcome.

Exemplo:

```text
decision:
NO_TRADE

P(up):
62%

1h return:
+1,4%
```

Isso permite medir:

```text
false negative
```

ou oportunidades que o sistema deixou passar.

---

# 14. Decision Ledger

O histórico lógico deve ser:

```text
Decision
    ↓
Outcome
    ↓
Evaluation
```

Nunca:

```text
Decision
    ↓
editar Decision depois do resultado
```

**REQ-LEARN-003** O resultado MUST ser entidade separada da decisão; a decisão
MUST NOT ser editada depois do resultado.

---

# 15. Métricas por versão

Para cada `model_version` calcular:

```text
sample_size
win_rate
average_return
median_return
average_win
average_loss
expectancy
profit_factor
max_drawdown
sharpe
sortino
MAE
MFE
calibration_error
```

Também separar por:

```text
regime
timeframe
confidence_bucket
probability_bucket
volatility_bucket
```

---

# 16. Probability Calibration

Se o modelo produz:

```text
P(up) = 70%
```

aproximadamente 70% das situações equivalentes deveriam resultar em alta dentro do horizonte definido.

Agrupar:

```text
50-55%
55-60%
60-65%
65-70%
70-75%
75-80%
80-90%
90-100%
```

Comparar:

```text
predicted probability
vs
observed frequency
```

---

# 17. Calibration Error

Usar uma métrica de calibração, por exemplo:

\[
ECE =
\sum_b
\frac{n_b}{N}
|\text{accuracy}_b-\text{confidence}_b|
\]

Quanto menor:

```text
melhor calibração
```

---

# 18. Performance por regime

Criar matriz:

```text
                    REGIME

           TrendUp  TrendDown  Range  HighVolatility
----------------------------------------------------
P > 70%
P 60-70%
P 50-60%
P < 50%
```

Para cada célula calcular:

```text
N
win_rate
EV
drawdown
```

Isso permite identificar onde o modelo realmente funciona.

---

# 19. Performance por volatilidade

Separar:

```text
LOW
NORMAL
HIGH
EXTREME
```

O modelo deve aprender se sua vantagem estatística depende de determinado nível de volatilidade.

---

# 20. Performance por confidence

Separar:

```text
0-20%
20-40%
40-60%
60-80%
80-100%
```

O objetivo é verificar se:

```text
higher confidence
```

realmente corresponde a:

```text
higher realized edge
```

---

# 21. Pattern Learning

Para cada novo estado `X_t`, buscar estados históricos semelhantes.

Usar distância normalizada:

\[
d(X_t,X_i)
=
\sqrt{
\sum_j w_j(X_{t,j}-X_{i,j})^2
}
\]

Selecionar os `K` vizinhos mais próximos.

---

# 22. Historical Pattern Record

Armazenar:

```text
pattern_id
feature_schema_version
timestamp
feature_vector
distance
outcome
regime
model_version
```

Nunca permitir que o resultado futuro altere o `feature_vector` histórico.

---

# 23. Minimum Sample

Não considerar um padrão confiável sem quantidade mínima de observações.

Exemplo:

```text
N < 100
    insuficiente

100-500
    baixa confiança

500-2.000
    moderada

> 2.000
    alta
```

Os thresholds devem ser validados.

---

# 24. Candidate Model

O modelo atual:

```text
LIVE_MODEL
```

não deve ser alterado diretamente pelo processo de aprendizado.

Criar:

```text
CANDIDATE_MODEL
```

Fluxo:

```text
LIVE_MODEL
    ↓
historical evaluation
    ↓
candidate generation
    ↓
backtest
    ↓
walk-forward
    ↓
paper trading
    ↓
validation
    ↓
promotion
```

---

# 25. Model Registry

Manter:

```text
ModelRegistry
```

com:

```text
model_id
version
created_at
training_period
validation_period
feature_schema_version
parameters
metrics
status
parent_model
```

Status:

```text
CANDIDATE
VALIDATING
PAPER
APPROVED
LIVE
REJECTED
RETIRED
```

---

# 26. Walk-Forward Validation

Nunca treinar e testar utilizando o mesmo período.

Exemplo:

```text
TRAIN:
2024

TEST:
2025 Q1
```

Depois:

```text
TRAIN:
2024 + 2025 Q1

TEST:
2025 Q2
```

E assim sucessivamente.

O teste sempre deve representar um período posterior ao treinamento.

---

# 27. Purged Time Split

Quando houver dependência temporal entre observações, remover uma janela de segurança entre treino e teste.

Exemplo:

```text
TRAIN
───────────────

GAP
──────

TEST
───────────────
```

Isso reduz leakage causado por labels sobrepostos.

---

# 28. Embargo

Se necessário, aplicar:

```text
embargo_period
```

após o final do conjunto de treinamento antes de iniciar o teste.

---

# 29. Data Leakage

**REQ-LEARN-004** É proibido utilizar no snapshot:

```text
future price
future volume
future volatility
future indicator
future outcome
future label
```

Também é proibido recalcular indicadores históricos usando dados futuros.

---

# 30. Look-Ahead Bias

Uma decisão em:

```text
10:00
```

somente pode utilizar:

```text
dados <= 10:00
```

Nunca:

```text
dados > 10:00
```

Essa regra deve ser testável automaticamente.

---

# 31. Leakage Test

**REQ-LEARN-005** MUST existir teste automatizado que verifique:

```text
snapshot.timestamp >= feature_source.timestamp
```

e que nenhum feature tenha dependência de timestamps futuros.

Falha:

```text
MODEL_VALIDATION = FAIL
```

---

# 32. Overfitting

Nunca aceitar um modelo apenas porque:

```text
backtest_profit > previous_profit
```

Avaliar:

```text
out-of-sample performance
stability
drawdown
expectancy
calibration
profit factor
performance across regimes
performance across time
```

---

# 33. Robustez

O modelo candidato deve ser testado com variações razoáveis de:

```text
fees
slippage
entry delay
exit delay
parameter perturbation
```

Se pequenas alterações destruírem a performance:

```text
MODEL = FRAGILE
```

e não deve ser promovido.

---

# 34. No Data Snooping

Não escolher parâmetros porque produziram o melhor resultado em um único período histórico.

Evitar:

```text
testar 1.000 parâmetros
escolher o melhor
```

sem correção e validação independente.

---

# 35. Feedback Loop

**REQ-LEARN-006** O modelo MUST NOT usar seu próprio resultado recente para
alterar parâmetro em live.

Errado:

```text
LOSS
↓
alterar modelo
↓
LOSS
↓
alterar novamente
```

Correto:

```text
RESULTS
↓
ACCUMULATE
↓
EVALUATE
↓
CANDIDATE
↓
VALIDATE
↓
PROMOTE
```

---

# 36. Learning Frequency

O aprendizado deve ser desacoplado da execução.

Exemplo:

```text
TRADING ENGINE
    execução contínua

LEARNING ENGINE
    avaliação periódica
```

Nunca alterar o modelo durante uma posição aberta.

---

# 37. Model Freeze

Depois de aprovado para live:

```text
LIVE_MODEL
```

MUST ser imutável (**REQ-LEARN-007**).

Qualquer alteração gera:

```text
new model_version
```

---

# 38. Promotion Criteria

Exemplo conceitual:

```text
candidate.out_of_sample_ev
    > live.out_of_sample_ev

AND

candidate.max_drawdown
    <= acceptable_limit

AND

candidate.calibration_error
    <= maximum_allowed_error

AND

candidate.sample_size
    >= minimum_sample

AND

candidate.performance_stable_across_regimes
```

**REQ-LEARN-008** A promoção MUST exigir todos os critérios obrigatórios, e cada um MUST ser
versionado — melhoria mínima, regressão máxima de drawdown, degradação máxima de
calibração, amostra mínima e período mínimo fora da amostra.

A promoção MAY passar por canary, limitando capital, símbolos e duração antes da
exposição completa.

---

# 39. Model Rollback

Se o modelo live apresentar deterioração significativa:

```text
LIVE_MODEL
    ↓
performance monitor
    ↓
threshold violation
    ↓
ROLLBACK
```

**REQ-LEARN-009** Rollback MUST restaurar uma versão anteriormente aprovada, e MUST ser
automático quando um threshold crítico versionado for violado.

O aprendizado MUST NOT alterar diretamente política de risco, kill switch,
credenciais ou permissões.

---

# 40. Model Drift

Monitorar mudança entre:

```text
historical distribution
vs
current distribution
```

Features prioritárias:

```text
returns
volatility
volume
trend
regime
signal_probability
expected_value
```

Detectar:

```text
distribution drift
performance drift
calibration drift
```

---

# 41. Regra de parada do modelo

Se:

```text
expected_value < minimum
```

ou:

```text
calibration_error > maximum
```

ou:

```text
drawdown > threshold
```

o modelo pode ser colocado em:

```text
DEGRADED
```

ou:

```text
PAUSED
```

A decisão deve ser separada do Risk & Recovery Engine.

---

# 42. Dataset Versioning

Todo dataset utilizado para treinamento/validação deve ter:

```text
dataset_id
created_at
source
time_range
feature_schema_version
label_schema_version
hash
```

O resultado deve ser reproduzível.

---

# 43. Label Versioning

Labels devem ser versionados.

Exemplo:

```text
label_v001:
return_1h

label_v002:
risk_adjusted_return_1h

label_v003:
return_after_costs_1h
```

Nunca alterar a definição de um label antigo.

---

# 44. Reprodutibilidade

Um experimento deve poder ser reconstruído utilizando:

```text
dataset_id
model_version
feature_schema_version
label_schema_version
configuration
random_seed
code_version
```

---

# 45. Experiment Registry

Registrar:

```text
experiment_id
dataset_id
model_version
parameters
training_period
validation_period
metrics
created_at
result
```

Resultado:

```text
ACCEPTED
REJECTED
INCONCLUSIVE
```

---

# 46. Métricas mínimas do experimento

Todo experimento deve produzir:

```text
sample_size
win_rate
expectancy
profit_factor
max_drawdown
sharpe
sortino
average_win
average_loss
MAE
MFE
calibration_error
```

Também:

```text
performance_by_regime
performance_by_volatility
performance_by_probability_bucket
```

---

# 47. Aprendizado do horizonte

O sistema deve avaliar:

```text
5m
15m
30m
1h
4h
24h
```

para descobrir onde o sinal apresenta maior:

```text
Expected Value
```

O horizonte de execução pode posteriormente ser tratado como parte do modelo.

---

# 48. Meta-Model

Após acumular histórico suficiente, pode existir um:

```text
MetaModel
```

Sua função é estimar:

> Em quais condições a previsão do modelo principal é confiável?

Exemplo:

```text
Primary Model:
P(up) = 72%
```

MetaModel:

```text
calibration_factor = 0.75
```

Resultado:

```text
adjusted_probability
```

O MetaModel não pode aumentar risco diretamente.

---

# 49. Aprendizado supervisionado

O primeiro estágio recomendado é supervisionado e offline.

Pipeline:

```text
Historical Data
    ↓
Feature Generation
    ↓
Decision Reconstruction
    ↓
Label Generation
    ↓
Training
    ↓
Validation
    ↓
Walk-Forward
```

Não começar com Reinforcement Learning.

---

# 50. Reinforcement Learning

RL só deve ser considerado depois que existir:

```text
dataset confiável
decision ledger
outcome ledger
simulador
execution model
cost model
walk-forward framework
```

O Risk Engine deve continuar independente mesmo se RL for introduzido.

---

# 51. Regra de ouro

O bot pode aprender:

```text
quando operar
quando não operar
qual sinal é mais confiável
qual horizonte funciona melhor
qual regime favorece a estratégia
```

**REQ-LEARN-010** O bot MUST NOT aprender livremente:

```text
quanto pode perder
quanto do depósito pode perder
quanto do lucro protegido pode arriscar
quando ignorar o Risk Engine
```

---

# 52. Arquitetura final

```text
                         MARKET DATA
                              │
                              ▼
                       FEATURE ENGINE
                              │
                              ▼
                        SIGNAL MODEL
                              │
                  ┌───────────┴───────────┐
                  ▼                       ▼
             PROBABILITY                  EV
                  │                       │
                  └───────────┬───────────┘
                              ▼
                         META MODEL
                              │
                              ▼
                       DECISION ENGINE
                              │
                              ▼
                     DECISION SNAPSHOT
                              │
                              ▼
                       RISK ENGINE
                              │
                      ┌───────┴───────┐
                      ▼               ▼
                    ALLOW            DENY
                      │
                      ▼
                   EXECUTE
                      │
                      ▼
                 OUTCOME ENGINE
                      │
                      ▼
                DECISION OUTCOME
                      │
                      ▼
               LEARNING DATABASE
                      │
                      ▼
               MODEL EVALUATOR
                      │
                      ▼
               CANDIDATE MODEL
                      │
                      ▼
               WALK-FORWARD TEST
                      │
                      ▼
                PAPER TRADING
                      │
                      ▼
                  VALIDATION
                      │
                      ▼
                MODEL REGISTRY
                      │
                 ┌────┴────┐
                 ▼         ▼
               LIVE      REJECTED
                 │
                 ▼
              MONITOR
                 │
                 ▼
              ROLLBACK
```

---

# 53. Regra definitiva

O sistema de aprendizado deve ser:

```text
offline-first
versionado
reprodutível
auditável
temporalmente correto
sem look-ahead
sem leakage
sem alteração retroativa
com validação out-of-sample
```

E principalmente:

> **O aprendizado pode melhorar a qualidade da decisão. Ele nunca pode aumentar a autoridade do modelo sobre o Risk Engine.**
