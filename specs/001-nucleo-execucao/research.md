# Phase 0 — Research: Núcleo de Execução e Risco

**Feature**: 001-nucleo-execucao | **Date**: 2026-09-20

Todas as incógnitas do Technical Context foram resolvidas. Nenhuma NEEDS
CLARIFICATION permanece.

---

## R-001 — Linguagem e ecossistema

**Decision**: Rust estável, edition 2024, workspace multi-crate.

**Rationale**: decisão do mantenedor. O ganho concreto que a linguagem traz a este
projeto específico é que **os Princípios II e V deixam de ser convenção**. Em Rust, o
grafo de dependências entre crates é verificado pelo compilador: se `trade-strategy`
não declara `trade-bybit` como dependência, nenhuma linha de estratégia consegue
importar a corretora, hoje ou daqui a um ano. A ausência de exceções e a exaustividade
obrigatória de `match` também casam bem com a classificação de anomalia (FR-024a/b),
onde esquecer um caso é justamente o tipo de defeito que custa dinheiro.

**Alternatives considered**:

- **Python** — recomendação original. Ecossistema de backtest muito mais maduro
  (pandas, numpy), iteração mais rápida, e a diferença de desempenho é irrelevante em
  velas de minuto. Rejeitado pelo mantenedor.
- **TypeScript/Node** — tipagem estática ajuda, mas o ecossistema de séries temporais
  é pobre e o `number` é IEEE-754, o que exigiria biblioteca decimal de qualquer modo.
- **Custo aceito**: mais linhas por funcionalidade, ecossistema de trading jovem em
  Rust, e curva de aprendizado maior. O plano compensa isso mantendo o escopo desta
  feature deliberadamente pequeno.

---

## R-002 — Aritmética monetária

**Decision**: `rust_decimal::Decimal` para todo preço, quantidade, saldo e resultado.
Nenhum `f32`/`f64` em qualquer caminho que produza valor financeiro.

**Rationale**: erro de ponto flutuante não levanta exceção — ele se acumula e reaparece
como **divergência entre a posição calculada e a posição reportada pela corretora**, que
é exatamente a anomalia de integridade que o Princípio II manda tratar como parada. Usar
`f64` seria construir a causa da parada dentro do detector dela. Além disso, a Bybit
devolve preços como **string** (ver R-005): converter string → Decimal é direto e exato,
enquanto string → f64 → Decimal introduz erro que não existia na fonte.

**Alternatives considered**:

- **Inteiro de ponto fixo (satoshis)** — exato e rápido, mas a escala varia entre preço
  (2 casas em USDT) e quantidade (até 8 casas em BTC), e cada operação exigiria
  reescalonamento manual. Fonte de erro por descuido.
- **`f64`** — rejeitado acima.

---

## R-003 — Persistência

**Decision**: SQLite via `rusqlite` com a feature `bundled`, em dois arquivos —
`data/market.db` (histórico) e `data/runs.db` (execuções e auditoria).

**Rationale**: decisão do mantenedor quanto ao SQLite, e ela resolve de graça o
requisito mais chato da coleta: **retomada sem duplicar (FR-014)**. Uma chave primária
em `(symbol, interval, open_time)` com `INSERT OR IGNORE` torna a coleta idempotente sem
nenhuma lógica de deduplicação escrita à mão. O ganho colateral é que o mantenedor
inspeciona o histórico em SQL pelo DBeaver, já instalado.

A feature `bundled` compila o SQLite dentro do binário: sem dependência de biblioteca do
sistema, o que mantém a promessa de binário único.

**Dois arquivos, não um**: o histórico é cache reconstruível da fonte; a auditoria é
insubstituível. Num arquivo só, "limpar o cache" e "destruir a auditoria" seriam a mesma
operação — inaceitável sob o Princípio IV.

**Alternatives considered**:

- **Parquet** — menor e mais rápido para varredura, mas coleta incremental exige
  gerenciar partições e reescrever arquivos, e não há chave primária que impeça
  duplicata. Mais código para satisfazer FR-014.
- **CSV** — nada impede linha duplicada; leitura lenta; espaço desproporcional.

