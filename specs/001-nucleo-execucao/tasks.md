# Tasks: Núcleo de Execução e Risco

**Input**: Design documents from `/specs/001-nucleo-execucao/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: obrigatórios. O Princípio III da constitution é NÃO-NEGOCIÁVEL para lógica
crítica — execução de ordens, gestão de risco e cálculo de posição exigem teste
escrito e **falhando** antes da implementação. Nas tarefas abaixo, todo teste de
lógica crítica precede a implementação correspondente e essa ordem não é negociável.

**Organization**: tarefas agrupadas por história de usuário.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: pode rodar em paralelo (arquivos diferentes, sem dependência pendente)
- **[Story]**: US1 a US5, conforme spec.md
- Caminhos de arquivo são exatos

## Path Conventions

Workspace Rust conforme plan.md: `crates/<nome>/src/`, testes de integração em
`crates/<nome>/tests/`, testes unitários em `#[cfg(test)]` no próprio arquivo.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: erguer o workspace e, antes de qualquer lógica, instalar as travas que
tornam os Princípios II e V verificáveis por máquina.

- [X] T001 Criar `Cargo.toml` do workspace na raiz declarando os 8 membros (`crates/trade-domain`, `trade-ports`, `trade-risk`, `trade-strategy`, `trade-backtest`, `trade-storage`, `trade-bybit`, `trade-cli`) e a seção `[workspace.dependencies]` com as crates do plan.md
- [X] T002 Criar `rust-toolchain.toml` na raiz fixando `channel = "1.98.1"` e componentes `rustfmt`, `clippy` — toolchain flutuante quebraria o determinismo exigido por FR-029
- [X] T003 [P] Criar `.gitignore` na raiz com `/target` e `/data` — os bancos não vão para o repositório
- [X] T004 [P] Criar `rustfmt.toml` na raiz e a seção `[workspace.lints]` no `Cargo.toml` com `unsafe_code = "forbid"` e `clippy::float_arithmetic = "deny"`
- [X] T005 Criar os 8 crates com `cargo new --lib` (e `--bin` para `trade-cli`) e declarar em cada `Cargo.toml` **exatamente** as dependências do grafo do plan.md — `trade-strategy`, `trade-risk` e `trade-backtest` NÃO declaram `trade-bybit` nem `ureq`
- [X] T006 Escrever teste de arquitetura em `tests/architecture.rs` que lê os `Cargo.toml` de `trade-strategy`, `trade-risk` e `trade-backtest` e **falha** se qualquer um declarar `trade-bybit`, `ureq` ou outra dependência de rede
- [X] T007 [P] Escrever teste em `tests/no_float.rs` que varre `crates/trade-domain/src`, `crates/trade-risk/src` e `crates/trade-backtest/src` e **falha** ao encontrar `f32` ou `f64` — ponto flutuante em caminho monetário é o defeito que R-002 existe para impedir
- [X] T008 [P] Criar `.github/workflows/ci.yml` rodando `cargo fmt --check`, `cargo clippy -- -D warnings` e `cargo test --workspace`

**Checkpoint**: o workspace compila vazio, e as travas de arquitetura já falham se alguém violar as fronteiras.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: tipos do domínio, portas e persistência. Nenhuma história pode começar
antes disso.

**⚠️ CRITICAL**: bloqueia todas as histórias.

### Tipos do domínio (`trade-domain`) — lógica crítica, teste antes

