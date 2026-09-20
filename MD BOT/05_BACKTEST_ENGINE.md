# 05 — Backtest Engine

**Status:** normativo · **Versão:** 1.1 · **Atualizado em:** 2026-09-20  
**Domínio de requisitos:** `REQ-BACKTEST-*`  
**Vocabulário:** [`00_GLOSSARIO.md`](00_GLOSSARIO.md)

---

## Objetivo
Permitir validar estratégia, modelo e Risk Engine usando dados históricos sem introduzir vieses.

## Princípio
**REQ-BACKTEST-001** O backtest MUST reutilizar a mesma lógica de produção:
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
**REQ-BACKTEST-002** O motor MUST garantir:

- sem look-ahead;
- sem future leakage;
- custos realistas;
- timestamps corretos;
- dados somente disponíveis naquele instante;
- decisões reproduzíveis.

## Walk-forward

**REQ-BACKTEST-003** A avaliação MUST ser walk-forward; treino e teste MUST NOT
compartilhar período.
```text
TRAIN → VALIDATE → TEST
       ↓
     MOVE
       ↓
TRAIN → VALIDATE → TEST
```

## Métricas
**REQ-BACKTEST-004** O motor MUST produzir:

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
O maior risco é o simulador divergir da execução real. Para elevar o nível, incorporar dados de execução real, latência, slippage dependente de liquidez, partial fills, funding e replay determinístico de eventos.
