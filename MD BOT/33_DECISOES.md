# 33 — Registro de decisões

**Status:** normativo quanto às decisões registradas · **Versão:** 1.0 ·
**Atualizado em:** 2026-09-20  
**Vocabulário:** [`00_GLOSSARIO.md`](00_GLOSSARIO.md)

---

## Para que serve

As especificações dizem o que o sistema faz. Não dizem por que se escolheu
assim, nem de onde veio cada número. Sem isso, toda revisão futura recomeça a
discussão do zero — e ninguém sabe distinguir uma escolha fundamentada de um
valor que alguém escreveu uma vez.

Este documento registra as decisões e, com igual importância, **marca as que não
têm racional registrado**. A segunda lista é a mais útil das duas.

Regra: uma decisão MUST ser registrada aqui quando altera arquitetura, autoridade
ou matemática. Alterar uma decisão MUST criar entrada nova, nunca reescrever a
anterior.

---

## A. Arquitetura e execução

| # | Decisão | Racional registrado | Onde vive |
|---|---|---|---|
| D-01 | WebSocket Order Entry é o caminho preferencial de `create/amend/cancel`; REST fica para recuperação, reconciliação e contingência | latência no caminho crítico; REST permanece por ser o caminho confiável quando o WS cai | `01` §Caminhos |
| D-02 | `execution.fast` é assinado em paralelo, não como fonte única | entrega só `execType=Trade` e menos campos; serve para detectar fill rápido, não para reconstruir lifecycle | `01` §Execution Fast |
| D-03 | ACK nunca é tratado como fill | ACK confirma aceitação da requisição, não execução | `REQ-EXEC-009` |
| D-04 | Identidade de execution é `symbol + seq + execId + orderId` | `seq` pode repetir entre transações simultâneas e entre símbolos | `REQ-RECON-003` |
| D-05 | Hot path sem SQLite, log síncrono ou aprendizado | preservar o caminho de decisão e o de emergência do custo de E/S | `REQ-PERF-001`, `REQ-PERF-007` |
| D-06 | Rate limit lido dos headers, nunca presumido | o limite publicado varia por endpoint e por conta | `REQ-PERF-004` |

## B. Matemática, risco e aprendizado

| # | Decisão | Racional registrado | Onde vive |
|---|---|---|---|
| D-07 | EV ternário é canônico; a forma binária é projeção com `P_neutral = 0` | com `P_neutral > 0`, `1 − P_win` não é `P_loss` — AUD-MATH-001 | `REQ-EV-001` |
| D-08 | `RecoveryEpisode` e `RecoveryBudgetSession` são grandezas distintas | condicionar episódio novo ao alvo não alcançado impediria qualquer episódio posterior — AUD-MATH-002 | `REQ-RECOVERY-006` |
| D-09 | `WorstCaseSessionExposure = MaxLossDeposit + RecoveryMaxSession`, exibidos separados antes da soma | são fontes de capital diferentes; somar sem decompor esconde a origem — AUD-MATH-003 | `REQ-RISK-007` |
| D-10 | Orçamento de Recovery é consumível e criado uma única vez | impedir o laço perda → orçamento novo → perda | `REQ-RECOVERY-003`, `REQ-RECOVERY-007` |
| D-11 | Nenhum ponto flutuante representa dinheiro | determinismo exato do cálculo financeiro | `REQ-RISK-009`, `REQ-STRATEGY-002` |
| D-12 | Aprendizado é offline-first; RL só depois de dataset, ledger, simulador e walk-forward | sem esses quatro, RL otimiza contra um ambiente que não representa o mercado | `DECISION_LEARNING.md` §49–50 |
| D-13 | `LIVE_MODEL` é imutável; aprendizado gera `CANDIDATE_MODEL` | impedir auto-modificação em execução | `REQ-LEARN-006`, `REQ-LEARN-007` |
| D-14 | Em spot, `PositionNotional` limitado ao caixa, com registro do limite vinculante | a fórmula de sizing produz ordem impossível quando o stop é mais curto que o risco autorizado | `REQ-SIZING-003`, `31` §6 |

## C. Estrutura do conjunto

| # | Decisão | Racional registrado | Onde vive |
|---|---|---|---|
| D-15 | Cada assunto tem um documento normativo; os resumos viraram índices | a correção do EV binário fora aplicada ao resumo e não ao original, e sobreviveu por isso | `00_MASTER_INDEX_FINAL.md` §1.1 |
| D-16 | Vocabulário canônico, com símbolos exclusivos | `C` designava custos e ciclos; `R` designava teto de Recovery e risco monetário | `00_GLOSSARIO.md` |
| D-17 | Identificador de requisito ao lado da regra, não em lista à parte | lista separada recria a segunda camada que D-15 eliminou | `00_GLOSSARIO.md` §10 |
| D-18 | Avaliação de maturidade sai da especificação e vai para a auditoria | uma especificação não se dá nota | `25` §13 |

---

## D. Valores sem racional registrado

Cada linha abaixo é um número que hoje governa comportamento sem que exista, em
nenhum documento, a derivação que o justifique. Nenhum deles é errado — mas
nenhum é defensável, e a distinção importa antes de qualquer promoção.

| Valor | Onde aparece | O que se sabe |
|---|---|---|
| `K = 100` vizinhos | quant §18, pattern §7 | apresentado como valor inicial; os próprios documentos mandam validar por walk-forward e não assumir ótimo |
| `k = 1,5` no stop por ATR | quant §21 | exemplo, não derivação; o documento diz que `k` só deve ser otimizado fora da amostra |
| Faixas de amostra 100 / 500 / 2.000 | quant §17, learning §23 | rotuladas "devem ser validadas empiricamente" |
| `λ` do peso exponencial | pattern §9 | declarado hiperparâmetro, sem valor nem faixa |
| `θ_up`, `θ_down` | pattern §11 | regra qualitativa — cobrir custos — sem valor |
| `EV_min`, `EV_min_conservative`, `ESS_min`, `min_similarity` | quant §15, pattern §32 | nomeados no Registry, sem valor e sem método para obtê-lo |
| SLO interno p99 ≤ 500 µs e p99.9 ≤ 1 ms | perf SLO 1 | declarados "metas de engenharia, não garantias da Bybit"; não derivam de perfil de execução nem da necessidade da estratégia |
| Emergency p99 ≤ 250 µs | perf SLO 2 | idem |
| Idade do dado 50 / 100 / 250 ms | `04` | "devem ser calibrados para o feed e a estratégia" |
| Headroom 30% / 10% de rate limit | perf SLO 5 | sem origem |
| RSS < 1% em 24h, CPU < 50% / < 80% | perf SLO 6–7 | "meta inicial", sem baseline definida |
| `D`, `L`, `P`, `R`, `T`, `RT`, `Emax`, `Amax` do exemplo | spec de risco §3 | exemplo didático, nunca proposto como configuração |

O mesmo vale, no repositório, para os limiares de `limits.toml` — perda diária
2%, posição 10%, exposição 20%, profit factor 1.3, drawdown 15% — que a
constitution registra como valores de partida provisórios e **exige** rever
antes da liberação para capital real.

O [`31_EXEMPLO_FIM_A_FIM.md`](31_EXEMPLO_FIM_A_FIM.md) mostra por que isso não é
detalhe: com os valores atuais, o teto de posição torna o limite de risco por
operação inalcançável por um fator de 16,7. Dois números razoáveis isoladamente,
incompatíveis juntos.
