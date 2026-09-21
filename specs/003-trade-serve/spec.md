<!--
Entrada vinda do projeto de desenho da interface (DsTrade), em
`design/11_SPEC_TRADE_SERVE.md` v1.1, estado **aceita**.

O formato foi escolhido pela decisão 020 do Jev (`spec_kit_pronta_para_copiar`,
0,80 de confiança) exatamente para que não houvesse retrabalho aqui. Abrir esta
feature é a decisão 036 (`abrir_a_spec_do_serve`, 0,63 · confiança 0,51 —
margem fina, contra 0,24 de parar e entregar o que já está pronto).

Esta feature foi aceita pela decisão 022 com **empate** (0,48 × 0,48,
confiança 0,22) e emendada pela 023, cuja resposta se espalhou por cinco
defeitos reais — todos corrigidos antes desta cópia. Quem for implementar deve
saber que a aceitação foi apertada.

Três coisas que a spec assume e declara, e que continuam verdadeiras em
2026-09-21, commit `6e49e41`:

- A **versão do código na tabela `run` não existe**. O `FR-006` exige o campo
  presente e nulo, e o `FR-007` obriga a comparação entre execuções a se
  declarar não confiável enquanto for assim. A decisão 036 confirmou que a
  interface pode nascer assim (`noul` 0,31): ausência declarada é resposta
  honesta, não defeito.
- **Nenhum limiar de desempenho foi medido.** Os dois que existem (`SC-013`,
  `SC-014`) nascem como configuração de partida.
- O **acompanhamento ao vivo está fora de escopo**. A folha 12 do DsTrade é
  entrada de uma feature posterior, não emenda a esta.
-->


# Feature Specification: Servidor local de leitura do registro (`trade serve`)

**Feature Branch**: `003-trade-serve`

**Created**: 2026-09-21

**Status**: Draft — entrada vinda do projeto de desenho da interface (DsTrade)

**Input**: "As nove telas da interface do robô precisam de dado, e não existe por
onde: o único binário é de linha de comando, nenhuma crate declara servidor
HTTP, e o registro vive em dois arquivos SQLite. Um servidor HTTP local, de
leitura, que exponha o que as telas perguntam — com os agregados já calculados
onde `Decimal` existe — e uma única rota de escrita: iniciar execução."

**Decisões tomadas na especificação**: resposta **assíncrona** para toda rota que
inicia trabalho; progresso por **consulta periódica**, não por conexão aberta; o
extrato sai **agrupado por episódio** por padrão; a rota de escrita exige
**token local e origem verificada**; todo valor monetário é **string** no JSON.

## Por que esta feature, e por que agora

O registro responde tudo e não responde a ninguém. O `docs/auditoria.md` já
mostra que a cadeia de uma decisão se reconstitui em SQL — e uma execução tem
143.135 eventos de auditoria, 28.633 ordens e 14.308 linhas de extrato, o que
torna a consulta à mão possível e inviável ao mesmo tempo.

Há também o que só se vê olhando: das nove execuções gravadas, **seis são
idênticas e três não**, e duas delas têm símbolo, período, estratégia,
parâmetros, limites e taxas iguais aos das seis — com resultado diferente,
porque o código mudou entre elas e **o registro não guarda qual versão produziu
cada execução**. Quem compara duas execuções hoje não tem como saber se a
comparação é válida. A feature não conserta isso sozinha, mas é onde a falta
aparece: a resposta tem de dizer, explicitamente, que não sabe.

## User Scenarios & Testing *(mandatory)*

O ator é o **mantenedor** — a mesma pessoa das features 001 e 002. A interface
que consome este servidor é operada por ele, na máquina dele.

### User Story 1 - Ver o que já rodou, sem escrever SQL (Priority: P1)

O mantenedor abre a interface e vê as execuções gravadas, agrupadas por
resultado idêntico, cada grupo com a cerca sob a qual correu.

