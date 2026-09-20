# Historical Pattern Matching → Probability → Expected Value

**Versão:** 1.1  
**Status:** **normativo** — esta é a fonte de verdade sobre pattern matching,
probabilidade e EV. Os documentos `12`, `13` e `14` são índices para cá.  
**Domínio de requisitos:** `REQ-PATTERN-* e `REQ-PROB-*`  
**Conformidade:** conforme  
**Vocabulário:** [`00_GLOSSARIO.md`](00_GLOSSARIO.md)  
**Escopo:** Transformação de padrões históricos em probabilidade, retorno esperado e valor esperado para decisões de trading

---

## 1. Objetivo

Definir matematicamente o pipeline:

```text
Historical Pattern Matching
        ↓
Probability
        ↓
Expected Return
        ↓
Expected Value
```

O pipeline deve produzir uma estimativa quantitativa da vantagem de uma oportunidade antes que ela seja submetida ao Risk Engine.

O resultado deste módulo **não autoriza uma operação por si só**.

---

# 2. Princípio fundamental

Para o estado atual do mercado `X_t`:

\[
X_t = [x_1,x_2,...,x_m]
\]

o sistema procura estados históricos semelhantes:

\[
X_1,X_2,...,X_N
\]

e utiliza os resultados futuros desses estados para estimar:

\[
P(UP|X_t)
\]

\[
P(DOWN|X_t)
\]

\[
P(NEUTRAL|X_t)
\]

e:

\[
E[R_h|X_t]
\]

para um horizonte `h`.

Depois, os resultados são convertidos em:

\[
EV_h
\]

considerando probabilidade, ganho esperado e custos.

---

# 3. Estado atual

O estado atual deve ser representado por um vetor de features:

```text
X_t = {
    return_1m
    return_5m
    return_15m
    return_1h
    return_4h
    return_24h

    volatility_5m
    volatility_1h
    volatility_4h
    volatility_24h

    volatility_ratio

    trend_score

    atr

    volume_zscore

    drawdown

    regime
}
```

Todas as features devem representar exclusivamente informações disponíveis até `timestamp = t`.

---

# 4. Regra temporal

Para uma decisão em `t`:

\[
Feature(X_t) \leftarrow Data[\leq t]
\]

Nunca:

\[
Feature(X_t) \leftarrow Data[>t]
\]

**REQ-PATTERN-001** Esta regra MUST ser respeitada, para impedir:

- look-ahead bias;
- data leakage;
- resultados artificiais no backtest.

---

# 5. Normalização

Antes do cálculo de similaridade, as features numéricas devem ser normalizadas.

Z-Score:

\[
Z_j =
\frac{x_j-\mu_j}{\sigma_j}
\]

onde:

```text
μ_j = média calculada no conjunto de treinamento
σ_j = desvio padrão calculado no conjunto de treinamento
```

Os parâmetros de normalização devem ser versionados.

**REQ-PATTERN-002** No período de teste:

```text
NÃO recalcular μ e σ.
```

Utilizar os parâmetros derivados exclusivamente do treinamento.

---

# 6. Distância entre padrões

Para o estado atual `X_t` e um estado histórico `X_i`:

\[
d_i =
\sqrt{
\sum_{j=1}^{m}
\alpha_j
(Z_{t,j}-Z_{i,j})^2
}
\]

onde:

```text
α_j = peso da feature j
```

Os pesos devem fazer parte da versão do modelo.

---

# 7. Seleção dos vizinhos

Ordenar:

\[
d_1 \leq d_2 \leq ... \leq d_N
\]

Selecionar os `K` menores valores.

Exemplo inicial:

```text
K = 100
```

O `K` deve ser configurável e validado por walk-forward.

**REQ-PATTERN-003** Não assumir que `K = 100` é matematicamente ótimo. `K` é
hiperparâmetro e MUST NOT ser otimizado no mesmo período usado para avaliar o
resultado.

A métrica de distância MUST ser versionada; a linha de base é a distância
euclidiana ponderada da seção 6. Extensões — Mahalanobis, por exemplo — só
entram após validação fora da amostra.

Os candidatos MUST ser filtrados por regime, timeframe, horizonte e
compatibilidade de versão de features antes do cálculo de distância.

O histórico MUST NOT ser consultado diretamente no hot path: a busca usa índice
ou estrutura em memória.

---

# 8. Similarity Weight

Os padrões mais semelhantes devem possuir maior influência.

Uma função inicial:

\[
q_i =
\frac{1}{d_i+\epsilon}
\]

onde:

\[
\epsilon > 0
\]

é uma constante pequena para evitar divisão por zero.

Normalização:

\[
w_i =
\frac{q_i}
{\sum_{j=1}^{K}q_j}
\]

Logo:

\[
\sum_{i=1}^{K}w_i=1
\]

---

# 9. Alternativa: peso exponencial

Uma alternativa para validação:

\[
q_i=e^{-\lambda d_i}
\]

e:

\[
w_i =
\frac{e^{-\lambda d_i}}
{\sum_{j=1}^{K}e^{-\lambda d_j}}
\]

onde `λ` controla a sensibilidade à distância.

Quanto maior `λ`, maior a concentração nos padrões mais semelhantes.

`λ` deve ser tratado como hiperparâmetro.

---

# 10. Resultado futuro de cada padrão

Para o padrão histórico `i` e horizonte `h`:

\[
R_{i,h}
=
\ln
\left(
\frac{P_{i+h}}{P_i}
\right)
\]

Exemplos de horizonte:

```text
5m
15m
30m
1h
4h
24h
```

Cada decisão pode gerar resultados para todos os horizontes.

---

# 11. Classificação dos resultados

Definir:

```text
θ_up
θ_down
```

Então:

\[
Y_i=
\begin{cases}
UP & R_i>\theta_{up}\\
DOWN & R_i<\theta_{down}\\
NEUTRAL & caso contrário
\end{cases}
\]

Os thresholds devem considerar custos quando apropriado.

Por exemplo, não classificar como `UP` um movimento de apenas `0,01%` se taxas e slippage esperados forem superiores a esse valor.

---

# 12. Probability

A probabilidade ponderada de alta:

\[
P(UP|X_t)
=
\sum_{i=1}^{K}
w_iI(Y_i=UP)
\]

A probabilidade de baixa:

\[
P(DOWN|X_t)
=
\sum_{i=1}^{K}
w_iI(Y_i=DOWN)
\]

A probabilidade neutra:

\[
P(NEUTRAL|X_t)
=
\sum_{i=1}^{K}
w_iI(Y_i=NEUTRAL)
\]

Consequentemente:

\[
P(UP)+P(DOWN)+P(NEUTRAL)=1
\]

**REQ-PROB-001** Em amostra pequena, a contagem ponderada MUST receber smoothing; a estimativa
MUST registrar amostra efetiva, `K`, horizonte, regime, intervalo de confiança e
versão.

---

# 13. Exemplo

Suponha:

```text
K = 100
```

Após aplicar os pesos:

```text
UP      = 0,684
DOWN    = 0,221
NEUTRAL = 0,095
```

Resultado:

```text
P(UP)      = 68,4%
P(DOWN)    = 22,1%
P(NEUTRAL) = 9,5%
```

Não interpretar automaticamente `68,4%` como garantia de alta.

É uma estimativa condicionada ao conjunto de padrões selecionado.

---

# 14. Probabilidade efetiva

**REQ-PROB-002** A probabilidade bruta MUST ser calibrada antes do uso.

Definir:

\[
P_{calibrated}
=
Calibration(P_{raw},Context)
\]

O mecanismo de calibração pode utilizar:

```text
Isotonic Regression
Platt Scaling
Beta Calibration
```

A escolha deve ser validada empiricamente.

---

# 15. Calibração

Se o sistema afirma:

```text
P(UP) ≈ 70%
```

então, em uma população suficientemente grande de situações equivalentes, aproximadamente 70% deveriam resultar em `UP`.

Avaliar:

\[
CalibrationError
\]

e também:

```text
reliability curve
Brier Score
ECE
```

---

# 16. Expected Return

O retorno esperado para o horizonte `h` é:

\[
ER_h
=
\sum_{i=1}^{K}
w_iR_{i,h}
\]

Esse cálculo preserva a magnitude dos retornos históricos, enquanto a probabilidade utiliza a classificação dos resultados.

---

# 17. Expected Return por cenário

Também calcular:

\[
ER_{UP}
=
E[R|UP]
\]

\[
ER_{DOWN}
=
E[R|DOWN]
\]

\[
ER_{NEUTRAL}
=
E[R|NEUTRAL]
\]

Então:

\[
ER =
P(UP)ER_{UP}
+
P(DOWN)ER_{DOWN}
+
P(NEUTRAL)ER_{NEUTRAL}
\]

Essa forma permite auditar a origem do retorno esperado.

---

# 18. Expected Value

A forma canônica e geral do EV vive em
[`MATHEMATICAL_QUANT_MODEL.md`](MATHEMATICAL_QUANT_MODEL.md) §15. O que segue é
a sua especialização para uma operação LONG, e MUST permanecer consistente com
ela.

Para uma operação LONG:

\[
EV =
P(UP)G
-
P(DOWN)L
+
P(NEUTRAL)N
-
C
\]

onde:

```text
G = ganho médio esperado
L = perda média esperada
N = retorno médio no cenário neutro
C = custos esperados
```

Se o cenário neutro for tratado como retorno zero:

\[
EV =
P(UP)G
-
P(DOWN)L
-
C
\]

---

# 19. Custos

Definir:

\[
C_{total} =
Fees
+
Slippage
+
Funding
+
ExecutionCost
\]

Quando aplicável.

Para operações com holding time variável, o funding esperado deve considerar o horizonte estimado.

---

# 20. EV líquido

O valor usado pelo Decision Engine deve ser:

\[
EV_{net}
=
EV_{gross}
-
C
\]

Nunca utilizar somente:

\[
EV_{gross}
\]

para autorizar uma operação real.

---

# 21. Break-even Probability

Para uma operação com:

```text
G = ganho médio
L = perda média
C = custo
```

a probabilidade mínima de ganho pode ser estimada por:

\[
P_{BE}
=
\frac{L+C}{G+L}
\]

O sinal só possui vantagem estatística se:

\[
P_{estimated}>P_{BE}
\]

considerando as premissas do modelo.

---

# 22. Relação Probability × EV

A decisão não deve depender apenas da probabilidade.

Exemplo:

```text
Modelo A:
P(win) = 70%
Gain = 0,3%
Loss = 1,0%
```

pode ser pior que:

```text
Modelo B:
P(win) = 55%
Gain = 2,0%
Loss = 0,8%
```

Portanto:

```text
Probability
     +
