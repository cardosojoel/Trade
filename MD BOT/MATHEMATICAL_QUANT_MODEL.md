# Trading Bot — Base Matemática Quantitativa

**Versão:** 1.0  
**Status:** Especificação base  
**Escopo:** Modelo matemático para geração, avaliação e controle de sinais de trading de Bitcoin

---

## 1. Objetivo

Definir uma base matemática determinística para que o bot:

1. transforme histórico de mercado em variáveis quantitativas;
2. identifique regimes de mercado;
3. estime probabilidade de movimentos futuros;
4. calcule retorno esperado e valor esperado;
5. determine se existe vantagem estatística suficiente para considerar uma operação;
6. dimensione a posição de acordo com o Risk Engine;
7. produza dados estruturados para posterior aprendizado.

Esta especificação **não permite que o modelo quantitativo ultrapasse os limites do Risk & Recovery Engine**.

---

# 2. Princípio de separação

O sistema deve separar quatro responsabilidades:

```text
MARKET MODEL
    ↓
SIGNAL MODEL
    ↓
RISK ENGINE
    ↓
EXECUTION
```

O modelo de mercado pode dizer:

```text
BUY
P(up) = 73%
EV = +0,31%
```

O Risk Engine pode responder:

```text
DENY
```

Nesse caso:

```text
DECISION = DENY
```

O modelo nunca pode alterar:

- depósito;
- limite de perda;
- Recovery Budget;
- risco máximo;
- regras de encerramento da sessão.

---

# 3. Dados de mercado

O modelo deve trabalhar inicialmente com:

```text
timestamp
open
high
low
close
volume
trade_count
```

Sempre preservar o timestamp original e a granularidade.

---

# 4. Retorno logarítmico

Para preço `P`:

\[
r_t = \ln(P_t/P_{t-1})
\]

Para horizonte `n`:

\[
R_n = \ln(P_t/P_{t-n})
\]

O retorno logarítmico será a base para:

- momentum;
- volatilidade;
- distribuição de retornos;
- comparação entre períodos.

---

# 5. Momentum

Calcular múltiplos horizontes:

```text
1m
5m
15m
1h
4h
24h
7d
```

Para cada horizonte:

\[
M_n = \ln(P_t/P_{t-n})
\]

Normalização:

\[
M_{norm,n} = M_n/\sigma_n
\]

Onde `σ_n` representa a volatilidade correspondente ao horizonte.

---

# 6. Volatilidade

Para retornos:

\[
\sigma_n = StdDev(r)
\]

Calcular pelo menos:

```text
volatility_5m
volatility_1h
volatility_4h
volatility_24h
volatility_7d
```

Também calcular:

\[
VR = \sigma_{curta}/\sigma_{longa}
\]

O `VR` representa expansão ou contração relativa de volatilidade.

---

# 7. Tendência

Utilizar inicialmente médias móveis exponenciais:

```text
EMA_fast
EMA_slow
```

Definir:

\[
TrendRaw = EMA_{fast} - EMA_{slow}
\]

Normalizar:

\[
TrendScore = TrendRaw/\sigma_{price}
\]

Interpretação:

```text
TrendScore > +1
    tendência positiva relevante

TrendScore < -1
    tendência negativa relevante

-1 <= TrendScore <= +1
    tendência indefinida/lateral
```

Os thresholds devem ser tratados como parâmetros de validação, não como constantes universais.

---

# 8. Volume

Calcular Z-Score:

\[
VolumeZ =
\frac{Volume_t-\mu_V}{\sigma_V}
\]

Uso:

- identificar expansão anormal de volume;
- confirmar movimentos;
- diferenciar movimento com baixa participação de movimento com alta participação.

Volume não deve ser usado isoladamente como sinal.

---

# 9. ATR

Calcular True Range:

\[
TR_t =
max(
High-Low,
|High-Close_{t-1}|,
|Low-Close_{t-1}|
)
\]

Depois:

\[
ATR_n = MA(TR,n)
\]

O ATR poderá ser utilizado para:

- Stop Loss;
- dimensionamento de posição;
- comparação de volatilidade;
- normalização de movimentos.

---

# 10. Drawdown

Definir:

\[
PeakPrice_t = max(P_0,...,P_t)
\]

\[
Drawdown_t =
(P_t-PeakPrice_t)/PeakPrice_t
\]

O drawdown deve ser uma feature independente.

---

# 11. Regime de mercado

O modelo deve classificar o mercado em categorias:

```text
BULL_TREND
BEAR_TREND
SIDEWAYS
HIGH_VOLATILITY
EXTREME_VOLATILITY
RECOVERY
UNKNOWN
```

A classificação deve utilizar múltiplas features.

Exemplo conceitual:

```text
TrendScore > threshold
AND
VolatilityRatio dentro do intervalo
→ BULL_TREND
```

A classificação deve ser determinística para um mesmo snapshot.

---

# 12. Feature Vector

Cada instante elegível deve gerar:

```text
FeatureVector {
    return_1m
    return_5m
    return_15m
    return_1h
    return_4h
    return_24h

    momentum_1m
    momentum_5m
    momentum_15m
    momentum_1h
    momentum_4h
    momentum_24h

    volatility_5m
    volatility_1h
    volatility_4h
    volatility_24h
    volatility_7d

    volatility_ratio

    ema_fast
    ema_slow
    trend_score

    atr

    volume_zscore

    drawdown

    regime
}
```

