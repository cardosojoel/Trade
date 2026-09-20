# 31 — Exemplo numérico fim a fim

**Status:** normativo · **Versão:** 1.0 · **Atualizado em:** 2026-09-20  
**Domínio de requisitos:** `REQ-SIZING-*` (a regra que este exemplo motivou vive
em [`MATHEMATICAL_QUANT_MODEL.md`](MATHEMATICAL_QUANT_MODEL.md) §22)  
**Conformidade:** conforme  
**Vocabulário:** [`00_GLOSSARIO.md`](00_GLOSSARIO.md)

---

## Por que este documento existe

Cada frente foi verificada isoladamente e nenhuma contradizia a outra dentro do
seu próprio escopo. Nenhum documento, porém, percorria a cadeia inteira com os
mesmos números — e é só atravessando que aparecem as incompatibilidades entre
parâmetros que, separados, parecem razoáveis.

Este documento percorre uma decisão, do dado de mercado à ordem, com aritmética
explícita. Os **números são ilustrativos**; os **invariantes que eles expõem são
normativos**. Valores operacionais continuam sendo autoridade do
[`27_CONFIGURATION_REGISTRY.md`](27_CONFIGURATION_REGISTRY.md).

## Premissas

| Grandeza | Valor | Origem |
|---|---|---|
| `D` | 10.000 USDT | parâmetro de sessão |
| `L` | 20% | `risk.max_loss_pct` |
| `T` | 2% | `risk.max_risk_per_trade_pct` |
| Teto de posição | 10% de `D` = 1.000 USDT | limite de risco do projeto |
| Preço BTCUSDT | 60.000 USDT | referência |
| ATR(24h) | 0,8% = 480 USDT | features, §9 do quant model |
| `k` do stop | 1,5 | parâmetro de estratégia |
| Taxa | 0,1% por perna, spot | premissa de custo |
| Slippage estimado | 0,02% por perna | premissa de custo |
| Funding | 0 | **spot** — não há funding |

Derivados: `MaxLossDeposit` = 2.000 · `CapitalFloor` = 8.000 ·
`NormalTradeRisk` = `D × T` = **200 USDT**.

## 1. Features e regime

`TrendScore` acima do limiar com `VolatilityRatio` dentro do intervalo →
`MarketRegime::TrendUp`, confiança suficiente. Horizonte avaliado: **1h**.

## 2. Pattern matching

`K = 100` vizinhos, `ESS = 82` — a amostra ponderada ainda é ampla, e a
qualidade de similaridade passa o mínimo.

## 3. Probabilidade

```text
P_up      = 0,684
P_neutral = 0,095
P_down    = 0,221
```

Soma = 1,000. ✔ `REQ-EV-001`

## 4. Retorno por estado e EV

Retornos observados nos vizinhos, para 1h:

```text
R_up      = +0,90%
R_neutral = +0,02%
R_down    = −0,80%
```

```text
ER = 0,684×0,90% + 0,095×0,02% + 0,221×(−0,80%)
   = 0,6156% + 0,0019% − 0,1768%
   = +0,4407%
```

O EV em dinheiro depende do tamanho — que só se conhece no passo 6. Voltamos
aqui com a posição efetiva.

## 5. Stop e risco unitário

```text
StopDistance = k × ATR = 1,5 × 0,8% = 1,20%  →  720 USDT por BTC
UnitRisk     = |E − S| × M = 720 USDT por BTC
```

## 6. Sizing — e o primeiro problema

```text
PositionSize = floor(AllowedTradeRisk / UnitRisk)
             = floor(200 / 720) BTC
             = 0,277777 BTC   (qty_step = 0,000001)

PositionNotional = 0,277777 × 60.000 = 16.666,62 USDT
```

**16.666 USDT de posição sobre um depósito de 10.000.** Em spot, sem
alavancagem, essa ordem não existe.

Não é erro de conta. É identidade:

```text
PositionNotional = AllowedTradeRisk / StopDistance%
                 = (D × T) / s
```

Com `T = 2%` e `s = 1,2%`, a razão é `T/s = 1,67`: o sizing pede 1,67 vez o
depósito. **Em spot, o risco por operação só é alcançável quando `s ≥ T`.** Com
stop mais curto que o risco autorizado, `T` é inatingível — e a fórmula de
sizing, sozinha, produz uma ordem impossível em silêncio.

