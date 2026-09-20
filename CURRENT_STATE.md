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

**194 testes verdes · clippy limpo · CI verde · tudo sincronizado com o remoto**

---

## Duas decisões esperando por você

Nenhuma bloqueia o que existe. Ambas mudam comportamento e não são minhas para
tomar.

### 1. A perda diária conta só o resultado realizado

`record_realized` só é chamado quando uma venda fecha operação. **Uma posição
aberta perdendo 50% não move o contador e nunca dispara o freio.** O robô pode
segurar um prejuízo indefinidamente com a cerca inteira intacta.

A constitution diz "perda máxima diária" sem qualificar. Incluir o não
realizado faz o freio disparar com oscilação normal e fechar o dia cedo demais;
não incluir permite segurar prejuízo sem limite. As duas leituras são
defensáveis.

### 2. `max_total_exposure` hoje não morde

Tamanho de posição e exposição total são calculados sobre a mesma grandeza
(`quantidade × preço`). Com um ativo e uma posição, o limite mais apertado
sempre vence. Na configuração atual (1000 vs 2000), a exposição é decorativa.

Não é defeito: os dois só divergem com múltiplos ativos ou posições
simultâneas, ambos fora do escopo desta feature. Mas hoje você tem, na prática,
um limite e não dois.

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

Medido sobre **dados reais** de doze meses (ver [docs/desempenho.md](docs/desempenho.md)):

| | |
|---|---|
| Coleta | 525.600 velas, 526 páginas, 4m48, pico de 12 MB |
| Backtest | 525.600 velas em **1,03s** (a meta era 60s), 15 MB |
| Conta | resultado reportado × soma do extrato: **divergência zero** em 14.308 operações |
| Auditoria | 143.135 eventos, `seq` sem buraco; 28.633 ordens e 28.633 decisões |
| Determinismo | duas execuções idênticas dígito a dígito |

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
| `.specify/memory/constitution.md` | Governa o projeto |
| `specs/001-nucleo-execucao/` | Spec, plano, pesquisa, contratos, tarefas |