**Why this priority**: é a porta de entrada de todas as telas. Sem ela não há
como escolher uma execução, e sem escolher não há nada a reconstituir.

**Independent Test**: subir o servidor sobre um `runs.db` conhecido e conferir
que o agrupamento devolvido bate com o `SELECT` equivalente feito à mão.

**Acceptance Scenarios**:

1. **Given** um `runs.db` com nove execuções das quais seis têm resultado
   idêntico, **When** a interface pede a lista, **Then** o servidor devolve
   quatro grupos, e o grupo das seis traz os seis identificadores e a contagem.
2. **Given** duas execuções com `limits_json` e `fees_json` iguais e resultado
   diferente, **When** a interface pede a lista, **Then** cada uma traz
   `versao_do_codigo` presente e nula, e `comparavel` falso, com o motivo
   declarado.
3. **Given** qualquer resposta desta rota, **When** ela é inspecionada, **Then**
   nenhum valor monetário, de quantidade ou de preço aparece como número JSON.

---

### User Story 2 - Reconstituir uma decisão até o sinal que a originou (Priority: P1)

O mantenedor escolhe uma operação e recebe a cadeia inteira — sinal, ordem,
decisão de risco, preenchimento e transição de estado — com os dados de entrada
que produziram cada elo.

**Why this priority**: é o Princípio IV virando alcançável sem SQL, e é a razão
de a interface existir.

**Independent Test**: pedir a cadeia de uma operação cujo `seq` é conhecido e
conferir elo a elo contra a consulta do `docs/auditoria.md`.

**Acceptance Scenarios**:

1. **Given** uma operação do extrato, **When** a interface pede a cadeia,
   **Then** o servidor devolve os cinco elos ordenados por `seq`, nunca por
   `at_ms`.
2. **Given** um elo `risk_decision`, **When** ele é devolvido, **Then** traz cada
   limite com o valor observado ao lado do teto, e não apenas o veredito.
3. **Given** uma ordem cuja quantidade preenchida difere da pedida, **When** a
   cadeia é devolvida, **Then** o elo `fill` traz a divergência assinalada e a
   proporção preenchida.
4. **Given** uma execução inteira, **When** se contam ordens e decisões de risco,
   **Then** os dois números coincidem — é o `SC-002` da feature 001, conferível
   por esta rota.

---

### User Story 3 - Iniciar uma execução sem sair da tela (Priority: P2)

O mantenedor dispara um backtest pela interface, declarando o modo, e acompanha
até o fim.

**Why this priority**: é a única ação da interface. Sem ela a tela é um museu.

**Independent Test**: disparar uma execução pela rota e conferir que ela aparece
no `runs.db` com os mesmos campos que o `trade backtest` grava.

**Acceptance Scenarios**:

1. **Given** uma requisição de início sem modo declarado, **When** ela chega,
   **Then** é recusada como erro de uso, e nenhuma execução começa.
2. **Given** uma requisição que peça modo `live`, **When** ela chega, **Then** é
   recusada com mensagem própria, distinta da anterior.
3. **Given** uma requisição válida, **When** ela é aceita, **Then** o servidor
   responde imediatamente com o identificador da execução e o estado
   `em_andamento`, sem esperar o fim.
4. **Given** uma requisição de início sem o token local, **When** ela chega,
   **Then** é recusada antes de qualquer trabalho começar.
5. **Given** uma requisição de início vinda de outra origem — uma página aberta
   no navegador —, **When** ela chega, **Then** é recusada pela verificação de
   origem, ainda que traga o token.

---

### User Story 4 - Saber se duas execuções são comparáveis (Priority: P2)

O mantenedor compara duas execuções e a interface lhe diz se a comparação se
sustenta.

**Why this priority**: uma comparação inválida apresentada como válida é pior do
que nenhuma comparação — e é o que acontece hoje.

**Independent Test**: pedir a comparação entre uma execução do grupo idêntico e
uma das outras, e conferir que a resposta declara a cerca igual e a comparação
não confiável.