- [X] T009 [P] Escrever teste falhando de `Candle::validate` em `crates/trade-domain/src/candle.rs`: rejeita `low > high`, `low > open`, `close > high`, e qualquer preço ≤ 0; aceita `volume` e `turnover` ≥ 0
- [X] T010 [P] Escrever teste falhando de `ExecutionMode` em `crates/trade-domain/src/mode.rs`: `"backtest"` faz parse; `"paper"` e `"live"` retornam erro **distinto** de valor desconhecido; o tipo **não** implementa `Default`
- [X] T011 [P] Escrever teste de propriedade com `proptest` em `crates/trade-domain/src/position.rs`: após qualquer sequência de fills válidos, `Position::qty` nunca fica negativa (SC-010)
- [X] T012 [P] Escrever teste falhando de `Position::apply_fill` em `crates/trade-domain/src/position.rs`: preço médio após compras sucessivas; venda parcial mantém posição; venda total emite `Trade` e zera; venda acima do detido é erro
- [X] T013 [P] Escrever teste falhando de `RunMetrics` em `crates/trade-domain/src/metrics.rs`: `profit_factor` é `None` quando `gross_loss` é zero (nunca infinito); `net_result` igual à soma dos `Trade`; drawdown máximo sobre a curva de capital
- [X] T014 Implementar `Candle` e `Candle::validate` em `crates/trade-domain/src/candle.rs`
- [X] T015 Implementar `ExecutionMode` (sem `Default`, só a variante `Backtest`) em `crates/trade-domain/src/mode.rs`
- [X] T016 Implementar `Position` e `apply_fill` em `crates/trade-domain/src/position.rs`
- [X] T017 Implementar `RunMetrics` e seu cálculo em `crates/trade-domain/src/metrics.rs`
- [X] T018 [P] Implementar `Symbol`, `Interval`, `Side`, `Intent`, `Signal`, `SignalInputs`, `Order`, `OrderId`, `SignalId`, `Fill`, `Trade` em `crates/trade-domain/src/types.rs`, todos com valores monetários em `rust_decimal::Decimal`
- [X] T019 [P] Implementar `RiskLimits`, `RiskState`, `Verdict`, `RiskDecision`, `LimitBreach` (enum exaustivo com `MaxPositionSize`, `MaxTotalExposure`, `DailyLossReached`, `MaxOrdersPerWindow`, `InsufficientBalance`, `SellExceedsHoldings`, `KillSwitchEngaged`) e `Anomaly` em `crates/trade-domain/src/risk_types.rs`

### Portas (`trade-ports`)

- [X] T020 [P] Implementar as traits `MarketDataSource`, `OrderExecutor`, `AccountView`, `Clock`, `AuditSink` e `CandleRepository` em `crates/trade-ports/src/lib.rs`, com as assinaturas normativas de `contracts/ports.md`
- [X] T021 [P] Implementar `MarketError`, `ExecError`, `StorageError` e `AuditError` com `thiserror` em `crates/trade-ports/src/errors.rs` — `ExecError` agnóstico de corretora, sem nenhum código de erro da Bybit (FR-010)
- [X] T022 [P] Implementar os duplos de teste `FakeClock`, `InMemoryAuditSink`, `StubOrderExecutor` e `VecMarketDataSource` em `crates/trade-ports/src/testing.rs`, sob a feature `testing`

### Persistência (`trade-storage`)

- [X] T023 [P] Escrever teste falhando de conversão `Decimal` ↔ `TEXT` em `crates/trade-storage/src/decimal_sql.rs`: ida e volta preserva o valor exato, inclusive com 8 casas decimais
- [X] T024 Implementar a conversão `Decimal` ↔ `TEXT` em `crates/trade-storage/src/decimal_sql.rs` — nunca via `REAL`, que é IEEE-754 e desfaria a exatidão no disco
- [X] T025 Criar `crates/trade-storage/src/schema_market.sql` com as tabelas `dataset`, `candle` (`PRIMARY KEY (symbol, interval, open_ms)`, `WITHOUT ROWID`, preços em `TEXT`) e `gap`, conforme data-model.md
- [X] T026 Criar `crates/trade-storage/src/schema_runs.sql` com as tabelas `run`, `audit_event` (`PRIMARY KEY (run_id, seq)`, `WITHOUT ROWID`), `trade` e `metrics` (`profit_factor` aceita `NULL`), conforme data-model.md
- [X] T027 Implementar abertura de banco e aplicação idempotente do esquema em `crates/trade-storage/src/db.rs`

### Configuração (`trade-cli`)