Payoff Distribution
     +
Costs
     ↓
Expected Value
```

é superior a utilizar `Win Rate` isoladamente.

---

# 23. Distribuição completa

Além da média, calcular:

```text
mean
median
std_dev
percentiles
p05
p25
p50
p75
p95
```

Para risco:

```text
MAE
MFE
```

A média isolada pode esconder caudas negativas.

---

# 24. Confidence da estimativa

A confiança deve considerar:

```text
sample_size
average_similarity
dispersion_of_returns
probability_calibration
regime_consistency
```

Uma grande probabilidade derivada de poucos padrões não deve receber alta confiança.

---

# 25. Effective Sample Size

Como os padrões possuem pesos diferentes, calcular:

\[
ESS =
\frac{1}
{\sum_{i=1}^{K}w_i^2}
\]

`ESS` representa aproximadamente o tamanho efetivo da amostra.

Exemplo:

```text
K = 100
ESS = 82
```

indica que a distribuição de pesos ainda representa uma amostra relativamente ampla.

Se:

```text
ESS = 8
```

mesmo com `K = 100`, a decisão está essencialmente baseada em poucos padrões.

---

# 26. Similarity Quality

Calcular:

\[
SimilarityQuality =
f(ESS,\bar d,d_{max})
\]

Uma decisão pode ser rejeitada se os padrões encontrados forem pouco semelhantes.

Exemplo:

```text
average_distance > maximum_allowed_distance
```

→

```text
NO_TRADE
```

**REQ-PATTERN-004** Sem amostra mínima ou sem qualidade de similaridade
suficiente, o módulo MUST retornar `INSUFFICIENT_EVIDENCE` — não uma probabilidade de baixa confiança
travestida de estimativa.

---

# 27. Regime Consistency

Verificar se os padrões pertencem ao mesmo regime do estado atual.

Exemplo:

```text
Current:
TrendUp
```

Se a maioria dos vizinhos:

```text
Range
TrendDown
```

a estimativa pode ser considerada menos confiável.

Uma penalização de confiança pode ser aplicada.

---

# 28. Regime-Conditional Probability

Em vez de calcular apenas:

\[
P(UP|X)
\]

podemos calcular:

\[
P(UP|X,Regime)
\]

Isso permite comparar:

```text
P(UP | TrendUp)
P(UP | TrendDown)
P(UP | Range)
```

---

# 29. Volatility-Conditional Probability

Também:

\[
P(UP|X,VolatilityRegime)
\]

permitindo:

```text
LOW_VOL
NORMAL_VOL
HIGH_VOL
EXTREME_VOL
```

---

# 30. Multi-Horizon Probability

O sistema deve produzir:

```text
P_5m
P_15m
P_30m
P_1h
P_4h
P_24h
```

Exemplo:

```text
5m   → 52%
15m  → 59%
30m  → 64%
1h   → 71%
4h   → 67%
24h  → 54%
```

Isso pode indicar que o sinal possui maior vantagem estatística em aproximadamente `1h`.

---

# 31. Multi-Horizon Expected Value

Calcular:

\[
EV_{5m}
\]

\[
EV_{15m}
\]

\[
EV_{30m}
\]

\[
EV_{1h}
\]

\[
EV_{4h}
\]

\[
EV_{24h}
\]

O horizonte escolhido deve maximizar o valor esperado ajustado ao risco e aos custos, e não simplesmente o retorno bruto.

---

# 32. Critério mínimo de qualidade

**REQ-PROB-003** Um candidato MUST satisfazer:

```text
P_calibrated >= P_min
AND
EV_net > EV_min
AND
ESS >= ESS_min
AND
similarity_quality >= minimum
AND
sample_size >= minimum
AND
regime_consistency >= minimum
```

---

# 33. Saída do módulo

O módulo deve produzir uma estrutura conceitual equivalente a:

```text
PatternAnalysis {
    horizon

    neighbors_count
    effective_sample_size

    average_distance
    max_distance

    probability_up
    probability_down
    probability_neutral

    calibrated_probability_up
    calibrated_probability_down
    calibrated_probability_neutral

    expected_return

    expected_gain
    expected_loss

    gross_expected_value
    estimated_cost
    net_expected_value

    mean_return
    median_return
    return_std_dev

    mae
    mfe

    regime_consistency
    similarity_quality

    confidence
}
```

---

# 34. Fluxo completo

```text
CURRENT MARKET STATE
        ↓
