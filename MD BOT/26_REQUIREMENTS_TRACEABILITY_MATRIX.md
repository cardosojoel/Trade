# 26 — Requirements Traceability Matrix

**Status:** normativo · **Versão:** 2.0 · **Atualizado em:** 2026-09-20  
**Conformidade:** conforme  
**Vocabulário:** [`00_GLOSSARIO.md`](00_GLOSSARIO.md)

---

## Objetivo
Rastreabilidade verificável entre requisito, especificação, implementação Rust,
teste, benchmark e métrica de produção.

## Como ler
O requisito **mora na especificação**, com o identificador escrito ao lado da
regra. Esta matriz é índice: não reenuncia a regra, apenas localiza-a e registra
o estado da evidência. A coluna *Rótulo* é o começo do requisito, truncado — não
é o texto normativo.

`REQ-<DOMÍNIO>-<NNN>`. O identificador é imutável: requisito que deixa de valer
é marcado como retirado, nunca renumerado nem reaproveitado.

## Estados
`DRAFT`, `SPECIFIED`, `IMPLEMENTED`, `TESTED`, `BENCHMARKED`, `OBSERVED`,
`VERIFIED`, `BLOCKED`.

Um requisito só chega a `VERIFIED` com especificação normativa, implementação
identificada, teste automatizado, benchmark ou backtest quando aplicável e
métrica operacional quando aplicável. Nenhum requisito crítico de Risk,
Execution, Reconciliation, Security ou Performance MAY chegar a `VERIFIED` por
revisão documental.

## Evidência

Os primeiros requisitos a sair de `SPECIFIED`. Esta seção é escrita à mão e
sobrevive à regeneração das tabelas abaixo.

| ID | Módulo Rust | Teste | Estado |
|---|---|---|---|
| `REQ-BYBIT-004` | `trade-domain/src/types.rs` — `Fill::fee_base` | `position::tests::compra_credita_a_quantidade_liquida_da_taxa_em_moeda_base` | **TESTED** |
| `REQ-BYBIT-005` | `trade-domain/src/instrumento.rs` | `position::tests::residuo_acumulado_volta_a_ser_vendavel` | **TESTED** |
| `REQ-SIZING-004` | `trade-domain/src/position.rs` — `apply_fill` | `position::tests::residuo_abaixo_do_passo_permanece_na_posicao` | **TESTED** |
| `REQ-BYBIT-006` | `trade-cli/src/config.rs` — `load_instrumento` | `config::tests::instrumento_e_lido_do_arquivo` | **TESTED** |
| `REQ-BYBIT-007` | `trade-cli/src/config.rs` — `validar_perfil` | `config::tests::perfil_cujo_teto_de_posicao_nao_paga_a_ordem_minima_nao_inicia` | **TESTED** |
| `REQ-BYBIT-008` | `examples/instrumento.toml` — só `minOrderAmt` é lido | — | **IMPLEMENTED** |
| Derivação do perfil | `trade-domain/src/perfil.rs` — `Perfil::derivar` | `perfil::tests::a_tabela_do_registry_e_reproduzida` | **TESTED** |
| `REQ-EXEC-010` | `trade-paper/src/ordem.rs` — `corpo_de_criacao` | `ordem::tests::a_compra_leva_stop_a_mercado_no_gatilho` | **TESTED** |
| `REQ-EXEC-003` | `trade-paper/src/erros.rs` — `de_transporte` | `erros::tests::resultado_desconhecido_e_integridade_e_nao_transitorio` | **TESTED** |
| `REQ-EXEC-011` | `trade-paper/src/executor.rs` — `enviar` | `executor::tests::sem_preco_de_referencia_a_entrada_nao_sai` | **TESTED** |
| `REQ-EXEC-004` | `trade-paper/src/ordem.rs` — `order_link_id` | `ordem::tests::ordens_diferentes_tem_identificadores_diferentes` | **TESTED** |
| `REQ-RECON-005` | `trade-paper/src/reconcile.rs` — `Veredito` | `reconcile::tests::desconhecido_bloqueia_como_divergencia_mas_nao_exige_revisao` | **TESTED** |

A derivação usa a tabela do `27_CONFIGURATION_REGISTRY.md` como oráculo de
teste: se código e documento divergirem, o teste quebra. É o que impede a folha
de parâmetros de virar ficção mantida à parte.

Nenhum chegou a `VERIFIED`: falta benchmark e métrica de produção, e a regra
deste documento é explícita em que revisão documental não promove requisito
crítico.

## Estado de hoje
**162 requisitos em 30 domínios, todos em `SPECIFIED`.** Nenhum aponta para
módulo Rust, teste ou métrica: a implementação que existe no repositório foi
construída a partir da constitution e da `specs/001-nucleo-execucao/`, não desta
SDD, e nenhuma correspondência foi verificada linha a linha. Preencher as três
colunas vazias é o gate declarado no `00_MASTER_INDEX_FINAL.md` §13 — trabalho
de leitura de código, não de documentação.

