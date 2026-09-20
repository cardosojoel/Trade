# 13 — Probability Model

Estimar `P(H | Pattern, Regime, Features)` para horizonte explícito.

Baseline:
`P = weighted_successes / weighted_observations`

Aplicar smoothing em amostras pequenas.

Registrar amostra efetiva, K, horizonte, regime, intervalo de confiança e versão.

Validar com Brier Score, log loss, reliability curve e calibration error.

Probability não substitui EV. Alta probabilidade pode resultar em `NO_TRADE`.

Monitorar drift de calibração por regime e tempo.
