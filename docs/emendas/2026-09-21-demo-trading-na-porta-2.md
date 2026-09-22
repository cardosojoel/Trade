# Proposta de emenda — o ambiente da Porta 2

**Proposta em:** 2026-09-21 · **Estado:** aguardando aprovação do mantenedor
**Constitution vigente:** 2.0.0 (`Ratified` 2026-09-20)
**Procedimento:** *Governança → Emendas* — proposta por escrito, com racional e
impacto sobre specs e código existentes, aprovada pelo mantenedor, com
atualização de versão e data no rodapé.

---

## O que se propõe mudar

Três trechos nomeiam a testnet:

| Onde | Texto vigente |
|---|---|
| *Restrições → Corretora* | "**Testnet da Bybit é o ambiente da porta de paper trading.**" |
| *Portas de Promoção*, porta 2 | "≥ 30 dias corridos ininterruptos **em testnet**, com o mesmo código que iria para `live`" |
| *Restrições → Segredos* | "Chaves de `paper`/testnet e de `live` MUST ser distintas e MUST NOT coexistir" |

A proposta troca o ambiente da Porta 2 de **testnet** para **Demo Trading**, e
acrescenta salvaguarda que a troca torna necessária.

## Por que

**A testnet não produz o dado que a Porta 2 existe para produzir.**

Dois critérios da feature 002 são medições sobre o mercado, não sobre o código:

- `SC-102` — slippage real sobre no mínimo cem preenchimentos, com mediana e
  p95, e o parâmetro promovido de `ASSUMED` a `MEASURED`;
- `SC-106` — divergência entre o resultado do paper e o do backtest no mesmo
  período, medida e explicada.

A testnet da Bybit tem livro de ofertas raso e movimento artificial. Slippage
medido contra esse livro é um número que sai e não significa: seria promovido
de `ASSUMED` a `MEASURED` sem que a medição tivesse medido o mercado. O Demo
Trading opera sobre o **livro e os preços de produção**, com saldo fictício —
para essas duas medições, e só para elas, é o único dos quatro ambientes que
serve.

A observação vale a pena explicitar: o que a Porta 2 valida não é apenas que o
código funciona. É **quanto custa operá-lo**. Essa pergunta não tem resposta
num mercado que não existe.

## O que isso custa, e a salvaguarda que passa a ser exigida

**A isolação enfraquece, e isso é o preço.**

A chave de testnet pertence a uma **conta inteiramente separada**: outro
cadastro, sem qualquer acesso a fundo real. É isolação de conta. A chave de
Demo Trading é criada **a partir da conta de produção** — entra-se no
`bybit.com` e alterna-se para o módulo de demo. É isolação de módulo dentro da
mesma conta.

E os domínios diferem por um prefixo:

```
https://api-demo.bybit.com     demo
https://api.bybit.com          produção
```

Hoje, apontar para produção por engano não produziria ordem alguma: a chave de
testnet não existe lá. Depois desta emenda, apontar para produção por engano
significa uma chave que a conta de produção **reconhece**.

Por isso a emenda **não é só de texto**. Ela exige três salvaguardas, e cada
uma vem com **o que a cobra** — porque invariante que não se confere é
recomendação, e recomendação não é o que separa um defeito de dinheiro real.

### 1. O domínio de produção é inalcançável em modo `paper`

Não por configuração. Por construção, e conferido pelo build.

- O literal `https://api.bybit.com` MUST aparecer em **um único lugar** do
  código: o braço `Ambiente::Producao`. Cobrado por teste que varre as fontes
  e falha se aparecer em qualquer outro arquivo — o mesmo formato de
  `tests/no_leverage.rs`, que já varre vocabulário proibido.
- O mapa de modo para domínio MUST ser **total e exaustivo**, sobre enums sem
  variante coringa, de modo que acrescentar um modo quebre a compilação em vez
  de cair num padrão. É a mesma garantia que `LimitBreach` já dá à camada de
  risco.