**Acceptance Scenarios**:

1. **Given** duas execuções com cercas iguais e resultados diferentes, **When** a
   comparação é pedida, **Then** a resposta traz os limites lado a lado, marca-os
   como iguais, e declara a comparação **não confiável** por falta da versão do
   código.
2. **Given** duas execuções cujas cercas divergem, **When** a comparação é
   pedida, **Then** cada divergência vem nomeada, campo a campo.

---

### User Story 5 - Acompanhar a coleta de histórico (Priority: P3)

O mantenedor dispara uma coleta e vê o progresso enquanto ela acontece.

**Why this priority**: é a única espera que a interface mede — 525.600 velas
vindas da rede. Um backtest de doze meses leva de 2 a 4 segundos; a coleta, não.

**Independent Test**: disparar uma coleta e conferir que a rota de progresso
devolve avanço monotônico e termina com o total coletado.

**Acceptance Scenarios**:

1. **Given** uma coleta em andamento, **When** a interface consulta o progresso,
   **Then** recebe quantas velas já vieram e o intervalo já coberto.
2. **Given** uma coleta terminada, **When** a interface consulta o progresso,
   **Then** recebe o estado final e o total, e a consulta seguinte devolve o
   mesmo — a rota é idempotente depois do fim.

---

### Edge Cases

- **`profit_factor` nulo**: execução sem operação perdedora tem `profit_factor`
  indefinido. A resposta traz `null` e um campo que diz por quê; **não** traz
  zero, infinito nem string vazia.
- **Execução interrompida**: `outcome` diferente de `completed` vem com o
  `halt_reason`, e a resposta marca que o período **não** foi completado.
- **Execução sem nenhuma operação**: as métricas vêm zeradas e distinguíveis de
  ausentes; a lista não omite a execução.
- **`runs.db` ausente ou ilegível**: o servidor recusa subir com mensagem que
  diga qual arquivo faltou — não sobe devolvendo lista vazia.
- **Execução em andamento consultada**: as rotas de leitura devolvem o que já
  está gravado e marcam a execução como em andamento.
- **Porta ocupada**: o servidor falha ao subir dizendo qual porta, e não escolhe
  outra sozinho.

## Requirements *(mandatory)*

### Functional Requirements

Cada requisito traz entre parênteses o requisito de interface que o originou, do
projeto de desenho.

- **FR-001**: O servidor MUST iniciar execução pela mesma composição que o
  `trade backtest` usa, sem montar o executor por outro caminho (`REQ-UI-034`).
- **FR-002**: O servidor MUST serializar valor monetário, de quantidade e de
  preço como **string** JSON. Número JSON MUST NOT aparecer em campo que alcance
  saldo, posição, P&L, risco ou ordem. Contagens e identificadores seguem número
  (`REQ-UI-035`).
- **FR-003**: O servidor MUST escutar exclusivamente em `127.0.0.1`, com porta
  explícita, sem padrão que exponha a máquina (`REQ-UI-036`).
- **FR-004**: Requisição que inicie execução MUST exigir o modo declarado, e a
  ausência MUST ser recusada como erro de uso. `live` MUST NOT ser iniciável
  pelo servidor (`REQ-UI-037`).
- **FR-005**: O servidor MUST NOT expor rota que emita ordem, altere limite ou
  taxa, acione ou libere o kill switch, ou retome execução parada. A superfície
  de escrita é **uma**: iniciar (`REQ-UI-038`).
- **FR-006**: Toda resposta que descreva uma execução MUST trazer o campo da
  versão do código que a produziu. Enquanto o registro não o tiver, o campo MUST
  vir **presente e nulo**, nunca omitido (`REQ-UI-044`).
- **FR-007**: A comparação entre execuções MUST declarar se é confiável, e MUST
  ser marcada como não confiável enquanto a versão do código não estiver no
  registro — ainda que limites e taxas sejam idênticos (`REQ-UI-039`).