- [X] T028 [P] Escrever teste falhando em `crates/trade-cli/src/config.rs`: `limits.toml` e `fees.toml` com valores em **string** viram `Decimal` exato; valor numérico no TOML é rejeitado com erro claro
- [X] T029 Implementar a carga de `limits.toml` e `fees.toml` em `crates/trade-cli/src/config.rs` — nenhum limiar embutido no código, conforme constitution v1.2.0

**Checkpoint**: domínio, portas e persistência prontos e testados. As histórias podem começar.

---

## Phase 3: User Story 2 - Impedir que qualquer ordem escape dos limites (Priority: P1)

**Goal**: a camada de risco, funil obrigatório entre estratégia e mercado, com os
quatro limites, kill switch, retomada automática e classificação de anomalia.

**Independent Test**: submeter ordens diretamente ao `RiskGuard` com um
`StubOrderExecutor`, sem nenhum motor de backtest, e verificar que toda tentativa
imprudente é recusada e registrada.

**Por que antes da US1**: FR-018 proíbe qualquer caminho até o mercado fora da camada
de risco. Construir o motor primeiro significaria, por algumas horas, existir um
caminho que a viola — e a constitution não admite isso nem temporariamente. A US2 é
plenamente testável sozinha; a US1 não é construível antes dela.

### Tests for User Story 2 ⚠️ escrever antes, garantir que falham

- [X] T030 [P] [US2] Teste de compilação com `trybuild` em `crates/trade-risk/tests/compile_fail/executor_escape.rs`: tentar obter o executor de dentro do `RiskGuard` **não compila** — FR-018 como propriedade do tipo
- [X] T031 [P] [US2] Teste de tamanho máximo de posição em `crates/trade-risk/tests/limits_size.rs`: valor **exatamente** igual ao limite é aceito; acima é recusado com `MaxPositionSize`; a ordem **não** é reduzida ao teto (FR-019a, FR-020)
- [X] T032 [P] [US2] Teste de exposição máxima total em `crates/trade-risk/tests/limits_exposure.rs`: ordem que levaria a exposição somada acima do limite é recusada com `MaxTotalExposure`
- [X] T033 [P] [US2] Teste de perda máxima diária em `crates/trade-risk/tests/limits_daily_loss.rs`: **atingir** o valor já bloqueia (FR-019a); nenhuma posição nova é aberta; venda que reduz exposição continua permitida (FR-022)
- [X] T034 [P] [US2] Teste de ordens por janela em `crates/trade-risk/tests/limits_rate.rs`: excedentes recusadas com `MaxOrdersPerWindow`
- [X] T035 [P] [US2] Teste de escopo spot comprado em `crates/trade-risk/tests/spot_only.rs`: venda acima do detido recusada com `SellExceedsHoldings`; compra sem saldo recusada com `InsufficientBalance` (FR-005, FR-006)
- [X] T036 [P] [US2] Teste de kill switch em `crates/trade-risk/tests/kill_switch.rs`: acionado, nenhuma ordem é aceita; a posição aberta **é mantida e reportada**, não liquidada (FR-023a); liberar exige chamada explícita
- [X] T037 [P] [US2] Teste de retomada automática em `crates/trade-risk/tests/day_boundary.rs`: bloqueio por perda diária cai em `on_day_boundary` sem nenhum ato humano (FR-022a, SC-014)
- [X] T038 [P] [US2] Teste de classificação de anomalia em `crates/trade-risk/tests/anomaly.rs`: falha transitória retenta até `max_transient_retries` sem interromper; esgotado, vira `Integrity` e exige ato humano; divergência de posição e preço implausível são `Integrity` desde a primeira ocorrência (FR-024a, FR-024b)
- [X] T039 [P] [US2] Teste em `crates/trade-risk/tests/decision_always.rs`: toda chamada a `submit` produz exatamente um `RiskDecision`, aceita ou recusada (SC-002)

### Implementation for User Story 2