---

## R-004 — Cliente HTTP da coleta

**Decision**: `ureq`, bloqueante. Nenhum runtime assíncrono nesta feature.

**Rationale**: o sistema inteiro é síncrono — o backtest é um laço sobre um cursor, e o
coletor faz algumas centenas de requisições sequenciais (ver R-005). Trazer `tokio` e
`reqwest` adicionaria um runtime assíncrono, `async` contaminando as assinaturas, e uma
fonte de não determinismo num sistema cujo requisito central é ser determinístico
(FR-029). `ureq` é bloqueante, tem poucas dependências e resolve o caso.

**Quando isso muda**: execução ao vivo precisará de WebSocket e provavelmente de um
runtime assíncrono. Essa é uma feature futura, e a troca ficará contida no adaptador da
corretora — que é exatamente o que a abstração do Princípio V existe para permitir.
Antecipar agora seria complexidade sem necessidade presente, contra a cláusula de
simplicidade da constitution.

**Alternatives considered**: `reqwest` + `tokio` — padrão do ecossistema, mas custo
desproporcional para requisições sequenciais bloqueantes.

---

## R-005 — Fonte do histórico: Bybit V5 Get Kline

**Decision**: `GET /v5/market/kline` com `category=spot`, `symbol=BTCUSDT`, paginação
para frente por `start`, `limit=1000`.

**Rationale**: verificado na documentação oficial em 2026-09-20. Fatos que moldam o
desenho do coletor:

| Fato documentado | Consequência no desenho |
|---|---|
| Endpoint **público, sem autenticação** | Confirma FR-012 — nenhuma credencial na coleta, Princípio VI intocado nesta feature |
| `category` aceita `spot`, `linear`, `inverse`, e **assume `linear` se omitido** | `category=spot` MUST ser enviado explicitamente. Omitir traria dados de perpétuo — outro mercado, contra FR-004 |
| `limit` máximo **1000**, padrão 200 | 12 meses de velas de 1 minuto ≈ 525.600 velas ≈ **526 requisições**. Paginação obrigatória |
| A resposta traz as `limit` velas **mais recentes** da janela pedida, e `end` é **inclusivo** | Verificado contra a API em 2026-09-20, **não** pela documentação. Ver a nota abaixo |
| Preços retornam como **string** | Converter direto para `Decimal`, sem passar por float (ver R-002) |
| Lista ordenada **do mais recente para o mais antigo** | O coletor inverte cada página antes de gravar. Errar isso produziria histórico invertido, que o backtest consumiria sem perceber |
| `closePrice` é *"the last traded price when the candle is not closed"* | **A vela corrente é parcial.** O coletor MUST descartar a última vela quando ela ainda não fechou |

**A armadilha da paginação** só apareceu numa coleta real, e a documentação não
a descreve. Pedir `start=00:00`, `end=24:00`, `limit=1000` para um dia de velas
de um minuto **não** devolve as mil primeiras: devolve as mil **últimas** da
janela, de 07:21 em diante. A paginação para frente encerra achando que
terminou, e 441 minutos nunca são buscados — sem erro, sem aviso, com a
resposta parecendo completa.

Some-se a isso que `end` é **inclusivo**: uma janela de mil passos contém 1001
velas, e o limite descarta a mais antiga — justamente a que se queria.

A correção é limitar a **janela** de cada requisição, e não só o cursor: cada
página pede `[cursor, cursor + (limit-1) passos]`, que contém exatamente
`limit` velas. Assim "as mais recentes da janela" e "as mais antigas a partir
do cursor" passam a ser o mesmo conjunto.

**A armadilha da vela aberta** merece destaque: gravar a vela em formação significa
gravar um preço de fechamento que ainda vai mudar. Duas coletas do mesmo período
produziriam históricos diferentes, quebrando o determinismo (FR-029), e um backtest
enxergaria um fechamento que não existiu — informação que no instante simulado não
estava disponível, violando FR-030. O coletor descarta toda vela cujo intervalo não
tenha terminado por completo.