- **FR-008**: Todo agregado MUST ser calculado no servidor, em decimal exato, e
  chegar pronto como string: bruto, razão custo sobre prejuízo, P&L por dia,
  pior dia, dias que romperam o limite, curva acumulada, acerto antes e depois
  do custo, e o estado das cinco exigências da Porta 1 (`REQ-UI-010`,
  `REQ-UI-011`).
- **FR-009**: O extrato MUST ser devolvido agrupado por episódio de posição —
  entrada, fechamento, número de saídas e resultado somado —, e as saídas de um
  episódio MUST ser pedidas à parte (`REQ-UI-029`).
- **FR-010**: Cada episódio MUST trazer o que o fechou: sinal da estratégia ou
  prazo de 72 horas. Enquanto o registro não distinguir, o campo MUST vir
  presente e nulo (`REQ-UI-042`).
- **FR-011**: Cada linha de saída MUST trazer a taxa ao lado do resultado
  (`REQ-UI-030`).
- **FR-012**: Toda linha do tempo MUST ser ordenada por `seq`, nunca por `at_ms`
  (`fontes-de-dados.md`).
- **FR-013**: O elo `risk_decision` MUST trazer, para cada limite, o valor
  observado ao lado do teto (`REQ-UI-025`).
- **FR-014**: Divergência entre quantidade pedida e preenchida MUST ser
  assinalada no elo `fill`, com a proporção (`REQ-UI-027`).
- **FR-015**: A resposta de histórico MUST trazer procedência e data da coleta, e
  MUST trazer as lacunas como lacunas, jamais interpoladas (`REQ-UI-007`).
- **FR-016**: O servidor MUST distinguir, em toda resposta, o que vem do
  `market.db`, que é cache reconstruível, do que vem do `runs.db`, que é
  insubstituível (`REQ-UI-028`).
- **FR-017**: Rota que inicia trabalho MUST responder imediatamente com o
  identificador e o estado, sem esperar a conclusão (decisão 020).
- **FR-018**: O progresso MUST ser obtido por consulta a uma rota de estado. O
  servidor MUST NOT manter conexão aberta para empurrar eventos (decisão 020).
- **FR-019**: A rota de escrita MUST exigir token local e MUST recusar
  requisição cujo `Origin` ou `Referer` aponte para outro lugar (decisão 021).
  O ciclo de vida do token é parte do requisito:
  - MUST ser gerado a cada início do servidor, de fonte aleatória própria para
    uso criptográfico, e MUST NOT ser derivado de caminho, porta ou horário;
  - MUST ser impresso no terminal que subiu o servidor, e MAY ser gravado em
    arquivo local de permissão `0600` para a interface ler;
  - MUST morrer com o processo: reiniciar o servidor invalida o token anterior,
    sem prazo de expiração próprio — o processo é o prazo;
  - MUST NOT aparecer no registro de auditoria, em log, em mensagem de erro nem
    em resposta de rota alguma (Princípio IV e Princípio VI);
  - requisição de **leitura** MUST NOT exigi-lo: quem lê não muda nada, e exigir
    token para ler faria a interface guardá-lo onde não precisa.
- **FR-020**: Nenhuma rota MAY aceitar, guardar ou devolver credencial. O token
  local do FR-019 não é credencial de corretora e MUST NOT ser gravado no
  registro (Princípio VI).
- **FR-021**: `profit_factor` nulo MUST ser devolvido como `null` acompanhado do
  motivo, e MUST NOT ser convertido em zero, infinito ou texto.
- **FR-022**: Erro MUST dizer o que falta e o comando que resolve — para
  histórico insuficiente, o trecho ausente e o `trade collect` com as datas já
  preenchidas (`REQ-UI-032`).