Este catálogo é a primeira passagem. Texto que ainda diz "deve" sem `MUST` é,
por convenção do glossário, explicação e não requisito — mas se numa releitura
algum deles se revelar obrigação, o correto é promovê-lo a `REQ-…` na própria
especificação, não abrir exceção à convenção.

## `REQ-ADR-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-ADR-001` | Toda ADR MUST declarar: o que contraria, onde essa regra vive, qual ato a… | `34_ADR_EMENDAS.md` Como funciona | — | — | — | SPECIFIED |
| `REQ-ADR-002` | Status possíveis: proposta, aceita, recusada, substituída por ADR-NNN.… | `34_ADR_EMENDAS.md` Como funciona | — | — | — | SPECIFIED |
| `REQ-ADR-003` | Enquanto uma ADR estiver em proposta, nenhum documento que dela dependa MAY… | `34_ADR_EMENDAS.md` Como funciona | — | — | — | SPECIFIED |

## `REQ-BACKTEST-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-BACKTEST-001` | O backtest MUST reutilizar a mesma lógica de produção | `05_BACKTEST_ENGINE.md` Princípio | — | — | — | SPECIFIED |
| `REQ-BACKTEST-002` | O motor MUST garantir | `05_BACKTEST_ENGINE.md` Regras | — | — | — | SPECIFIED |
| `REQ-BACKTEST-003` | A avaliação MUST ser walk-forward; treino e teste MUST NOT compartilhar… | `05_BACKTEST_ENGINE.md` Walk-forward | — | — | — | SPECIFIED |
| `REQ-BACKTEST-004` | O motor MUST produzir | `05_BACKTEST_ENGINE.md` Métricas | — | — | — | SPECIFIED |

## `REQ-BYBIT-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-BYBIT-001` | A validação de ordem MUST garantir | `28_BYBIT_INSTRUMENT_REGISTRY.md` Regras | — | — | — | SPECIFIED |
| `REQ-BYBIT-002` | O adapter MUST obter, atualizar e versionar as especificações do… | `28_BYBIT_INSTRUMENT_REGISTRY.md` Atualização | — | — | — | SPECIFIED |
| `REQ-BYBIT-003` | Mudança de instrumento MUST NOT aumentar automaticamente o risco permitido… | `28_BYBIT_INSTRUMENT_REGISTRY.md` Segurança | — | — | — | SPECIFIED |
| `REQ-BYBIT-004` | O adapter MUST registrar, por execução, a quantidade efetivamente recebida… | `28_BYBIT_INSTRUMENT_REGISTRY.md` Moeda da taxa e quantização | — | — | — | SPECIFIED |
| `REQ-BYBIT-005` | O saldo residual da moeda base — o que sobra abaixo de qty_step ou abaixo… | `28_BYBIT_INSTRUMENT_REGISTRY.md` Moeda da taxa e quantização | — | — | — | SPECIFIED |
| `REQ-BYBIT-006` | A especificação MUST ser relida no início de toda sessão e MUST NOT ser… | `28_BYBIT_INSTRUMENT_REGISTRY.md` Atualização | — | — | — | SPECIFIED |
| `REQ-BYBIT-007` | Se a releitura invalidar o perfil derivado — por exemplo, se min_order_amt… | `28_BYBIT_INSTRUMENT_REGISTRY.md` Atualização | — | — | — | SPECIFIED |
| `REQ-BYBIT-008` | min_order_qty está deprecado na API e MUST NOT ser usado como critério de… | `28_BYBIT_INSTRUMENT_REGISTRY.md` Atualização | — | — | — | SPECIFIED |

## `REQ-CFG-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-CFG-001` | Cada parâmetro MUST possuir | `27_CONFIGURATION_REGISTRY.md` Contrato | — | — | — | SPECIFIED |
| `REQ-CFG-002` | O startup MUST rejeitar configuração fora dos limites ou inconsistente | `27_CONFIGURATION_REGISTRY.md` Validação | — | — | — | SPECIFIED |
| `REQ-CFG-003` | O runtime MUST ler o Registry versionado; o código MUST conter apenas… | `27_CONFIGURATION_REGISTRY.md` Precedência | — | — | — | SPECIFIED |
| `REQ-CFG-004` | Todo parâmetro MUST declarar a origem do seu valor em source, com um destes… | `27_CONFIGURATION_REGISTRY.md` Origem do valor | — | — | — | SPECIFIED |
| `REQ-CFG-005` | Todo perfil MUST declarar o resultado operacional esperado sem vantagem — o… | `27_CONFIGURATION_REGISTRY.md` Resultado esperado sem vantagem | — | — | — | SPECIFIED |
| `REQ-CFG-006` | O método que produz esse número MUST ser reproduzível e MUST declarar… | `27_CONFIGURATION_REGISTRY.md` Resultado esperado sem vantagem | — | — | — | SPECIFIED |
| `REQ-CFG-007` | Enquanto REQ-SIZING-004 e REQ-BYBIT-005 não estiverem implementados e… | `27_CONFIGURATION_REGISTRY.md` Condição de validade dos números acima | — | — | — | SPECIFIED |

