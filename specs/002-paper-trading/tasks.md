# Tasks: Paper Trading na Testnet da Bybit

**Spec**: [spec.md](spec.md) · **Plano**: [plan.md](plan.md) · **Constitution**: v2.0.0

Marcação: `[P]` roda em paralelo com a anterior. Lógica crítica traz **(TF)** —
teste escrito e **falhando** antes da implementação, conforme o Princípio III.

## Fatia 1 — o modo `paper` existe

Verificável sem rede e sem credencial.

- [X] T001 **(TF)** `ExecutionMode::Paper` passa a existir; `live` continua recusado
- [X] T002 **(TF)** Leitura de `BYBIT_TESTNET_KEY` e `BYBIT_TESTNET_SECRET` do ambiente
- [X] T003 **(TF)** Ausência de credencial aborta a inicialização em `paper` (FR-102)
- [X] T004 **(TF)** Chave de testnet e de produção no mesmo ambiente aborta (FR-104)
- [X] T005 Segredo MUST NOT aparecer em `Debug`, log ou erro — tipo que redige a si mesmo
- [X] T006 [P] `trade-paper` criada, declarada no workspace, sem código ainda
- [X] T007 Teste de arquitetura passa a cobrar que estratégia, risco e backtest não declarem `trade-paper`

## Fatia 2 — a assinatura

- [X] T008 **(TF)** HMAC-SHA256 sobre vetor conhecido da documentação da Bybit
- [X] T009 **(TF)** Ordenação de parâmetros e `recv_window` na string assinada
- [X] T010 **(TF)** Timestamp fora da janela é recusado antes do envio
- [X] T011 Cliente autenticado em `trade-bybit/src/auth.rs`, sobre o `ureq` já existente
- [X] T012 **(TF)** Verificação de permissão de saque na chave; com saque, aborta (FR-103)

## Fatia 3 — o adaptador

- [X] T013 **(TF)** `OrderExecutor` para paper, contra duplo HTTP
- [X] T014 **(TF)** Identificador de cliente único por ordem (FR-106)
- [X] T015 **(TF)** Ordem de entrada carrega stop a mercado no gatilho (FR-113, REQ-EXEC-010)
- [X] T016 **(TF)** Stop recusado impede a posição; se já aberta, encerra (REQ-EXEC-011)
- [X] T017 **(TF)** Resultado desconhecido não é reenviado antes de reconciliar (FR-107)
- [X] T018 **(TF)** Preenchimento registra referência, obtido, taxa e moeda da taxa (FR-109)
- [X] T019 **(TF)** Preenchimento parcial abaixo da ordem mínima deixa resíduo, como no backtest
- [X] T020 Tradução de erro da Bybit para `ExecError`, com transitório e integridade separados

## Fatia 4 — reconciliação

- [X] T021 **(TF)** Comparação de posição local × reportada, com veredito
- [X] T022 **(TF)** Divergência classifica como falha de integridade e exige revisão humana
- [X] T023 **(TF)** `DESCONHECIDO` bloqueia nova entrada até resolver (FR-108)
- [ ] T024 **(TF)** Estado recuperado do `runs.db` no início da sessão (FR-112)
- [ ] T025 **(TF)** Contadores de risco — perda diária, ordens na janela — sobrevivem ao reinício

## Fatia 5 — o laço contínuo

- [ ] T026 **(TF)** `Clock` real; o de backtest é inalcançável em paper (FR-115)
- [ ] T027 **(TF)** Posição que atinge 72h é encerrada (FR-114, emenda 2.0.0)
- [ ] T028 **(TF)** Falha transitória retenta dentro do limite; acima dele, para (FR-111)
- [ ] T029 Laço lê vela em tempo real, avalia, encaminha pelo `RiskGuard`
- [ ] T030 **(TF)** Virada de dia zera o freio com posição aberta atravessando (FR-019b)
- [ ] T031 Encerramento limpo: sinal do sistema fecha o registro sem perder evento

## Fatia 6 — primeira ordem real *(exige credencial)*

- [ ] T032 Chave de testnet criada, sem permissão de saque, e verificada
- [ ] T033 Primeira ordem real na testnet, com registro completo
- [ ] T034 Divergências entre testnet e o que o simulador previa: registradas

## Fatia 7 — os 30 dias *(exige credencial e tempo)*

- [ ] T035 Operação contínua por 30 dias corridos (SC-101)
- [ ] T036 Slippage real sobre ≥ 100 preenchimentos: mediana e p95 (SC-102)
- [ ] T037 Promover `slippage` de `ASSUMED` para `MEASURED` no registry
- [ ] T038 Divergência paper × backtest no mesmo período: medida e explicada (SC-106)
- [ ] T039 Relatório da Porta 2, para a revisão que precede a Porta 3

---

**Situação em 2026-09-20**: 25 de 31 tarefas das fatias 1 a 5 concluídas.
Feito: modo `paper`, credenciais redigidas, assinatura conferida contra a
documentação, verificação de permissão da chave, montagem de ordem com stop a
mercado no gatilho, identificador de cliente, tradução de erro, cliente HTTP
autenticado, o comando `trade paper verificar`, o executor ligando as peças e a
reconciliação com tolerância. Falta a recuperação de estado do `runs.db` e o
laço contínuo — as duas dependem de decisões de persistência, não de
credencial.

**Fatias 1 a 5**: 31 tarefas, nenhuma precisa de credencial.
**Fatias 6 e 7**: 8 tarefas, todas dependem do mantenedor criar a chave de
testnet e do tempo correr.
