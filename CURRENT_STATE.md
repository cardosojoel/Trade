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

**Fronteira declarada: o desenho pode andar em paralelo sem tocar no código.**
`MD BOT/00_FRONTEIRA.md` fixa autoridade em três níveis — constitution governa,
`CLAUDE.md` e invariantes do build cobram, o conjunto de desenho **propõe** — e
uma saída única e unidirecional: proposta vira ADR em `MD BOT/34_ADR_EMENDAS.md`
e só chega ao código depois de aceita, como emenda ou como feature nova pelo
Spec Kit. Todo documento normativo passou a declarar `Conformidade: conforme` ou
`exige emenda (ADR-NNN)`; seis declaram o segundo. A constitution está ancorada
por hash (`82b24e3c…`), e hash diferente obriga a reavaliar todas as
conformidades.

**A verificação da fronteira desmentiu uma coisa que este arquivo vinha
afirmando.** Eu repeti quatro vezes que havia "quatro conflitos com a
constitution". São quatro divergências, mas **só duas são com a constitution**:

| Divergência | Onde a regra contrária realmente vive |
|---|---|
| Recovery depois do freio | constitution, Princípio II, linha 44 |
| Retomada por confirmação humana | constitution, Princípio II, linha 45 |
| Derivativos e alavancagem | `CLAUDE.md` linhas 91–92 — **não está na constitution** |
| `f64` em caminho monetário | `CLAUDE.md` e `tests/no_float.rs` — **não está na constitution** |

A constitution não menciona spot, alavancagem, venda a descoberto, float nem
decimal em nenhuma das suas 220 linhas. Isso é uma lacuna, não um detalhe: as
duas regras que limitam a perda máxima possível — o pior caso ser o depósito, e
o cálculo do saldo estar certo — vivem num arquivo de diretrizes que qualquer
sessão reescreve sem racional nem aprovação. A segunda ao menos tem o
`tests/no_float.rs` barrando o merge; **a primeira não tem nenhuma proteção
executável.** A ADR-005 propõe promovê-las à constitution; é decisão sua.

**Portabilidade medida:** o conjunto tem oito referências para fora dele, em 45
documentos. Movê-lo para outro repositório é `git mv` mais a conversão dessas
oito em citação com origem declarada. Nada mais — não há inclusão de arquivo,
script compartilhado nem dependência de build. A decisão de mover continua
aberta e ficou barata nos dois sentidos.

**Auditoria de risco contra a documentação da Bybit — 2026-09-20.** Onze
achados; o relatório completo está publicado como artefato privado
(`claude.ai/artifact/HSzcFir83CmzKVvLKzyLFk`) e os três acionáveis viraram
requisito:

| Achado | Virou | O que estabelece |
|---|---|---|
| Pó da moeda base | `REQ-SIZING-004`, `REQ-BYBIT-004/005` | a taxa do spot é cobrada **em BTC** e a quantidade recebida não é múltiplo de `qty_step`; o resíduo é saldo e soma-se à ordem seguinte |
| Piso muda dias 3 e 17 | `REQ-BYBIT-006/007/008` | instrumento relido a cada sessão; perfil invalidado impede o início; `min_order_qty` está deprecado |
| Banda de preço | `REQ-EXEC-010/011` | stop vai a mercado no gatilho; posição sem stop aceito não existe |

**O número que a auditoria mudou:** sem o tratamento do resíduo, o custo real
por operação sai de 0,25% para **0,94%** numa posição de US$ 5,88. A banca de
US$ 10 deixa de existir — empatar exigiria **89%** de acerto — e a de US$ 100 vê
a barra subir de 59,4% para **63,0%**. Daí o `REQ-CFG-007`: enquanto o resíduo
não for tratado, depósito abaixo de US$ 160 não é executável.

Duas premissas foram confirmadas, e uma delas era a mais perigosa: **o stop
existe no mercado à vista da Bybit** — se não existisse, todo o modelo de risco
cairia. E os doze meses coletados **cumprem o requisito de dados da Porta 1**:
queda de 54,1% em 267 dias e o par −14,02%/+11,92% de fevereiro de 2026. É o
único requisito formal de validação que o projeto cumpre hoje.

O que a auditoria não alcançou, por não ter credencial: taxa efetiva da conta,
tipo de conta e permissões da chave. A taxa é 80% do custo — confirmá-la muda
mais o resultado do que qualquer recalibração.

**Os primeiros valores com origem medida.** O
`MD BOT/27_CONFIGURATION_REGISTRY.md` deixou de ser só contrato e passou a
guardar dois perfis completos — `banca-12` e `banca-100` — derivados de medição
sobre os doze meses do `market.db` e dos limites reais do instrumento, lidos da
API da Bybit (`minOrderAmt` 5 USDT).

O que a medição estabeleceu, e vale para qualquer capital:

