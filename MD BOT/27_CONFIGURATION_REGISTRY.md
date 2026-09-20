# 27 — Configuration Registry

**Status:** normativo · **Versão:** 2.0 · **Atualizado em:** 2026-09-20  
**Domínio de requisitos:** `REQ-CFG-*`  
**Vocabulário:** [`00_GLOSSARIO.md`](00_GLOSSARIO.md)

---

## Objetivo
Ser a única fonte operacional dos valores configuráveis do sistema.

## Contrato
**REQ-CFG-001** Cada parâmetro MUST possuir:
```text
parameter_id
value
type
unit
scope
min
max
default
version
effective_from
status
source
```

## Escopos
`GLOBAL`, `SESSION`, `SYMBOL`, `MODEL`, `ENVIRONMENT`.

## Parâmetros mínimos
```text
risk.max_loss_pct
risk.recovery_profit_pct
risk.recovery_max_pct
risk.max_risk_per_trade_pct
risk.max_recovery_risk_pct
risk.max_recovery_episodes
risk.max_recovery_attempts
strategy.min_probability
strategy.min_ev
strategy.min_ev_conservative
pattern.k
pattern.min_samples
pattern.min_similarity
performance.data_max_age_us
performance.decision_p99_us
performance.decision_p999_us
performance.emergency_p99_us

risk.position_cap_pct
risk.max_total_exposure_pct
risk.daily_loss_limit_pct
risk.max_drawdown_stop_pct
strategy.stop_pct
strategy.target_pct
strategy.max_trades_per_day
strategy.max_holding_minutes
```

`risk.max_loss_pct` e `risk.daily_loss_limit_pct` MUST NOT ser confundidos:
o primeiro é `L`, a perda máxima do **depósito da sessão**; o segundo é a perda
máxima do **período diário**, com retomada automática na virada. São conceitos
de documentos diferentes e escopos diferentes — `SESSION` e `GLOBAL`.

Os valores concretos devem ser preenchidos por ambiente e validação; este documento define nomes, tipos e invariantes, não números arbitrários.

## Validação
**REQ-CFG-002** O startup MUST rejeitar configuração fora dos limites ou
inconsistente.

## Precedência
**REQ-CFG-003** O runtime MUST ler o Registry versionado; o código MUST conter
apenas invariantes não configuráveis.

## Origem do valor

**REQ-CFG-004** Todo parâmetro MUST declarar a origem do seu valor em `source`,
com um destes rótulos:

| `source` | Significado |
|---|---|
| `MEASURED` | derivado de dado histórico, com a medição identificável |
| `EXCHANGE` | lido da corretora, com data da leitura |
| `DERIVED` | calculado de outros parâmetros por identidade declarada |
| `CONSTITUTION` | fixado pela constitution do projeto |
| `ASSUMED` | premissa não verificada — MUST NOT governar capital real sem revisão |

Um parâmetro `ASSUMED` MAY existir em `backtest` e em `paper`. Promover para
`live` com parâmetro `ASSUMED` é violação da revisão de limiares que a
constitution exige antes da Porta 3.

---

# Perfis medidos — 2026-09-20

Dois perfis completos, derivados de medição sobre doze meses reais de BTCUSDT.
São os primeiros valores deste projeto com origem rastreável.

## Base da medição

| | |
|---|---|
| Dados | `data/market.db` — 525.600 velas de 1 min, 2025-09-20 a 2026-09-20 |
| Instrumento | Bybit spot BTCUSDT, lido da API em 2026-09-20: `minOrderAmt` 5 USDT, `minOrderQty` 0,000001 BTC, `basePrecision` 0,000001, `tickSize` 0,1 — valores pertencem ao [`28_BYBIT_INSTRUMENT_REGISTRY.md`](28_BYBIT_INSTRUMENT_REGISTRY.md) |
| Custo por operação | 0,25% do valor negociado — 0,20% de taxa (0,1% por perna, VIP0) + 0,05% de slippage e spread |
| Preço de referência | 81.233,70 USDT |

### Medições que sustentam os perfis

| Medição | Resultado |
|---|---|
| ATR de 1 min | 0,061% do preço (mediana de 364 dias) |
| Excursão adversa em 4h | mediana 0,46% · p75 0,90% · p90 1,52% |
| Movimento mediano por horizonte | 5 min 0,056% · 1 h 0,193% · 4 h 0,407% · 24 h 1,291% |
| Custo ÷ movimento mediano | 5 min **4,5×** · 1 h **1,3×** · 4 h 0,61× · 24 h 0,19× |
| Bracket 1:1 de 1,0% em 24h | 43,6% ganho · 46,2% perda · 10,1% aberto |
| Bracket 1:1 de 2,0% em 24h | 27,3% ganho · 31,7% perda · 41,0% aberto |

**Consequência normativa da terceira linha:** abaixo de 1 hora o custo excede o
movimento típico do ativo. Nenhum perfil MAY operar com horizonte inferior a
4 horas enquanto o custo por operação for de 0,25%.

## Identidades que restringem os perfis

```text
PositionNotional     = AllowedTradeRisk / stop_pct          (REQ-SIZING-003)
risk_per_trade_pct   = position_cap_pct × stop_pct
position_cap_pct    >= instrument.min_order_amt / D
chão de operação     = instrument.min_order_amt / position_cap_pct
```

Da segunda identidade, com `position_cap_pct <= 100%` em mercado à vista:

> **Em spot, o risco por operação nunca MAY exceder a distância do stop.**
> `risk.max_risk_per_trade_pct <= strategy.stop_pct`, sempre.

## Perfil `banca-12` — D = 12 USDT