- **FR-023**: Toda recusa MUST usar o mesmo corpo, com código próprio, para que
  a tela distinga uma da outra sem interpretar texto:

  ```json
  { "erro": { "codigo": "historico_insuficiente",
              "mensagem": "faltam velas de 2025-09-20 a 2025-10-04",
              "o_que_falta": { "de_ms": 1758326400000, "ate_ms": 1759536000000 },
              "comando": "trade collect --symbol BTCUSDT --interval 1m --from 2025-09-20 --to 2025-10-04" } }
  ```

  | `codigo` | Situação | HTTP |
  |---|---|---|
  | `modo_ausente` | início sem modo declarado | 400 |
  | `modo_recusado` | pediu `live` | 403 |
  | `token_ausente` ou `token_invalido` | escrita sem token válido | 401 |
  | `origem_recusada` | `Origin`/`Referer` de outro lugar | 403 |
  | `historico_insuficiente` | falta vela no período pedido | 409 |
  | `execucao_desconhecida` | identificador que não existe | 404 |
  | `registro_indisponivel` | `runs.db` ilegível | 503 |

  `comando` MUST vir presente e nulo quando não houver comando que resolva —
  ausência declarada, pelo mesmo motivo do FR-006.
- **FR-024**: As cinco exigências da Porta 1, que o servidor MUST devolver
  calculadas, são as da constitution, e o servidor MUST lê-las de configuração,
  nunca de constante embutida — a própria constitution manda tratá-las como
  valores de partida ajustáveis:

  | # | Exigência | Valor de partida |
  |---|---|---|
  | 1 | cobertura mínima de histórico, incluindo um mercado de baixa sustentado e um evento de alta volatilidade | ≥ 12 meses |
  | 2 | número de operações | ≥ 100 |
  | 3 | taxas e slippage modelados explicitamente | presentes e discriminados |
  | 4 | profit factor | ≥ 1,3 |
  | 5 | drawdown máximo | ≤ 15% do capital |

  Cada exigência MUST vir com o valor observado ao lado do exigido, e com o
  veredito — é o `REQ-UI-006`, e é a mesma regra do `REQ-UI-025`: o veredito diz
  que passou, a folga diz o quanto faltava.
- **FR-025**: Toda rota que devolva coleção MUST declarar teto, e MUST dizer na
  resposta quando cortou. Os tetos de partida: episódios, sem teto — são 146 ou
  14.306, e ambos cabem; saídas de um episódio, 500 por resposta, com o maior
  episódio observado tendo 432; eventos de progresso, os 100 mais recentes. A
  cadeia de uma decisão não tem teto porque tem tamanho fixo: cinco elos.

### Key Entities

- **Grupo de execuções**: conjunto de execuções de resultado idêntico, com os
  identificadores, a contagem, a cerca comum e a marca de comparabilidade.
- **Execução**: modo, símbolo, intervalo, período, capital inicial, cerca,
  estratégia e parâmetros, início e fim, desfecho, razão da parada, versão do
  código (hoje nula).
- **Episódio de posição**: entrada, fechamento, causa do fechamento, número de
  saídas, resultado somado, taxas somadas.
- **Elo da cadeia**: tipo, `seq`, instante, e o conteúdo próprio do tipo —
  entradas do sinal, ordem derivada, veredito de risco com folgas,
  preenchimento com divergência, transição de estado com a aritmética.
- **Cobertura de histórico**: símbolo, intervalo, procedência, primeiro e último
  instante, data da coleta, lacunas.
- **Trabalho em andamento**: identificador, tipo, estado, progresso, começo, fim.

## Contrato das rotas

Formato de todas as respostas: JSON, UTF-8. **Valor monetário, quantidade e
preço são string.** Contagem, `seq` e identificador numérico são número.