**Alternatives considered**: arquivo de histórico fornecido manualmente pelo mantenedor
— descartado na especificação; download de dumps públicos de terceiros — acrescenta
dependência de fonte não oficial sem ganho.

---

## R-006 — Limite de requisições e retentativa

**Decision**: respeitar o limite de IP documentado, tratar `retCode: 10006` e HTTP 403
como falha transitória com recuo exponencial, e ler os cabeçalhos de quota.

**Rationale**: a documentação indica **600 requisições em janela de 5 segundos por IP**,
e retorno `retCode: 10006` / `retMsg: "Too many visits!"` com HTTP 403 quando excedido.
Os cabeçalhos `X-Bapi-Limit`, `X-Bapi-Limit-Status` e `X-Bapi-Limit-Reset-Timestamp`
informam a quota restante.

A implicação é tranquilizadora: as ~526 requisições de uma coleta anual cabem
folgadamente no limite. **A restrição real não é vazão, é educação** — o coletor não
precisa de paralelismo nem de otimização, precisa apenas não ser rude e saber recuar.

Isso liga direto à classificação de FR-024a: indisponibilidade momentânea e excesso de
requisições são **falha transitória**, retentadas automaticamente até um máximo
configurável. Esgotado o máximo, viram **falha de integridade** (FR-024b) e a coleta para
reportando, sem corromper o que já gravou (FR-015) — garantido pela gravação
transacional por página.

---

## R-007 — Fronteira estratégia / execução

**Decision**: a estratégia não recebe nem pode obter uma referência ao executor de
ordens. Ela implementa `Strategy`, que recebe o estado do mercado e devolve
`Option<Signal>`. O motor converte sinal em ordem e a encaminha ao `RiskGuard`, que
**possui** o executor interno.

**Rationale**: FR-018 exige que não exista caminho alternativo até a corretora, e FR-021
exige que a estratégia não possa alterar limites. Implementado como regra de revisão,
isso depende de alguém reparar. Implementado como tipo, é impossível: `trade-strategy`
não declara dependência de `OrderExecutor`, e o executor concreto é movido para dentro
do `RiskGuard` na composição — quem quiser o executor tem que atravessar o guard, porque
não há outra referência viva para ele.

**Alternatives considered**:

- **Estratégia envia ordens através de um handle "seguro"** — o handle precisaria ser
  público para a estratégia usar, e o que é público pode ser usado errado. Troca uma
  garantia de compilação por uma convenção.
- **Verificação de risco dentro do executor** — misturaria as duas responsabilidades e
  tornaria impossível testar risco sem um executor.

---

## R-008 — Determinismo

**Decision**: tempo injetado por `Clock`; percurso de velas por cursor ordenado;
nenhuma iteração sobre `HashMap` em caminho que afete resultado; nenhum paralelismo no
motor.

**Rationale**: FR-029 exige resultado idêntico para entrada idêntica, e SC-004 o mede
em 100% das execuções. As três fontes clássicas de não determinismo em Rust são relógio
do sistema, ordem de iteração de `HashMap` (aleatorizada por semente a cada processo) e
escalonamento de tarefas concorrentes. As três são eliminadas por construção, não por
cuidado.

Onde ordenação estável importa dentro do motor, usa-se `BTreeMap` ou `Vec` ordenado.

---

## R-009 — Estratégia de referência

**Decision**: cruzamento de duas médias móveis simples sobre o preço de fechamento,
com parâmetros configuráveis; compra ao cruzar para cima, vende ao cruzar para baixo.

**Rationale**: FR-037 pede comportamento previsível para exercitar o ciclo, e FR-038
proíbe apresentá-la como estratégia de investimento. Média móvel serve porque o
resultado é calculável à mão sobre um conjunto pequeno — o que permite o teste de
aceitação da US1, em que o resultado do motor é conferido contra cálculo manual. É
também o exemplo mais banal do gênero, o que reduz o risco de alguém confundi-la com
recomendação.

**Alternatives considered**: estratégia que compra em todo candle — exercita o ciclo,
mas não produz operações fechadas suficientes para as métricas terem significado; sinal
aleatório com semente fixa — determinístico, mas não permite conferência manual
intuitiva.
