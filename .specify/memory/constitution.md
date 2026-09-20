# Trade Constitution

Robô de day trade automatizado especializado em Bitcoin.

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
admissível sob limites que a estratégia não possa contornar.

- Os limites MUST residir em uma camada de risco independente, posicionada entre a
  estratégia e o executor de ordens. Toda ordem MUST atravessá-la; não pode existir
  caminho de código que alcance a corretora sem passar por essa camada.
- A camada de risco MUST impor, no mínimo: perda máxima diária, tamanho máximo de
  posição, exposição máxima total e número máximo de ordens por intervalo.
- Atingido o limite de perda diária, o sistema MUST cessar a abertura de novas
  posições até o próximo período, sem exceção configurável em tempo de execução.
- Um kill switch manual MUST existir, ser acionável sem acesso ao código e ter efeito
  sobre um processo já em operação.
- O sistema MUST parar automaticamente diante de anomalia — perda de conexão com a
  corretora além do limiar, divergência entre posição local e posição reportada pela
  corretora, preço fora de faixa plausível, ou falha repetida de ordem.
- A estratégia MUST NOT ter capacidade de desabilitar, elevar ou contornar qualquer
  um desses limites.

**Racional:** um robô autônomo sem freio transforma qualquer bug, flash crash ou
falha de API em perda ilimitada. O limite precisa viver fora da lógica que ele
restringe, senão não é limite.

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
- Chaves de `paper`/testnet e de `live` MUST ser distintas e MUST NOT coexistir na
  mesma configuração carregada.

**Racional:** a chave com permissão de saque transforma qualquer comprometimento —
do código, da máquina, de uma dependência — em perda total em vez de perda limitada.

## Restrições Operacionais e de Segurança

**Corretora:** Bybit, acessada exclusivamente através da camada de abstração do
Princípio V. Testnet da Bybit é o ambiente da porta de paper trading.

**Ativo:** Bitcoin. Qualquer outro ativo está fora de escopo até emenda MINOR.

**Portas de Promoção** — cumulativas e em ordem. Os limiares abaixo são valores
iniciais e MUST ser confirmados ou ajustados pelo mantenedor antes da primeira
execução em paper trading:

| Porta | Critério mínimo |
|---|---|
| 1. Backtest | ≥ 12 meses de dados, incluindo ao menos um mercado de baixa sustentado e um evento de alta volatilidade; ≥ 100 operações; taxas e slippage modelados explicitamente |
| 2. Paper trading | ≥ 30 dias corridos ininterruptos em testnet, com o mesmo código que iria para `live` |
| 3. Liberação | Ato humano registrado, após revisão dos resultados das portas 1 e 2 |

**Métricas mínimas** exigidas nas portas 1 e 2 (valores iniciais, sujeitos a
confirmação): profit factor ≥ 1.3; drawdown máximo ≤ 15% do capital; a divergência
entre o resultado do paper trading e o do backtest no mesmo período MUST ser
investigada e explicada antes da porta 3, não apenas tolerada.

**Limites de risco iniciais** (Princípio II; valores sujeitos a confirmação):
perda máxima diária 2% do capital; tamanho máximo por posição 10% do capital;
exposição máxima total 20% do capital.

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

**Version**: 1.0.0 | **Ratified**: 2026-09-20 | **Last Amended**: 2026-09-20
