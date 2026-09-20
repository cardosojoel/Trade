# 27 — Configuration Registry

**Status:** normativo · **Versão:** 3.1 · **Atualizado em:** 2026-09-20  
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

## Resultado esperado sem vantagem

**REQ-CFG-005** Todo perfil MUST declarar o **resultado operacional esperado sem
vantagem** — o saldo final esperado depois de todos os custos, supondo que a
estratégia escolha a hora de entrar tão bem quanto o acaso. Ele MUST ser
recalculado quando o custo por operação, a distribuição de resultados ou
qualquer parâmetro do perfil mudar.

Não é previsão do sistema: é a linha que a estratégia precisa cruzar para que
operar seja melhor que não operar. Um perfil sem esse número não pode ser
avaliado — só executado às cegas.

**REQ-CFG-006** O método que produz esse número MUST ser reproduzível e MUST
declarar amostra, número de execuções, semente, e todas as regras aplicadas na
simulação.

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

## Função de derivação

**A pessoa informa apenas o depósito.** Os parâmetros saem destas regras — não
são escolhidos, e não podem ser afrouxados por quem opera.

```text
risco_por_operacao = drawdown_de_parada / 15        = 1,00%
teto_minimo        = min_order_amt / (D x 0,85)
stop               = min(2,00% ; risco_por_operacao / teto_minimo)
teto_de_posicao    = risco_por_operacao / stop
operacoes_por_dia  = piso(1 / teto_de_posicao)
limite_diario      = operacoes_por_dia x risco_por_operacao
chao_de_operacao   = min_order_amt / teto_de_posicao
prazo_maximo       = 1440 min
```

Origem de cada regra:

| Regra | Origem |
|---|---|
| risco = drawdown ÷ 15 | `DERIVED` — a parada por drawdown tolera 15 perdas cheias |
| teto mínimo | `DERIVED` — a posição MUST continuar emitível depois do drawdown máximo |
| stop de 2,00% | `MEASURED` — minimiza o acerto necessário em prazo de 24h |
| operações por dia | `DERIVED` — mantém o custo mensal em ~7% do capital |
| prazo de 24h | `MEASURED` — abaixo de 4h o custo excede o movimento mediano |

**Viabilidade:** `D >= min_order_amt / 0,85` = **5,88 USDT**. Abaixo disso nem a
primeira posição sobrevive ao drawdown, e a sessão MUST NOT iniciar.

## Família derivada

| Depósito | Teto | Stop | Risco | Posição | Risco $ | Ops/dia | Lim. diário | Chão | Faixa |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| $10 | 58,8% | 1,70% | 1,00% | $5,88 | $0,10 | 1 | 1,0% | $8,50 | 1 ordem |
| $100 | 50,0% | 2,00% | 1,00% | $50,00 | $1,00 | 2 | 2,0% | $10,00 | 10 ordens |
| $200 | 50,0% | 2,00% | 1,00% | $100,00 | $2,00 | 2 | 2,0% | $10,00 | 20 ordens |
| $500 | 50,0% | 2,00% | 1,00% | $250,00 | $5,00 | 2 | 2,0% | $10,00 | 50 ordens |
| $900 | 50,0% | 2,00% | 1,00% | $450,00 | $9,00 | 2 | 2,0% | $10,00 | 90 ordens |

Acima de **11,76 USDT** o stop atinge o ótimo medido e os percentuais
**congelam**: $200, $500 e $900 recebem exatamente a mesma configuração. Custo,
risco e posição são todos proporcionais ao capital; só a ordem mínima é
absoluta, e ela deixa de morder cedo. O que o capital compra é granularidade
(faixa de tamanhos), saída parcial — possível a partir de 20 USDT — e folga até
o chão.

## Perfil de referência — D = 100 USDT

