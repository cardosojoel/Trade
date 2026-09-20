# Feature Specification: Núcleo de Execução e Risco

**Feature Branch**: `001-nucleo-execucao`

**Created**: 2026-09-20

**Status**: Draft

**Input**: User description: "Núcleo de execução: esqueleto que roda uma estratégia trivial de ponta a ponta em modo backtest sobre dados históricos de BTC, com os três componentes que a constitution exige — camada de abstração de corretora, camada de risco independente e motor de backtest com métricas e registro auditável. A estratégia em si fica fora do escopo. Paper trading e execução live ficam para features futuras, mas a interface de corretora deve acomodá-los sem alteração em estratégia ou risco."

**Decisões tomadas na especificação**: mercado **spot, apenas comprado** (sem
alavancagem, sem venda a descoberto); histórico de BTC **coletado pelo próprio
sistema a partir dos dados públicos da Bybit** e armazenado localmente.

## User Scenarios & Testing *(mandatory)*

O ator desta feature é o **mantenedor** do robô — a pessoa que desenvolve, avalia e
autoriza estratégias. Não há usuário final além dele nesta fase.

### User Story 1 - Avaliar uma estratégia sobre histórico de BTC (Priority: P1)

O mantenedor aponta o sistema para um período histórico de Bitcoin, indica qual
estratégia usar e qual capital inicial simular, e recebe de volta o resultado
financeiro daquele período junto com as métricas que a Porta 1 de promoção exige.

**Why this priority**: é a única entrega que produz valor observável sozinha. Sem ela
não existe forma de responder "essa estratégia presta?", que é a pergunta que o
projeto inteiro serve para responder. Todas as demais histórias qualificam ou
protegem este fluxo.

**Independent Test**: rodar o sistema sobre um conjunto histórico conhecido com uma
estratégia de referência de comportamento previsível, e conferir que o resultado
financeiro e as métricas batem com o cálculo feito à mão sobre o mesmo conjunto.

**Acceptance Scenarios**:

1. **Given** um período histórico de BTC e um capital inicial, **When** o mantenedor
   executa o backtest com a estratégia de referência, **Then** o sistema devolve
   profit factor, drawdown máximo, número de operações, resultado líquido e o extrato
   de todas as operações realizadas.
2. **Given** um backtest concluído, **When** o mantenedor compara o resultado líquido
   com a soma do extrato de operações, **Then** os dois valores coincidem.
3. **Given** o mesmo período histórico, a mesma estratégia e a mesma configuração,
   **When** o backtest é executado duas vezes, **Then** os dois resultados são
   idênticos.
4. **Given** uma configuração de backtest, **When** o mantenedor a executa,
   **Then** taxas e slippage aparecem descontados no resultado e discriminados
   separadamente, nunca embutidos silenciosamente no preço.
5. **Given** um período histórico com lacunas ou velas ausentes, **When** o backtest é
   executado, **Then** o sistema reporta as lacunas encontradas em vez de tratá-las
   como continuidade de preço.
6. **Given** uma posição comprada em aberto, **When** a estratégia solicita venda de
   quantidade maior que a detida, **Then** a ordem é recusada — o sistema não abre
   posição vendida.

---

### User Story 2 - Impedir que qualquer ordem escape dos limites de risco (Priority: P1)

O mantenedor define limites de perda e exposição. Toda ordem que a estratégia tente
emitir passa obrigatoriamente por uma verificação desses limites antes de chegar ao
mercado, e a estratégia não tem meio de afrouxá-los ou contorná-los.

**Why this priority**: o Princípio II da constitution é não-negociável e esta história
é a sua encarnação. É também o que torna a autonomia do robô admissível — sem ela,
não existe caminho legítimo para as features de paper trading e live. A história tem
duas faces igualmente obrigatórias: o robô **não pode** ultrapassar a cerca, e dentro
da cerca **não pode** parar para pedir permissão.

**Independent Test**: submeter uma estratégia deliberadamente imprudente — que tenta
posição maior que o permitido, que insiste em operar depois de estourar a perda
diária, e que tenta alterar os próprios limites — e verificar que toda tentativa é
recusada e registrada.

**Acceptance Scenarios**:

1. **Given** um limite de tamanho máximo de posição, **When** a estratégia solicita
   uma ordem acima desse tamanho, **Then** a ordem é recusada e o motivo registrado;
   a ordem não é silenciosamente reduzida ao teto.