| Rota | O que devolve |
|---|---|
| `GET /runs` | grupos de execuções, com cerca e comparabilidade |
| `GET /runs/{id}` | métricas, decomposição do custo, estado da Porta 1 |
| `GET /runs/{id}/daily` | P&L por dia, pior dia, dias que romperam o limite |
| `GET /runs/{id}/episodes` | episódios de posição |
| `GET /runs/{id}/episodes/{seq}/saidas` | as saídas de um episódio |
| `GET /runs/{id}/chain/{seq}` | os cinco elos de uma decisão |
| `GET /runs/compare?a={id}&b={id}` | as duas cercas lado a lado, e se a comparação se sustenta |
| `GET /datasets` | cobertura, procedência, lacunas |
| `GET /runs/match` | o aviso de repetição, antes de iniciar |
| `POST /runs` | **a única escrita** — inicia execução |
| `GET /jobs/{id}` | progresso de trabalho em andamento |

### `GET /runs`

```json
{
  "grupos": [
    {
      "identicas": 6,
      "execucoes": ["01M2ZG1ENV7PMGQS5AK4PQ1M3Z", "01M2ZG2A0KYKB4A5DS40ZB13MJ"],
      "modo": "backtest",
      "simbolo": "BTCUSDT",
      "intervalo": "1m",
      "periodo": { "de_ms": 1758326400000, "ate_ms": 1789862340000 },
      "desfecho": "completed",
      "periodo_completado": true,
      "metricas": {
        "net_result": "-9871.30084728",
        "profit_factor": "0.07054142259174230675942491",
        "max_drawdown": "9871.30084728",
        "trade_count": 14308
      },
      "cerca": {
        "limits": { "max_daily_loss": "200.00", "max_position_size": "1000.00",
                    "max_total_exposure": "2000.00", "max_orders_per_window": 10,
                    "window_minutes": 60 },
        "fees": { "taker_fee_rate": "0.001", "slippage_rate": "0.0005" }
      },
      "versao_do_codigo": null,
      "comparavel": false,
      "por_que_nao_comparavel": "o registro não guarda a versão do código que produziu a execução"
    }
  ],
  "fonte": "runs.db",
  "natureza_da_fonte": "insubstituivel"
}
```

### `GET /runs/{id}/episodes`

```json
{
  "execucao": "01M30EXVCXVQ63XS0SSRSPDRJ8",
  "episodios": [
    {
      "seq_entrada": 12,
      "entrada_ms": 1758327000000,
      "fechamento_ms": 1758542400000,
      "duracao_horas": "59.7",
      "fechado_por": null,
      "saidas": 98,
      "resultado": "-64.21883411",
      "taxas": "43.09112004"
    }
  ],
  "total_de_episodios": 146,
  "aviso": "fechado_por é nulo em todas: o registro não distingue fechamento por sinal de fechamento por prazo"
}
```

### `POST /runs`

Requisição:

```json
{ "modo": "backtest", "simbolo": "BTCUSDT", "intervalo": "1m",
  "de_ms": 1758326400000, "ate_ms": 1789862340000,
  "capital_inicial": "10000", "estrategia": "sma-cross",
  "parametros": { "fast": "9", "slow": "21", "position_fraction": "0.10" } }
```

Resposta, imediata:

```json
{ "trabalho": "job_01M30F…", "execucao": "01M30F…", "estado": "em_andamento" }
```

Recusas, cada uma com código próprio: modo ausente, modo `live`, token ausente
ou inválido, origem não permitida, histórico insuficiente — esta última com o
trecho que falta e o `trade collect` já preenchido.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: O mantenedor reconstitui qualquer operação do extrato até o sinal
  que a originou, pela interface, sem escrever uma linha de SQL.
- **SC-002**: Varrendo toda resposta do servidor, **zero** valores monetários, de
  quantidade ou de preço aparecem como número JSON.
- **SC-003**: Em 100% das tentativas, requisição sem modo é recusada, e nenhuma
  execução começa.
- **SC-004**: Em 100% das tentativas, requisição pedindo `live` é recusada, com
  mensagem distinta da anterior.
- **SC-005**: Em 100% das tentativas, requisição de escrita sem token válido é
  recusada antes de qualquer trabalho começar.
- **SC-006**: Em 100% das tentativas, requisição de escrita com `Origin` de
  outro lugar é recusada, mesmo com token válido.
