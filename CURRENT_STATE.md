# Estado atual do projeto

**Projeto:** Trade — robô de day trade automatizado de Bitcoin
**Atualizado em:** 2026-09-20
**Repositório:** https://github.com/cardosojoel/Trade (privado)

---

## Em uma frase

**A feature 001 está completa: 105 de 105 tarefas.** O robô coleta histórico da
Bybit, avalia estratégias sobre ele, registra toda decisão de forma
reconstituível e mantém toda ordem sob uma camada de risco que a estratégia não
consegue contornar. Tudo em modo backtest — paper trading e capital real são
recusados explicitamente.

**207 testes verdes · clippy limpo · CI verde · tudo sincronizado com o remoto**

---

## Decisões esperando por você

Nenhuma bloqueia o que existe. Mudam comportamento e não são minhas para tomar.

### `max_total_exposure` hoje não morde

Tamanho de posição e exposição total são calculados sobre a mesma grandeza
(`quantidade × preço`). Com um ativo e uma posição, o limite mais apertado
sempre vence. Na configuração atual (1000 vs 2000), a exposição é decorativa.

Não é defeito: os dois só divergem com múltiplos ativos ou posições
simultâneas, ambos fora do escopo desta feature. Mas hoje você tem, na prática,
um limite e não dois.

---

### A SDD v3 (`MD BOT/`) está no repositório como referência, não como plano

Quarenta documentos de especificação — nove frentes de baseline, nove de
estratégia e quant, seis de plataforma, cinco de governança P0 e quatro
especificações matemáticas. **Nada dela foi implementado, e ela não descreve o
que existe hoje.** Versionada em 2026-09-20 para que a discussão aconteça sobre
texto rastreável.

Três pontos conflitam com a constitution e só uma emenda formal resolve:

| Conflito | SDD | Constitution |
|---|---|---|
| **Mercado** | derivativos — `leverage_min/max`, funding, `reduce-only`, "execução de derivativos em alta volatilidade" | spot, **apenas comprado**, sem alavancagem |
| **Recovery** | depois de atingido o limite de perda, recoloca lucro realizado em risco com risco por operação maior (`RT` 25% contra `T` 2%) | atingido o limite, **cessar a abertura** até o próximo período, "sem exceção configurável em tempo de execução" |
| **Tipos** | `29_RUST_CONTRACTS.md` usa `f64` para `tick_size`, `qty_step`, `min_notional` e EV | `tests/no_float.rs` reprova `f64` em caminho monetário — e a própria `trading_risk_recovery_mathematical_spec.md` §33 proíbe |

O Recovery é o conflito de fundo, e não é descuido: a especificação tem regra
anti-Martingale, orçamento consumível e risco que diminui a cada perda. É
disciplinado. Mas continua sendo autorização para operar depois do freio, que é
exatamente o que o Princípio II proíbe sem emenda.

**Corrigido em 2026-09-20 — o conjunto tinha duas camadas dizendo a mesma
coisa.** Os quatro documentos matemáticos eram os originais; `03`, `07` e
`10`–`18` eram versões comprimidas deles. Era essa duplicação que produzia as
contradições da auditoria: o P0 fora aplicado aos resumos e não aos originais,
então o EV binário e o vocabulário duplicado de regime sobreviviam onde a
matemática de fato mora.

O que mudou: cada assunto tem agora **um** documento normativo; os onze resumos
viraram índices que apontam para ele e não enunciam regra própria; as correções
P0 foram portadas para os originais; e o novo `00_GLOSSARIO.md` é a autoridade
sobre nomes — incluindo o símbolo `C`, que designava custos num documento e
ciclos de Recovery em outro, e `R`, que era teto de Recovery e risco monetário
ao mesmo tempo.

O gate que a própria SDD define: preencher `26_REQUIREMENTS_TRACEABILITY_MATRIX.md`
com referências reais ao código antes de abrir qualquer frente nova.

