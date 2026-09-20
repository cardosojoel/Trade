# 34 — ADR: decisões que exigem assinatura

**Status:** normativo · **Versão:** 1.0 · **Atualizado em:** 2026-09-20  
**Domínio de requisitos:** `REQ-ADR-*`  
**Conformidade:** conforme  
**Vocabulário:** [`00_GLOSSARIO.md`](00_GLOSSARIO.md)

---

## Como funciona

Este é o único caminho entre o desenho e o código, conforme
[`00_FRONTEIRA.md`](00_FRONTEIRA.md) §2. Uma proposta que contrarie a
constitution, o `CLAUDE.md` ou um invariante do build vira ADR aqui e **para**,
até ser aceita ou recusada pelo mantenedor.

**REQ-ADR-001** Toda ADR MUST declarar: o que contraria, onde essa regra vive,
qual ato a resolveria, e o que se perde ao aceitar. Uma ADR sem a última linha é
proposta, não decisão.

**REQ-ADR-002** Status possíveis: `proposta`, `aceita`, `recusada`,
`substituída por ADR-NNN`. Status MUST NOT ser alterado por quem escreve a
proposta — apenas por registro de decisão do mantenedor, com data.

**REQ-ADR-003** Enquanto uma ADR estiver em `proposta`, nenhum documento que
dela dependa MAY gerar tarefa de implementação no repositório Trade.

### Situação