- [X] T040 [US2] Implementar `RiskState` (perda do dia, exposição, contagem de ordens na janela) em `crates/trade-risk/src/state.rs`
- [X] T041 [US2] Implementar `RiskGuard` em `crates/trade-risk/src/guard.rs` com `inner: E` **privado e movido**, `limits` **sem setter público**, e `submit` devolvendo `(RiskDecision, Option<Fill>)` sempre
- [X] T042 [US2] Implementar a avaliação dos limites em `crates/trade-risk/src/rules.rs`, com a fronteira de FR-019a explícita em cada regra
- [X] T043 [US2] Implementar `KillSwitch` em `crates/trade-risk/src/kill_switch.rs` por arquivo sentinela cujo caminho é configurável, verificado a cada `submit` — acionável sem acesso ao código e com efeito sobre processo em operação (FR-023)
- [X] T044 [US2] Implementar `on_day_boundary` em `crates/trade-risk/src/guard.rs`, zerando a perda do dia na virada em UTC
- [X] T045 [US2] Implementar `classify` e a política de retentativa em `crates/trade-risk/src/anomaly.rs`
- [X] T046 [US2] Emitir os eventos `risk_decision`, `halt`, `resume` e `anomaly` pelo `AuditSink` em `crates/trade-risk/src/guard.rs`, com causa, classificação e número de tentativas (FR-024c)
- [X] T047 [US2] Implementar a estratégia de teste `reckless` em `crates/trade-strategy/src/reckless.rs`, sob a feature `testing` — tenta posição acima do teto, insiste após estourar a perda diária e tenta alterar os limites
- [X] T048 [US2] Escrever teste de integração em `crates/trade-risk/tests/reckless_contained.rs`: a estratégia imprudente não ultrapassa **nenhum** dos limites em nenhuma tentativa (SC-003)

**Checkpoint**: a camada de risco existe, é testada sozinha e nenhum caminho até um executor a contorna.

---

## Phase 4: User Story 1 - Avaliar uma estratégia sobre histórico de BTC (Priority: P1) 🎯 MVP

**Goal**: o motor de backtest completo — ciclo sinal → risco → ordem → posição sobre
o histórico, com taxas, slippage, métricas e extrato.

**Independent Test**: rodar sobre um conjunto histórico pequeno e conhecido, inserido
direto no SQLite, e conferir resultado e métricas contra cálculo feito à mão.

**Depends on**: Phase 3 (o motor encaminha ordens pelo `RiskGuard`).

### Tests for User Story 1 ⚠️ escrever antes, garantir que falham

- [X] T049 [P] [US1] Teste em `crates/trade-backtest/tests/known_dataset.rs`: sobre um conjunto de ~50 velas com resultado calculado à mão, o motor devolve profit factor, drawdown, número de operações, resultado líquido e extrato idênticos ao esperado
- [X] T050 [P] [US1] Teste de determinismo em `crates/trade-backtest/tests/determinism.rs`: duas execuções com a mesma entrada produzem resultados idênticos dígito a dígito (FR-029, SC-004)
- [X] T051 [P] [US1] Teste em `crates/trade-backtest/tests/no_lookahead.rs`: o preenchimento ocorre na vela **seguinte** ao sinal, nunca na que o originou (FR-030)
- [X] T052 [P] [US1] Teste em `crates/trade-backtest/tests/costs.rs`: taxa e slippage aparecem **discriminados** no `Fill` e no resultado, nunca embutidos no preço (FR-027)
- [X] T053 [P] [US1] Teste em `crates/trade-backtest/tests/gaps.rs`: lacunas no histórico são reportadas, nunca interpoladas como continuidade de preço (FR-031)
- [X] T054 [P] [US1] Teste em `crates/trade-backtest/tests/capital_exhausted.rs`: capital esgotado encerra de forma controlada e reporta o instante exato, sem saldo negativo (FR-032)
- [X] T055 [P] [US1] Teste em `crates/trade-backtest/tests/accounting.rs`: `net_result` coincide com a soma do extrato, divergência zero (SC-009)
- [X] T056 [P] [US1] Teste em `crates/trade-backtest/tests/partial_fill.rs`: volume insuficiente na vela produz preenchimento parcial ou recusa **explícitos** e registrados
- [X] T057 [P] [US1] Teste de CLI em `crates/trade-cli/tests/mode_required.rs`: sem `--mode` falha com código 2 e mensagem de obrigatoriedade; `--mode paper` e `--mode live` falham com "ainda não implementado", **não** com "valor inválido" (FR-001 a FR-003, SC-007)

