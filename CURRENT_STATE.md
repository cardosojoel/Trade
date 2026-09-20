# Estado atual do projeto

**Projeto:** Trade — robô de day trade automatizado de Bitcoin
**Atualizado em:** 2026-09-20
**Repositório:** https://github.com/cardosojoel/Trade (privado)

---

## Em uma frase

O núcleo está pronto e funcionando: `trade backtest` avalia uma estratégia
sobre histórico de BTC com toda ordem passando por uma camada de risco que a
estratégia não consegue contornar. Falta a coleta de histórico, a persistência
da auditoria e o polimento.

**67 de 105 tarefas · 147 testes verdes · 4.879 linhas de código, 2.510 de teste**

---

## Decisões que definem o projeto

| | |
|---|---|
| **Domínio** | Robô de day trade de Bitcoin |
| **Mercado** | Spot, **apenas comprado** — sem alavancagem, sem venda a descoberto |
| **Corretora** | Bybit (histórico público, sem credencial nesta fase) |
| **Autonomia** | Total dentro dos limites; sem aprovação humana por ordem |
| **Linguagem** | Rust estável 1.98.1, edition 2024 |
| **Armazenamento** | SQLite em dois arquivos: `market.db` (cache) e `runs.db` (auditoria) |
| **Testes** | TDD obrigatório na lógica crítica — ordens, risco, posição e P&L |

A constitution (`.specify/memory/constitution.md`, **v1.2.0**) governa tudo e tem
precedência sobre qualquer outra prática.

---

## O que funciona hoje

```bash
trade backtest --mode backtest --symbol BTCUSDT --interval 1m \
  --from 2026-01-01 --to 2026-01-04 --capital 10000 \
  --strategy sma-cross --strategy-params fast=9,slow=21 \
  --limits examples/limits.toml --fees examples/fees.toml
```

Saída de uma execução real sobre 3.000 velas:

```
  Capital inicial            10000
  Resultado líquido         -75.09     -0.8%
  Taxas                     113.71
  Slippage                   57.35

  Operações                     57
  Profit factor               0.48
  Drawdown máximo            75.09     +0.8%

  Ordens recusadas              10   (MaxPositionSize 10)
  Velas percorridas           3000
```

As 10 recusas são a cerca funcionando **e aparecendo**: a estratégia pediu 10%
do saldo com posição já aberta e o teto barrou.

Também existe `trade kill` para acionar e liberar o kill switch.

---

## As três garantias estruturais

Não são convenção nem disciplina — o build recusa a violação. Verifiquei as três
violando de propósito e confirmando que falham.

| Garantia | Onde | O que acontece se for violada |
|---|---|---|
| Estratégia, risco e backtest não alcançam a corretora nem a rede | `tests/architecture.rs` | Teste falha, CI barra o merge |
| Nenhum `f32`/`f64` em caminho monetário | `tests/no_float.rs` | Teste falha, CI barra o merge |
| Obter o executor de dentro do `RiskGuard` | `crates/trade-risk/tests/compile_fail/` | **Não compila** |

`trade-strategy` não declara `trade-ports` como dependência e por isso não
consegue sequer **nomear** `OrderExecutor`. A estratégia devolve `Signal`; quem
converte em ordem é o motor, e o caminho do motor até o mercado atravessa o
`RiskGuard`, que **possui** o executor.

---

## Estrutura

```
crates/
├── trade-domain/      tipos puros, sem E/S            33 testes
├── trade-ports/       as traits de fronteira
├── trade-risk/        camada de risco                 51 testes
├── trade-strategy/    sma-cross e reckless             5 testes
├── trade-backtest/    o motor                         22 testes
├── trade-storage/     SQLite                          18 testes
├── trade-bybit/       coleta (vazio — Fase 5)
└── trade-cli/         binário `trade`                 13 testes
tests/                 travas de arquitetura            4 testes
```

---

## O que falta

| Fase | História | Tarefas |
|---|---|---|
| 5 | **US3** — coleta do histórico da Bybit | 0/16 |
| 6 | **US4** — auditoria persistente | 0/10 |
| 7 | **US5** — troca de provedor | 0/5 |
| 8 | Polish, README e quickstart completo | 0/7 |

Fases 1 a 4 concluídas (67/67).

---

## Pendências e decisões em aberto

### 🔴 Auditoria não persiste