2. **Given** um limite de perda máxima diária, **When** a perda acumulada do dia
   atinge o limite, **Then** nenhuma nova posição é aberta até o próximo período,
   independentemente do que a estratégia solicite.
3. **Given** um limite de exposição total, **When** uma nova ordem levaria a exposição
   somada acima do limite, **Then** a ordem é recusada.
4. **Given** um limite de ordens por intervalo, **When** a estratégia excede esse
   número, **Then** as ordens excedentes são recusadas.
5. **Given** uma estratégia que tenta elevar, desabilitar ou contornar qualquer
   limite, **When** ela executa, **Then** a tentativa não tem efeito sobre os limites
   vigentes.
6. **Given** um kill switch acionado, **When** a estratégia solicita qualquer ordem,
   **Then** nenhuma ordem é aceita e o sistema encerra de forma controlada.
7. **Given** uma condição anômala — preço fora de faixa plausível, divergência entre
   a posição calculada e a posição reportada pela fonte, ou falha repetida de ordem —
   **When** ela ocorre, **Then** o sistema interrompe a operação em vez de prosseguir.
8. **Given** o limite de perda diária atingido com posição comprada em aberto,
   **When** a estratégia solicita encerrar essa posição, **Then** a venda é permitida
   — o bloqueio impede abrir exposição nova, não impede reduzir a existente.
9. **Given** todos os valores dentro dos limites, **When** a estratégia opera ao longo
   do período, **Then** nenhuma ordem exige aprovação humana e a execução segue do
   início ao fim sem intervenção.
10. **Given** o bloqueio por perda diária vigente, **When** o dia vira em UTC,
    **Then** a operação retoma sozinha, sem nenhum ato humano.
11. **Given** uma falha transitória do provedor, **When** ela ocorre dentro do número
    máximo de tentativas, **Then** o sistema retenta e prossegue sem interromper a
    operação.
12. **Given** uma falha de integridade — posição divergente, preço implausível ou
    tentativas esgotadas — **When** ela ocorre, **Then** o sistema para e só retoma
    após ato humano explícito.

---

### User Story 3 - Obter o histórico de BTC sem coleta manual (Priority: P2)

O mantenedor pede o histórico de um período e o sistema o busca nos dados públicos da
Bybit, guarda localmente e o deixa pronto para os backtests seguintes, sem que ele
precise montar arquivos à mão.

**Why this priority**: sem histórico não há backtest com significado. Fica abaixo de
P1 porque o motor pode ser desenvolvido e testado contra um conjunto pequeno de
referência; a coleta é o que torna a avaliação de estratégias praticável de verdade.

**Independent Test**: pedir um período conhecido, verificar que o histórico foi
gravado localmente, e então executar um backtest sobre ele sem nenhum acesso à rede.

**Acceptance Scenarios**:

1. **Given** um par, uma granularidade e um período, **When** o mantenedor solicita a
   coleta, **Then** o histórico correspondente fica armazenado localmente.
2. **Given** um histórico já coletado, **When** o backtest é executado sobre ele,
   **Then** a execução não faz nenhum acesso à rede.
3. **Given** uma coleta interrompida, **When** o mantenedor a repete, **Then** o
   sistema completa o que falta sem duplicar o que já tem.
4. **Given** um período em que a fonte não possui dados completos, **When** a coleta
   termina, **Then** as lacunas são reportadas explicitamente e registradas junto ao
   histórico.
5. **Given** uma coleta em andamento, **When** a fonte recusa por excesso de
   requisições ou fica indisponível, **Then** o sistema respeita o limite e informa o
   ocorrido, sem corromper o que já foi gravado.
6. **Given** qualquer coleta, **When** ela é executada, **Then** nenhuma credencial de
   corretora é exigida.

---

### User Story 4 - Reconstituir o que o robô fez e por quê (Priority: P2)

Depois de um backtest, o mantenedor consegue explicar cada decisão tomada usando
apenas o registro produzido, sem precisar reexecutar nada nem recorrer à memória.

**Why this priority**: é o Princípio IV, e sem ele não há como distinguir estratégia
ruim de defeito de implementação. Fica abaixo de P1 porque um backtest ainda entrega
valor com registro parcial, mas a investigação de qualquer anomalia depende dele.

**Independent Test**: escolher uma operação qualquer do extrato e reconstruir, só a
partir do registro, qual sinal a originou, quais dados de entrada produziram esse
sinal, o que a verificação de risco decidiu e qual foi o preenchimento resultante.

**Acceptance Scenarios**:

1. **Given** um backtest concluído, **When** o mantenedor consulta o registro,
   **Then** cada sinal, cada ordem, cada resposta de execução e cada acionamento de
   limite aparece com data e hora em UTC, modo de execução e um identificador que os
   correlaciona entre si.
2. **Given** um sinal registrado, **When** o mantenedor o examina, **Then** os dados
   de entrada que o produziram estão registrados junto, não apenas o sinal.
3. **Given** qualquer registro produzido, **When** ele é inspecionado, **Then** não
   contém chave, segredo ou credencial.
4. **Given** o registro de um backtest, **When** ele é processado por outro programa,
   **Then** é legível por máquina sem análise de texto livre.

---

### User Story 5 - Trocar a fonte de mercado sem tocar em estratégia ou risco (Priority: P3)

O mantenedor substitui a origem dos dados e o destino das ordens — de histórico para
um simulador, para um duplo de teste, e futuramente para a Bybit ao vivo — sem
alterar uma linha da estratégia ou da camada de risco.

**Why this priority**: é o Princípio V e a condição de possibilidade das features
futuras de paper trading e live. Vem por último porque seu valor se realiza na
próxima feature; aqui basta que a costura exista e esteja provada.

**Independent Test**: executar a mesma estratégia, sem modificá-la, contra dois
provedores de mercado distintos, e verificar que ambos completam o ciclo.

**Acceptance Scenarios**:

1. **Given** uma estratégia e uma configuração de risco, **When** o provedor de
   mercado é trocado por outro, **Then** nenhuma alteração em estratégia ou risco é
   necessária para o sistema voltar a operar.
2. **Given** a estratégia e a camada de risco, **When** seu conteúdo é inspecionado,
   **Then** não há referência a nenhuma corretora específica.
3. **Given** um provedor de mercado que falha ou fica indisponível, **When** a
   estratégia opera, **Then** ela recebe o insucesso pela mesma forma
   independentemente de qual provedor falhou.

---

### Edge Cases

- **Modo de execução ausente**: o sistema é iniciado sem que o modo tenha sido
  declarado. Deve abortar a inicialização, nunca assumir um padrão.
- **Modo não suportado**: o mantenedor pede `paper` ou `live`, que ainda não existem.
  Deve recusar explicitamente informando que apenas `backtest` está disponível,
  jamais executar algo parecido.
- **Venda maior que a posição detida**: recusada. Não existe posição vendida no
  escopo spot comprado.
- **Compra sem saldo suficiente**: recusada antes de chegar ao mercado, com o motivo
  registrado.
- **Capital esgotado no meio do período**: a simulação deve encerrar de forma
  controlada e reportar o momento exato, em vez de operar com saldo negativo.
- **Limite atingido exatamente no valor de fronteira**: resolvido em FR-019a —
  tamanho, exposição e contagem de ordens permitem o valor exato; perda diária
  bloqueia ao atingi-lo.
- **Ordem sem contraparte no histórico**: volume insuficiente na vela para o tamanho
  solicitado. O preenchimento parcial ou a recusa devem ser explícitos e registrados.
- **Virada de dia dentro do período**: a perda diária deve zerar na fronteira do
  período definido, e essa fronteira precisa ser inequívoca em UTC.
- **Conjunto histórico vazio ou fora do período pedido**: deve falhar com mensagem
  clara antes de iniciar a simulação.
- **Kill switch acionado com posição aberta**: resolvido em FR-023a — a posição é
  mantida e reportada, não liquidada automaticamente.
- **Coleta pedida para um período futuro ou inexistente**: deve falhar com mensagem
  clara, sem gravar histórico vazio.
- **Falha transitória repetida até esgotar as tentativas**: deixa de ser transitória e
  vira falha de integridade — para e aguarda revisão humana (FR-024a, FR-024b).
- **Virada de dia com bloqueio de perda diária vigente**: o bloqueio cai sozinho e a
  operação retoma sem ato humano (FR-022a).
- **Recusa de ordem em sequência**: recusas sucessivas por limite não configuram
  anomalia e não interrompem a execução (FR-020a).

## Requirements *(mandatory)*

### Functional Requirements

**Modo de execução**

- **FR-001**: O sistema MUST exigir que o modo de execução seja declarado
  explicitamente na inicialização, sem valor padrão de nenhuma espécie.
- **FR-002**: O sistema MUST abortar a inicialização quando o modo não for declarado
  ou não for suportado, informando quais modos existem.