### Implementation for User Story 1

- [X] T058 [US1] Implementar `SimulatedExecutor` em `crates/trade-backtest/src/executor.rs`: aplica taxa e slippage configurados, resolve preenchimento parcial pelo volume da vela, devolve `Fill` com `fee` e `slippage` discriminados
- [X] T059 [US1] Implementar `BacktestClock` em `crates/trade-backtest/src/clock.rs`, devolvendo o **instante simulado** da vela corrente — nunca o relógio da máquina (R-008)
- [X] T060 [US1] Implementar `BacktestEngine` em `crates/trade-backtest/src/engine.rs`: cursor de velas, ciclo sinal → `RiskGuard` → fill → posição, detecção da virada de dia UTC e chamada a `on_day_boundary`
- [X] T061 [US1] Implementar acumulação de métricas e extrato em `crates/trade-backtest/src/run.rs`
- [X] T062 [P] [US1] Implementar a estratégia de referência `sma-cross` em `crates/trade-strategy/src/sma_cross.rs`, com `SignalInputs` carregando as duas médias e os parâmetros vigentes (FR-034, FR-037)
- [X] T063 [US1] Implementar `SqliteMarketDataSource` em `crates/trade-storage/src/market_source.rs`, percorrendo as velas por cursor — memória constante em relação ao período
- [X] T064 [US1] Implementar a persistência de `run`, `trade` e `metrics` em `crates/trade-storage/src/runs_repo.rs`, gravando `limits_json` e `fees_json` junto (FR-025)
- [X] T065 [US1] Implementar o comando `trade backtest` em `crates/trade-cli/src/cmd_backtest.rs` com `--mode` obrigatório e sem valor padrão
- [X] T066 [US1] Implementar os códigos de saída 0, 2, 3, 4 e 5 em `crates/trade-cli/src/main.rs`, conforme `contracts/cli.md`
- [X] T067 [US1] Implementar a saída formatada em `crates/trade-cli/src/report.rs`, exibindo `indefinido` para profit factor sem operação perdedora, nunca `∞`

**Checkpoint**: 🎯 **MVP completo.** É possível avaliar uma estratégia sobre histórico e obter as métricas da Porta 1, com todo o caminho de ordens sob a camada de risco.

---

## Phase 5: User Story 3 - Obter o histórico de BTC sem coleta manual (Priority: P2)

**Goal**: coletar o histórico dos dados públicos da Bybit e gravá-lo localmente, de
forma retomável e sem exigir credencial.

**Independent Test**: pedir um período conhecido, verificar a gravação local, e então
rodar um backtest sobre ele com a rede desligada.

### Tests for User Story 3 ⚠️

- [X] T068 [P] [US3] Teste em `crates/trade-bybit/tests/ordering.rs`: a página devolvida pela Bybit vem do mais recente para o mais antigo e é **invertida** antes de gravar (R-005)
- [X] T069 [P] [US3] Teste em `crates/trade-bybit/tests/open_candle.rs`: a vela cujo intervalo ainda não terminou é **descartada** — gravá-la quebraria FR-029 e FR-030 em silêncio
- [X] T070 [P] [US3] Teste em `crates/trade-bybit/tests/resume.rs`: repetir uma coleta interrompida completa o que falta e não duplica nada (FR-014)
- [X] T071 [P] [US3] Teste em `crates/trade-bybit/tests/rate_limit.rs`: HTTP 403 com `retCode 10006` é tratado como transitório com recuo exponencial; esgotadas as tentativas vira falha de integridade, sem corromper o já gravado (FR-015)
- [X] T072 [P] [US3] Teste em `crates/trade-bybit/tests/gaps.rs`: períodos sem dados na fonte são detectados e gravados na tabela `gap` (FR-016)
- [X] T073 [P] [US3] Teste em `crates/trade-cli/tests/collect_no_credentials.rs`: o comando `collect` **não expõe** nenhum parâmetro de credencial e não lê variável de ambiente de segredo (FR-012)

