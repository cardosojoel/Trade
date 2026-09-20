# 28 — Bybit Instrument Registry

**Status:** normativo · **Versão:** 1.1 · **Atualizado em:** 2026-09-20  
**Domínio de requisitos:** `REQ-BYBIT-*`  
**Vocabulário:** [`00_GLOSSARIO.md`](00_GLOSSARIO.md)

---

## Objetivo
Centralizar as regras do instrumento usadas por sizing, order validation e execution.

## InstrumentSpec
```text
symbol
category
status
base_coin
quote_coin
tick_size
qty_step
min_order_qty
max_order_qty
min_notional
max_market_order_qty
price_precision
qty_precision
leverage_min
leverage_max
contract_multiplier
updated_at
source_version
```

## Uso
Position Sizing calcula quantidade teórica → Instrument Registry arredonda/valida → Risk Engine valida limites → Execution Engine monta a ordem.

## Regras
**REQ-BYBIT-001** A validação de ordem MUST garantir:

- nunca enviar quantidade incompatível com `qty_step`;
- nunca enviar preço incompatível com `tick_size`;
- respeitar mínimos/máximos;
- respeitar limites de leverage/notional;
- rejeitar especificação stale quando a política exigir atualização.

## Atualização
**REQ-BYBIT-002** O adapter MUST obter, atualizar e versionar as especificações
do instrumento; alteração MUST gerar evento de configuração.

## Segurança
**REQ-BYBIT-003** Mudança de instrumento MUST NOT aumentar automaticamente o
risco permitido pela política da sessão.
