# 11 — Feature Engineering

Cada feature deve possuir `name`, `formula`, `window`, `source`, `unit`, `normalization`, `timestamp` e `version`.

## Anti-leakage
`Feature(t) = f(X[−∞, t])`. Nenhum dado posterior a `t` pode participar da feature.

## Classes
Preço/retorno, volume/fluxo, volatilidade, momentum, order book, trades, funding/open interest quando disponíveis, tempo e regime.

## Performance
Features do hot path devem ser calculadas incrementalmente em RAM, evitando I/O, scans históricos e alocações desnecessárias.

## Testes
Fórmula, bordas, NaN/overflow, temporalidade e regressão.