### Implementation for User Story 3

- [X] T074 [US3] Implementar o cliente HTTP com `ureq` em `crates/trade-bybit/src/client.rs`, chamando `GET /v5/market/kline` com `category=spot` **explícito** — omitir traria perpétuo, contra FR-004
- [X] T075 [US3] Implementar o parse da resposta V5 em `crates/trade-bybit/src/parse.rs`: os 7 elementos na ordem `startTime, open, high, low, close, volume, turnover`, convertendo as strings direto para `Decimal` sem passar por float
- [X] T076 [US3] Implementar a paginação para frente por `start` com `limit=1000` em `crates/trade-bybit/src/collector.rs`
- [X] T077 [US3] Implementar o descarte da vela em formação em `crates/trade-bybit/src/collector.rs`
- [X] T078 [US3] Implementar o recuo exponencial e a leitura dos cabeçalhos `X-Bapi-Limit`, `X-Bapi-Limit-Status` e `X-Bapi-Limit-Reset-Timestamp` em `crates/trade-bybit/src/backoff.rs`
- [X] T079 [US3] Implementar a tradução de erros da Bybit para `ExecError`/`MarketError` agnósticos em `crates/trade-bybit/src/errors.rs` — nenhum código da corretora vaza para fora desta crate (FR-010)
- [X] T080 [US3] Implementar `SqliteCandleRepository` em `crates/trade-storage/src/candle_repo.rs`: `upsert_page` com `INSERT OR IGNORE` **em uma transação por página**, mais `coverage` e `record_gaps`
- [X] T081 [US3] Implementar a detecção de lacunas em `crates/trade-bybit/src/gaps.rs`
- [X] T082 [US3] Implementar o comando `trade collect` em `crates/trade-cli/src/cmd_collect.rs`, gravando a procedência (fonte, par, granularidade, período, instante) na tabela `dataset` (FR-017)
- [X] T083 [US3] Escrever teste de integração em `crates/trade-bybit/tests/local_server.rs` contra um servidor HTTP local que reproduz o formato da resposta V5

**Checkpoint**: histórico coletável em um comando, retomável, sem credencial.

---

## Phase 6: User Story 4 - Reconstituir o que o robô fez e por quê (Priority: P2)

**Goal**: auditoria persistente e consultável, suficiente para reconstruir qualquer
decisão sem reexecutar nada.

**Independent Test**: escolher uma operação do extrato e chegar, só com SQL, até os
dados de entrada que produziram o sinal que a originou.

### Tests for User Story 4 ⚠️

- [ ] T084 [P] [US4] Teste em `crates/trade-storage/tests/audit_envelope.rs`: todo evento carrega `run_id`, `seq`, `at` em UTC e `mode`, sem exceção (FR-033)
- [ ] T085 [P] [US4] Teste em `crates/trade-storage/tests/audit_signal_inputs.rs`: o evento `signal` carrega os `inputs` que o produziram, não apenas a intenção (FR-034)
- [ ] T086 [P] [US4] Teste em `crates/trade-storage/tests/audit_seq.rs`: `seq` é ordem total dentro da execução, sem buraco e sem repetição, inclusive entre eventos do mesmo instante simulado
- [ ] T087 [P] [US4] Teste em `crates/trade-storage/tests/audit_no_json_numbers.rs`: todo valor monetário é serializado como **string**, nunca como número JSON — número JSON é IEEE-754 na maioria dos leitores
- [ ] T088 [P] [US4] Teste em `crates/trade-storage/tests/audit_no_secrets.rs`: nenhum campo de nenhum evento contém chave, segredo ou token (FR-036)
- [ ] T089 [P] [US4] Teste em `crates/trade-risk/tests/no_auto_resume.rs`: nenhum `resume` com `automatic: true` sucede um `halt` com `requires_human: true` (SC-015)

### Implementation for User Story 4

