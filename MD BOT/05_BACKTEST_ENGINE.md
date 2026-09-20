# 05 — Backtest Engine

## Objetivo
Permitir validar estratégia, modelo e Risk Engine usando dados históricos sem introduzir vieses.

## Princípio
O backtest deve reutilizar a mesma lógica de:
```text
features
signal
probability
EV
risk
position sizing
```

usada em produção, sempre que possível.

## Pipeline
```text
Historical Data
 ↓
Market Data Adapter
 ↓
Feature Engine
 ↓
Pattern Matching
 ↓
Probability
 ↓
Expected Value
 ↓
Risk Engine
 ↓
Execution Simulator
 ↓
Portfolio
 ↓
Metrics
```

## Regras
- sem look-ahead;
- sem future leakage;
- custos realistas;
- timestamps corretos;
- dados somente disponíveis naquele instante;
- decisões reproduzíveis.

## Walk-forward
```text
TRAIN → VALIDATE → TEST
       ↓
     MOVE
       ↓
TRAIN → VALIDATE → TEST
```

## Métricas
```text
Net PnL
Profit Factor
Expectancy
Max Drawdown
Sharpe
Sortino
Calmar
Win Rate
Tail Loss
CVaR
```

## Conclusão
**Estado: forte, mas não estado da arte ainda.**

O maior risco é o simulador divergir da execução real. Para elevar o nível, incorporar dados de execução real, latência, slippage dependente de liquidez, partial fills, funding e replay determinístico de eventos.
