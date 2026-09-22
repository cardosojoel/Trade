# Trade Constitution

Robô de negociação automatizada especializado em Bitcoin, com prazo máximo de
posição de 72 horas.

Este documento governa todas as decisões de especificação, plano, tarefas e
implementação do projeto. Onde houver conflito entre esta constitution e qualquer
outra prática, documento ou preferência, esta constitution prevalece.

## Core Principles

### I. Validação Antes de Capital Real (NÃO-NEGOCIÁVEL)

Nenhuma ordem com capital real pode ser enviada por código que não tenha atravessado,
nesta ordem, as três portas de promoção definidas em *Restrições Operacionais e de
Segurança*: backtest, paper trading e liberação explícita do mantenedor.

- O sistema MUST tratar o modo de execução (`backtest`, `paper`, `live`) como
  configuração explícita e obrigatória. Não pode existir valor padrão que resulte em
  `live`; a ausência de configuração MUST abortar a inicialização.
- A promoção de `paper` para `live` MUST ser um ato humano registrado, nunca uma
  consequência automática de uma métrica atingida.
- Qualquer alteração em lógica de estratégia, risco ou execução MUST reiniciar a
  contagem da porta de paper trading para o código alterado.

**Racional:** o custo de um erro em `backtest` e `paper` é zero; em `live` é
irreversível. A separação de modos é a única barreira que impede um bug de
desenvolvimento de alcançar o dinheiro.

### II. Limites de Risco Invioláveis (NÃO-NEGOCIÁVEL)

O robô opera de forma autônoma, sem aprovação humana por ordem. Essa autonomia só é
admissível sob limites que a estratégia não possa contornar. Dentro dos limites, a
autonomia não é apenas permitida — é a operação esperada: enquanto todos os valores
estiverem dentro da regra, o sistema MUST seguir operando sem intervenção humana.

- O sistema MUST NOT exigir aprovação humana por ordem enquanto os limites estiverem
  respeitados. Interrupção é exceção, não regime de funcionamento.
- Os limites MUST residir em uma camada de risco independente, posicionada entre a
  estratégia e o executor de ordens. Toda ordem MUST atravessá-la; não pode existir
  caminho de código que alcance a corretora sem passar por essa camada.
- A camada de risco MUST impor, no mínimo: perda máxima diária, tamanho máximo de
  posição, exposição máxima total e número máximo de ordens por intervalo.
- Atingido o limite de perda diária, o sistema MUST cessar a abertura de novas
  posições até o próximo período, sem exceção configurável em tempo de execução, e
  MUST retomar a operação automaticamente na virada do período.
- **Recovery — a única exceção admitida ao parágrafo anterior**, introduzida pela
  emenda 2.0.0. Atingido o limite, o sistema MAY recolocar em risco lucro
  **realizado** na própria sessão, e somente sob todas estas condições, que MUST
  ser cumulativas:
  - o orçamento MUST ser criado **uma única vez** por sessão, a partir de lucro
    realizado; lucro não realizado MUST NOT gerar orçamento;
  - o orçamento MUST ser consumível e MUST NOT ser reposto por ganho posterior;
  - o risco por operação em Recovery MUST decrescer conforme o orçamento é
    consumido;
  - aumentar risco em função de perdas consecutivas, do valor da última perda ou
    da distância até o alvo MUST NOT ocorrer — Martingale é proibido;
  - o número de episódios e de tentativas por episódio MUST ter teto configurado
    antes do início da sessão;
  - o Recovery MUST NOT elevar a perda máxima do capital depositado.

  **Racional:** o freio deixa de ser absoluto, e isso é uma perda real — era a
  única regra do sistema sem exceção configurável. A troca foi aceita porque o
  que volta a risco é lucro que já existiu, com teto fixado antes e risco que só
  diminui; o capital depositado continua protegido pelo limite que o Recovery
  MUST NOT elevar. Esta exceção MUST NOT ser usada como precedente: qualquer
  outra exceção ao Princípio II exige emenda própria.
- Ordem recusada por limite de tamanho, exposição ou frequência MUST NOT interromper
  a operação: recusa-se a ordem e o ciclo segue.