A mesma conta reprova o exemplo que já existia no conjunto: depósito de R$ 50,
`T = 2%` → risco de R$ 1; stop de 1,2% → posição de **R$ 83,33** sobre R$ 50.

Daí a regra, registrada no quant model §22:

> **REQ-SIZING-003** Em mercado à vista, `PositionNotional` MUST NOT exceder o
> caixa disponível. Quando a fórmula de sizing produzir valor maior, a posição
> MUST ser reduzida ao caixa e o limite vinculante MUST ser registrado — o
> sistema MUST NOT emitir ordem impossível nem silenciar a redução.

## 7. Risco — e o segundo problema

O teto de posição do projeto é 10% de `D`:

```text
AllowedNotional = min(16.666,62 ; 10.000 ; 1.000) = 1.000 USDT
PositionSize    = 1.000 / 60.000 = 0,016666 BTC
RiscoEfetivo    = 1.000 × 1,20%  = 12,00 USDT
```

O risco autorizado era 200 USDT. O risco que a cerca permite é **12 USDT** —
`0,12%` de `D`, não `2%`. O teto de posição é mais apertado que o limite de
risco por um fator de **16,7**.

Isto reproduz, na aritmética, o que a medição de doze meses do repositório já
mostrara por outro caminho: os limites de risco não são a cerca que morde.
Quem governa o tamanho é o teto de posição; `T` nunca chega a ser consultado. É
o mesmo achado do `max_total_exposure` decorativo, um nível acima.

## 8. EV com a posição efetiva

```text
G = 0,90% × 1.000 = 9,00 USDT
L = 0,80% × 1.000 = 8,00 USDT
N = 0,02% × 1.000 = 0,20 USDT

C_total = taxas 0,20% + slippage 0,04% + spread 0,01% sobre 1.000
        = 2,00 + 0,40 + 0,10 = 2,50 USDT     (funding = 0, spot)

EV_gross = 0,684×9,00 + 0,095×0,20 + 0,221×(−8,00)
         = 6,156 + 0,019 − 1,768 = +4,407 USDT
EV_net   = 4,407 − 2,50 = +1,91 USDT
```

Break-even:

```text
P_BE = (L + C_total) / (G + L) = (8,00 + 2,50) / (9,00 + 8,00) = 61,8%
P_up = 68,4% > 61,8%   → há vantagem nominal
```

## 9. EV conservador — e o terceiro problema

Degradando conforme `REQ-EV-002`: `P_up` −10% relativo (0,684 → 0,616, a
diferença migra para `P_down`), `R_up` −20% (0,90% → 0,72%), slippage ×2.

```text
G' = 0,72% × 1.000 = 7,20    C'_total = 2,90
EV_gross' = 0,616×7,20 + 0,095×0,20 + 0,289×(−8,00)
          = 4,435 + 0,019 − 2,312 = +2,142
EV_net'   = 2,142 − 2,90 = −0,76 USDT
```

```text
P_BE' = (8,00 + 2,90) / (7,20 + 8,00) = 71,7%
P_up' = 61,6% < 71,7%
```

**Decisão: `NO_TRADE`.** O EV nominal é positivo, o conservador é negativo, e é o
conservador que decide. Registra-se a decisão com os dois valores, os
thresholds e as versões — a oportunidade recusada é dado, não silêncio
(`REQ-LEARN-002`).

## 10. O que este exemplo prova

| # | Achado | Onde virou regra |
|---|---|---|
| 1 | Sizing pode exigir posição maior que o caixa; em spot, `T` exige `s ≥ T` | `REQ-SIZING-003` |
| 2 | O teto de posição torna `T` inalcançável por 16,7× — o limite de risco não morde | calibração, `27_CONFIGURATION_REGISTRY.md` |
| 3 | Com posição de 1.000, o custo consome 57% do EV bruto, e o cenário conservador inverte o sinal | `REQ-EV-002` |

Nenhum dos três aparece lendo os documentos separadamente. Os três aparecem na
primeira vez que alguém atravessa a cadeia com os mesmos números.

## 11. O que falta medir

Os custos aqui são premissa, não medição: taxa, slippage e spread precisam vir
do Registry, calibrados contra execuções reais. O `06_EXECUTION_SIMULATOR.md`
continua sendo o elo mais fraco entre este exemplo e a realidade.