- [ ] T090 [US4] Implementar a serialização das variantes de `AuditEvent` em `crates/trade-domain/src/audit.rs`, conforme `contracts/audit-event.md`
- [ ] T091 [US4] Implementar `SqliteAuditSink` em `crates/trade-storage/src/audit_sink.rs`, com escrita em lote transacional e `flush` explícito
- [ ] T092 [US4] Ligar o `SqliteAuditSink` ao motor e ao `RiskGuard` no ponto de composição em `crates/trade-cli/src/wiring.rs`
- [ ] T093 [US4] Escrever teste de reconstituição em `crates/trade-storage/tests/reconstitute.rs`: partindo de uma operação do extrato, alcançar `fill` → `order` → `signal` → `inputs` **só por consulta SQL**, sem reexecutar nada (SC-005)

**Checkpoint**: qualquer decisão do robô é explicável a partir do registro.

---

## Phase 7: User Story 5 - Trocar a fonte de mercado sem tocar em estratégia ou risco (Priority: P3)

**Goal**: provar que a abstração de corretora segura o peso — a mesma estratégia roda
contra provedores diferentes sem alteração.

**Independent Test**: executar a mesma estratégia, sem modificá-la, contra dois
provedores de mercado distintos.

### Tests for User Story 5 ⚠️

- [ ] T094 [P] [US5] Teste em `crates/trade-backtest/tests/provider_swap.rs`: a mesma instância de `sma-cross`, sem alteração, completa o ciclo contra `SqliteMarketDataSource` e contra `VecMarketDataSource` (SC-006)
- [ ] T095 [P] [US5] Teste em `crates/trade-backtest/tests/uniform_failure.rs`: falha de provedor chega à estratégia como o mesmo `ExecError`, independentemente de qual provedor falhou (FR-010)
- [ ] T096 [P] [US5] Teste em `crates/trade-strategy/tests/no_exchange_reference.rs`: o código de `trade-strategy` e `trade-risk` não contém nenhuma referência textual a corretora específica

### Implementation for User Story 5

- [ ] T097 [US5] Implementar a seleção de provedor no ponto de composição em `crates/trade-cli/src/wiring.rs` — o único lugar do sistema que conhece as duas pontas
- [ ] T098 [US5] Estender `tests/architecture.rs` para rodar no CI como portão de merge, falhando o build se o grafo de crates for violado

**Checkpoint**: todas as histórias funcionam de forma independente.

---

## Phase 8: Polish & Cross-Cutting Concerns

- [ ] T099 [P] Escrever `README.md` na raiz com instalação, os dois comandos e um exemplo de ponta a ponta
- [ ] T100 [P] Versionar `examples/limits.toml` e `examples/fees.toml` com os valores de partida provisórios da constitution v1.2.0, marcados como tal
- [ ] T101 Executar o roteiro completo de `specs/001-nucleo-execucao/quickstart.md`, cenários A a J, e registrar o resultado de cada um
- [ ] T102 Medir o backtest de 12 meses de velas de 1 minuto e registrar o tempo em `docs/desempenho.md`, confirmando execução abaixo de 60 segundos
- [ ] T103 Comparar o uso de memória entre um backtest de 5 dias e um de 5 anos e registrar em `docs/desempenho.md`, confirmando que é constante em relação ao período
- [ ] T104 [P] Documentar em `docs/auditoria.md` as consultas SQL de reconstituição e de conferência, prontas para colar no DBeaver
- [ ] T105 Revisar a superfície pública de `crates/trade-ports/src/lib.rs`, `crates/trade-risk/src/guard.rs` e `crates/trade-domain/src/lib.rs`, reduzindo ao mínimo o que é `pub` — cada item público é uma porta a mais para contornar uma fronteira

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: sem dependências
- **Foundational (Phase 2)**: depende da Phase 1 — **bloqueia todas as histórias**
- **US2 (Phase 3)**: depende da Phase 2
- **US1 (Phase 4)**: depende da Phase 3 — **única dependência entre histórias**, explicada abaixo
- **US3 (Phase 5)**: depende da Phase 2 apenas — pode correr em paralelo com US1 e US2
- **US4 (Phase 6)**: depende da Phase 2; o teste T093 precisa de uma execução real, portanto da US1
- **US5 (Phase 7)**: depende da US1 (precisa de um ciclo completo para trocar o provedor)
- **Polish (Phase 8)**: depende de todas