| `parameter_id` | `value` | `unit` | `source` | Justificativa |
|---|---:|---|---|---|
| `risk.position_cap_pct` | 50 | % | `DERIVED` | `risco ÷ stop` = 1,0/2,0; o piso seria 5% |
| `risk.max_drawdown_stop_pct` | 15 | % | `CONSTITUTION` | tolera 15 perdas cheias — é o que fixa o risco em 1,00% |
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

## Resultado operacional esperado — `REQ-CFG-005`

Saldo esperado ao fim de 30 dias **sem vantagem nenhuma**: a estratégia escolhe
a hora de entrar tão bem quanto o acaso, e só o custo e a assimetria do ativo
agem.

| Depósito | Saldo médio | Resultado | Mediana | Faixa p5–p95 | Parou por drawdown |
|---:|---:|---:|---:|---:|---:|
| $10 | $9,53 | **−$0,47 (−4,7%)** | $9,52 | $8,81 – $10,28 | 1,1% |
| $100 | $91,50 | **−$8,50 (−8,5%)** | $91,08 | $84,26 – $101,02 | 16,0% |
| $200 | $183,00 | **−$17,00 (−8,5%)** | $182,20 | $168,44 – $201,98 | 15,8% |
| $500 | $457,47 | **−$42,53 (−8,5%)** | $455,38 | $421,17 – $505,25 | 15,6% |
| $900 | $823,25 | **−$76,75 (−8,5%)** | $819,48 | $757,88 – $908,79 | 16,2% |

O perfil de $10 perde menos em percentual porque executa 1 operação por dia
contra 2 — metade do giro, metade do custo. Paga isso em tempo: as 100
operações que a Porta 1 exige levam 100 dias em vez de 50.

### Acerto necessário e sensibilidade

O empate exige **59,4%** de acerto entre as operações que resolvem. A entrada
aleatória entrega 46,5%. A lacuna que a estratégia precisa produzir é de
**12,9 pontos percentuais**.

| Acerto entre resolvidos | $10 em 30d | $100 em 30d | Parou por drawdown |
|---:|---:|---:|---:|
| 46,5% — medido, acaso | −4,7% | −8,5% | 16,1% |
| 50,0% | −3,8% | −6,3% | 8,4% |
| 55,0% | −1,9% | −3,0% | 2,6% |
| **59,4% — empate** | −0,0% | −0,0% | 0,8% |
| 62,0% | +1,0% | +1,8% | 0,3% |
| 65,0% | +2,3% | +4,0% | 0,1% |

Entre 55% e 62% de acerto — sete pontos — o resultado mensal de $100 vai de
−3,0% a +1,8%. Cada ponto percentual de acerto vale cerca de 0,7% ao mês.
**Um ponto de acerto vale mais que qualquer recalibração de percentual de
risco.**

## Condição de validade dos números acima

A tabela anterior supõe custo de **0,25%** por operação. Esse número só vale se
`REQ-SIZING-004` e `REQ-BYBIT-005` estiverem implementados — isto é, se o robô
somar o resíduo da moeda base à ordem seguinte em vez de montar cada ordem do
zero.

Sem eles, a taxa cobrada em BTC e o arredondamento por `qty_step` deixam presos
cerca de **US$ 0,04 por ida e volta**, e o custo real passa a depender do
tamanho da posição:

| Depósito | Posição | Resíduo, % da posição | Custo real | Resultado em 30 dias | Contas paradas |
|---:|---:|---:|---:|---:|---:|
| $10 | $5,88 | 0,691% | **0,94%** | **−14,4%** | **67%** |
| $100 | $50,00 | 0,081% | 0,33% | −10,5% | 28% |
| $200 | $100,00 | 0,041% | 0,29% | −9,5% | 22% |
| $500 | $250,00 | 0,016% | 0,27% | −8,9% | 18% |
| $900 | $450,00 | 0,009% | 0,26% | −8,8% | 17% |

**O perfil de $10 deixa de existir nessa condição:** empatar exigiria 89% de
acerto, contra os 59,5% do custo modelado. O de $100 sobrevive, mas a barra sobe
de 59,4% para 63,0%.

