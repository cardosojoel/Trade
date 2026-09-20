# 06 — Execution Simulator

**Status:** normativo · **Versão:** 1.1 · **Atualizado em:** 2026-09-20  
**Domínio de requisitos:** `REQ-SIM-*`  
**Vocabulário:** [`00_GLOSSARIO.md`](00_GLOSSARIO.md)

---

## Objetivo
Simular de forma realista o comportamento da execução antes de operar capital real.

## Componentes
**REQ-SIM-001** O simulador MUST modelar:

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

**REQ-SIM-002** O slippage MUST depender do tamanho da ordem e da liquidez
disponível — MUST NOT ser constante.

## Latência
**REQ-SIM-003** O simulador MUST simular cada etapa separadamente:
```text
market → decision
decision → submit
submit → exchange
exchange → fill
```

## Ordens
**REQ-SIM-004** O simulador MUST suportar:
```text
market
limit
reduce-only
stop
take-profit
```

## Cenários adversos
**REQ-SIM-005** O simulador MUST ser exercitado sob:

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
É uma das áreas mais importantes para transformar backtest em evidência confiável. Um simulador simples de candle não representa adequadamente execução de derivativos em alta volatilidade. O nível de excelência exige calibração com fills reais e replay de microestrutura.