### A dependência entre US1 e US2

É a única quebra de independência entre histórias, e é deliberada. FR-018 proíbe
qualquer caminho até o mercado fora da camada de risco. Construir o motor antes do
`RiskGuard` criaria, ainda que por algumas horas, exatamente o caminho que o Princípio
II proíbe. Inverter a ordem custa nada: a US2 é plenamente testável sozinha, com
ordens sintéticas e um executor de mentira, sem motor nenhum.

### Within Each User Story

- Testes de lógica crítica **escritos e falhando** antes da implementação — Princípio III
- Tipos antes de serviços; serviços antes de comandos
- História completa e verificada antes da seguinte

### Parallel Opportunities

- Setup: T003, T004, T007 e T008 em paralelo
- Foundational: T009 a T013 em paralelo (arquivos distintos); T018 a T023 em paralelo
- US2: T030 a T039 em paralelo — são dez arquivos de teste independentes
- US1: T049 a T057 em paralelo
- US3 pode correr inteira em paralelo com US1 e US2 depois da Phase 2, por não compartilhar arquivo com elas
- US3, US4 e US5 têm seus blocos de teste paralelizáveis

---

## Parallel Example: User Story 2

```bash
# Os dez testes da camada de risco, escritos juntos — arquivos distintos:
Task: "Teste de tamanho máximo em crates/trade-risk/tests/limits_size.rs"
Task: "Teste de exposição total em crates/trade-risk/tests/limits_exposure.rs"
Task: "Teste de perda diária em crates/trade-risk/tests/limits_daily_loss.rs"
Task: "Teste de ordens por janela em crates/trade-risk/tests/limits_rate.rs"
Task: "Teste de escopo spot comprado em crates/trade-risk/tests/spot_only.rs"
Task: "Teste de kill switch em crates/trade-risk/tests/kill_switch.rs"
Task: "Teste de retomada automática em crates/trade-risk/tests/day_boundary.rs"
Task: "Teste de classificação de anomalia em crates/trade-risk/tests/anomaly.rs"
```

---

## Implementation Strategy

### MVP: Phases 1 a 4

O MVP são **duas** histórias, não uma — US2 e US1, ambas P1. A camada de risco sem o
motor não avalia estratégia nenhuma; o motor sem a camada de risco viola o Princípio
II. Juntas entregam o que a feature promete: avaliar uma estratégia sobre histórico,
com todo o caminho de ordens sob a cerca.

1. Phase 1 — Setup, incluindo as travas de arquitetura
2. Phase 2 — Foundational
3. Phase 3 — US2, camada de risco
4. Phase 4 — US1, motor de backtest
5. **PARAR E VALIDAR** — cenários A, D, F, G, H e I do quickstart

Nesse ponto ainda é preciso inserir histórico manualmente no SQLite para rodar. É
suficiente para provar o núcleo; a US3 remove esse trabalho manual.

### Entrega incremental

1. Setup + Foundational → base pronta
2. + US2 → a cerca existe e contém uma estratégia imprudente
3. + US1 → 🎯 MVP: estratégia avaliável sobre histórico
4. + US3 → histórico em um comando, 12 meses sem trabalho manual
5. + US4 → toda decisão explicável por SQL
6. + US5 → abstração provada, caminho aberto para paper trading

### Ordem de commits e o Princípio III

O Princípio III é verificável no histórico do git: em lógica crítica, o commit do
teste precede o commit da implementação, e o teste falha no commit em que nasce. Isso
não é burocracia — é a única evidência posterior de que o teste foi escrito antes e
não ajustado depois para passar.

---

## Notes

- `[P]` = arquivos diferentes, sem dependência pendente
- Confirmar que o teste **falha** antes de implementar; um teste que passa recém-escrito não testa nada
- Commitar a cada tarefa ou grupo lógico
- Parar em qualquer checkpoint para validar a história isoladamente
- Os limiares numéricos vivem em `limits.toml`, nunca no código (constitution v1.2.0)
