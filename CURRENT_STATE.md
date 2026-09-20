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

**270 testes verdes · clippy limpo · CI verde · tudo sincronizado com o remoto**

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

Quarenta e sete documentos — três de índice, glossário e fronteira, nove
frentes de baseline, nove de estratégia e quant, seis de plataforma, cinco de
governança P0, cinco de escopo, decisões e limites, quatro especificações
matemáticas e cinco de registro histórico. **Nada dela foi implementado, e ela
não descreve o que existe hoje.** Versionada em 2026-09-20 para que a discussão aconteça sobre
texto rastreável.

**Quatro divergências entre o conjunto e o que o Trade autoriza**, todas
registradas como ADR em `MD BOT/34_ADR_EMENDAS.md` e nenhuma decidida. Desde a
emenda 1.3.0 as quatro contrariam a **constitution**; antes dela, duas
contrariavam apenas o `CLAUDE.md`, e foi essa verificação que expôs a lacuna.

| Divergência | ADR | O que o conjunto pressupõe |
|---|---|---|
| **Mercado** | ADR-001 | derivativos — `leverage_min/max`, funding, `reduce-only`, microestrutura de derivativos no simulador |
| **Recovery** | ADR-002 | depois de atingido o limite de perda, recolocar lucro realizado em risco com risco por operação maior (`RT` 25% contra `T` 2%) |
| **Sessão** | ADR-003 | ciclo com depósito, estado `STOPPED` e confirmação humana para retomar, contra a retomada automática na virada do período |
| **Tipos** | ADR-004 | `f64` para `tick_size`, `qty_step`, `min_notional` e EV — que a própria ADR-004 já classificou como erro de redação do `29`, não escolha de desenho: não há decisão a tomar, há correção a fazer |

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
por hash (`38814ec1…`, 251 linhas), e hash diferente obriga a reavaliar todas as
conformidades.

O `MD BOT/35_LIMITES.md` completa o par: a fronteira diz **quem decide**, os
limites dizem **o que cabe**. Numa folha só, o envelope inteiro — limites duros
da corretora e da aritmética, medidos do mercado e do capital, de autoridade, de
promoção, e os limites da própria medição. Um desenho que não passa nessa folha
não chega a precisar de ADR.

## Em andamento: feature 002, paper trading

Spec, plano e tarefas escritos. **15 das 31 tarefas que não precisam de
credencial estão concluídas**, em quatro commits.

O que existe: modo `paper` no domínio e recusado no comando `backtest`;
credenciais lidas do ambiente com o segredo redigido em `Debug` e `Display`;
assinatura HMAC-SHA256 conferida contra a documentação da Bybit e validada por
vetor conhecido; verificação de que a chave não tem permissão de saque, por
lista de negação; montagem de ordem com identificador de cliente, stop a
mercado no gatilho e corpo JSON assinado byte a byte; tradução de erro
separando transitório, recusa e **desconhecido**.

O desconhecido é o que essa camada acrescenta e o backtest nunca teve: a
requisição saiu, a resposta não voltou, e ninguém sabe se a ordem existe.
Retentar pode duplicar, desistir pode deixar posição órfã. Ele é classificado
como falha de integridade de propósito — é o que impede a retentativa
automática e força a reconciliação.

O que falta sem credencial: o executor ligando as peças, a reconciliação de
posição e o laço contínuo. **O que só o mantenedor destrava**: criar a chave de
testnet sem permissão de saque, e os 30 dias correrem.

---

## Decidido em 2026-09-20: a constitution passou a 2.0.0

**Nove decisões, e nenhuma divergência permanece aberta.** As cinco ADRs foram
decididas e duas delas viraram emenda MAJOR — a primeira exceção a um princípio
não-negociável neste projeto.

| Decisão | Resultado |
|---|---|
| Recovery depois do freio (ADR-002) | **aceito**, sob seis condições cumulativas no Princípio II |
| Prazo máximo de posição (ADR-006) | **72 horas** — o domínio deixa de ser day trade |
| Derivativos e alavancagem (ADR-001) | **recusados** — a perda segue limitada ao depósito |
| Sessão com confirmação humana (ADR-003) | **recusada**; confirmação só quando os parâmetros derivados mudam |
| `f64` nos contratos (ADR-004) | **corrigido** para `Decimal` — era erro de redação |
| Desconto de taxa via MNT | **recusado** — exigiria manter um segundo ativo |
| Depósito na Bybit | **nenhum ainda**; a conta serve para testnet e para conferir a taxa |
| `MD BOT/` | **fica** no repositório |
| Próximo trabalho | **implementar as regras já decididas** |