## `REQ-DATA-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-DATA-001` | O SLO sub-milissegundo aplica-se ao processamento interno, não à frequência… | `04_MARKET_DATA_QUALITY.md` Não confundir frequência do feed com latência do bot | — | — | — | SPECIFIED |
| `REQ-DATA-002` | O processamento do book MUST tratar corretamente | `04_MARKET_DATA_QUALITY.md` Order Book | — | — | — | SPECIFIED |
| `REQ-DATA-003` | Uma nova mensagem snapshot MUST provocar reconstrução do book local. Bybit… | `04_MARKET_DATA_QUALITY.md` Order Book | — | — | — | SPECIFIED |
| `REQ-DATA-004` | ). Bybit… | `04_MARKET_DATA_QUALITY.md` Trades | — | — | — | SPECIFIED |
| `REQ-DATA-005` | A idade do dado MUST ser classificada e MUST bloquear nova entrada acima do… | `04_MARKET_DATA_QUALITY.md` Data Age | — | — | — | SPECIFIED |
| `REQ-DATA-006` | O sistema MUST detectar | `04_MARKET_DATA_QUALITY.md` Integridade | — | — | — | SPECIFIED |
| `REQ-DATA-007` | Dados inconsistentes MUST NOT ser corrigidos silenciosamente | `04_MARKET_DATA_QUALITY.md` Integridade | — | — | — | SPECIFIED |
| `REQ-DATA-008` | O benchmark MUST utilizar eventos reais ou replay e bursts de até 1024… | `04_MARKET_DATA_QUALITY.md` Conclusão | — | — | — | SPECIFIED |

## `REQ-DB-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-DB-001` | A separação MUST ser respeitada | `19_DATABASE_SCHEMA.md` Princípio | — | — | — | SPECIFIED |
| `REQ-DB-002` | A persistência MUST usar | `19_DATABASE_SCHEMA.md` SQLite | — | — | — | SPECIFIED |
| `REQ-DB-003` | Falha de persistência MUST NOT alterar silenciosamente o Risk State em RAM:… | `19_DATABASE_SCHEMA.md` Integridade | — | — | — | SPECIFIED |
| `REQ-DB-004` | Dados necessários a auditoria, replay e aprendizado MUST NOT ser apagados… | `19_DATABASE_SCHEMA.md` Retenção | — | — | — | SPECIFIED |

## `REQ-EV-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-EV-001` | O modelo canônico é ternário, consistente com a seção 13 | `MATHEMATICAL_QUANT_MODEL.md` §15 | — | — | — | SPECIFIED |
| `REQ-EV-002` | O sistema MUST calcular também um cenário conservador, degradando conforme… | `MATHEMATICAL_QUANT_MODEL.md` §15 | — | — | — | SPECIFIED |
| `REQ-EV-003` | Mas EV_net > 0 sozinho MUST NOT autorizar execução: a autoridade é do Risk… | `MATHEMATICAL_QUANT_MODEL.md` §15 | — | — | — | SPECIFIED |

## `REQ-EVENT-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-EVENT-001` | Todo evento MUST carregar o envelope completo: event_id, event_type,… | `20_EVENT_MODEL.md` §20 | — | — | — | SPECIFIED |
| `REQ-EVENT-002` | Eventos MUST ser append-only; correção MUST ser evento novo, nunca… | `20_EVENT_MODEL.md` §20 | — | — | — | SPECIFIED |
| `REQ-EVENT-003` | Causalidade MUST NOT ser inferida apenas de timestamp: sequência,… | `20_EVENT_MODEL.md` §20 | — | — | — | SPECIFIED |