- **SC-007**: O servidor não aceita conexão em nenhum endereço que não seja
  `127.0.0.1`, conferível por tentativa a partir de outra interface de rede.
- **SC-008**: O servidor não declara dependência que alcance a corretora fora da
  composição do `trade-cli`, conferível pelo `tests/architecture.rs`.
- **SC-009**: Em 100% das respostas que descrevem execução, o campo da versão do
  código está presente — nulo enquanto o registro não o tiver, nunca ausente.
- **SC-010**: Duas execuções com cercas idênticas e resultados diferentes são
  devolvidas como **não comparáveis** em 100% dos casos.
- **SC-011**: A soma dos resultados dos episódios devolvidos coincide
  exatamente com o `net_result` da execução, divergência zero.
- **SC-012**: Nenhuma resposta do servidor contém credencial, em nenhuma rota,
  conferível por varredura.
- **SC-013**: Requisição de início responde sem esperar a execução terminar —
  conferível por uma execução cujo fim demore, comparando o instante da resposta
  com o instante do fim gravado. **Não há limiar de tempo medido aqui**: o teto
  de 100 ms que a versão anterior desta spec trazia era número escolhido, não
  medido, e virou configuração de partida (`limiar_resposta_inicio_ms`), a ser
  calibrada contra a máquina real como a constitution manda calibrar limiar.
- **SC-014**: Toda rota de leitura responde sobre a maior execução gravada —
  143.135 eventos, 14.308 linhas de extrato — sem carregar a execução inteira em
  memória, conferível por medição de memória durante a resposta. O tempo é
  registrado a cada medição e vira o valor de partida de
  `limiar_resposta_leitura_ms`; o **único** número já medido, e declarado como
  tal, é o do backtest de doze meses: de 2 a 4 segundos do início ao fim, lido
  em `started_at`/`ended_at` das nove execuções em 2026-09-21.
- **SC-015**: `profit_factor` indefinido é devolvido como `null` com motivo em
  100% dos casos, e nunca como zero ou infinito.

## Assumptions

- O servidor lê os mesmos arquivos que o CLI escreve, na mesma máquina. Não há
  banco remoto, não há sincronização, não há segundo processo escrevendo.
- Um leitor pode ler enquanto uma execução escreve; o SQLite em modo WAL dá
  conta disso, e as rotas de leitura devolvem o que já está gravado.
- A versão do código na tabela `run` **não existe hoje**. Esta spec assume que
  ela pode passar a existir, e exige apenas que a resposta seja honesta
  enquanto não existir.
- O prazo de 72 horas da constitution 2.0.0 ainda não é implementado por
  nenhuma execução gravada; o campo `fechado_por` nasce nulo pela mesma razão.
- A interface é operada pelo mantenedor, na máquina dele. Não há múltiplos
  usuários, não há sessão, não há perfil.
- **Nenhum limiar de desempenho desta spec foi medido**, e por isso nenhum
  aparece como critério de aceitação com número. Os dois que existem nascem como
  configuração de partida, e a primeira medição contra a máquina real é que os
  fixa — o mesmo tratamento que a constitution dá aos limiares das portas de
  promoção.

## Fora de escopo

- Emitir ordem, alterar limite ou taxa, acionar ou liberar o kill switch,
  retomar execução parada — nada disso ganha rota, nem agora nem depois.
- Iniciar `live`. A promoção continua sendo ato humano registrado, fora daqui.
- Acompanhar execução `paper` ao vivo: o modo existe no Trade desde
  2026-09-20, mas o que ele grava ainda não foi levantado pelo projeto de
  desenho, e desenhar contra dado não levantado é inventar dado.
- Recovery: a constitution 2.0.0 o admite, nenhuma execução o usa, e as rotas do
  painel do limite só o expõem quando ele existir — `REQ-UI-040` e `REQ-UI-041`
  ficam registrados para esse momento.
- Autenticação de usuário, perfis, acesso remoto, TLS. O servidor é local e de
  um dono só.