| ADR | Assunto | Contraria | Status |
|---|---|---|---|
| [ADR-001](#adr-001) | Derivativos e alavancagem | constitution → Mercado | **recusada em 2026-09-20** |
| [ADR-002](#adr-002) | Recovery depois do freio diário | constitution, Princípio II | **aceita em 2026-09-20** — emenda 2.0.0 |
| [ADR-003](#adr-003) | Sessão com depósito e confirmação humana | constitution, Princípio II | **recusada em 2026-09-20**, com encaminhamento |
| [ADR-004](#adr-004) | `f64` nos contratos Rust | contradição interna ao conjunto | **encerrada em 2026-09-20** — corrigida |
| [ADR-006](#adr-006) | Prazo máximo de posição de 72 horas | constitution → domínio | **aceita em 2026-09-20** — emenda 2.0.0 |
| [ADR-005](#adr-005) | Promover duas restrições à constitution | nada — corrigia uma lacuna | **aceita em 2026-09-20** |

**Nenhuma em proposta.** Três aceitas (005, 002, 006), duas recusadas (001, 003)
e uma encerrada por correção (004). As decisões foram tomadas pelo mantenedor em
2026-09-20 e produziram duas emendas: a 1.3.0 e a 2.0.0.

Com a ADR-001 recusada e a ADR-004 corrigida, **nenhum documento deste conjunto
permanece marcado `exige emenda`**.

---

## ADR-001 — Derivativos e alavancagem {#adr-001}

**Status:** **recusada em 2026-09-20** pelo mantenedor · **Contraria:**
constitution, *Restrições Operacionais e de Segurança* → **Mercado**

### Resultado
Recusada. O mercado permanece à vista, apenas comprado, sem alavancagem — a
perda continua limitada ao depósito. Os documentos que pressupunham funding,
`leverage_min/max` e `reduce-only` deixam de ter caminho para implementação;
`tests/no_leverage.rs` passa a cobrar algo que ninguém mais pretende contrariar.

Registrado para quem reabrir o assunto: a tabela de taxas da conta, lida em
2026-09-20, mostra derivativos **mais baratos** que spot — futuros a 0,02%
maker e 0,055% taker, contra 0,1% em ambos no spot. A recusa não foi por custo;
foi porque com alavancagem o pior caso deixa de ser o depósito.

### Contexto
Boa parte deste conjunto pressupõe mercado de derivativos: `leverage_min` e
`leverage_max` na ficha do instrumento, funding no cálculo de custo,
`reduce-only` entre os tipos de ordem, e microestrutura de derivativos no
simulador de execução.

O `CLAUDE.md` determina mercado à vista, apenas comprado, sem alavancagem e sem
venda a descoberto. A constitution **não trata do assunto** — ver ADR-005.

### Decisão proposta
Autorizar derivativos com alavancagem limitada, com teto declarado em
configuração e sujeito ao `RiskGuard`.

### O que se perde ao aceitar
A perda deixa de ser limitada ao depósito. Hoje, `PositionNotional` limitado ao
caixa (`REQ-SIZING-003`) garante que o pior caso é perder o que se depositou. Com
alavancagem, o pior caso passa a depender da liquidação da corretora — e o
racional do Princípio II é exatamente impedir que um bug vire perda ilimitada.

Também perde-se a simplicidade que sustenta metade das medições deste conjunto:
os perfis de banca, o piso de operação e o resultado esperado foram todos
derivados supondo caixa próprio.

---

## ADR-002 — Recovery depois do freio diário {#adr-002}

**Status:** **aceita em 2026-09-20** pelo mantenedor · **Contraria:**
constitution, Princípio II

### Resultado
Aceita com as seis travas, e incorporada ao Princípio II pela **emenda 2.0.0** —
MAJOR, por ser a primeira exceção a um princípio não-negociável neste projeto.
As travas viraram condições cumulativas no texto da constitution: orçamento
criado uma única vez a partir de lucro realizado, consumível e sem reposição,
risco decrescente, Martingale proibido, teto de episódios e tentativas fixado
antes da sessão, e proibição de elevar a perda máxima do depósito.

A emenda registra que esta exceção **MUST NOT ser usada como precedente**.

### Contexto
A constitution determina: *"Atingido o limite de perda diária, o sistema MUST
cessar a abertura de novas posições até o próximo período, sem exceção
configurável em tempo de execução."*

A especificação de risco deste conjunto define um mecanismo de Recovery que
recoloca lucro **realizado** em risco depois de atingido o limite, com risco por
operação maior que o normal.

### Decisão proposta
Autorizar o Recovery como exceção formal ao Princípio II, com as travas que a
própria especificação já define: orçamento consumível criado uma única vez,
risco decrescente a cada perda, regra anti-Martingale explícita e teto de
episódios.

### O que se perde ao aceitar
O freio deixa de ser absoluto. Hoje ele é a única regra do sistema sem exceção
configurável — e é essa ausência de exceção que o torna confiável. Um freio com
uma exceção bem desenhada continua sendo um freio com exceção, e a próxima
exceção terá um precedente.

O desenho do Recovery é disciplinado; a objeção é de autoridade, não de
qualidade.

---

## ADR-003 — Sessão com depósito e confirmação humana {#adr-003}

**Status:** **recusada em 2026-09-20** pelo mantenedor · **Contraria:**
constitution, Princípio II

### Resultado
Recusada na forma proposta, e encaminhada na forma intermediária que esta
própria ADR havia levantado: **mantém-se a retomada automática na virada do
período**, e a confirmação humana passa a ser exigida apenas quando os
parâmetros derivados mudarem — o caso que o `REQ-BYBIT-007` já prevê, quando a
Bybit revisa a ordem mínima nos dias 3 e 17. Isso não contraria a constitution e
não precisou de emenda.

Fica um ponto aberto no desenho, e é consequência de aceitar a ADR-002: o
orçamento de Recovery é definido **por sessão**, e sessão deixou de ser a
unidade de tempo do sistema. Qual período delimita o orçamento — o dia, a
semana, a vida da configuração — não foi decidido.

### Contexto
A constitution determina retomada **automática** na virada do período. Este
conjunto modela o ciclo de vida como sessão: depósito, capital floor, estado
`STOPPED` e nova sessão exigindo confirmação humana explícita dos parâmetros.

### Decisão proposta
Substituir o período diário com retomada automática pelo ciclo de sessão com
confirmação.

### O que se perde ao aceitar
A operação desassistida. O racional do Princípio II diz que um robô que pede
permissão a cada passo não é robô de day trade — a janela fecha antes da
resposta. Com confirmação por sessão, o robô para todo dia até alguém aparecer.

Há uma forma intermediária que não exige emenda e não foi explorada: manter a
retomada automática e usar a confirmação **apenas** quando os parâmetros
derivados mudarem — que é o caso previsto em `REQ-BYBIT-007`.

---

## ADR-004 — `f64` nos contratos Rust {#adr-004}

**Status:** **encerrada em 2026-09-20** — corrigida, sem decisão a tomar ·
**Contradizia:** a `trading_risk_recovery_mathematical_spec.md` §33 e, desde a
emenda 1.3.0, a constitution

### Resultado
O `29_RUST_CONTRACTS.md` foi corrigido: `Decimal` em todo campo monetário e de
quantidade, `f64` mantido apenas onde a grandeza é adimensional e não alimenta
cálculo de dinheiro. A conformidade daquele documento passou de `exige correção`
para `conforme`, e esta ADR encerra sem ir à fila de assinatura — como previsto
desde a sua abertura.

### Contexto
O `29_RUST_CONTRACTS.md` usa `f64` para `tick_size`, `qty_step`, `min_notional`
e para os campos de `ExpectedValue`. O `tests/no_float.rs` do Trade reprova
`f32`/`f64` em caminho monetário, e a própria especificação de risco deste
conjunto proíbe ponto flutuante para dinheiro (§33). A constitution **não trata
do assunto** — ver ADR-005.

### Decisão proposta
Nenhuma. Esta ADR existe para registrar que a divergência é **erro de redação**
do `29`, não escolha de desenho: dois documentos deste mesmo conjunto se
contradizem.

### Encaminhamento
Corrigir o `29` para `Decimal` em todo campo monetário e de quantidade, mantendo
`f64` apenas onde a grandeza é adimensional e não alimenta cálculo de dinheiro —
probabilidade, por exemplo. Isso **não** exige emenda; exige correção. Depois
disso, a ADR-004 é encerrada como `recusada` por não haver decisão a tomar.

---

## ADR-006 — Prazo máximo de posição de 72 horas {#adr-006}

**Status:** **aceita em 2026-09-20** pelo mantenedor · **Contrariava:** o domínio
declarado na constitution — *robô de day trade*

### Contexto
Medido sobre doze meses reais, um alvo simétrico de 2% resolve em **94%** das
janelas de 72h contra **59%** nas de 24h. As posições que expiram sem resolver
pagam custo sem produzir resultado, e eram 41% do total.

### Decisão
Prazo máximo de posição de 72 horas, incorporado pela emenda 2.0.0. O domínio
deixa de ser day trade.

### O que se ganha, medido
| | 24h | 72h |
|---|---:|---:|
| Posições que resolvem | 59% | **94%** |
| Acerto necessário | 59,4% | **56,6%** |
| Lacuna sobre o acaso | 12,9 pt | **8,2 pt** |
| Frequência natural | 2 op/dia | **0,95 op/dia** |
| Custo mensal do capital | 7,2% | **3,5%** |

### O que se perde
O tempo até a evidência dobra: as 100 operações que a Porta 1 exige passam de 50
para **105 dias**. E o projeto deixa de ser o que o seu nome e a sua
documentação diziam ser — o que exigiu MAJOR, não MINOR.

---

## ADR-005 — Promover duas restrições à constitution {#adr-005}

**Status:** **aceita em 2026-09-20** pelo mantenedor · **Contraria:** nada —
corrigia uma lacuna

### Resultado
Emenda **1.3.0** aplicada à `.specify/memory/constitution.md`, em *Restrições
Operacionais e de Segurança*: a seção **Mercado** e a seção **Representação de
valores monetários**, cada uma com o seu racional. Rodapé da constitution
atualizado, com registro de impacto.

**Impacto sobre código e specs: nenhum.** As duas restrições já eram observadas
— a feature 001 opera apenas em spot comprado e usa `rust_decimal` em todo
caminho monetário. A emenda muda o ato necessário para alterá-las, não o
comportamento do sistema.

Consequências registradas: âncora de hash de `00_FRONTEIRA.md` substituída e
conformidades reavaliadas (nenhuma mudou); ADR-001 e ADR-004 passaram a
contrariar a constitution.

**A lacuna que esta ADR levantou foi fechada junto.** O texto abaixo dizia que a
restrição de mercado à vista não tinha proteção executável. Passou a ter:
`tests/no_leverage.rs`, quatro travas cobradas pelo CI — vocabulário de
alavancagem e derivativo, `category=spot` explícito, `Side` limitado a
`Buy`/`Sell`, e a permanência do erro que impede posição negativa. As quatro
foram verificadas violando de propósito.

### Contexto
A verificação de 2026-09-20 encontrou que duas das restrições mais
consequentes do projeto **não estão na constitution**:

| Restrição | Onde vive hoje |
|---|---|
| Mercado à vista, apenas comprado, sem alavancagem | `CLAUDE.md` linhas 91–92 |
| Nenhum ponto flutuante em caminho monetário | `CLAUDE.md` e `tests/no_float.rs` |

A constitution não menciona "spot", "alavancagem", "venda a descoberto",
"float", `f64` nem "decimal" em nenhuma linha.

### Por que isso é um risco

*Texto da proposta, preservado como estava em 2026-09-20 antes da aceitação. Os
dois pontos levantados aqui foram fechados: ver **Resultado**, acima.*

São as duas regras que limitam a perda máxima possível. A primeira garante que
o pior caso é perder o depósito. A segunda garante que o cálculo do que se tem
está certo. Ambas vivem hoje em um arquivo de diretrizes de trabalho, que
qualquer sessão pode reescrever sem racional, sem aprovação e sem registro —
enquanto a constitution exige emenda escrita e aprovada para qualquer mudança.

O `tests/no_float.rs` protege a segunda de fato, porque o CI barra o merge. A
primeira **não tem nenhuma proteção executável**: nenhum teste impede que
alavancagem entre no código.

### Decisão proposta
Emendar a constitution (MINOR) para incorporar as duas restrições ao corpo
normativo, em *Restrições Operacionais e de Segurança*, com o racional de que
ambas derivam do Princípio II — limitar a perda ao que foi deliberadamente
arriscado.

### O que se perde ao aceitar
Flexibilidade de rotina: mudar de ideia sobre mercado à vista passaria a exigir
emenda formal em vez de edição de arquivo. Esse é o objetivo.

### Observação
ADR-001 e ADR-004 dependem desta. Enquanto ADR-005 não for decidida, as duas
contrariam apenas uma diretriz de trabalho — e, portanto, o ato que as
resolveria é mais leve do que este conjunto vinha afirmando.