## `REQ-EXEC-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-EXEC-001` | Operações de create, amend e cancel MUST usar o WebSocket Order Entry como… | `01_EXECUTION_ENGINE.md` Hot path | — | — | — | SPECIFIED |
| `REQ-EXEC-002` | REST MAY ser usado para | `01_EXECUTION_ENGINE.md` Recovery / fallback | — | — | — | SPECIFIED |
| `REQ-EXEC-003` | O sistema MUST NOT repetir uma ordem cujo resultado seja desconhecido antes… | `01_EXECUTION_ENGINE.md` Recovery / fallback | — | — | — | SPECIFIED |
| `REQ-EXEC-004` | Cada intenção MUST carregar | `01_EXECUTION_ENGINE.md` Idempotência | — | — | — | SPECIFIED |
| `REQ-EXEC-005` | reqId MUST ser único dentro da conexão, para evitar duplicação; a Bybit… | `01_EXECUTION_ENGINE.md` Idempotência | — | — | — | SPECIFIED |
| `REQ-EXEC-006` | Ordem em UNKNOWN MUST ser reconciliada antes de qualquer nova ordem… | `01_EXECUTION_ENGINE.md` Estados | — | — | — | SPECIFIED |
| `REQ-EXEC-007` | O sistema MUST assinar execution.fast em paralelo ao stream completo, e… | `01_EXECUTION_ENGINE.md` Execution Fast | — | — | — | SPECIFIED |
| `REQ-EXEC-008` | O sistema MUST registrar timestamps monotônicos locais e os timestamps da… | `01_EXECUTION_ENGINE.md` Métricas | — | — | — | SPECIFIED |
| `REQ-EXEC-009` | O sistema MUST NOT interpretar ACK como fill | `01_EXECUTION_ENGINE.md` Conclusão | — | — | — | SPECIFIED |
| `REQ-EXEC-010` | O stop MUST ser enviado como ordem a mercado no gatilho (slOrderType:… | `01_EXECUTION_ENGINE.md` Stop no mercado à vista | — | — | — | SPECIFIED |
| `REQ-EXEC-011` | A confirmação de que o stop foi aceito MUST preceder a existência da… | `01_EXECUTION_ENGINE.md` Stop no mercado à vista | — | — | — | SPECIFIED |

## `REQ-FAIL-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-FAIL-001` | A reconexão MUST seguir a sequência completa, sem pular etapa | `22_FAILURE_RECOVERY.md` Bybit reconnect | — | — | — | SPECIFIED |
| `REQ-FAIL-002` | Novas entradas MUST permanecer bloqueadas até a reconciliação obrigatória… | `22_FAILURE_RECOVERY.md` Bybit reconnect | — | — | — | SPECIFIED |
| `REQ-FAIL-003` | Ordem de resultado desconhecido MUST NOT ser repetida antes da… | `22_FAILURE_RECOVERY.md` UNKNOWN order | — | — | — | SPECIFIED |
| `REQ-FAIL-004` | Incerteza material sobre posição, ordem, saldo ou risco MUST bloquear novas… | `22_FAILURE_RECOVERY.md` Safe state | — | — | — | SPECIFIED |

## `REQ-FEATURE-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-FEATURE-001` | Cada feature MUST declarar name, formula, window, source, unit,… | `MATHEMATICAL_QUANT_MODEL.md` §12 | — | — | — | SPECIFIED |

## `REQ-FRONTEIRA-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-FRONTEIRA-001` | A autoridade sobre o que o robô pode fazer é, nesta ordem | `00_FRONTEIRA.md` §1 | — | — | — | SPECIFIED |
| `REQ-FRONTEIRA-002` | Se o hash mudar, toda a coluna Conformidade deste conjunto MUST ser… | `00_FRONTEIRA.md` Âncora de sincronia | — | — | — | SPECIFIED |
| `REQ-FRONTEIRA-003` | Existe uma única saída deste conjunto para o Trade, e ela é unidirecional | `00_FRONTEIRA.md` §2 | — | — | — | SPECIFIED |
| `REQ-FRONTEIRA-004` | Este conjunto MUST NOT alterar, direta ou indiretamente, nenhum arquivo do… | `00_FRONTEIRA.md` §2 | — | — | — | SPECIFIED |
| `REQ-FRONTEIRA-005` | Todo documento normativo deste conjunto MUST declarar no cabeçalho | `00_FRONTEIRA.md` §3 | — | — | — | SPECIFIED |
| `REQ-FRONTEIRA-006` | Uma proposta MUST nomear qual documento ela contraria. "Conflita com a… | `00_FRONTEIRA.md` §4 | — | — | — | SPECIFIED |
| `REQ-FRONTEIRA-007` | Mover este conjunto para outro repositório MUST converter cada referência… | `00_FRONTEIRA.md` Apoio factual | — | — | — | SPECIFIED |
| `REQ-FRONTEIRA-008` | Este inventário MUST ser refeito sempre que a âncora da seção 1 mudar. Foi… | `00_FRONTEIRA.md` §5 | — | — | — | SPECIFIED |

## `REQ-GOV-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-GOV-001` | Nenhum documento MAY ser fonte concorrente de valores concretos: regras… | `24_CONFIGURATION_AND_GOVERNANCE.md` Autoridade dos parâmetros | — | — | — | SPECIFIED |
| `REQ-GOV-002` | Cada parâmetro MUST possuir: parameter_id, value, type, unit, scope, min,… | `24_CONFIGURATION_AND_GOVERNANCE.md` Registro | — | — | — | SPECIFIED |
| `REQ-GOV-003` | Parâmetros estruturais de risco MUST ser congelados no início da sessão | `24_CONFIGURATION_AND_GOVERNANCE.md` Sessão | — | — | — | SPECIFIED |
| `REQ-GOV-004` | Toda alteração MUST registrar actor, timestamp, reason, versão anterior e… | `24_CONFIGURATION_AND_GOVERNANCE.md` Governança | — | — | — | SPECIFIED |
| `REQ-GOV-005` | Segredos MUST NOT existir em Markdown, em SQLite em texto puro ou em… | `24_CONFIGURATION_AND_GOVERNANCE.md` Segurança | — | — | — | SPECIFIED |
| `REQ-GOV-006` | Configuração MAY parametrizar comportamento e MUST NOT contornar invariante… | `24_CONFIGURATION_AND_GOVERNANCE.md` Invariantes | — | — | — | SPECIFIED |

## `REQ-LEARN-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-LEARN-001` | Depois de criado | `DECISION_LEARNING.md` §3 | — | — | — | SPECIFIED |
| `REQ-LEARN-002` | O ledger MUST registrar toda decisão, não apenas as operações executadas | `DECISION_LEARNING.md` §4 | — | — | — | SPECIFIED |
| `REQ-LEARN-003` | O resultado MUST ser entidade separada da decisão; a decisão MUST NOT ser… | `DECISION_LEARNING.md` §14 | — | — | — | SPECIFIED |
| `REQ-LEARN-004` | É proibido utilizar no snapshot | `DECISION_LEARNING.md` §29 | — | — | — | SPECIFIED |
| `REQ-LEARN-005` | MUST existir teste automatizado que verifique | `DECISION_LEARNING.md` §31 | — | — | — | SPECIFIED |
| `REQ-LEARN-006` | O modelo MUST NOT usar seu próprio resultado recente para alterar parâmetro… | `DECISION_LEARNING.md` §35 | — | — | — | SPECIFIED |
| `REQ-LEARN-007` | ) | `DECISION_LEARNING.md` §37 | — | — | — | SPECIFIED |
| `REQ-LEARN-008` | A promoção MUST exigir todos os critérios obrigatórios, e cada um MUST ser… | `DECISION_LEARNING.md` §38 | — | — | — | SPECIFIED |
| `REQ-LEARN-009` | Rollback MUST restaurar uma versão anteriormente aprovada, e MUST ser… | `DECISION_LEARNING.md` §39 | — | — | — | SPECIFIED |
| `REQ-LEARN-010` | O bot MUST NOT aprender livremente | `DECISION_LEARNING.md` §51 | — | — | — | SPECIFIED |

