# Feature Specification: Paper Trading na Testnet da Bybit

**Feature Branch**: `002-paper-trading`

**Created**: 2026-09-20

**Status**: Draft

**Input**: Adaptador de paper trading contra a testnet da Bybit, com o mesmo
código que iria para `live`. É o que destrava a Porta 2 da constitution — 30
dias corridos ininterruptos — e o que mede pela primeira vez o slippage real,
hoje a única premissa de custo ainda presumida.

**Depende de**: constitution v2.0.0. As duas emendas mudam esta feature de
forma material: o Recovery admitido no Princípio II e o prazo máximo de posição
de 72 horas.

## Por que esta feature, e por que agora

O backtest terminou. Ele responde o que teria acontecido; não responde o que
acontece. Três coisas que o projeto hoje presume e só a testnet mede:

1. **Slippage.** 0,05% por ida e volta é analogia, nunca medida. É um quinto do
   custo total, e o custo é o que decide se a estratégia vive.
2. **Latência e preenchimento.** O simulador preenche na abertura da vela. A
   corretora preenche quando preenche, e às vezes parcialmente.
3. **Divergência de posição.** O backtest não tem como divergir de si mesmo. A
   conta na corretora tem, e o Princípio II manda parar quando isso acontece.

A Porta 2 exige 30 dias **ininterruptos**. Isso, e não a integração em si, é o
que define o desenho: o sistema precisa sobreviver a reinício, queda de rede e
manutenção da corretora sem perder o fio.

## User Scenarios & Testing *(mandatory)*

O ator continua sendo o **mantenedor**.

### User Story 1 - Operar na testnet com o mesmo código que iria para live (Priority: P1)

O mantenedor configura credenciais de testnet e roda `trade run --mode paper`.
O sistema opera continuamente: lê velas em tempo real, avalia a estratégia,
envia ordens pela camada de risco e registra tudo de forma reconstituível.

**Por que P1**: é a feature. Sem isso não há Porta 2, e sem Porta 2 não há
caminho para capital real.

**Teste de aceitação**:
1. **Dado** credenciais de testnet válidas e modo `paper`, **quando** a
   estratégia emite um sinal de compra dentro dos limites, **então** uma ordem
   é enviada à testnet e o preenchimento recebido atualiza a posição local.
2. **Dado** que o modo é `paper`, **quando** o código de estratégia e de risco
   é inspecionado, **então** ele é byte a byte o mesmo que rodaria em `live` —
   o que muda é apenas qual implementação de `OrderExecutor` é composta.
3. **Dado** que nenhuma credencial está configurada, **quando** o modo `paper`
   é solicitado, **então** a inicialização aborta antes de qualquer ordem.

### User Story 2 - Medir o custo real de execução (Priority: P1)

Cada preenchimento registra o preço de referência no instante da decisão e o
preço efetivamente obtido. A diferença é o slippage real, medido e não
presumido.

**Por que P1**: é o insumo que falta para o `27_CONFIGURATION_REGISTRY.md`
deixar de ter parâmetro `ASSUMED` governando decisão.

**Teste de aceitação**:
1. **Dado** um preenchimento na testnet, **quando** o evento é registrado,
   **então** ele contém preço de referência, preço obtido, taxa cobrada e a
   moeda em que a taxa saiu.
2. **Dado** uma série de preenchimentos, **quando** o relatório é gerado,
   **então** ele mostra a distribuição do slippage — mediana e p95 —, não só a
   média.

### User Story 3 - Sobreviver a 30 dias ininterruptos (Priority: P1)

O sistema reinicia sozinho depois de queda de rede, reconcilia a posição com a
corretora antes de voltar a operar, e retoma sem perder o registro.

**Por que P1**: a Porta 2 pede 30 dias **ininterruptos**. Uma queda que exija
intervenção manual reinicia a contagem.

**Teste de aceitação**:
1. **Dado** o sistema operando com posição aberta, **quando** o processo é
   morto e reiniciado, **então** ele recupera a posição do registro, consulta a
   corretora e só volta a operar se as duas coincidirem.
2. **Dado** divergência entre posição local e reportada, **quando** a
   reconciliação roda, **então** o sistema para e exige revisão humana —
   falha de integridade, não transitória.
3. **Dado** perda de conexão por menos que o limiar, **quando** a conexão
   volta, **então** a operação segue sem intervenção.

### User Story 4 - Ordem com stop aceito antes da posição existir (Priority: P2)

Toda entrada leva stop. Se o stop não for aceito pela corretora, a posição não
é aberta.

**Por que P2**: depende da US1, mas é o que torna o risco executável fora do
simulador.

**Teste de aceitação**:
1. **Dado** um sinal de compra, **quando** a ordem é montada, **então** ela
   carrega stop a mercado no gatilho (`REQ-EXEC-010`).
2. **Dado** que a corretora recusa o stop, **quando** isso ocorre, **então**
   a posição não é aberta; se já tiver sido, é encerrada (`REQ-EXEC-011`).

### Edge Cases