O `AuditSink` em uso descarta os eventos (`trade-storage/src/audit_noop.rs`).
**O Princípio IV não está cumprido em execução real.** Não é configurável — não
há bandeira que o selecione — e o arquivo sai quando a US4 chegar.

### 🟡 A perda diária conta só o resultado realizado

`record_realized` só é chamado quando uma venda fecha operação. Uma posição
aberta perdendo 50% não move o contador e **nunca dispara o freio**.

A constitution diz "perda máxima diária" sem qualificar. Incluir o não realizado
faria o freio disparar com oscilação normal e fechar o dia cedo demais; não
incluir permite segurar prejuízo indefinidamente. **Decisão pendente do
mantenedor.**

### 🟡 `max_total_exposure` hoje não morde

Tamanho de posição e exposição total são calculados sobre a mesma grandeza
(`quantidade × preço`). Com um ativo e uma posição, o limite mais apertado
sempre vence. Na configuração atual (1000 vs 2000), a exposição é decorativa.

Não é defeito: os dois só divergem com múltiplos ativos ou posições
simultâneas, ambos fora do escopo desta feature.

### 🟡 Os limiares ainda não foram calibrados

Perda diária 2%, posição 10%, exposição 20%, profit factor 1.3, drawdown 15% —
todos **propostos pelo assistente e aceitos como valores de partida
provisórios**, não medidos. Vivem em `limits.toml`, nunca no código.

A constitution **exige** que sejam revistos contra o capital real antes da
Porta 3, a liberação para capital real.

### 🟠 `git push` bloqueado

8 commits locais aguardando. O token do `gh` tem `gist`, `read:org` e `repo`;
falta `workflow`, necessário desde que o CI entrou no repositório.

```
gh auth refresh -h github.com -s workflow
```

---

## Defeitos encontrados e corrigidos

Dois, ambos só visíveis rodando de verdade — e ambos com teste de regressão.

**Divergência na soma do extrato.** `net_result` e a soma das operações
divergiam em 9×10⁻²⁷ quando a soma era refeita em SQL. Os testes em Rust
comparavam duas contas feitas com a mesma aritmética e concordavam sempre. A
causa: quantidades vindas de divisão carregavam 28 dígitos — `0,01709352543…`
BTC é uma ordem que corretora nenhuma aceita. Corrigido truncando quantidade,
taxa, slippage e resultado a 8 casas, a precisão do satoshi. Divergência agora
é **zero**, verificada em SQL.

**Saldo estourado.** A cerca avaliava o custo de uma compra como
`quantidade × preço de referência`, mas o débito real inclui slippage e taxa.
Capital de 1000 terminava em −15,05. Corrigido avaliando o custo de pior caso
e descontando o custo de transação no dimensionamento padrão.

A lição das duas: **verificação por fora, com outra aritmética, pega o que
teste interno não pega.**

---

## Portões de qualidade

```bash
cargo test --workspace --all-features                              # 147 verdes
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
```

O CI roda as travas de arquitetura **primeiro e isoladas**: se uma delas falha,
o problema é arquitetural e não adianta saber se o resto passou.

---

## Próximo passo

Fase 5 — coleta do histórico da Bybit (T068–T083). Fatos já verificados da API
que moldam o coletor:

- `GET /v5/market/kline`, **público, sem autenticação**
- `category` assume `linear` se omitido — `spot` é obrigatório
- `limit` máximo 1000; 12 meses de velas de 1 minuto ≈ 526 requisições
- Lista vem do **mais recente para o mais antigo**; o coletor inverte
- Preços como **string**, convertidos direto para `Decimal`
- **`closePrice` é o último preço negociado enquanto a vela não fechou** — a
  vela em formação é descartada, ou o determinismo quebra em silêncio
- Limite de 600 requisições por 5 segundos por IP; `retCode 10006` + HTTP 403

---

## Documentos do projeto

| Arquivo | O que é |
|---|---|
| `.specify/memory/constitution.md` | Governa tudo. v1.2.0 |
| `specs/001-nucleo-execucao/spec.md` | O que o núcleo faz, 46 requisitos |
| `specs/001-nucleo-execucao/plan.md` | Como, e por que assim |
| `specs/001-nucleo-execucao/research.md` | As 9 decisões técnicas com alternativas |
| `specs/001-nucleo-execucao/tasks.md` | As 105 tarefas e o que já foi feito |
| `specs/001-nucleo-execucao/quickstart.md` | Roteiro de validação, cenários A–J |
| `CLAUDE.md` | Diretrizes de trabalho e invariantes do repositório |