## `REQ-LIMITE-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-LIMITE-001` | Uma proposta que viole limite duro MUST ser recusada na origem: não há ADR… | `35_LIMITES.md` Três naturezas, três consequências | — | — | — | SPECIFIED |
| `REQ-LIMITE-002` | Um limite medido MAY ser revisto, e apenas por medição nova com método… | `35_LIMITES.md` Três naturezas, três consequências | — | — | — | SPECIFIED |
| `REQ-LIMITE-003` | Um limite de autoridade MUST passar por… | `35_LIMITES.md` Três naturezas, três consequências | — | — | — | SPECIFIED |
| `REQ-LIMITE-004` | Esta tabela MUST declarar a data da leitura. Tabela sem data é valor… | `35_LIMITES.md` §1 | — | — | — | SPECIFIED |
| `REQ-LIMITE-005` | Um número desta folha MUST NOT ser citado sem a limitação correspondente… | `35_LIMITES.md` §7 | — | — | — | SPECIFIED |

## `REQ-PATTERN-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-PATTERN-001` | Esta regra MUST ser respeitada, para impedir | `HISTORICAL_PATTERN_PROBABILITY_EV.md` §4 | — | — | — | SPECIFIED |
| `REQ-PATTERN-002` | No período de teste | `HISTORICAL_PATTERN_PROBABILITY_EV.md` §5 | — | — | — | SPECIFIED |
| `REQ-PATTERN-003` | Não assumir que K = 100 é matematicamente ótimo. K é hiperparâmetro e MUST… | `HISTORICAL_PATTERN_PROBABILITY_EV.md` §7 | — | — | — | SPECIFIED |
| `REQ-PATTERN-004` | Sem amostra mínima ou sem qualidade de similaridade suficiente, o módulo… | `HISTORICAL_PATTERN_PROBABILITY_EV.md` §26 | — | — | — | SPECIFIED |
| `REQ-PATTERN-005` | Para uma decisão em t | `HISTORICAL_PATTERN_PROBABILITY_EV.md` §38 | — | — | — | SPECIFIED |

