# 28 — Bybit Instrument Registry

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
- nunca enviar quantidade incompatível com `qty_step`;
- nunca enviar preço incompatível com `tick_size`;
- respeitar mínimos/máximos;
- respeitar limites de leverage/notional;
- rejeitar especificação stale quando a política exigir atualização.

## Atualização
O adapter deve obter/atualizar especificações do instrumento e versioná-las. Alterações devem gerar evento de configuração.

## Segurança
Uma mudança de instrumento não pode aumentar automaticamente o risco permitido pela política de sessão.
