# 32 — Não-objetivos

**Status:** normativo · **Versão:** 1.0 · **Atualizado em:** 2026-09-20  
**Domínio de requisitos:** `REQ-SCOPE-*`  
**Vocabulário:** [`00_GLOSSARIO.md`](00_GLOSSARIO.md)

---

## Por que declarar o que não se faz

Uma especificação que só diz o que o sistema faz cresce por acréscimo: cada
frente nova parece compatível porque nada a proíbe. Este documento fixa o
contorno — e torna visível quando uma proposta está fora dele.

**A constitution do repositório (`.specify/memory/constitution.md`, v1.2.0)
prevalece sobre este documento e sobre todo o resto da SDD.** Onde os dois
divergirem, o conflito está listado na seção 3 e resolve-se por emenda formal,
nunca por decisão de implementação.

## 1. Fora de escopo por decisão do projeto

**REQ-SCOPE-001** O sistema MUST NOT operar fora destes limites:

| Não-objetivo | Consequência prática |
|---|---|
| Ativo diferente de Bitcoin | nenhuma frente MAY assumir carteira multiativo |
| Venda a descoberto | o robô só compra e vende o que comprou |
| Alavancagem | `PositionNotional` limitado ao caixa — `REQ-SIZING-003` |
| Aprovação humana por ordem | dentro da cerca, o robô decide sozinho |
| Corretora fora da camada de abstração | estratégia e risco MUST NOT nomear a Bybit |

## 2. Fora de escopo por maturidade

**REQ-SCOPE-002** Estes itens MUST NOT ser implementados antes da evidência que
cada um exige:

| Não-objetivo por ora | O que precisa existir antes |
|---|---|
| Reinforcement learning | dataset confiável, ledger, simulador, cost model e walk-forward — `DECISION_LEARNING.md` §50 |
| Promoção automática de modelo | critérios versionados e canary — `REQ-LEARN-008` |
| Budgets de latência por estágio | profiling real — `REQ-PERF-008` |
| SLO absoluto de fill | baseline medida em testnet — `09_PERFORMANCE_LOW_LATENCY.md` SLO 4 |
| Métrica de distância além da euclidiana ponderada | validação fora da amostra — `REQ-PATTERN-003` |

**REQ-SCOPE-003** Nenhuma frente nova MAY ser aberta antes de o
[`26_REQUIREMENTS_TRACEABILITY_MATRIX.md`](26_REQUIREMENTS_TRACEABILITY_MATRIX.md)
apontar para código e teste reais.

## 3. O que esta SDD pressupõe e o projeto não autoriza

Quatro pontos onde a SDD assume um sistema que a constitution não permite.
**Nenhum foi decidido.** Estão aqui para que a divergência seja visível a quem
ler a SDD isoladamente.

| # | A SDD pressupõe | A constitution determina |
|---|---|---|
| 1 | derivativos — `leverage_min/max`, funding, `reduce-only`, microestrutura de derivativos | spot, apenas comprado, sem alavancagem |
| 2 | Recovery: recolocar lucro realizado em risco depois de atingido o limite de perda, com risco por operação maior | atingido o limite, cessar a abertura até o próximo período, sem exceção configurável em tempo de execução |
| 3 | sessão com depósito, `CapitalFloor` e `STOPPED` — nova sessão exige confirmação humana | período diário, com retomada automática na virada |
| 4 | `f64` para `tick_size`, `qty_step`, `min_notional` e EV em `29_RUST_CONTRACTS.md` | nenhum ponto flutuante em caminho monetário — e a própria `trading_risk_recovery_mathematical_spec.md` §33 proíbe |

**REQ-SCOPE-004** Nenhum destes quatro MAY ser implementado antes de emenda
formal à constitution. Implementar primeiro e regularizar depois é a ordem
inversa da que o projeto adotou.

O ponto 2 merece nota: o Recovery é disciplinado — orçamento consumível, risco
decrescente, regra anti-Martingale explícita. A objeção não é à qualidade do
desenho, é à autoridade: ele autoriza abrir posição depois do freio, e o freio é
não-negociável por princípio, não por parâmetro.