- **Credencial com permissão de saque.** O sistema verifica e recusa iniciar.
- **Ordem em estado desconhecido** após timeout: reconciliar antes de qualquer
  ordem nova, nunca repetir cegamente.
- **Preenchimento parcial** que deixa posição abaixo da ordem mínima: o
  resíduo espera, como no backtest.
- **Manutenção da corretora**: tratada como falha transitória enquanto durar o
  limiar; acima dele, parada com classificação.
- **Relógio da máquina divergindo** do da corretora: o timestamp da corretora
  é o que vale para ordenação de eventos dela.
- **Virada de dia durante posição aberta**: o freio diário zera, mas a posição
  de 72h atravessa a virada — e o prejuízo não realizado dela continua contando
  (FR-019b).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-101**: O sistema MUST aceitar `--mode paper` e MUST recusar `live`.
- **FR-102**: As credenciais MUST vir de variável de ambiente, nunca de arquivo
  versionado, e MUST NOT aparecer em log, erro ou registro de auditoria.
- **FR-103**: O sistema MUST verificar, antes da primeira ordem, que a chave
  não tem permissão de saque, e MUST abortar se tiver.
- **FR-104**: Chave de testnet e de produção MUST NOT coexistir na mesma
  configuração carregada.
- **FR-105**: O adaptador de paper MUST implementar `OrderExecutor` sem exigir
  alteração em estratégia, risco ou motor.
- **FR-106**: Toda ordem MUST carregar identificador de cliente próprio, único,
  para que uma reenvio não vire ordem duplicada.
- **FR-107**: O sistema MUST NOT reenviar ordem cujo resultado seja
  desconhecido antes de reconciliar.
- **FR-108**: O sistema MUST reconciliar posição e saldo contra a corretora no
  início de cada sessão e depois de cada reconexão, e MUST NOT operar enquanto
  houver divergência.
- **FR-109**: Cada preenchimento MUST registrar preço de referência, preço
  obtido, taxa, moeda da taxa e quantidade recebida.
- **FR-110**: O relatório MUST apresentar a distribuição do slippage observado,
  com mediana e p95.
- **FR-111**: O sistema MUST distinguir falha transitória de falha de
  integridade, retentando a primeira dentro do limite configurado.
- **FR-112**: O estado MUST sobreviver a reinício do processo: posição, ordens
  em aberto e contadores de risco vêm do registro, não da memória.
- **FR-113**: A ordem de entrada MUST carregar stop a mercado no gatilho, e a
  posição MUST NOT existir sem stop aceito.
- **FR-114**: O sistema MUST respeitar o prazo máximo de posição de 72 horas,
  encerrando a que o atingir.
- **FR-115**: O tempo MUST vir de um `Clock` real, e o `Clock` de backtest MUST
  NOT ser alcançável em modo paper.

### Key Entities

- **Credencial**: chave e segredo de testnet, lidos do ambiente. Nunca
  persistidos, nunca registrados.
- **OrdemRemota**: a ordem como a corretora a conhece, com o identificador dela
  e o nosso, e o estado do ciclo de vida.
- **Reconciliação**: comparação entre posição local e remota, com veredito
  `SYNCED`, `DIVERGENTE` ou `DESCONHECIDO`.
- **AmostraDeSlippage**: preço de referência, preço obtido, lado e instante.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-101**: 30 dias corridos de operação ininterrupta na testnet, sem
  intervenção manual, com o registro completo e sem buraco de `seq`.
- **SC-102**: Slippage real medido sobre no mínimo 100 preenchimentos, com
  mediana e p95 registrados no `27_CONFIGURATION_REGISTRY.md` e o parâmetro
  promovido de `ASSUMED` a `MEASURED`.
- **SC-103**: Divergência entre posição local e reportada: zero ocorrências não
  detectadas. Toda divergência que ocorrer MUST aparecer no registro com
  classificação.
- **SC-104**: Nenhuma ordem duplicada em todo o período, verificável por
  identificador de cliente no registro.
- **SC-105**: Reinício do processo com posição aberta: posição recuperada e
  reconciliada em toda ocorrência.
- **SC-106**: Divergência entre o resultado do paper e o do backtest no mesmo
  período: medida, explicada e registrada antes de qualquer pedido de Porta 3.

## Assumptions

- A testnet da Bybit tem o mesmo comportamento de API que a produção. Onde
  divergir, a divergência é achado a registrar, não a contornar.
- O histórico público continua vindo do endpoint sem credencial, como hoje.
- Uma máquina com conectividade estável está disponível pelos 30 dias. Esta
  feature não trata de infraestrutura.

## Fora de escopo

- Execução com capital real. Continua recusada, e a promoção é ato humano
  registrado após as portas 1 e 2.
- WebSocket. A primeira versão usa REST; o hot path por WebSocket é desenho
  registrado no `MD BOT/01_EXECUTION_ENGINE.md` e não é exigido pela Porta 2.
- Recovery. Foi admitido pela emenda 2.0.0, mas o período que delimita o
  orçamento não foi decidido — ADR-003. Não gera tarefa aqui.
- Estratégia. Continua fora: a Porta 2 valida execução, não vantagem.
