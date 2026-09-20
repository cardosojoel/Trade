# 06 — Execution Simulator

## Objetivo
Simular de forma realista o comportamento da execução antes de operar capital real.

## Componentes
```text
Spread
Fees
Slippage
Latency
Order Type
Liquidity
Partial Fill
Funding
Stop Execution
Take Profit
Order Rejection
```

## Modelo
Para cada ordem:
\[
NetPnL = GrossPnL - Fees - Slippage - Funding
\]

Slippage deve depender do tamanho da ordem e da liquidez disponível.

## Latência
Simular:
```text
market → decision
decision → submit
submit → exchange
exchange → fill
```

## Ordens
Suportar:
```text
market
limit
reduce-only
stop
take-profit
```

## Cenários adversos
```text
high volatility
thin liquidity
partial fill
API delay
WebSocket delay
order rejection
connection loss
```

## Conclusão
**Estado: ainda precisa de evolução significativa.**

É uma das áreas mais importantes para transformar backtest em evidência confiável. Um simulador simples de candle não representa adequadamente execução de derivativos em alta volatilidade. O nível de excelência exige calibração com fills reais e replay de microestrutura.