- **FR-003**: O sistema MUST suportar apenas o modo `backtest` nesta feature, e MUST
  recusar `paper` e `live` de forma explícita.

**Escopo de mercado**

- **FR-004**: O sistema MUST operar exclusivamente BTC no mercado spot.
- **FR-005**: O sistema MUST NOT abrir posição vendida e MUST NOT usar alavancagem.
  Toda venda MUST ser limitada à quantidade detida.
- **FR-006**: O sistema MUST recusar compra cujo custo total, incluindo taxa, exceda
  o saldo disponível.

**Provedor de mercado**

- **FR-007**: O sistema MUST expor uma forma única de obter cotações, enviar ordens,
  consultar posição e consultar saldo, comum a todos os provedores de mercado.
- **FR-008**: O sistema MUST fornecer um provedor histórico que reproduza um período
  de BTC a partir do histórico armazenado localmente.
- **FR-009**: A estratégia e a camada de risco MUST NOT depender de qualquer
  característica de uma corretora específica.
- **FR-010**: O sistema MUST reportar falhas do provedor de mercado de forma
  uniforme, independentemente de qual provedor falhou.

**Coleta de histórico**

- **FR-011**: O sistema MUST coletar histórico de BTC a partir dos dados públicos da
  Bybit, para um par, granularidade e período informados pelo mantenedor.
- **FR-012**: A coleta MUST NOT exigir credencial de corretora.
- **FR-013**: O sistema MUST armazenar o histórico coletado localmente, de modo que
  qualquer backtest posterior seja executável sem acesso à rede.
- **FR-014**: A coleta MUST ser retomável: repetir uma coleta interrompida MUST
  completar o que falta sem duplicar o que já existe.
- **FR-015**: O sistema MUST respeitar os limites de requisição da fonte e MUST
  informar indisponibilidade sem corromper o histórico já gravado.
- **FR-016**: O sistema MUST detectar e reportar lacunas no histórico coletado, e
  MUST registrá-las junto ao conjunto armazenado.
- **FR-017**: O sistema MUST registrar a procedência de cada conjunto armazenado —
  fonte, par, granularidade, período e instante da coleta.

**Camada de risco**

- **FR-018**: Toda ordem MUST atravessar a verificação de risco antes de alcançar o
  provedor de mercado; MUST NOT existir caminho alternativo.
- **FR-018a**: O sistema MUST NOT exigir aprovação humana por ordem enquanto os
  limites estiverem respeitados. Com todos os valores dentro da regra, a operação
  MUST prosseguir sozinha até o fim do período.
- **FR-019**: A verificação de risco MUST impor perda máxima diária, tamanho máximo
  de posição, exposição máxima total e número máximo de ordens por intervalo.
- **FR-019a**: Para tamanho de posição, exposição e número de ordens, o limite MUST
  ser tratado como valor permitido: a violação ocorre ao **ultrapassá-lo**, não ao
  atingi-lo. Para a perda máxima diária vale o inverso — **atingir** o valor já
  aciona o bloqueio, conforme o Princípio II da constitution.
- **FR-020**: A camada de risco MUST recusar a ordem que viole um limite, MUST NOT
  ajustá-la silenciosamente para caber.
- **FR-020a**: Recusa por tamanho de posição, exposição ou frequência MUST NOT
  interromper a execução — recusa-se a ordem e o ciclo continua no passo seguinte.
- **FR-021**: A estratégia MUST NOT ter capacidade de alterar, elevar ou desabilitar
  qualquer limite em tempo de execução.
- **FR-022**: O sistema MUST cessar a abertura de novas posições pelo resto do
  período quando a perda máxima diária for atingida, e MUST continuar permitindo
  ordens que reduzam a exposição existente.
- **FR-022a**: O bloqueio por perda diária MUST ser suspenso automaticamente na
  virada do dia em UTC, sem intervenção humana, e a operação MUST retomar sozinha.
- **FR-023**: O sistema MUST oferecer um kill switch que interrompa a aceitação de
  ordens sem exigir alteração de código.
- **FR-023a**: O kill switch MUST NOT liquidar automaticamente posição aberta. Ele
  interrompe a aceitação de novas ordens, encerra a operação de forma controlada e
  MUST reportar a posição que ficou em aberto. Liquidar sob pânico é decisão do
  mantenedor, não do robô — uma venda automática em mercado desordenado pode custar
  mais que a posição mantida.