- Um kill switch manual MUST existir, ser acionável sem acesso ao código e ter efeito
  sobre um processo já em operação. Uma vez acionado, a retomada MUST ser um ato
  humano explícito.
- O sistema MUST parar automaticamente diante de anomalia — perda de conexão com a
  corretora além do limiar, divergência entre posição local e posição reportada pela
  corretora, preço fora de faixa plausível, ou falha repetida de ordem.
- A anomalia MUST ser classificada como transitória ou de integridade. Falha
  transitória MUST ser retentada automaticamente dentro de um número limitado de
  tentativas, sem intervenção humana. Falha de integridade — divergência de posição,
  preço implausível, ou esgotamento das tentativas — MUST exigir revisão humana antes
  da retomada.
- Toda parada e toda retomada MUST ser registrada com a causa e a classificação.
- A estratégia MUST NOT ter capacidade de desabilitar, elevar ou contornar qualquer
  um desses limites.

**Racional:** um robô autônomo sem freio transforma qualquer bug, flash crash ou
falha de API em perda ilimitada. O limite precisa viver fora da lógica que ele
restringe, senão não é limite. Na direção oposta, um robô que pede permissão a cada
passo não é um robô de day trade — a janela fecha antes da resposta. Por isso a
distinção entre falha transitória e falha de integridade: parar por uma queda de rede
de três segundos custa oportunidade sem comprar segurança; seguir operando com a
posição divergindo da corretora custa dinheiro.

### III. Test-First na Lógica Crítica (NÃO-NEGOCIÁVEL)

Lógica crítica é todo código que executa ordens, aplica limites de risco ou calcula
tamanho de posição, saldo e P&L.

- Em lógica crítica, o teste MUST ser escrito antes da implementação, MUST falhar
  antes de o código existir e MUST passar depois. Ciclo Red-Green-Refactor sem
  exceção.
- Módulos não críticos (visualização, relatórios, scripts auxiliares, ingestão de
  dados históricos) MUST ter teste, escrito antes ou depois conforme conveniência.
- Todo caminho de erro da camada de execução MUST ter teste — rejeição de ordem,
  timeout, preenchimento parcial, desconexão no meio de uma operação.
- Nenhum teste de lógica crítica pode depender de rede ou da API real da corretora.
  A corretora MUST ser substituível por duplo de teste.

**Racional:** erro em cálculo de posição não aparece como exceção, aparece como
prejuízo. Teste escrito depois documenta o comportamento que existe; teste escrito
antes define o comportamento que deveria existir.

### IV. Auditabilidade Reconstituível

Qualquer decisão do robô MUST poder ser reconstruída depois do fato, a partir apenas
do registro, sem recorrer à memória de quem estava presente.

- Todo sinal gerado, ordem enviada, resposta da corretora, acionamento de limite de
  risco e transição de estado MUST ser registrado com timestamp em UTC, modo de
  execução e identificador correlacionável.
- O registro MUST ser estruturado (legível por máquina) e MUST incluir os dados de
  entrada que produziram a decisão, não apenas a decisão.
- Registro MUST NOT conter chaves, segredos ou tokens.
- O registro de execuções `live` MUST ser persistente e MUST NOT ser apagado por
  rotina automática de limpeza.

**Racional:** sem reconstituição não há como distinguir estratégia ruim de bug, e a
correção vira adivinhação. Também é o que torna possível responder por resultados.

### V. Independência de Corretora

A corretora alvo é a Bybit. A lógica de estratégia e de risco MUST NOT conhecê-la.

- Toda interação com a corretora MUST passar por uma interface de abstração definida
  pelo projeto — cotações, envio de ordem, consulta de posição, saldo.
- Estratégia e camada de risco MUST depender apenas dessa interface, jamais de tipos,
  erros ou formatos de dados específicos da Bybit.
- O adaptador da Bybit MUST ser substituível por um adaptador de paper trading e por
  um duplo de teste sem alteração em estratégia ou risco.

**Racional:** acoplar a estratégia à API de uma corretora impede testar sem rede,
impede paper trading honesto e transforma uma mudança de API alheia em reescrita.

### VI. Segurança de Credenciais

- Chaves de API MUST NOT ser commitadas, em nenhuma forma, em nenhum momento do
  histórico. O repositório MUST ter proteção configurada contra isso.