## `REQ-PERF-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-PERF-001` | O caminho de emergência MUST NOT aguardar | `09_PERFORMANCE_LOW_LATENCY.md` Emergency Path | — | — | — | SPECIFIED |
| `REQ-PERF-002` | Meta do caminho interno de decisão | `09_PERFORMANCE_LOW_LATENCY.md` SLO 1 | — | — | — | SPECIFIED |
| `REQ-PERF-003` | Meta do caminho interno de emergência | `09_PERFORMANCE_LOW_LATENCY.md` SLO 2 | — | — | — | SPECIFIED |
| `REQ-PERF-004` | O rate limit MUST ser monitorado dinamicamente pelos headers, nunca… | `09_PERFORMANCE_LOW_LATENCY.md` SLO 5 | — | — | — | SPECIFIED |
| `REQ-PERF-005` | Memória | `09_PERFORMANCE_LOW_LATENCY.md` SLO 6 | — | — | — | SPECIFIED |
| `REQ-PERF-006` | CPU em operação normal | `09_PERFORMANCE_LOW_LATENCY.md` SLO 7 | — | — | — | SPECIFIED |
| `REQ-PERF-007` | O hot path MUST NOT conter | `09_PERFORMANCE_LOW_LATENCY.md` Hot Path | — | — | — | SPECIFIED |
| `REQ-PERF-008` | Os budgets por estágio MUST ser obtidos por profiling e MUST NOT ser… | `09_PERFORMANCE_LOW_LATENCY.md` Latency Budget | — | — | — | SPECIFIED |
| `REQ-PERF-009` | Nenhuma promoção sem que todos estes critérios passem | `09_PERFORMANCE_LOW_LATENCY.md` Critério de aprovação | — | — | — | SPECIFIED |

## `REQ-PROB-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-PROB-001` | Em amostra pequena, a contagem ponderada MUST receber smoothing; a… | `HISTORICAL_PATTERN_PROBABILITY_EV.md` §12 | — | — | — | SPECIFIED |
| `REQ-PROB-002` | A probabilidade bruta MUST ser calibrada antes do uso | `HISTORICAL_PATTERN_PROBABILITY_EV.md` §14 | — | — | — | SPECIFIED |
| `REQ-PROB-003` | Um candidato MUST satisfazer | `HISTORICAL_PATTERN_PROBABILITY_EV.md` §32 | — | — | — | SPECIFIED |

## `REQ-RECON-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-RECON-001` | A reconciliação MUST considerar todas estas fontes | `02_POSITION_RECONCILIATION.md` Fontes | — | — | — | SPECIFIED |
| `REQ-RECON-002` | execution.fast serve ao caminho rápido de execução e MUST NOT ser fonte… | `02_POSITION_RECONCILIATION.md` Regra fundamental | — | — | — | SPECIFIED |
| `REQ-RECON-003` | A identidade de uma execution MUST ser composta | `02_POSITION_RECONCILIATION.md` Identidade | — | — | — | SPECIFIED |
| `REQ-RECON-004` | A habilitação de trading MUST ser o último passo da sequência de startup | `02_POSITION_RECONCILIATION.md` Startup | — | — | — | SPECIFIED |
| `REQ-RECON-005` | Qualquer CONFLICT ou UNKNOWN relevante MUST bloquear novas entradas | `02_POSITION_RECONCILIATION.md` Divergência | — | — | — | SPECIFIED |

## `REQ-RECOVERY-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-RECOVERY-001` | Somente lucro realizado MAY gerar Recovery Budget | `trading_risk_recovery_mathematical_spec.md` §7 | — | — | — | SPECIFIED |
| `REQ-RECOVERY-002` | O orçamento máximo de Recovery MUST ser o menor dos dois limites | `trading_risk_recovery_mathematical_spec.md` §10 | — | — | — | SPECIFIED |
| `REQ-RECOVERY-003` | O Recovery Budget é orçamento consumível: perdas o reduzem e ganhos MUST… | `trading_risk_recovery_mathematical_spec.md` §12 | — | — | — | SPECIFIED |
| `REQ-RECOVERY-004` | O Recovery MUST NOT ser ativado exceto quando | `trading_risk_recovery_mathematical_spec.md` §13 | — | — | — | SPECIFIED |
| `REQ-RECOVERY-005` | A recuperação MUST falhar quando qualquer uma destas condições ocorrer | `trading_risk_recovery_mathematical_spec.md` §19 | — | — | — | SPECIFIED |
| `REQ-RECOVERY-006` | Um novo episódio MUST ser condicionado a | `trading_risk_recovery_mathematical_spec.md` §28 | — | — | — | SPECIFIED |
| `REQ-RECOVERY-007` | É proibido | `trading_risk_recovery_mathematical_spec.md` §29 | — | — | — | SPECIFIED |

