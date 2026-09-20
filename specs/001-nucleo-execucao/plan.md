# Implementation Plan: Núcleo de Execução e Risco

**Branch**: `001-nucleo-execucao` | **Date**: 2026-09-20 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/001-nucleo-execucao/spec.md`

## Summary

Construir o esqueleto que a constitution exige — abstração de corretora, camada de
risco independente e motor de backtest — como um **workspace Rust de oito crates**,
em que o grafo de dependências entre crates torna impossível, em tempo de compilação,
que a estratégia alcance a corretora ou contorne a camada de risco.

A escolha central do desenho: **a estratégia não envia ordens**. Ela recebe o estado
do mercado e devolve um sinal. Quem converte sinal em ordem é o motor, e o caminho
do motor até a corretora atravessa obrigatoriamente a camada de risco. Não existe
"caminho alternativo" a proibir por disciplina (FR-018) porque não existe tipo que o
permita — a estratégia não tem, e não pode obter, uma referência ao executor.

Histórico de BTC coletado dos dados públicos da Bybit e gravado em SQLite; backtest
consome apenas o banco local e nunca toca a rede.

## Technical Context

**Language/Version**: Rust estável, edition 2024. Versão exata fixada em
`rust-toolchain.toml` na primeira compilação — determinismo de build é requisito
(FR-029), então a toolchain não pode flutuar.

**Primary Dependencies**:

| Crate | Para quê | Por que esta |
|---|---|---|
| `rust_decimal` | preço, quantidade, saldo, P&L | ponto flutuante é inaceitável em cálculo monetário; erro de arredondamento em `f64` aparece como divergência de posição, não como exceção |
| `rusqlite` (feature `bundled`) | persistência | SQLite embutido no binário, sem dependência de biblioteca do sistema |
| `serde` + `serde_json` | configuração e auditoria | registro legível por máquina (FR-035) |
| `clap` (derive) | CLI | modo de execução como argumento obrigatório (FR-001) |
| `chrono` | tempo em UTC | todo registro e a fronteira do dia são UTC (FR-033) |
| `thiserror` | tipos de erro | falha uniforme do provedor (FR-010) |
| `toml` | arquivos de limites e taxas | configuração legível e editável à mão; valores como string, parseados direto para `Decimal` |
| `ulid` | identificador de execução | ordenável por tempo, ao contrário de UUID v4 — listar execuções em ordem cronológica vira `ORDER BY run_id` |
| `ureq` | HTTP da coleta | cliente bloqueante e enxuto; evita trazer runtime assíncrono para um sistema que hoje é inteiramente síncrono |
| `rstest` (dev) | testes parametrizados | os limites de risco pedem tabela de casos |
| `proptest` (dev) | testes de propriedade | invariantes como "quantidade detida nunca fica negativa" (SC-010) são mais bem expressas como propriedade que como exemplo |

Versões são fixadas em `Cargo.lock` na implementação, não aqui.

**Storage**: SQLite, em dois arquivos com ciclos de vida distintos:

- `data/market.db` — histórico coletado. É cache: pode ser reconstruído da fonte.
- `data/runs.db` — execuções, auditoria, operações e métricas. É insubstituível.

A separação existe porque apagar o cache é rotina e apagar a auditoria é perda
irreversível (Princípio IV). Ambos ficam em `data/`, ignorado pelo git.

**Testing**: `cargo test` (harness nativo), com `rstest` para tabelas de caso e
`proptest` para invariantes. Nenhum teste de domínio, risco ou backtest toca a rede
(SC-008) — a coleta é a única fronteira de rede, e seus testes usam servidor local.

**Target Platform**: Linux x86-64. Binário único, sem runtime externo.

**Project Type**: workspace Rust — biblioteca em crates + um binário CLI.

**Performance Goals**: backtest de 12 meses de velas de 1 minuto (~525.600 velas) em
menos de 60 segundos. Não é requisito de mercado, é requisito de iteração: avaliar
uma estratégia precisa ser barato o bastante para se fazer muitas vezes ao dia.

**Constraints**:

- **Memória constante em relação ao período**: as velas são percorridas em cursor
  sobre o SQLite, nunca carregadas inteiras na memória. Backtest de 5 anos deve usar
  a mesma memória que um de 5 dias.
- **Backtest sem rede**: o motor não tem, em nenhuma crate de que depende, um cliente
  HTTP. Não é disciplina, é ausência de dependência (SC-011).
- **Determinismo**: mesma entrada, mesmo resultado, bit a bit (FR-029). Sem
  paralelismo não determinístico, sem iteração sobre `HashMap` em caminho que afete
  resultado, sem relógio do sistema — o tempo vem do `Clock` injetado.

**Scale/Scope**: um par (BTC), um mercado (spot), um modo (`backtest`). Cerca de
8 crates e uma estratégia de referência.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Princípio | Como o desenho satisfaz | Verificável por |
|---|---|---|
| **I. Validação antes de capital real** | `ExecutionMode` é enum sem `Default`, exigido pela CLI. Só a variante `Backtest` existe; `paper` e `live` são recusados com mensagem explícita | teste de CLI sem `--mode`; teste com `--mode live` |
| **II. Limites invioláveis** | A estratégia devolve `Signal` e não tem acesso a `OrderExecutor`. O motor encaminha toda ordem por `RiskGuard`, que **possui** o executor interno — não há como obter o executor sem atravessar o guard | grafo de crates; `trade-strategy` não depende de `trade-ports::OrderExecutor` |
| **III. Test-first na lógica crítica** | `trade-domain`, `trade-risk` e o cálculo de posição do `trade-backtest` são lógica crítica: teste escrito e falhando antes | ordem dos commits; `tasks.md` gera os testes antes da implementação |
| **IV. Auditabilidade** | `AuditSink` é porta obrigatória do motor. Todo sinal, ordem, decisão e preenchimento vira evento com `run_id`, `seq`, instante UTC e modo | consulta SQL em `runs.db` reconstrói qualquer operação |
| **V. Independência de corretora** | `trade-bybit` não é dependência de `trade-strategy`, `trade-risk` nem `trade-backtest`. Só o binário conhece as duas pontas | `cargo tree`; teste que roda a mesma estratégia contra dois provedores |
| **VI. Segurança de credenciais** | A coleta usa endpoint público e não recebe credencial. Nenhuma crate lê variável de ambiente de segredo nesta feature | ausência de qualquer leitura de credencial no código |

**Resultado do gate: PASSA.** Nenhuma violação a justificar. A escolha de dividir em
oito crates é registrada em *Complexity Tracking* por ser complexidade adicionada
conscientemente, ainda que a serviço de dois princípios.

## Project Structure

### Documentation (this feature)

```text
specs/001-nucleo-execucao/
├── plan.md              # Este arquivo
├── research.md          # Fase 0
├── data-model.md        # Fase 1
├── quickstart.md        # Fase 1
├── contracts/           # Fase 1
│   ├── ports.md         # As traits que definem as fronteiras
│   ├── cli.md           # Contrato da linha de comando
│   └── audit-event.md   # Esquema do evento de auditoria
├── checklists/
│   └── requirements.md
└── tasks.md             # Fase 2 — criado por /speckit-tasks
```

### Source Code (repository root)

```text
Cargo.toml                  # workspace
rust-toolchain.toml         # toolchain fixada
crates/
├── trade-domain/           # tipos puros; nenhuma E/S, nenhuma dependência de infra
│   └── src/                #   Money, Qty, Candle, Signal, Order, Fill, Position,
│                           #   Trade, ExecutionMode, RiskLimits
├── trade-ports/            # as fronteiras, como traits
│   └── src/                #   MarketDataSource, OrderExecutor, AccountView,
│                           #   Clock, AuditSink, CandleRepository
├── trade-risk/             # camada de risco; possui o executor interno
│   └── src/                #   RiskGuard, avaliação de limites, kill switch,
│                           #   classificação de anomalia e retentativa
├── trade-strategy/         # trait Strategy + estratégia de referência
│   └── src/                #   NÃO depende de OrderExecutor — só devolve Signal
├── trade-backtest/         # motor: cursor de velas, ciclo, simulação de fill
│   └── src/                #   taxas, slippage, métricas, extrato
├── trade-storage/          # SQLite: candles, datasets, runs, auditoria
│   └── src/                #   implementa CandleRepository e AuditSink
├── trade-bybit/            # coleta do histórico público; única crate com HTTP
│   └── src/
└── trade-cli/              # binário `trade`: comandos collect e backtest
    └── src/