Continua aberto no conjunto de documentos: linguagem normativa (MUST/SHOULD) e
IDs de requisito por documento, cabeçalho de estado em todos, autoavaliações que
ainda vivem dentro das especificações `01`–`09`, e um exemplo numérico único que
atravesse o sistema inteiro — cruzando os exemplos que já existem, um depósito
de R$ 50 com risco de 2% e stop de 1,2% pede uma posição de R$ 83, impossível em
spot sem alavancagem, e nenhum documento cruza os dois.

---

## Decidido em 2026-09-20: a perda diária conta o não realizado

O contador do dia passou a ser **realizado no dia + variação do não realizado
desde a virada** (FR-019b). Antes, `record_realized` só se movia quando uma
venda fechava operação, e uma posição aberta perdendo 50% nunca acionava o
freio.

O que a constitution decidiu, e não eu:

| Texto | Consequência no desenho |
|---|---|
| "cessar a **abertura** de novas posições" | o freio barra compra; venda continua passando — não há liquidação forçada |
| "**até o próximo período**" | o bloqueio **trava**; recuperação de preço no mesmo dia não o solta |
| "MUST retomar automaticamente na virada" | o não realizado entra por **variação**: o prejuízo aberto de ontem é linha de base de hoje, senão o freio re-armaria toda madrugada |

**O que isto não resolve:** o freio impede risco novo, não estanca o prejuízo
que corre. Uma posição aberta pode continuar afundando depois de acionado o
bloqueio. Liquidação automática seria outra decisão — e a constitution, como
está escrita, não a pede.

`RiskGuard::mark_to_market` é a entrada nova; o motor chama a cada vela
fechada. `record_realized` passou a receber o não realizado no mesmo argumento
— uma venda move as duas parcelas ao mesmo tempo, e atualizar só uma contaria
o mesmo prejuízo duas vezes.

**Medido depois: o freio não dispara neste cenário.** Eu previ que o número de
operações cairia. Não caiu — os doze meses deram resultado idêntico ao da regra
antiga, com **zero** decisões de freio armado em 28.633 e pior dia em −143,80
contra um limite de −200.

A `sma-cross` não perde em quedas diárias violentas: ela sangra custo de
transação, cerca de −27 por dia. E o teto de posição (1.000) limita o prejuízo
aberto a uma queda de 20% do preço no mesmo dia. **Um limite de perda diária de
2% simplesmente não é a cerca que contém esta estratégia** — insumo direto para
a calibração que a constitution exige antes da Porta 3.

---

## Decisões que definem o projeto

| | |
|---|---|
| **Domínio** | Robô de day trade de Bitcoin |
| **Mercado** | Spot, **apenas comprado** — sem alavancagem, sem venda a descoberto |
| **Corretora** | Bybit (histórico público, sem credencial) |
| **Autonomia** | Total dentro dos limites; sem aprovação humana por ordem |
| **Linguagem** | Rust estável 1.98.1, edition 2024 |
| **Armazenamento** | SQLite em dois arquivos: `market.db` (cache) e `runs.db` (auditoria) |

A constitution (`.specify/memory/constitution.md`, **v1.2.0**) governa tudo e
tem precedência sobre qualquer outra prática.

---

## O que funciona

```bash
trade collect  --symbol BTCUSDT --interval 1m --from 2025-09-20 --to 2026-09-20
trade backtest --mode backtest --from 2025-09-20 --to 2026-09-20 --capital 10000 \
               --limits limits.toml --fees fees.toml
trade kill                  # aciona o freio; --release libera
```

Medido sobre **dados reais** de doze meses, recoletados e remedidos **depois**
da mudança da perda diária (ver [docs/desempenho.md](docs/desempenho.md)):

| | |
|---|---|
| Coleta | 525.600 velas, 526 páginas, 4m46, pico de 11,4 MB |
| Backtest | 525.600 velas em **2,46s** (a meta era 60s), 14,6 MB |
| Conta | resultado reportado × soma do extrato: **divergência zero** em 14.308 operações |
| Auditoria | 143.135 eventos, `seq` sem buraco; 28.633 ordens e 28.633 decisões |
| Determinismo | duas execuções idênticas dígito a dígito |