## `REQ-REGIME-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-REGIME-001` | O modelo MUST classificar o mercado no enum canônico do glossário | `MATHEMATICAL_QUANT_MODEL.md` §11 | — | — | — | SPECIFIED |

## `REQ-REPLAY-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-REPLAY-001` | Mesmo conjunto de eventos, versões, configuração, seed e relógio lógico… | `21_REPLAY_ENGINE.md` §21 | — | — | — | SPECIFIED |
| `REQ-REPLAY-002` | A comparação MUST cobrir decision_id, signal, probability, EV, resultado de… | `21_REPLAY_ENGINE.md` §21 | — | — | — | SPECIFIED |
| `REQ-REPLAY-003` | Replay de regressão MUST preceder qualquer alteração crítica | `21_REPLAY_ENGINE.md` §21 | — | — | — | SPECIFIED |

## `REQ-RISK-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-RISK-001` | O limite máximo de perda do capital originalmente depositado é | `trading_risk_recovery_mathematical_spec.md` §4 | — | — | — | SPECIFIED |
| `REQ-RISK-002` | Durante Recovery, o risco MUST ser calculado sobre o orçamento restante | `trading_risk_recovery_mathematical_spec.md` §21 | — | — | — | SPECIFIED |
| `REQ-RISK-003` | O risco permitido para uma operação MUST ser | `trading_risk_recovery_mathematical_spec.md` §22 | — | — | — | SPECIFIED |
| `REQ-RISK-004` | Uma perda MUST NOT aumentar automaticamente | `trading_risk_recovery_mathematical_spec.md` §23 | — | — | — | SPECIFIED |
| `REQ-RISK-005` | A posição MUST NOT ser autorizada exceto quando | `trading_risk_recovery_mathematical_spec.md` §25 | — | — | — | SPECIFIED |
| `REQ-RISK-006` | A operação MUST NOT ser autorizada exceto quando | `trading_risk_recovery_mathematical_spec.md` §26 | — | — | — | SPECIFIED |
| `REQ-RISK-007` | A soma existe, tem nome próprio e significado restrito | `trading_risk_recovery_mathematical_spec.md` §31 | — | — | — | SPECIFIED |
| `REQ-RISK-008` | ) | `trading_risk_recovery_mathematical_spec.md` §32 | — | — | — | SPECIFIED |
| `REQ-RISK-009` | É proibido utilizar | `trading_risk_recovery_mathematical_spec.md` §33 | — | — | — | SPECIFIED |
| `REQ-RISK-010` | As seguintes condições MUST NOT ser violadas | `trading_risk_recovery_mathematical_spec.md` §34 | — | — | — | SPECIFIED |
| `REQ-RISK-011` | Se qualquer cálculo necessário para autorizar uma operação não puder ser… | `trading_risk_recovery_mathematical_spec.md` §35 | — | — | — | SPECIFIED |
| `REQ-RISK-012` | Complementam os invariantes matemáticos da seção 34 e valem sobre o sistema… | `trading_risk_recovery_mathematical_spec.md` §40 | — | — | — | SPECIFIED |

## `REQ-RUST-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-RUST-001` | Os tipos MUST garantir | `29_RUST_CONTRACTS.md` Regras de segurança de tipos | — | — | — | SPECIFIED |
| `REQ-RUST-002` | Interfaces críticas MUST NOT exigir I/O síncrono, lock global ou alocação… | `29_RUST_CONTRACTS.md` Hot path | — | — | — | SPECIFIED |

## `REQ-SCOPE-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-SCOPE-001` | O sistema MUST NOT operar fora destes limites | `32_NAO_OBJETIVOS.md` §1 | — | — | — | SPECIFIED |
| `REQ-SCOPE-002` | Estes itens MUST NOT ser implementados antes da evidência que cada um exige | `32_NAO_OBJETIVOS.md` §2 | — | — | — | SPECIFIED |
| `REQ-SCOPE-003` | Nenhuma frente nova MAY ser aberta antes de o… | `32_NAO_OBJETIVOS.md` §2 | — | — | — | SPECIFIED |
| `REQ-SCOPE-004` | Nenhum destes quatro MAY ser implementado antes de a ADR correspondente ser… | `32_NAO_OBJETIVOS.md` §3 | — | — | — | SPECIFIED |
| `REQ-SCOPE-005` | Uma proposta MUST nomear o documento que contraria. "Conflita com a… | `32_NAO_OBJETIVOS.md` §3 | — | — | — | SPECIFIED |

