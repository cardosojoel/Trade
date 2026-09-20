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
| [ADR-001](#adr-001) | Derivativos e alavancagem | `CLAUDE.md` | **proposta** |
| [ADR-002](#adr-002) | Recovery depois do freio diário | constitution, Princípio II | **proposta** |
| [ADR-003](#adr-003) | Sessão com depósito e confirmação humana | constitution, Princípio II | **proposta** |
| [ADR-004](#adr-004) | `f64` nos contratos Rust | `CLAUDE.md` e `tests/no_float.rs` | **proposta** |
| [ADR-005](#adr-005) | Promover duas restrições à constitution | nada — corrigia uma lacuna | **aceita em 2026-09-20** |

Uma aceita, quatro em proposta. A aceitação da ADR-005 elevou a autoridade
contrariada pelas ADR-001 e ADR-004: antes da emenda 1.3.0 elas contrariavam o
`CLAUDE.md`; agora contrariam a constitution.

---

## ADR-001 — Derivativos e alavancagem {#adr-001}

**Status:** proposta · **Contraria:** constitution, *Restrições Operacionais e de
Segurança* → **Mercado** (desde a emenda 1.3.0; antes dela, `CLAUDE.md`)

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

**Status:** proposta · **Contraria:** constitution, Princípio II, linha 44

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

**Status:** proposta · **Contraria:** constitution, Princípio II, linha 45

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

**Status:** proposta · **Contraria:** constitution, *Restrições Operacionais e de
Segurança* → **Representação de valores monetários** (desde a emenda 1.3.0), mais
`tests/no_float.rs`

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