- Teste que, para **todo** `ExecutionMode` que não seja `live`, o domínio
  resolvido nunca é o de produção. Percorre as variantes, não um exemplo.

**Por que assim:** um modo que pudesse escolher o domínio por texto de
configuração seria configuração, e configuração se erra em silêncio. O que não
compila não se erra.

### 2. Permissão de saque é condição de partida, não checagem

Numa chave emitida pela conta de produção, permissão de saque é acesso a fundo
real — deixa de ser precaução abstrata.

- A sessão MUST abortar antes da primeira ordem se a chave tiver qualquer
  permissão de carteira que mova fundos. Já existe `verificar_sem_saque`, com
  lista de **negação** e não de aceitação, para que um nome novo que dê saque
  seja recusado por não estar previsto.
- Cobrado por teste sobre duplo de transporte: uma chave com saque faz
  `paper rodar` terminar com código de erro e **zero ordens emitidas** —
  contando as que alcançaram o executor, não só o código de saída.

### 3. A não coexistência passa a valer entre três, e é conferida

Hoje o sistema aborta se `BYBIT_TESTNET_KEY` e `BYBIT_KEY` estiverem presentes
juntas. Isso funcionava porque as duas vinham de contas diferentes. Com demo
são credenciais irmãs da mesma conta, e separá-las por nome de variável passa a
ser convenção.

- A regra MUST valer para **qualquer par** entre as três credenciais —
  testnet, demo e produção. Duas presentes, a inicialização aborta.
- Cobrado por teste que percorre **todas as combinações** de presença das três,
  e não três casos escolhidos a dedo: das oito combinações possíveis, as quatro
  com mais de uma credencial MUST abortar.

**O que isto não resolve, e é honesto dizer:** nenhuma das três impede quem
opera de exportar deliberadamente a credencial de produção e apontar para lá. O
que elas impedem é o **engano** — domínio digitado errado, modo herdado de uma
sessão anterior, variável que sobrou no ambiente. A barreira contra o ato
deliberado é a Porta 3, e ela é humana por definição.

## Impacto sobre specs e código existentes

**Nenhuma execução de paper existe.** O `runs.db` tem nove execuções, todas em
modo `backtest`, e zero linhas com `mode = 'paper'` (lido em 2026-09-21, commit
`b701977`). Nada que já rodou deixa de estar conforme — o que muda é o que pode
ser construído daqui em diante.

O que precisa mudar no código:

| Onde | O quê |
|---|---|
| `trade-bybit/src/credencial.rs` | `Ambiente` conhece dois domínios; passa a conhecer três |
| `trade-cli` | composição do modo `paper` aponta para demo |
| `specs/002-paper-trading/spec.md` | `SC-101` e os `FR-1xx` que dizem testnet |
| `specs/002-paper-trading/tasks.md` | T032, que diz "chave de testnet" |
| `tests/` | invariante novo, cobrando que produção seja inalcançável em `paper` |

As fatias 1 a 5c da feature 002 **não** mudam: transporte, assinatura,
adaptador, reconciliação, laço e medição são independentes do domínio.

## Versão

A classificação é parte do que se pede que o mantenedor decida.

- **MAJOR**, pela definição do próprio documento — *"redefinição que invalide
  código ou processo já em conformidade"* —, se se entender que a Porta 2
  definida sobre testnet é processo em conformidade que esta emenda invalida.
- **MINOR**, se se entender que nenhuma execução de paper existiu, nada foi
  invalidado, e o que há é expansão material de orientação.

O Princípio V, onde o trecho da corretora vive, **não** é marcado
NÃO-NEGOCIÁVEL. O Princípio I, que é, não nomeia ambiente: ele exige as três
portas na ordem, e continua exigindo.

## O que esta emenda não faz

Não antecipa a Porta 3, não altera os trinta dias, não altera métrica alguma, e
não toca os princípios não-negociáveis. A promoção para capital real continua
sendo ato humano registrado, depois da revisão das portas 1 e 2.