## `REQ-SEC-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-SEC-001` | As credenciais de API MUST permanecer | `08_SECURITY_OBSERVABILITY.md` Secrets | — | — | — | SPECIFIED |
| `REQ-SEC-002` | Os ambientes MUST ser separados e MUST NOT coexistir na mesma configuração… | `08_SECURITY_OBSERVABILITY.md` Secrets | — | — | — | SPECIFIED |
| `REQ-SEC-003` | MUST existir kill switch manual, acionável sem acesso ao código | `08_SECURITY_OBSERVABILITY.md` Kill Switch | — | — | — | SPECIFIED |
| `REQ-SEC-004` | O kill switch MUST ser acionado automaticamente em | `08_SECURITY_OBSERVABILITY.md` Kill Switch | — | — | — | SPECIFIED |
| `REQ-SEC-005` | Os logs MUST ser estruturados e MUST NOT conter segredo | `08_SECURITY_OBSERVABILITY.md` Observabilidade | — | — | — | SPECIFIED |
| `REQ-SEC-006` | MUST ser registrado, no mínimo | `08_SECURITY_OBSERVABILITY.md` Auditoria | — | — | — | SPECIFIED |

## `REQ-SIM-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-SIM-001` | O simulador MUST modelar | `06_EXECUTION_SIMULATOR.md` Componentes | — | — | — | SPECIFIED |
| `REQ-SIM-002` | O slippage MUST depender do tamanho da ordem e da liquidez disponível —… | `06_EXECUTION_SIMULATOR.md` Modelo | — | — | — | SPECIFIED |
| `REQ-SIM-003` | O simulador MUST simular cada etapa separadamente | `06_EXECUTION_SIMULATOR.md` Latência | — | — | — | SPECIFIED |
| `REQ-SIM-004` | O simulador MUST suportar | `06_EXECUTION_SIMULATOR.md` Ordens | — | — | — | SPECIFIED |
| `REQ-SIM-005` | O simulador MUST ser exercitado sob | `06_EXECUTION_SIMULATOR.md` Cenários adversos | — | — | — | SPECIFIED |

## `REQ-SIZING-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-SIZING-001` | O sizing ocorre depois de existir vantagem estatística e antes da… | `MATHEMATICAL_QUANT_MODEL.md` §22 | — | — | — | SPECIFIED |
| `REQ-SIZING-002` | A posição MUST NOT ser autorizada exceto quando | `MATHEMATICAL_QUANT_MODEL.md` §23 | — | — | — | SPECIFIED |
| `REQ-SIZING-003` | Em mercado à vista, PositionNotional MUST NOT exceder o caixa disponível.… | `MATHEMATICAL_QUANT_MODEL.md` §22 | — | — | — | SPECIFIED |
| `REQ-SIZING-004` | O sizing MUST partir do saldo efetivo da moeda base — o que a conta de fato… | `MATHEMATICAL_QUANT_MODEL.md` §22 | — | — | — | SPECIFIED |

## `REQ-STRATEGY-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-STRATEGY-001` | Ele MUST NOT aumentar automaticamente | `MATHEMATICAL_QUANT_MODEL.md` §25 | — | — | — | SPECIFIED |
| `REQ-STRATEGY-002` | Ponto flutuante MUST NOT representar dinheiro | `MATHEMATICAL_QUANT_MODEL.md` §26 | — | — | — | SPECIFIED |
| `REQ-STRATEGY-003` | o resultado MUST ser reproduzível, e a randomização MUST ser explicitamente… | `MATHEMATICAL_QUANT_MODEL.md` §27 | — | — | — | SPECIFIED |
| `REQ-STRATEGY-004` | Regras da cadeia | `MATHEMATICAL_QUANT_MODEL.md` §28 | — | — | — | SPECIFIED |

## `REQ-TEST-*`

| ID | Rótulo | Especificação | Módulo Rust | Teste | Métrica | Status |
|---|---|---|---|---|---|---|
| `REQ-TEST-001` | MUST existir teste automatizado para cada invariante | `23_TESTING_STRATEGY.md` Invariantes matemáticos | — | — | — | SPECIFIED |
| `REQ-TEST-002` | MUST ser testado: múltiplos episódios, sucesso seguido de nova… | `23_TESTING_STRATEGY.md` Risk / Recovery | — | — | — | SPECIFIED |
| `REQ-TEST-003` | MUST ser testado: ACK vs fill, duplicate prevention, partial fill,… | `23_TESTING_STRATEGY.md` Bybit | — | — | — | SPECIFIED |
| `REQ-TEST-004` | MUST ser medido p50/p95/p99/p99.9/max e jitter, por estágio e end-to-end… | `23_TESTING_STRATEGY.md` Performance | — | — | — | SPECIFIED |
| `REQ-TEST-005` | Nenhum build de produção MAY ser promovido sem testes críticos,… | `23_TESTING_STRATEGY.md` Gate | — | — | — | SPECIFIED |
| `REQ-TEST-006` | MUST ser testado: resíduo da moeda base somado à ordem seguinte; quantidade… | `23_TESTING_STRATEGY.md` Instrumento e resíduo | — | — | — | SPECIFIED |