| Medição | Resultado |
|---|---|
| Custo ÷ movimento mediano do BTC | 5 min **4,5×** · 1 h **1,3×** · 4 h 0,61× · 24 h 0,19× |
| Entrada aleatória, bracket 1:1, doze meses | perde 0,26% a 0,38% por operação — o custo, mais assimetria |
| Acerto que a estratégia precisa adicionar sobre o acaso | ~14 pontos percentuais |

Abaixo de uma hora o custo excede o movimento típico do ativo. Isso não é
característica da `sma-cross` — é aritmética do par custo/volatilidade, e
nenhum ajuste de parâmetro a contorna. É a resposta quantificada para a dúvida
que já estava registrada aqui sobre a granularidade de um minuto.

**O projeto passou a ter um número que nunca teve: o resultado esperado sem
vantagem.** Simulação Monte Carlo sobre a distribuição medida — 8.736 operações
reamostradas em 20.000 execuções de 30 dias, com custo, limite diário, parada
por drawdown e chão de operação aplicados — diz que uma banca de $100 termina o
mês em **−8,5%** se a estratégia acertar a hora de entrar tão bem quanto o
acaso, e que 16% das contas batem a parada por drawdown antes do fim do mês.
Empatar exige **59,4%** de acerto entre as operações que resolvem, contra os
46,5% que o acaso entrega: uma lacuna de **12,9 pontos percentuais**. Esse
número substitui `profit factor ≥ 1.3` como alvo de desenvolvimento, porque é
medível hoje contra os mesmos dados, sem esperar por porta nenhuma.

A pessoa informa apenas o depósito; stop, teto, risco, prazo e limites são
derivados por função registrada. Acima de $11,76 os percentuais **congelam** —
$200, $500 e $900 recebem configuração idêntica. Capital maior não baixa a barra
de acerto: compra granularidade, saída parcial e folga até o chão.

Em spot vale também `risco por operação ≤ distância do stop`, sempre, porque o
teto de posição não passa de 100%. Com banca pequena o teto sobe e o stop encurta, mas a
derivação correta mantém o acerto necessário praticamente constante — a primeira
versão deste registro dizia que capital maior baixava a barra de 63,9% para
60,6%, e isso **estava errado**: vinha de um teto fixado à mão em 100%, não
derivado. A correção está registrada no `27_CONFIGURATION_REGISTRY.md`.

Também em 2026-09-20: os requisitos passaram a ter identificador. **127
`REQ-<DOMÍNIO>-<NNN>` em 26 domínios**, escritos ao lado da regra na
especificação — nunca em lista à parte, que seria uma segunda camada a manter —
com linguagem RFC 2119. O `26_REQUIREMENTS_TRACEABILITY_MATRIX.md` deixou de ter
seis linhas de exemplo e passou a catalogar os 127, todos em `SPECIFIED`: nenhum
aponta para módulo Rust, teste ou métrica, porque o código existente nasceu da
constitution e da `specs/001-nucleo-execucao/`, não desta SDD. Os 41 documentos
ganharam cabeçalho de estado, e as autoavaliações que viviam dentro das
especificações ("Estado: forte", "não é estado da arte ainda") foram para a
auditoria, que é onde se avalia.

**O exemplo fim a fim foi escrito, e mudou o que se sabe sobre os limiares.**
O `31_EXEMPLO_FIM_A_FIM.md` atravessa a cadeia inteira com os mesmos números e
expõe três coisas que nenhum documento via sozinho:

1. `PositionNotional = AllowedTradeRisk / StopDistance%`. Com risco de 2% e stop
   de 1,2%, o sizing pede **1,67 vez o depósito** — ordem impossível em spot.
   Em mercado à vista, o risco por operação só é alcançável quando a distância
   do stop for maior ou igual à fração de risco. Virou `REQ-SIZING-003`.
2. Com o teto de posição de 10%, o risco efetivo por operação cai para **0,12%
   do capital contra os 2% autorizados** — um fator de 16,7. O limite de risco
   por operação nunca chega a ser consultado; quem governa o tamanho é o teto
   de posição. É o mesmo achado do `max_total_exposure` decorativo, um nível
   acima, e é insumo direto para a calibração que a constitution exige.
3. Com posição de 1.000, o custo come 57% do EV bruto e o cenário conservador
   inverte o sinal: EV nominal +1,91, conservador −0,76 → `NO_TRADE`.

Também entraram o `32_NAO_OBJETIVOS.md`, que fixa o contorno do sistema e
registra os quatro conflitos com a constitution como pendentes de emenda, e o
`33_DECISOES.md`, que registra 18 decisões com seu racional — e, mais útil,
**doze valores que hoje governam comportamento sem racional registrado**: `K`,
`k` do stop, `λ`, os limiares de amostra, os SLOs de latência, a idade máxima do
dado, o headroom de rate limit. Nenhum é errado; nenhum é defensável.

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
