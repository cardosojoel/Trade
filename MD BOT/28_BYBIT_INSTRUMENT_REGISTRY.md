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

## Moeda da taxa e quantização

No mercado à vista a taxa da **compra** é cobrada na moeda base — quem compra
BTC recebe BTC já descontado da taxa. A quantidade recebida não é múltiplo de
`qty_step`, e a diferença não é vendável.

**REQ-BYBIT-004** O adapter MUST registrar, por execução, a quantidade
efetivamente recebida depois da taxa, e MUST NOT presumir que ela seja igual à
quantidade solicitada.

**REQ-BYBIT-005** O saldo residual da moeda base — o que sobra abaixo de
`qty_step` ou abaixo de `min_order_amt` — MUST ser tratado como saldo, não como
perda: ele MUST ser somado à próxima ordem do mesmo símbolo. O sistema MUST NOT
montar cada ordem a partir do zero ignorando o resíduo.

Medido em 2026-09-20 sobre BTCUSDT: `qty_step` de 0,000001 BTC vale cerca de
US$ 0,08, e o resíduo médio por ida e volta é de US$ 0,04. Sobre uma posição de
US$ 5,88 isso é 0,69% — quase três vezes o custo de taxa e slippage somados.
Ignorar estes dois requisitos multiplica o custo real por até 3,8.

## Atualização
**REQ-BYBIT-002** O adapter MUST obter, atualizar e versionar as especificações
do instrumento; alteração MUST gerar evento de configuração.

**REQ-BYBIT-006** A especificação MUST ser relida no início de toda sessão e
MUST NOT ser embutida em código nem herdada de sessão anterior. A Bybit revisa
estes valores **nos dias 3 e 17 de cada mês, às 08h00 UTC+8**, e a própria
documentação adverte que não se deve assumir que permaneçam constantes.

**REQ-BYBIT-007** Se a releitura invalidar o perfil derivado — por exemplo, se
`min_order_amt` subir acima do que o depósito comporta — a sessão MUST NOT
iniciar. Reduzir o perfil em silêncio para caber nos novos limites é violação:
o perfil é derivado do depósito e dos limites, e limites novos exigem derivação
nova, confirmada.

**REQ-BYBIT-008** `min_order_qty` está **deprecado** na API e MUST NOT ser usado
como critério de validação. A validação de tamanho mínimo MUST usar
`min_order_amt`.

## Segurança
**REQ-BYBIT-003** Mudança de instrumento MUST NOT aumentar automaticamente o
risco permitido pela política da sessão.