FEATURE VECTOR
        ↓
NORMALIZATION
        ↓
HISTORICAL PATTERN SEARCH
        ↓
DISTANCE CALCULATION
        ↓
TOP-K PATTERNS
        ↓
SIMILARITY WEIGHTS
        ↓
HISTORICAL OUTCOMES
        ↓
┌─────────────────────────────┐
│         PROBABILITY         │
│ UP / DOWN / NEUTRAL         │
└──────────────┬──────────────┘
               ↓
        CALIBRATION
               ↓
┌─────────────────────────────┐
│      EXPECTED RETURN        │
│   Distribution of Returns   │
└──────────────┬──────────────┘
               ↓
       COST ADJUSTMENT
               ↓
┌─────────────────────────────┐
│      EXPECTED VALUE         │
│          EV NET             │
└──────────────┬──────────────┘
               ↓
       DECISION ENGINE
               ↓
          RISK ENGINE
               ↓
        ALLOW / DENY
```

---

# 35. Regra de autoridade

Este módulo pode produzir:

```text
Probability
Expected Return
Expected Value
Confidence
```

Mas não pode modificar:

```text
Session Deposit
Maximum Loss
Recovery Budget
Maximum Recovery Attempts
Risk Per Trade
Session Stop
Kill Switch
```

Esses parâmetros pertencem exclusivamente ao Risk & Recovery Engine.

---

# 36. Requisitos contra overfitting

Os seguintes parâmetros devem ser validados fora da amostra:

```text
K
feature_weights
distance_metric
similarity_function
λ
probability_threshold
UP threshold
DOWN threshold
minimum_EV
minimum_ESS
minimum_similarity
```

Não selecionar parâmetros apenas pelo maior lucro histórico.

---

# 37. Validação temporal

O pipeline deve ser avaliado utilizando:

```text
TRAIN
    ↓
VALIDATION
    ↓
TEST
```

e posteriormente:

```text
WALK-FORWARD
```

O cálculo de padrões históricos deve respeitar o corte temporal de cada período.

---

# 38. Integridade temporal

**REQ-PATTERN-005** Para uma decisão em `t`:

```text
Allowed:
market_data <= t
historical_patterns < t
outcomes dos padrões < t
```

Proibido:

```text
market_data > t
future outcomes
future normalization parameters
future labels
future feature statistics
```

---

# 39. Princípio matemático final

A cadeia de decisão quantitativa deve ser:

\[
HistoricalPatterns
\rightarrow
Probability
\rightarrow
ExpectedReturn
\rightarrow
ExpectedValue
\]

ou, operacionalmente:

```text
"Quais situações históricas se parecem com esta?"
                    ↓
"Qual foi a distribuição de resultados?"
                    ↓
"Qual a probabilidade dos cenários?"
                    ↓
"Qual retorno podemos esperar?"
                    ↓
"Depois dos custos, existe vantagem?"
                    ↓
"O Risk Engine permite assumir esse risco?"
```

A última pergunta pertence ao Risk Engine e nunca ao modelo estatístico.
