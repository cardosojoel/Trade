# 15 — Position Sizing

Sizing ocorre depois de existir edge e antes da autorização final de risco.

Métodos suportados: fixed fractional, risk-based, volatility-adjusted e fractional Kelly opcional.

Para risco monetário R e distância de risco D:
`PositionSize = R / D`

O cálculo final deve incorporar unidade do contrato, tick/lot size, fees e slippage.

Aplicar caps de posição, notional, leverage, exposição por símbolo, exposição total e orçamento de risco.

Arredondamento deve respeitar as regras do instrumento da Bybit.

Registrar sizing bruto, limites aplicados e sizing final.
