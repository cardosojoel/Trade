# 32 — Não-objetivos

**Status:** normativo · **Versão:** 1.0 · **Atualizado em:** 2026-09-20  
**Domínio de requisitos:** `REQ-SCOPE-*`  
**Conformidade:** conforme  
**Vocabulário:** [`00_GLOSSARIO.md`](00_GLOSSARIO.md)

---

## Por que declarar o que não se faz

Uma especificação que só diz o que o sistema faz cresce por acréscimo: cada
frente nova parece compatível porque nada a proíbe. Este documento fixa o
contorno — e torna visível quando uma proposta está fora dele.

**A constitution do repositório (`.specify/memory/constitution.md`, v1.3.0)
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

Quatro pontos onde a SDD assume um sistema que o projeto não autoriza. **Nenhum
foi decidido**, e cada um tem ADR aberta em
[`34_ADR_EMENDAS.md`](34_ADR_EMENDAS.md).

Desde a emenda **1.3.0**, aceita em 2026-09-20 pela ADR-005, os quatro
contrariam a constitution. Até aquele dia, dois contrariavam apenas uma diretriz
de trabalho — a coluna da direita preserva esse histórico, porque foi ele que
motivou a emenda.

**Todas decididas em 2026-09-20. Nenhuma divergência permanece aberta.**

| # | O que era | Decisão | Efeito |
|---|---|---|---|
| 1 | derivativos, alavancagem, funding | **recusada** (ADR-001) | o escopo fica à vista; os campos de alavancagem não geram tarefa |
| 2 | Recovery depois do freio diário | **aceita** (ADR-002) | incorporado ao Princípio II pela emenda 2.0.0, sob seis condições cumulativas |
| 3 | sessão com confirmação humana | **recusada** (ADR-003) | retomada automática mantida; confirmação só quando os parâmetros derivados mudam |
| 4 | `f64` em caminho monetário | **encerrada** (ADR-004) | era erro de redação; o `29` foi corrigido para `Decimal` |

**REQ-SCOPE-004** Nenhum destes quatro MAY ser implementado antes de a ADR
correspondente ser aceita. Implementar primeiro e regularizar depois é a ordem
inversa da que o projeto adotou.

**REQ-SCOPE-005** Uma proposta MUST nomear o documento que contraria. "Conflita
com a constitution" MUST NOT ser usado como fórmula genérica, ainda que hoje ela
seja verdadeira para as quatro: foi a verificação documento a documento que
revelou a lacuna fechada pela emenda 1.3.0.

A ADR-005 tratou dessa lacuna e **foi aceita em 2026-09-20**. As duas restrições
que limitam a perda máxima possível — mercado à vista sem alavancagem e ausência
de ponto flutuante em caminho monetário — passaram a viver na constitution, onde
alterá-las exige emenda escrita e aprovada.

O ponto que a ADR-005 levantou foi fechado no mesmo dia: a restrição de mercado
à vista **tem proteção executável** desde `tests/no_leverage.rs`, que barra
vocabulário de alavancagem e derivativo em qualquer crate, exige `category=spot`
explícito em toda chamada à Bybit, trava `Side` em `Buy`/`Sell` e verifica que o
domínio mantém o erro que impede posição negativa. Cada uma das quatro travas
foi verificada violando de propósito.

O ponto 2 merece nota: o Recovery é disciplinado — orçamento consumível, risco
decrescente, regra anti-Martingale explícita. A objeção não é à qualidade do
desenho, é à autoridade: ele autoriza abrir posição depois do freio, e o freio é
não-negociável por princípio, não por parâmetro.