**O prazo de 72h foi a decisão com maior efeito medido:**

| | 24h | 72h |
|---|---:|---:|
| Posições que resolvem | 59% | **94%** |
| Acerto necessário | 59,4% | **56,6%** |
| Lacuna sobre o acaso | 12,9 pt | **8,2 pt** |
| Custo mensal do capital | 7,2% | **3,5%** |
| Resultado sem vantagem, 30 dias | −8,5% | **−4,2%** |
| Contas paradas por drawdown | 16% | **1%** |
| Tempo até as 100 operações da Porta 1 | 50 dias | 105 dias |

A frequência deixou de ser escolhida: com uma posição por vez e resolução média
de 25,2h, saem **0,95 operação por dia**. O custo caiu porque o giro caiu, não
porque a taxa mudou.

**A taxa foi confirmada, e era a última premissa grande.** Lida na conta em
2026-09-20: spot 0,1% por perna, maker e taker. O valor presumido estava certo,
e o parâmetro passa de `ASSUMED` a `EXCHANGE`. Sobra o slippage de 0,05%, que
só a Porta 2 mede.

---

## Separação concluída em 2026-09-20: nasceu o DsTrade

O desenho **da interface** saiu para `/home/c/Projetos/DsTrade`, repositório
próprio, fora da árvore do Trade. O desenho **de sistema** — o `MD BOT/` —
permaneceu aqui: são conjuntos diferentes e só um precisava sair.

A regra de só leitura do DsTrade sobre o Trade não é promessa: é `deny` no
`settings.json` para Edit/Write, mais um gancho `PreToolUse` de Bash com lista
branca, verificado por 42 casos. O limite honesto está escrito lá — o gancho lê
o texto do comando, e um programa que receba o caminho por `stdin` não é
visível para ele; a rede final é este repositório estar commitado e no remoto.

Dali veio o primeiro achado externo, e era real: o `docs/auditoria.md`
documentava **sete** dos oito tipos de `kind` do `audit_event`. Faltava
`state_transition` — o evento que registra de que posição o robô partiu a cada
mudança de estado. Uma reconstituição que o ignore responde o que foi decidido,
não de onde se partiu. Corrigido com consulta própria e os oito enumerados.

---

## Decidido em 2026-09-20: a constitution passou a 1.3.0

**A ADR-005 foi aceita.** As duas restrições que limitam a perda máxima possível
subiram para o corpo normativo:

| Seção nova na constitution | O que fixa |
|---|---|
| *Restrições Operacionais* → **Mercado** | à vista, apenas comprado; sem alavancagem, sem venda a descoberto, e ordem nunca excede o caixa disponível |
| *Restrições Operacionais* → **Representação de valores monetários** | nenhum ponto flutuante em caminho que alcance saldo, posição, P&L, risco ou ordem |

**Impacto sobre código e specs: nenhum.** As duas já eram observadas — a feature
001 opera só em spot comprado e usa `rust_decimal` em todo caminho monetário. A
emenda muda o **ato necessário para alterá-las**, não o comportamento: antes
bastava editar o `CLAUDE.md`; agora exige emenda escrita, com racional e
aprovação registrada.

Consequências em cadeia, todas aplicadas: a âncora de hash da fronteira passou a
`38814ec1…` (251 linhas); `REQ-FRONTEIRA-002` obrigou a reavaliar toda a coluna
`Conformidade`, e **nenhum documento mudou de estado** — o que mudou foi a
autoridade contrariada por ADR-001 e ADR-004, que subiu de diretriz de trabalho
para constitution. As quatro divergências abertas agora pesam igual.

**O que a ADR-005 levantou, e que foi resolvido no mesmo dia:** a restrição de
mercado à vista não tinha proteção executável. Agora tem — `tests/no_leverage.rs`,
quatro travas que o CI cobra:

| Trava | O que barra |
|---|---|
| Vocabulário | `leverage`, `margin`, `reduce_only`, `funding_rate`, `perpetual`, `descoberto` e mais 14 termos, em qualquer crate. Comentário é ignorado de propósito: explicar por que não há alavancagem é o que se espera encontrar |
| Categoria | toda linha que nomeia `category` fixa `"spot"`; literais `"linear"`, `"inverse"` e `"option"` são recusados. A Bybit assume `linear` quando o parâmetro é omitido — esquecer não dá erro, dá derivativo |
| `Side` | admite exatamente `Buy` e `Sell`; `Intent`, exatamente `Buy`, `Sell` e `Hold` |
| Domínio | `PositionError::SellExceedsHoldings` tem de continuar existindo: é o que impede a quantidade detida de ficar negativa |

**Verifiquei as quatro violando de propósito**, como as outras três garantias:
injetei `LEVERAGE_MAX` em `trade-risk`, troquei `category=spot` por `linear` no
cliente, acrescentei uma variante `Short` ao `Side` e renomeei o erro de venda
acima do detido. As quatro falharam, cada uma com a sua mensagem. Restaurado
tudo em seguida.

---

**Como a lacuna apareceu.** Eu repeti em cinco commits que havia "quatro
conflitos com a constitution". A verificação documento a documento mostrou que
eram quatro divergências, mas **só duas com a constitution** — derivativos e
`f64` contrariavam o `CLAUDE.md`, cujo ato de mudança é uma edição de arquivo.
Nas 220 linhas da 1.2.0 não havia "spot", "alavancagem", "venda a descoberto",
"float" nem "decimal".

Foi o que motivou a ADR-005. **Desde a emenda 1.3.0 as quatro contrariam a
constitution**, e a divergência de mercado à vista ganhou trava de build. A
distinção de autoridade fica registrada aqui porque foi ela que expôs a lacuna,
não porque ainda valha.

**Portabilidade medida, e remedida em 2026-09-20.** O conjunto tem 43
referências para fora, em 8 arquivos-alvo e 10 dos 47 documentos. Só **quatro**
são apoio factual e se quebram ao mudar de endereço — `market.db` no `27`,
`specs/001` no `26` e `limits.toml` no `33`. As outras 39 são a camada de
governança nomeando a autoridade, e ao mover passam a apontar para a cópia
somente leitura em `constraints/`. Não há inclusão de arquivo, script
compartilhado nem dependência de build. A decisão de mover continua aberta e
ficou barata nos dois sentidos.

A contagem anterior deste arquivo — oito referências em 45 documentos — media o
conjunto antes de a camada de governança existir, e já afirmava duas coisas
falsas. `REQ-FRONTEIRA-008` passou a exigir que o inventário seja refeito
sempre que a âncora de hash mudar.

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

Também em 2026-09-20: os requisitos passaram a ter identificador. **162
`REQ-<DOMÍNIO>-<NNN>` em 30 domínios**, escritos ao lado da regra na
especificação — nunca em lista à parte, que seria uma segunda camada a manter —
com linguagem RFC 2119. O `26_REQUIREMENTS_TRACEABILITY_MATRIX.md` deixou de ter
seis linhas de exemplo e passou a catalogar os 162, todos em `SPECIFIED`: nenhum
aponta para módulo Rust, teste ou métrica, porque o código existente nasceu da
constitution e da `specs/001-nucleo-execucao/`, não desta SDD. Os 47 documentos
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

A constitution (`.specify/memory/constitution.md`, **v1.3.0**) governa tudo e
tem precedência sobre qualquer outra prática.

---

## O que funciona

```bash
trade collect  --symbol BTCUSDT --interval 1m --from 2025-09-20 --to 2026-09-20
trade backtest --mode backtest --from 2025-09-20 --to 2026-09-20 --capital 10000 \
               --limits limits.toml --fees fees.toml \
               --instrumento instrumento.toml
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

## As quatro garantias estruturais

Não são convenção. Verifiquei as quatro violando de propósito.

| Garantia | O que acontece se for violada |
|---|---|
| Estratégia, risco e backtest não alcançam corretora nem rede | `tests/architecture.rs` falha, CI barra o merge |
| Nenhum `f32`/`f64` em caminho monetário | `tests/no_float.rs` falha, CI barra o merge |
| Nenhuma alavancagem, derivativo ou venda a descoberto | `tests/no_leverage.rs` falha, CI barra o merge |
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