- Credenciais MUST vir de variáveis de ambiente ou de arquivo local explicitamente
  ignorado pelo git.
- A chave de API usada em produção MUST ter permissão mínima: negociação sim,
  **saque não**. A ausência de permissão de saque MUST ser verificada antes da
  primeira execução `live`.
- Chaves de `paper` e de `live` MUST ser distintas e MUST NOT coexistir na mesma
  configuração carregada. A regra vale para **qualquer par** entre os ambientes
  reconhecidos, e a conferência MUST percorrer todas as combinações de presença,
  não um subconjunto escolhido (emenda 2.1.0).
- A ausência de permissão de saque MUST ser condição de partida da sessão de
  `paper`, e não apenas da primeira execução `live`: a chave de Demo Trading é
  emitida pela conta de produção, e ali permissão de saque é acesso a fundo real
  (emenda 2.1.0).

**Racional:** a chave com permissão de saque transforma qualquer comprometimento —
do código, da máquina, de uma dependência — em perda total em vez de perda limitada.

## Restrições Operacionais e de Segurança

**Corretora:** Bybit, acessada exclusivamente através da camada de abstração do
Princípio V. O **Demo Trading** da Bybit é o ambiente da porta de paper trading
(emenda 2.1.0).

O domínio de produção MUST ser inalcançável em modo `paper`, e a garantia MUST
ser de construção e conferida pelo build — não de configuração. Configuração se
erra em silêncio; o que não compila não se erra.

**Ativo:** Bitcoin. Qualquer outro ativo está fora de escopo até emenda MINOR.
A restrição vale para posição de qualquer natureza, inclusive a mantida apenas
para obter desconto de taxa.

**Prazo de posição:** máximo de **72 horas**. O robô MUST encerrar posição que
atinja esse prazo, e MUST NOT abrir posição cujo horizonte previsto o exceda.

**Racional:** medido sobre doze meses reais, um alvo simétrico de 2% resolve em
94% das janelas de 72h contra 59% nas de 24h. As 41% que expiram sem resolver
pagam custo sem produzir resultado, e é isso que o prazo maior elimina — a
vantagem exigida da estratégia cai de 12,9 para 8,2 pontos percentuais sobre a
entrada aleatória, e o custo mensal cai de 7,2% para 3,5% do capital. O preço é
o tempo até a evidência: as 100 operações da Porta 1 passam de 50 para 105 dias.

**Mercado:** exclusivamente à vista (*spot*), **apenas comprado**. O sistema MUST NOT
operar com alavancagem, MUST NOT vender a descoberto e MUST NOT emitir ordem cujo valor
nocional exceda o caixa disponível. Derivativos estão fora de escopo até emenda MINOR.

**Racional:** o mercado à vista é o que garante que o pior caso seja perder o capital
depositado. Com alavancagem, o pior caso passa a depender da liquidação da corretora, e
o Princípio II — cujo racional é impedir que um bug, um flash crash ou uma falha de API
virem perda ilimitada — perde a sua garantia mais forte. Esta restrição vivia apenas no
`CLAUDE.md`, onde podia ser alterada sem emenda, sem racional escrito e sem aprovação;
a emenda 1.3.0 a traz para onde ela pertence.

**Portas de Promoção** — cumulativas e em ordem. Os limiares abaixo foram propostos
pelo assistente e **aceitos pelo mantenedor em 2026-09-20 como valores de partida
provisórios**, não como medição. Permanecem ajustáveis: toda implementação MUST
tratá-los como configuração externa, nunca como constante embutida em código, e
alterá-los MUST ser troca de configuração, não alteração de programa.

| Porta | Critério mínimo |
|---|---|
| 1. Backtest | ≥ 12 meses de dados, incluindo ao menos um mercado de baixa sustentado e um evento de alta volatilidade; ≥ 100 operações; taxas e slippage modelados explicitamente |
| 2. Paper trading | ≥ 30 dias corridos ininterruptos em **Demo Trading**, com o mesmo código que iria para `live` |
| 3. Liberação | Ato humano registrado, após revisão dos resultados das portas 1 e 2 |