O backtest anterior marcava 1,03s. A diferença é a máquina, não a marcação a
mercado: o binário antigo, sem marcação, roda em 2,44s aqui.

---

## As três garantias estruturais

Não são convenção. Verifiquei as três violando de propósito.

| Garantia | O que acontece se for violada |
|---|---|
| Estratégia, risco e backtest não alcançam corretora nem rede | `tests/architecture.rs` falha, CI barra o merge |
| Nenhum `f32`/`f64` em caminho monetário | `tests/no_float.rs` falha, CI barra o merge |
| Obter o executor de dentro do `RiskGuard` | **Não compila** |

`trade-strategy` não declara `trade-ports` e por isso não consegue sequer
**nomear** `OrderExecutor`. Um guarda adicional impede que estratégia ou risco
mencionem nome de corretora no próprio texto.

---

## Estrutura

```
crates/
├── trade-domain/      tipos puros, sem E/S
├── trade-ports/       as traits de fronteira
├── trade-risk/        camada de risco; possui o executor
├── trade-strategy/    sma-cross e reckless; depende só do domínio
├── trade-backtest/    o motor
├── trade-storage/     SQLite: histórico, execuções, auditoria
├── trade-bybit/       coleta; única crate com HTTP
└── trade-cli/         binário `trade`; ponto de composição
tests/                 travas de arquitetura
```

---

## Pendências

### 🟡 Os limiares ainda não foram calibrados

Perda diária 2%, posição 10%, exposição 20%, profit factor 1.3, drawdown 15% —
**propostos pelo assistente e aceitos como valores de partida provisórios**, não
medidos. Vivem em `limits.toml`, nunca no código.

A constitution **exige** revisão contra o capital real antes da Porta 3.

A medição de 2026-09-20 deu a primeira evidência concreta para essa revisão: o
limite de perda diária **nunca é alcançado** pela estratégia de referência, nem
contando prejuízo aberto. Um limiar que não dispara em doze meses ou está
folgado demais, ou está medindo a grandeza errada para este perfil de perda.

### 🟡 A estratégia de referência perde dinheiro, e isso é informação

Sobre doze meses reais: **−98,7%**, com 9.664 de custo de transação sobre 10.000
de capital, em 14.308 operações.

Não é defeito do motor — é o motor funcionando. Uma estratégia que opera 14 mil
vezes por ano em velas de um minuto não sobrevive ao próprio custo. Antes de
buscar estratégia melhor, vale decidir se a granularidade de um minuto faz
sentido para este projeto.

### 🟢 Escopo concluído

Não há tarefa pendente na feature 001.

---

## O que vem depois

A feature 001 entregou o núcleo. A constitution define as próximas portas:

| Porta | O que exige |
|---|---|
| **1. Backtest** | ≥ 12 meses com mercado de baixa e evento de alta volatilidade; ≥ 100 operações; profit factor ≥ 1.3; drawdown ≤ 15% |
| **2. Paper trading** | ≥ 30 dias ininterruptos em testnet, com o mesmo código que iria para live |
| **3. Liberação** | Ato humano registrado |

Nenhuma estratégia passou a Porta 1 — a de referência não passa nem perto, e
nem deveria. **O próximo trabalho substantivo é uma feature nova**: ou pesquisa
de estratégia, ou o adaptador de paper trading da Bybit (feature 002).

---

## Documentos

| Arquivo | O que é |
|---|---|
| [README.md](README.md) | Visão geral, instalação, uso |
| [CLAUDE.md](CLAUDE.md) | Diretrizes de trabalho e invariantes |
| [docs/auditoria.md](docs/auditoria.md) | Consultas de reconstituição, prontas para o DBeaver |
| [docs/desempenho.md](docs/desempenho.md) | Medições e cenários do quickstart |
| `MD BOT/` | SDD v3 — especificação de referência, não implementada |
| `.specify/memory/constitution.md` | Governa o projeto |
| `specs/001-nucleo-execucao/` | Spec, plano, pesquisa, contratos, tarefas |
