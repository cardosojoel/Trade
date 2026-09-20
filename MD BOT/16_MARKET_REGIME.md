# 16 — Market Regime

Regimes baseline:
`TREND_UP`, `TREND_DOWN`, `RANGE`, `HIGH_VOLATILITY`, `LOW_VOLATILITY`, `BREAKOUT`, `CRASH`, `UNKNOWN`.

Regime é hipótese estatística, não verdade absoluta.

Usar histerese para evitar alternância excessiva. Se confiança for insuficiente, usar `UNKNOWN` e aplicar a política correspondente, podendo bloquear entradas.

Somente dados disponíveis no instante de classificação podem ser usados.

Registrar regime, confiança, timestamp e versão.