data/                       # market.db e runs.db — ignorado pelo git
tests/                      # testes de integração que cruzam crates
```

**Structure Decision**: workspace de oito crates em vez de um único crate com
módulos. A razão é que **módulo não impede dependência, crate impede**. Um `mod
strategy` dentro de um crate único pode, a qualquer momento, chamar o executor de
ordens — só uma revisão humana atenta pegaria. Como crate separado, `trade-strategy`
simplesmente não declara `trade-bybit` em seu `Cargo.toml`, e a tentativa não
compila. Os Princípios II e V deixam de depender de disciplina e passam a depender do
compilador. É o argumento mais forte a favor do Rust neste projeto, e o desenho é
construído para cobrá-lo.

O binário `trade-cli` é o único lugar onde a ponta concreta (Bybit, SQLite) encontra
a lógica abstrata — o ponto de composição.

## Post-Design Constitution Re-Check

*Reavaliação após Fase 1, conforme exigido pelo gate.*

O desenho detalhado **reforçou** os gates em vez de afrouxá-los. Três pontos em que
artefatos da Fase 1 tornaram um princípio mais verificável do que era no gate inicial:

| Princípio | O que a Fase 1 acrescentou |
|---|---|
| **II** | `RiskGuard::submit` devolve `RiskDecision` **sempre**, mesmo em recusa. Com isso, SC-002 (toda ordem com decisão registrada) deixa de ser disciplina e vira consequência do tipo de retorno — não existe caminho que produza `Fill` sem produzir a decisão |
| **IV** | O campo `seq` em `audit_event` dá **ordem total** aos eventos dentro de uma execução. Sem ele, dois eventos na mesma vela ficariam sem ordem definida e a cadeia de reconstituição teria um elo ambíguo |
| **I** | `ExecutionMode` sem `Default` transforma FR-001 em erro de compilação para quem tentar um valor padrão, não em verificação de runtime |

Uma descoberta da Fase 0 alterou o desenho do coletor e merece registro no gate: a
Bybit documenta que o preço de fechamento da vela corrente *"is the last traded price
when the candle is not closed"*. Gravar essa vela quebraria FR-029 (duas coletas do
mesmo período produziriam históricos diferentes) e FR-030 (o backtest enxergaria um
fechamento que ainda não existia no instante simulado). O coletor descarta toda vela
cujo intervalo não tenha terminado — regra que não estava no gate inicial porque
dependia de conhecer a fonte.

**Resultado da reavaliação: PASSA.** Nenhuma violação nova, nenhuma justificativa
adicional necessária além das já registradas abaixo.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| 8 crates em vez de 1 crate com módulos | Os Princípios II e V exigem que estratégia e risco não alcancem a corretora. Com crates, a proibição é verificada pelo compilador e por `cargo tree` | Módulos dentro de um crate compartilham visibilidade de crate: `pub(crate)` vaza entre módulos, e nada impede a estratégia de importar o executor. A proibição viraria convenção, e convenção não é garantia |
| Dois arquivos SQLite em vez de um | Histórico é cache reconstruível; auditoria é insubstituível. Ciclos de vida e políticas de descarte diferentes | Arquivo único faria "apagar o cache" e "apagar a auditoria" serem a mesma operação, contra o Princípio IV |
| `rust_decimal` em vez de tipos nativos | Erro de arredondamento em `f64` se manifesta como divergência de posição — exatamente a anomalia de integridade que a constitution manda tratar como parada | `f64` é mais rápido e não requer dependência, mas troca correção por desempenho numa feature cuja finalidade é medir dinheiro corretamente |