**REQ-CFG-007** Enquanto `REQ-SIZING-004` e `REQ-BYBIT-005` não estiverem
implementados e testados, nenhum perfil com depósito abaixo de **US$ 160** MAY
ser executado — abaixo disso o resíduo passa de 20% do custo total. Abaixo de
US$ 100 passa de um terço.

Origem: auditoria de risco de 2026-09-20, sobre a documentação da Bybit e os
limites do instrumento lidos ao vivo.

## Método de simulação — `REQ-CFG-006`

| | |
|---|---|
| Amostra | 8.736 operações por perfil: uma entrada a cada 60 min sobre os doze meses de `market.db` |
| Desfechos | alvo atingido (`+R`), stop atingido (`−R`) ou **expirada** — liquidada a mercado ao fim de 1440 min, com o retorno real medido, nunca tratada como zero |
| Reamostragem | bootstrap sobre a amostra medida, 20.000 execuções de 30 dias |
| Semente | fixa (`42` para a família, `7` para a sensibilidade) |
| Custo aplicado | 0,25% do valor negociado, por operação |
| Posição | `teto × capital corrente` — capitaliza a cada operação |
| Regras ativas | operações/dia, limite diário interrompe o dia, parada por drawdown de 15% do depósito, chão de operação de 5 USDT interrompe a sessão |

**Por que as expiradas importam:** 31% a 41% das posições atingem o prazo sem
tocar stop nem alvo, e liquidadas a mercado rendem **+0,068%** em média.
Tratá-las como zero elevava o acerto necessário de 59,4% para 60,6% — erro de
1,2 ponto numa grandeza cuja sensibilidade é de 0,7% ao mês por ponto.

## Correção registrada — 2026-09-20

A primeira versão deste registro trazia um perfil `banca-12` com teto de posição
de 100% e stop de 1,0%. Aqueles valores foram **escolhidos, não derivados**: o
teto foi fixado em 100% por julgamento, o que amarrava o stop ao risco e elevava
o custo a 25% do valor arriscado.

A função de derivação desta versão produz, para o mesmo capital, teto de ~59% e
stop de ~1,70%, com acerto necessário praticamente igual ao de $100. A conclusão
que aquele perfil sustentava — de que capital maior baixaria a barra de acerto —
**estava errada**. Capital maior compra granularidade, saída parcial e folga até
o chão; a barra de acerto permanece em ~59,4% em qualquer tamanho de banca.

## Valores ainda `ASSUMED`

| Premissa | Valor usado | Efeito se estiver errada |
|---|---|---|
| Taxa por perna | 0,1% (VIP0 público) | é ~80% do custo total; a 0,06% o resultado esperado de $100 sem vantagem sobe de −8,5% para cerca de −5,5%, e o acerto de empate cai de 59,4% para ~57% |
| Slippage + spread | 0,05% por round trip | não medido contra execução real; depende do `06_EXECUTION_SIMULATOR.md` |

Ambas MUST ser confirmadas antes de qualquer uso com capital real
(`REQ-CFG-004`).

## O que estes perfis não resolvem

Os percentuais dimensionam e protegem. Não criam vantagem. A entrada aleatória
medida sobre os mesmos doze meses perde em todas as combinações testadas —
o custo, mais uma pequena assimetria desfavorável. **A estratégia precisa
adicionar 12,9 pontos percentuais de acerto sobre o acaso** para empatar. Esse
número, e não `profit factor >= 1.3`, é o alvo verificável de desenvolvimento:
é medível hoje, contra os mesmos dados, sem esperar por porta nenhuma.

Medição registrada: estender o prazo máximo de posição de 24h para 72h reduz a
fração de posições que expiram sem resolver de 41% para 6%, e com ela a barra de
acerto. Isso conflita com o domínio declarado na constitution — day trade — e
**não foi decidido**.
