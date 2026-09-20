# 27 — Configuration Registry

**Status:** normativo · **Versão:** 4.0 · **Atualizado em:** 2026-09-20  
**Domínio de requisitos:** `REQ-CFG-*`  
**Conformidade:** conforme  
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
| Custo por operação | 0,25% — **taxa confirmada** em 0,1% por perna, maker e taker, lida na conta em 2026-09-20 (`EXCHANGE`), mais 0,05% de slippage e spread ainda presumidos |
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
prazo_maximo       = 4320 min                        (72h — emenda 2.0.0)
operacoes_por_dia  = 1440 / tempo_medio_ate_resolver (uma posição por vez)
limite_diario      = operacoes_por_dia x risco_por_operacao
chao_de_operacao   = min_order_amt / teto_de_posicao
```

**A frequência deixou de ser escolhida e passou a ser consequência.** Com uma
posição por vez e prazo de 72h, o que limita o giro é o tempo até a posição
resolver — medido em 25,2h de média, 19h de mediana. Daí **0,95 operação por
dia**, contra as 2 que o prazo de 24h permitia.

Origem de cada regra:

| Regra | Origem |
|---|---|
| risco = drawdown ÷ 15 | `DERIVED` — a parada por drawdown tolera 15 perdas cheias |
| teto mínimo | `DERIVED` — a posição MUST continuar emitível depois do drawdown máximo |
| stop de 2,00% | `MEASURED` — minimiza o acerto necessário |
| operações por dia | `DERIVED` — mantém o custo mensal em ~7% do capital |
| prazo de 72h | `MEASURED` — resolve 94% das janelas contra 59% em 24h; abaixo de 4h o custo excede o movimento mediano |

**Viabilidade:** `D >= min_order_amt / 0,85` = **5,88 USDT**. Abaixo disso nem a
primeira posição sobrevive ao drawdown, e a sessão MUST NOT iniciar.

## Família derivada

Com prazo de 72h, uma posição por vez e frequência de 0,95 operação/dia.

| Depósito | Teto | Stop | Risco | Posição | Risco $ | Lim. diário | Chão | Faixa |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| $200 | 50,0% | 2,00% | 1,00% | $100,00 | $2,00 | 1,0% | $10,00 | 20 ordens |
| $500 | 50,0% | 2,00% | 1,00% | $250,00 | $5,00 | 1,0% | $10,00 | 50 ordens |
| $900 | 50,0% | 2,00% | 1,00% | $450,00 | $9,00 | 1,0% | $10,00 | 90 ordens |

Acima de **11,76 USDT** o stop atinge o ótimo medido e os percentuais
**congelam**. O que o capital compra é granularidade, saída parcial — possível a
partir de 20 USDT — e folga até o chão. O `REQ-CFG-007` mantém o piso prático em
**US$ 160** enquanto o resíduo da moeda base não for tratado no código.

O perfil de US$ 10 foi retirado desta tabela: com o custo real do resíduo ele
exigiria 89% de acerto, e não existe.

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

Saldo esperado ao fim de 30 dias **sem vantagem nenhuma**, no prazo de 72h:
20.000 simulações reamostrando 8.688 operações medidas.

| Depósito | Resultado | Faixa p5–p95 | Parou por drawdown |
|---:|---:|---:|---:|
| $200 | **−4,3%** | $175 – $208 | **1%** |
| $500 | **−4,2%** | $439 – $521 | **1%** |
| $900 | **−4,2%** | $790 – $938 | **1%** |

**O prazo de 72h cortou o custo pela metade e o risco de parada por dezesseis.**
No prazo de 24h os mesmos perfis perdiam 8,5% ao mês e 16% das contas batiam a
parada por drawdown antes do fim do mês. A razão é a mesma nos dois casos: 41%
das posições de 24h expiravam sem resolver e pagavam custo sem produzir
resultado; em 72h são 6%.

### Acerto necessário e sensibilidade

O empate exige **56,6%** de acerto entre as operações que resolvem. A entrada
aleatória entrega 48,4%. A lacuna que a estratégia precisa produzir é de
**8,2 pontos percentuais** — era 12,9 no prazo de 24h.

| Acerto entre resolvidos | Resultado em 30 dias |
|---:|---:|
| **48,4% — medido, acaso** | **−4,2%** |
| 52,0% | −2,3% |
| **56,6% — empate** | 0,0% |
| 60,0% | +1,8% |
| 64,0% | +3,9% |

A derivada é de **0,53% ao mês por ponto** de acerto, no prazo de 72h com 0,95
operação/dia — era 0,7% no prazo de 24h, porque lá o giro era o dobro. O
resultado é menos sensível ao acerto e também menos sensível ao erro.

**Um ponto de acerto continua valendo mais que qualquer recalibração de
percentual de risco.** A diferença é que a barra a vencer caiu de 12,9 para 8,2
pontos, e não por a estratégia ter melhorado: por parar de pagar custo em
posições que expiravam sem resolver.

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
de 59,4% para 63,0% — números do perfil de 24h, que era o vigente quando o
resíduo foi medido. O multiplicador de custo do resíduo não depende do prazo; o
acerto de empate correspondente, sim.

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
o chão; a barra de acerto não depende do tamanho da banca. Ela depende do prazo:
56,6% em 72h, 59,4% em 24h.

## Valores ainda `ASSUMED`

| Premissa | Valor usado | Efeito se estiver errada |
|---|---|---|
| ~~Taxa por perna~~ | **confirmada** | Lida na conta em 2026-09-20: spot 0,1% maker e taker, nível "usuário comum". Era a única premissa que respondia por 80% do custo, e o valor presumido estava certo. Passa de `ASSUMED` a `EXCHANGE`. |
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