**Métricas mínimas** exigidas nas portas 1 e 2 (valores de partida provisórios):
profit factor ≥ 1.3; drawdown máximo ≤ 15% do capital; a divergência
entre o resultado do paper trading e o do backtest no mesmo período MUST ser
investigada e explicada antes da porta 3, não apenas tolerada.

**Limites de risco iniciais** (Princípio II; valores de partida provisórios):
perda máxima diária 2% do capital; tamanho máximo por posição 10% do capital;
exposição máxima total 20% do capital.

**Revisão obrigatória dos limiares.** Aceitar valores de partida não dispensa
calibrá-los. Os limiares MUST ser revistos pelo mantenedor contra o capital real e os
resultados observados **antes da Porta 3** — a liberação para capital real. Até lá
servem para que o sistema seja construído e exercitado; a partir dali passam a
governar dinheiro, e um número que ninguém mediu não deve governar dinheiro.

**Representação de valores monetários:** nenhum valor monetário, de quantidade ou de
preço MAY ser representado em ponto flutuante — `f32`, `f64` ou equivalente — em
qualquer caminho que alcance cálculo de saldo, posição, P&L, limite de risco ou ordem.
A representação MUST ser decimal exata.

**Racional:** erro de arredondamento em ponto flutuante não aparece como exceção,
aparece como divergência silenciosa entre o que o sistema acredita ter e o que de fato
tem. Os Princípios II e IV dependem de o número estar certo: um limite calculado sobre
valor errado não é limite, e um registro reconstituível sobre valor errado não
reconstitui nada. O invariante já era cobrado por `tests/no_float.rs`; a emenda 1.3.0 o
eleva de prática de repositório a regra de governança.

**Segredos:** nenhum segredo no repositório. `.gitignore` MUST cobrir arquivos de
credencial antes de qualquer código de integração ser escrito.

## Fluxo de Desenvolvimento e Portões de Qualidade

O projeto segue o fluxo do GitHub Spec Kit: `/speckit-constitution` →
`/speckit-specify` → `/speckit-plan` → `/speckit-tasks` → `/speckit-implement`.
Nenhuma feature é implementada sem spec e plano correspondentes.

**Portões que bloqueiam merge:**

1. Testes de lógica crítica passando, escritos antes da implementação (Princípio III).
2. Nenhuma ordem alcançando a corretora fora da camada de risco (Princípio II).
3. Nenhum segredo adicionado ao repositório (Princípio VI).
4. Toda decisão nova registrada de forma reconstituível (Princípio IV).
5. Nenhuma dependência direta da API da Bybit fora do adaptador (Princípio V).

**Revisão:** toda alteração em estratégia, risco ou execução MUST ser revisada contra
esta constitution antes do merge. A revisão MUST declarar explicitamente qual porta
de promoção o código alterado precisa refazer.

**Complexidade:** toda complexidade adicionada MUST ser justificada contra a
alternativa mais simples que resolveria o mesmo problema. Na ausência de
justificativa, a alternativa simples vence.

## Governance

Esta constitution prevalece sobre qualquer outra prática, documento ou preferência do
projeto. Conflito entre esta constitution e um spec, plano ou tarefa resolve-se em
favor da constitution; o artefato conflitante MUST ser corrigido.

**Emendas.** Uma emenda MUST ser proposta por escrito, com o racional da mudança e o
impacto sobre specs e código existentes, e MUST ser aprovada pelo mantenedor. A
emenda MUST atualizar a versão e a data de última alteração no rodapé deste arquivo.

**Versionamento.** Semântico:

- **MAJOR** — remoção de princípio, ou redefinição que invalide código ou processo já
  em conformidade.
- **MINOR** — novo princípio ou seção, ou expansão material de orientação existente.
- **PATCH** — esclarecimento, correção de redação, ajuste não semântico.

**Conformidade.** Toda revisão de código MUST verificar aderência aos princípios
aplicáveis. Os três princípios marcados NÃO-NEGOCIÁVEL MUST NOT ser dispensados por
conveniência, prazo ou exceção pontual; a única via para deixar de cumpri-los é a
emenda formal desta constitution.

**Orientação de runtime.** Enquanto o projeto não tiver um `CLAUDE.md`, este
documento é a única fonte de orientação de desenvolvimento em tempo de execução.