- **FR-024**: O sistema MUST interromper a operação automaticamente diante de preço
  fora de faixa plausível, divergência entre posição calculada e posição reportada
  pelo provedor, ou falha repetida de ordem.
- **FR-024a**: O sistema MUST classificar toda anomalia como **transitória** ou **de
  integridade**. Falha transitória — indisponibilidade momentânea do provedor, falha
  isolada de ordem — MUST ser retentada automaticamente até um número máximo
  configurável de tentativas, sem interromper a operação nem exigir intervenção.
- **FR-024b**: Falha de integridade — divergência entre posição calculada e reportada,
  preço fora de faixa plausível, ou esgotamento das tentativas de uma falha
  transitória — MUST interromper a operação e MUST exigir ato humano explícito para
  retomar.
- **FR-024c**: O sistema MUST registrar toda parada e toda retomada com a causa, a
  classificação da anomalia e o número de tentativas realizadas.
- **FR-025**: Os limites MUST ser configuráveis pelo mantenedor e MUST ser
  registrados junto ao resultado de cada execução.

**Motor de backtest**

- **FR-026**: O sistema MUST executar o ciclo completo sinal → verificação de risco →
  ordem → atualização de posição sobre cada passo do período histórico.
- **FR-027**: O sistema MUST modelar taxas e slippage de forma explícita e
  configurável, e MUST discriminá-los separadamente no resultado.
- **FR-028**: O sistema MUST produzir, ao fim da execução, profit factor, drawdown
  máximo, número de operações, resultado líquido e o extrato de operações.
- **FR-029**: O sistema MUST produzir resultado idêntico para entradas idênticas.
- **FR-030**: O sistema MUST NOT permitir que uma decisão use qualquer informação
  posterior ao instante simulado.
- **FR-031**: O sistema MUST reportar lacunas e inconsistências no conjunto histórico
  em vez de interpolá-las silenciosamente.
- **FR-032**: O sistema MUST encerrar de forma controlada e reportar o instante
  quando o capital simulado se esgotar.

**Registro auditável**

- **FR-033**: O sistema MUST registrar todo sinal, ordem, resposta de execução,
  decisão de risco e transição de estado, com data e hora em UTC, modo de execução e
  identificador correlacionável.
- **FR-034**: O registro de um sinal MUST incluir os dados de entrada que o
  produziram.
- **FR-035**: O registro MUST ser legível por máquina.
- **FR-036**: O registro MUST NOT conter chave, segredo ou credencial.

**Estratégia de referência**

- **FR-037**: O sistema MUST incluir uma estratégia de referência de comportamento
  previsível, cuja finalidade é exercitar o ciclo completo.
- **FR-038**: A estratégia de referência MUST NOT ser apresentada nem usada como
  estratégia de investimento.

### Key Entities

- **Modo de execução**: declara se a operação é `backtest`, `paper` ou `live`.
  Obrigatório, sem padrão. Acompanha todo registro produzido.
- **Conjunto histórico**: o histórico armazenado de um par, granularidade e período,
  com sua procedência e suas lacunas conhecidas.
- **Vela histórica**: um instante do mercado de BTC — abertura, máxima, mínima,
  fechamento, volume e o instante a que se refere.
- **Sinal**: a intenção da estratégia em um instante — comprar, vender ou não agir —
  e os dados de entrada que a motivaram.
- **Ordem**: a solicitação derivada de um sinal — direção, tamanho, instante e o
  sinal que a originou.
- **Decisão de risco**: o veredito sobre uma ordem — aceita ou recusada, qual limite
  motivou a recusa, e os valores vigentes no momento da avaliação.
- **Preenchimento**: o resultado da ordem no mercado — preço efetivo, quantidade
  executada, taxa cobrada e slippage aplicado.
- **Posição**: a quantidade de BTC detida, o preço médio de aquisição e o resultado
  não realizado.
- **Operação**: um ciclo completo de compra e venda, com resultado realizado.
- **Configuração de limites**: perda máxima diária, tamanho máximo de posição,
  exposição máxima total e número máximo de ordens por intervalo.
- **Resultado de execução**: o conjunto final — métricas, extrato de operações,
  configuração usada e referência ao registro correspondente.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: O mantenedor consegue avaliar uma estratégia sobre um período histórico
  e obter as métricas da Porta 1 de promoção sem intervenção manual durante a
  execução.
- **SC-002**: 100% das ordens emitidas em uma execução aparecem no registro com a
  decisão de risco correspondente. Nenhuma ordem alcança o mercado sem esse registro.