Nenhuma feature pode utilizar dados posteriores ao timestamp do snapshot.

---

# 13. Probabilidade

O sistema pode produzir:

```text
P(up)
P(down)
P(neutral)
```

Com:

\[
P(up)+P(down)+P(neutral)=1
\]

As probabilidades devem ser calibradas utilizando dados históricos.

Não tratar a saída bruta de um modelo como probabilidade verdadeira sem validação.

---

# 14. Expected Return

Definir:

\[
ER =
\sum_i P_i \times R_i
\]

Para três estados:

\[
ER =
P(up)R_{up}
+
P(neutral)R_{neutral}
+
P(down)R_{down}
\]

---

# 15. Expected Value

Para uma operação:

\[
EV =
P(win)\times AvgWin
-
P(loss)\times AvgLoss
-
Costs
\]

Onde:

```text
Costs = fees + estimated_slippage + outros custos mensuráveis
```

Uma operação só pode ser considerada quantitativamente favorável quando:

```text
EV > 0
```

Mas `EV > 0` sozinho não autoriza execução.

---

# 16. Confidence

Confidence deve representar a confiabilidade estimada do sinal/modelo.

Ela não deve ser confundida com:

```text
P(up)
```

Exemplo:

```text
P(up) = 72%
Confidence = 81%
```

significa:

```text
o modelo estima 72% de probabilidade,
com determinada confiabilidade estatística.
```

---

# 17. Amostra mínima

Decisões baseadas em poucas observações devem ser rejeitadas ou classificadas como baixa confiança.

Exemplo inicial:

```text
N < 100
    insuficiente

100 <= N < 500
    baixa confiança

500 <= N < 2.000
    confiança moderada

N >= 2.000
    confiança alta
```

Esses valores devem ser validados empiricamente.

---

# 18. Historical Pattern Matching

Para um estado atual `X_t`, comparar com estados históricos `X_i`.

Distância normalizada:

\[
d(X_t,X_i)
=
\sqrt{
\sum_j w_j(X_{t,j}-X_{i,j})^2
}
\]

Selecionar os `K` estados mais próximos.

Inicialmente:

```text
K = 100
```

O valor deve ser configurável e validado.

---

# 19. Resultado dos padrões

Para cada conjunto de estados semelhantes, calcular:

```text
N
win_rate
loss_rate
average_return
median_return
profit_factor
expected_value
maximum_adverse_excursion
maximum_favorable_excursion
```

Separar por horizonte:

```text
5m
15m
30m
1h
4h
24h
```

---

# 20. Critério de entrada

Conceitualmente:

```text
ENTRY_ALLOWED =
    probability >= minimum_probability
    AND expected_value > minimum_ev
    AND sample_size >= minimum_sample
    AND regime_allowed
    AND confidence >= minimum_confidence
    AND risk_engine == ALLOW
```

Nenhum desses parâmetros deve ser considerado definitivo sem validação.

---

# 21. Stop Loss baseado em volatilidade

Uma abordagem inicial:

\[
StopDistance = k \times ATR
\]

Exemplo:

```text
ATR = 0,8%
k = 1,5

StopDistance = 1,2%
```

O `k` deve ser otimizado apenas dentro de processo de validação fora da amostra.

---

# 22. Position Sizing

Se:

```text
R = risco monetário permitido
E = preço de entrada
S = stop
M = multiplicador
```

Então:

\[
UnitRisk = |E-S|\times M
\]

\[
PositionSize =
floor(R/UnitRisk)
\]

Sempre arredondar para baixo.

---

# 23. Risco total

\[
TotalRisk =
PositionRisk + Fees + Slippage
\]

A posição somente poderá ser autorizada quando:

\[
TotalRisk \le AllowedTradeRisk
\]

---

# 24. Métricas obrigatórias

O modelo deve acompanhar:

```text
accuracy
precision
recall
win_rate
average_win
average_loss
expectancy
profit_factor
sharpe
sortino
max_drawdown
MAE
MFE
calibration_error
```

Accuracy nunca deve ser usada isoladamente.

---

# 25. Princípio de não aumento de risco

Um sinal mais forte pode aumentar a confiança estatística.

Ele não pode aumentar automaticamente:

```text
Risk Budget
Loss Limit
Recovery Budget
Session Loss Limit
```

O Risk Engine permanece soberano.

---

# 26. Precisão numérica

Não utilizar floating point para dinheiro.

Preferir:

```text
inteiros em unidades mínimas
```

ou decimal exato.

Percentuais podem ser armazenados em basis points:

```text
1%  = 100 bps
2%  = 200 bps
10% = 1000 bps
```

---

# 27. Determinismo

Dado o mesmo:

```text
Market Snapshot
Model Version
Configuration
```

o resultado deve ser reproduzível.

Randomização deve ser explicitamente controlada e versionada.

---

# 28. Relação com o Risk & Recovery Engine

Fluxo obrigatório:

```text
Market Data
    ↓
Features
    ↓
Regime
    ↓
Signal
    ↓
Probability
    ↓
Expected Value
    ↓
Trade Candidate
    ↓
Risk & Recovery Engine
    ↓
ALLOW / DENY
```

O modelo quantitativo nunca contorna o Risk Engine.

---

# 29. Princípio final

O objetivo não é maximizar:

```text
acerticidade
```

O objetivo é maximizar:

```text
Expected Risk-Adjusted Return
```

com controle explícito de:

```text
drawdown
tail risk
model uncertainty
execution costs
overfitting
```