**Version**: 2.1.0 | **Ratified**: 2026-09-20 | **Last Amended**: 2026-09-21

**Emenda 2.1.0** — MINOR. Proposta em
`docs/emendas/2026-09-21-demo-trading-na-porta-2.md` e aprovada pelo mantenedor
em 2026-09-21. O ambiente da porta de paper trading passa de **testnet** para
**Demo Trading**.

**Racional.** A testnet da Bybit tem livro de ofertas raso e movimento
artificial. Dois critérios da Porta 2 são medições sobre o mercado, não sobre o
código: slippage real sobre no mínimo cem preenchimentos, e a divergência entre
o resultado do paper e o do backtest. Medidos contra um livro que quase não tem
ordens, os dois produzem números que saem e não significam — e o de slippage
seria promovido de `ASSUMED` a `MEASURED` sem que a medição tivesse medido o
mercado. O Demo Trading opera sobre o livro e os preços de produção, com saldo
fictício. O que a Porta 2 valida não é apenas que o código funciona: é **quanto
custa operá-lo**, e essa pergunta não tem resposta num mercado que não existe.

**O que a troca custa.** A isolação enfraquece, e é o preço declarado. A chave
de testnet pertence a conta inteiramente separada; a de Demo Trading é emitida
**a partir da conta de produção**, e `api-demo.bybit.com` difere de
`api.bybit.com` por um prefixo. Antes desta emenda, apontar para produção por
engano não produziria ordem alguma — a chave de testnet não existe lá. Depois
dela, produziria. Por isso a emenda traz três salvaguardas, cada uma com o que a
cobra no build, descritas na proposta.

**Impacto sobre specs e código existentes:** nenhuma execução de `paper`
existe — o `runs.db` tem nove execuções, todas em `backtest`, e zero linhas com
`mode = 'paper'` (lido em 2026-09-21). Nada que já rodou deixa de estar
conforme. As fatias 1 a 5c da feature 002 não mudam: transporte, assinatura,
adaptador, reconciliação, laço e medição são independentes do domínio. O que
muda é a composição, o conjunto de ambientes reconhecidos, e os invariantes que
o build passa a cobrar.

**Por que MINOR e não MAJOR:** nenhuma execução de paper existiu, então não há
processo em conformidade a invalidar. O Princípio V, onde o trecho da corretora
vive, não é marcado NÃO-NEGOCIÁVEL. O Princípio I, que é, não nomeia ambiente —
exige as três portas na ordem, e continua exigindo.

**Emenda 2.0.0** — MAJOR, por redefinir um princípio marcado NÃO-NEGOCIÁVEL.
Duas mudanças, propostas em `MD BOT/34_ADR_EMENDAS.md` e aceitas pelo mantenedor
em 2026-09-20:

1. **Recovery** (ADR-002) — o Princípio II passa a admitir uma exceção ao freio
   diário, sob seis condições cumulativas. É o primeiro caso de exceção a um
   não-negociável neste projeto, e por isso a versão é MAJOR e não MINOR: quem
   ler "o freio não tem exceção" em qualquer artefato anterior a esta emenda
   está lendo o que já não vale.
2. **Prazo de posição de 72 horas** — o domínio deixa de ser day trade.

**Impacto sobre specs e código existentes:** a feature 001 não implementa
Recovery e não mantém posição entre períodos, então nada do que existe deixa de
estar conforme. O que muda é o que pode ser construído daqui em diante. A
`specs/001-nucleo-execucao/` MUST ser relida antes de qualquer feature nova que
toque risco, porque o texto dela pressupõe o freio sem exceção.

**Emenda 1.3.0** — incorpora ao corpo normativo duas restrições que vigoravam apenas no
`CLAUDE.md`: mercado à vista apenas comprado sem alavancagem, e proibição de ponto
flutuante em caminho monetário. Proposta e racional em `MD BOT/34_ADR_EMENDAS.md`,
ADR-005. **Impacto sobre specs e código existentes: nenhum.** As duas restrições já
eram observadas — a feature 001 opera apenas em spot comprado e usa `rust_decimal` em
todo caminho monetário, com `tests/no_float.rs` cobrando o invariante. A emenda muda o
ato necessário para alterá-las, não o comportamento do sistema.
