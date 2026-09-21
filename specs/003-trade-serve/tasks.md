# Tasks: Servidor local de leitura do registro (`trade serve`)

**Spec**: [spec.md](spec.md) · **Plano**: [plan.md](plan.md) · **Pesquisa**: [research.md](research.md)

**(TF)** marca o que a constitution exige com teste escrito e **falhando** antes
da implementação.

Nenhuma tarefa aqui exige credencial ou rede: o servidor é local e os dois
bancos são arquivos. Diferente da feature 002, **nada espera ato do dono**.

## Fatia 1 — o servidor existe e recusa

As recusas vêm antes das rotas de dado de propósito: são o que mais custa
errar, e são o que o `SC-003` ao `SC-007` cobram.

- [X] T001 Crate `trade-serve` criada, declarada no workspace, e na lista `ISOLATED` de `tests/architecture.rs`
- [X] T002 **(TF)** O teste de arquitetura falha se `trade-serve` declarar corretora, rede, risco, backtest ou a própria CLI
- [X] T003 **(TF)** Escuta em `127.0.0.1` com porta explícita; o endereço vem de `Ipv4Addr::LOCALHOST`, nunca de texto (FR-003, SC-007)
- [X] T004 **(TF)** Corpo único de recusa, com código próprio, igual em toda rota (FR-023)
- [X] T005 **(TF)** Token local exigido na rota de escrita, recusando **antes** de qualquer trabalho (FR-019, SC-005)
- [X] T006 **(TF)** `Origin` ou `Referer` de outro lugar é recusado, mesmo com token válido (FR-019, SC-006)
- [X] T007 **(TF)** Caminho desconhecido e método errado usam o mesmo corpo de recusa

## Fatia 2 — leitura do que já rodou

Cada tarefa traz consulta nova em `trade-storage`: o lado de leitura do
registro não existe hoje (pesquisa, item 1).

- [ ] T008 **(TF)** `trade-storage` lê execuções: cabeçalho, cerca e taxas, a partir do `runs_json`
- [ ] T009 **(TF)** `trade-storage` lê métricas e extrato de uma execução
- [ ] T010 **(TF)** Todo valor monetário sai como **string** JSON; nenhum como número (FR-002, SC-002)
- [ ] T011 **(TF)** A versão do código vem **presente e nula** em toda resposta que descreva execução (FR-006, SC-009)
- [ ] T012 `GET /runs` — grupos de execuções, com cerca e comparabilidade
- [ ] T013 `GET /runs/{id}` — métricas, decomposição do custo, estado da Porta 1
- [ ] T014 **(TF)** As cinco exigências da Porta 1 vêm de configuração, nunca de constante embutida (FR-024)
- [ ] T015 **(TF)** `profit_factor` indefinido vem `null` com o motivo, nunca zero, infinito ou texto (FR-021)
- [ ] T016 **(TF)** `GET /runs/{id}/daily` — P&L por dia, pior dia, dias que romperam o limite, curva acumulada
- [ ] T017 **(TF)** Todo agregado é calculado em decimal exato no servidor (FR-008)

## Fatia 3 — reconstituição

- [ ] T018 **(TF)** `trade-storage` lê a linha do tempo de uma execução, ordenada por `seq` e nunca por `at_ms` (FR-012)
- [ ] T019 **(TF)** `GET /runs/{id}/episodes` — episódios com entrada, fechamento, número de saídas e resultado somado (FR-009)
- [ ] T020 **(TF)** Cada episódio traz `fechado_por`; presente e nulo nos gravados antes do campo existir (FR-010)
- [ ] T021 **(TF)** `GET /runs/{id}/episodes/{seq}/saidas` — cada linha com a taxa ao lado do resultado (FR-011)
- [ ] T022 **(TF)** `GET /runs/{id}/chain/{seq}` — os cinco elos de uma decisão
- [ ] T023 **(TF)** O elo `risk_decision` traz, para cada limite, o valor observado ao lado do teto (FR-013)
- [ ] T024 **(TF)** Divergência entre pedido e preenchido é assinalada no elo `fill`, com a proporção (FR-014)
- [ ] T025 **(TF)** Toda coleção declara teto e diz quando cortou (FR-025)

## Fatia 4 — comparação e histórico

- [ ] T026 **(TF)** `GET /runs/compare` — as duas cercas lado a lado
- [ ] T027 **(TF)** A comparação se declara **não confiável** enquanto a versão do código não existir, ainda que cerca e taxas sejam idênticas (FR-007, SC-010)
- [ ] T028 **(TF)** `GET /datasets` — cobertura e procedência, com as lacunas **como lacunas**, jamais interpoladas (FR-015)
- [ ] T029 **(TF)** Toda resposta distingue o que vem do `market.db`, cache reconstruível, do que vem do `runs.db`, insubstituível (FR-016)
- [ ] T030 **(TF)** `GET /runs/match` — o aviso de repetição, antes de iniciar
- [ ] T031 **(TF)** Erro de histórico insuficiente diz o trecho ausente e o `trade collect` com as datas preenchidas (FR-022)

## Fatia 5 — a escrita

- [ ] T032 **(TF)** Trait de ligação em `trade-ports`: aceita os parâmetros e devolve o identificador do trabalho
- [ ] T033 **(TF)** `POST /runs` exige o modo declarado; ausência é recusada e nenhuma execução começa (FR-004, SC-003)
- [ ] T034 **(TF)** `live` é recusado, com mensagem **distinta** da ausência de modo (FR-004, SC-004)
- [ ] T035 **(TF)** `POST /runs` responde imediatamente com identificador e estado, sem esperar a conclusão (FR-017)
- [ ] T036 **(TF)** `GET /jobs/{id}` — progresso por consulta; nenhuma conexão fica aberta empurrando evento (FR-018)
- [ ] T037 **(TF)** Nenhuma rota aceita, guarda ou devolve credencial; o token não vai para o registro (FR-020)
- [ ] T038 **(TF)** Não existe rota que emita ordem, altere limite ou taxa, toque o kill switch ou retome execução parada (FR-005)

## Fatia 6 — o comando

- [ ] T039 `trade serve` na CLI, implementando a trait de ligação pela composição do `backtest` (FR-001)
- [ ] T040 **(TF)** O token nasce a cada início do servidor e é mostrado uma vez, nunca gravado no registro (FR-019)
- [ ] T041 Guia de verificação do `quickstart.md` roda de ponta a ponta sobre um `runs.db` real