- **SC-003**: Uma estratégia deliberadamente imprudente não consegue, em nenhuma
  tentativa, ultrapassar qualquer um dos quatro limites configurados.
- **SC-004**: Executar o mesmo período com a mesma configuração duas vezes produz
  resultados idênticos em 100% das execuções.
- **SC-005**: Qualquer operação do extrato pode ser reconstituída até o sinal que a
  originou e até os dados de entrada desse sinal, usando apenas o registro.
- **SC-006**: A mesma estratégia, sem nenhuma alteração, completa o ciclo contra dois
  provedores de mercado diferentes.
- **SC-007**: Iniciar o sistema sem declarar o modo de execução falha em 100% das
  tentativas.
- **SC-008**: Nenhum teste da lógica de ordens, risco ou cálculo de posição depende de
  acesso à rede.
- **SC-009**: O resultado líquido reportado coincide com a soma do extrato de
  operações, com divergência zero.
- **SC-010**: Em nenhuma execução a quantidade detida de BTC fica negativa.
- **SC-011**: Um backtest sobre histórico já coletado é executável com a rede
  desligada, em 100% das tentativas.
- **SC-012**: O mantenedor obtém 12 meses de histórico de BTC em uma única solicitação,
  sem montar arquivos manualmente — volume exigido pela Porta 1 de promoção.
- **SC-013**: Uma execução inteira com todos os valores dentro dos limites acontece com
  zero intervenções humanas, do primeiro ao último passo do período.
- **SC-014**: Após um bloqueio por perda diária, a operação retoma na virada do dia em
  100% dos casos, sem nenhum ato humano.
- **SC-015**: Nenhuma falha de integridade é retomada sem ato humano explícito, em 100%
  dos casos.

## Assumptions

- **Mercado e direção**: spot, apenas comprado, sem alavancagem e sem venda a
  descoberto. Decidido na especificação. Perpétuos, alavancagem e posição vendida
  ficam para emenda futura da constitution e feature própria.
- **Capital e referência**: o capital inicial é declarado pelo mantenedor e o
  resultado é expresso na moeda de cotação do par negociado. O sistema não converte
  para outra moeda.
- **Período diário**: "dia", para efeito da perda máxima diária, é o dia em UTC —
  consistente com o registro, que também é em UTC.
- **Granularidade**: a granularidade das velas é configurável; a estratégia de
  referência e os testes usam uma granularidade intradiária compatível com day trade.
  Nenhuma granularidade é assumida como fixa pelo motor.
- **Dados públicos**: o histórico da Bybit usado aqui é público e não exige
  autenticação. Caso a fonte passe a exigir credencial, isso vira decisão nova sujeita
  ao Princípio VI.
- **Coleta separada da execução**: coletar histórico e executar backtest são ações
  distintas. O backtest nunca acessa a rede; consome apenas o histórico já gravado.
  É o que mantém o backtest determinístico e os testes offline.
- **Execução de ordens no backtest**: o preenchimento é simulado sobre a vela
  seguinte ao sinal, nunca sobre a vela que o originou, para impedir uso de
  informação futura.
- **Uma posição por vez**: a estratégia de referência mantém no máximo uma posição
  aberta. O modelo de posição não impede acumulação, mas exercitá-la está fora do
  escopo desta feature.
- **Escopo excluído**: descoberta ou otimização de estratégia, interface gráfica,
  paper trading, execução com capital real, envio de ordens à Bybit, múltiplos ativos
  além de BTC e notificações estão fora desta feature.
- **Dependência de constitution**: esta feature implementa diretamente os Princípios
  I, II, IV e V, e está sujeita ao Princípio III — a lógica de ordens, risco e
  cálculo de posição exige teste escrito antes da implementação.
- **Limites numéricos**: os valores iniciais de limite e de métrica mínima vêm da
  constitution e permanecem sujeitos à confirmação do mantenedor; a feature os trata
  como configuração, não como constante.
- **Autonomia**: o regime normal é operação automática ininterrupta. Parada é exceção,
  e cada tipo de parada tem regra de retomada explícita — automática na virada do dia
  (perda diária), automática por retentativa (falha transitória), humana (kill switch
  e falha de integridade).
- **Anomalias em modo backtest**: sobre histórico local não existe falha de rede. A
  detecção de integridade — preço implausível, divergência de posição — é exercitada
  sobre os próprios dados; a trilha de retentativa é exercitada por duplo de teste,
  já que precisa existir e estar provada antes das features de paper trading e live.