| `parameter_id` | `value` | `unit` | `source` | Justificativa |
|---|---:|---|---|---|
| `risk.position_cap_pct` | 100 | % | `DERIVED` | `5/12` = 41,7% é o piso; 100% afasta o chão de operação para −58% |
| `risk.max_total_exposure_pct` | 100 | % | `DERIVED` | ativo único, posição única |
| `strategy.stop_pct` | 1,0 | % | `MEASURED` | tocado por ruído em 21% das janelas de 4h; a 0,5% seriam 47% |
| `risk.max_risk_per_trade_pct` | 1,0 | % | `DERIVED` | `teto × stop`; com teto em 100% o risco fica amarrado ao stop |
| `strategy.target_pct` | 1,0 | % | `MEASURED` | 1:1; brackets 2:1 resolvem em 22,2% em 24h contra 43,6% do 1:1 |
| `strategy.max_holding_minutes` | 1440 | min | `MEASURED` | 24h; abaixo de 4h o custo excede o movimento |
| `strategy.max_trades_per_day` | 2 | ops | `DERIVED` | 0,25% de custo por operação sobre 100% do capital → 14% ao mês |
| `risk.daily_loss_limit_pct` | 2,0 | % | `DERIVED` | dispara na 2ª perda; a perda máxima aritmética do dia é 2,5% |
| `risk.max_drawdown_stop_pct` | 15 | % | `CONSTITUTION` | métrica de porta; equivale a 15 perdas cheias |
| — chão de operação | 5,00 | USDT | `EXCHANGE` | abaixo disso nenhuma ordem é possível: −58% |
| — acerto necessário | 63,9 | % | `MEASURED` | dos resolvidos; entrada aleatória entrega 48,6% |

**Restrições desta banca:** não há dimensionamento de posição — toda ordem é a
conta inteira, com faixa de apenas 2 ordens mínimas. Saída parcial é impossível
(metade de uma posição de 10 USDT é exatamente o mínimo). O custo pesa 25% do
valor arriscado, porque o stop não pode ser alargado sem elevar o risco.

## Perfil `banca-100` — D = 100 USDT

| `parameter_id` | `value` | `unit` | `source` | Justificativa |
|---|---:|---|---|---|
| `risk.position_cap_pct` | 50 | % | `DERIVED` | `risco ÷ stop` = 1,0/2,0; o piso seria 5% |
| `risk.max_total_exposure_pct` | 50 | % | `DERIVED` | ativo único, posição única |
| `strategy.stop_pct` | 2,0 | % | `MEASURED` | tocado por ruído em 6% das janelas de 4h |
| `risk.max_risk_per_trade_pct` | 1,0 | % | `MEASURED` | escolhido; possível porque o teto se solta do stop |
| `strategy.target_pct` | 2,0 | % | `MEASURED` | 1:1 |
| `strategy.max_holding_minutes` | 1440 | min | `MEASURED` | 24h |
| `strategy.max_trades_per_day` | 2 | ops | `DERIVED` | 0,125% de custo por operação sobre o capital → 7% ao mês |
| `risk.daily_loss_limit_pct` | 2,0 | % | `DERIVED` | dispara na 2ª perda |
| `risk.max_drawdown_stop_pct` | 15 | % | `CONSTITUTION` | equivale a 15 perdas cheias |
| — chão de operação | 10,00 | USDT | `DERIVED` | `5 / 50%`: −90% |
| — acerto necessário | 60,6 | % | `MEASURED` | dos resolvidos; entrada aleatória entrega 46,3% |

**O que esta banca compra:** o teto de posição se solta do stop, e é isso que
importa. O stop dobra de 1,0% para 2,0% mantendo o risco em 1,0%, e o custo cai
de 25% para **12,5% do valor arriscado**. A faixa de dimensionamento vai de 2
para 10 ordens mínimas, e a saída parcial passa a ser possível.

## Comparação

| | `banca-12` | `banca-100` |
|---|---:|---:|
| Teto de posição | 100% (forçado) | 50% (escolhido) |
| Stop com risco de 1% | 1,0% | 2,0% |
| Custo ÷ valor arriscado | 25,0% | **12,5%** |
| Acerto necessário | 63,9% | **60,6%** |
| Faixa de dimensionamento | 2 ordens | 10 ordens |
| Saída parcial | não | sim |
| Custo mensal a 2 ops/dia | 14% | 7% |
| Chão de operação | −58% | −90% |

## Valores ainda `ASSUMED` nestes perfis

| Premissa | Valor usado | Efeito se estiver errada |
|---|---|---|
| Taxa por perna | 0,1% (VIP0 público) | é ~80% do custo total; a 0,06% o acerto necessário de `banca-100` cai para ~57% |
| Slippage + spread | 0,05% por round trip | não medido contra execução real; depende do `06_EXECUTION_SIMULATOR.md` |

Ambas MUST ser confirmadas antes de qualquer uso com capital real
(`REQ-CFG-004`).

## O que estes perfis não resolvem

Os percentuais dimensionam e protegem. Não criam vantagem. A entrada aleatória
medida sobre os mesmos doze meses perde entre 0,26% e 0,38% por operação em
todas as combinações testadas — exatamente o custo, mais uma pequena assimetria
desfavorável. **A estratégia precisa adicionar cerca de 14 pontos percentuais de
acerto sobre o acaso** para empatar no perfil `banca-100`. Esse número, e não
`profit factor >= 1.3`, é o alvo verificável de desenvolvimento.

Medição registrada: estender o prazo máximo de posição de 24h para 72h reduz a
barra de 60,6% para 56,7%, porque a fração de posições que expiram sem resolver
cai de 41% para 6%. Isso conflita com o domínio declarado na constitution — day
trade — e **não foi decidido**.
