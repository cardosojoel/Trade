# Implementation Plan: Paper Trading na Testnet da Bybit

**Branch**: `002-paper-trading` · **Spec**: [spec.md](spec.md)

**Constitution**: v2.0.0

## Summary

Uma crate nova, `trade-paper`, que implementa `OrderExecutor` contra a testnet
da Bybit, e um laço de execução contínua que substitui o laço de backtest sem
tocar em estratégia, risco ou motor de decisão.

A escolha estruturante é onde a assinatura HMAC vive: em `trade-bybit`, ao lado
do cliente HTTP que já existe, e não numa crate nova. O adaptador de paper é
composição — cliente autenticado mais tradução de ordem —, não um segundo
cliente.

## Technical Context

**Linguagem**: Rust estável 1.98.1, edition 2024. Sem async: o laço de paper
opera na casa de uma ordem por hora, e um runtime assíncrono seria complexidade
sem benefício mensurável. `REQ-PERF-*` do `MD BOT/` trata do hot path por
WebSocket, que está fora de escopo.

**HTTP**: `ureq`, já em uso no coletor. A assinatura é HMAC-SHA256 sobre
`timestamp + api_key + recv_window + payload`, conforme a documentação da
Bybit.

**Credenciais**: variáveis de ambiente `BYBIT_TESTNET_KEY` e
`BYBIT_TESTNET_SECRET`. Nunca arquivo, nunca argumento de linha de comando — o
argumento aparece em `ps`.

**Estado**: o `runs.db` que já existe passa a ser fonte de recuperação, não só
de auditoria. Reiniciar lê de lá.

**Relógio**: `Clock` real. O `BacktestClock` fica inalcançável em paper por
construção — a composição em `trade-cli` decide qual entra, e o modo decide a
composição.

## Constitution Check

| Princípio | Como esta feature o respeita |
|---|---|
| I — validação antes de capital real | `paper` é a Porta 2. `live` continua recusado no parse do modo. |
| II — limites invioláveis | O adaptador implementa `OrderExecutor`, e o `RiskGuard` continua sendo o único caminho até ele. Nada nesta feature alcança a corretora por fora. |
| III — test-first na lógica crítica | Assinatura, reconciliação e tradução de ordem são lógica crítica: teste escrito e falhando antes. |
| IV — auditabilidade | Cada preenchimento registra referência, obtido, taxa e moeda da taxa. O `seq` continua sem buraco entre reinícios. |
| V — independência de corretora | Estratégia e risco continuam sem declarar `trade-paper`. O teste de arquitetura passa a cobrar isso também. |
| VI — segurança de credenciais | Ambiente, permissão de saque verificada antes da primeira ordem, e nada de segredo em log ou registro. |

**Emenda 2.0.0**: o prazo de 72h entra como regra do laço — posição que o
atinge é encerrada. O Recovery **não** entra: a ADR-003 deixou em aberto qual
período delimita o orçamento, e `REQ-ADR-003` proíbe gerar tarefa até que seja
decidido.

## Project Structure

```
crates/
├── trade-bybit/          ganha o cliente autenticado e a assinatura
│   └── src/auth.rs       HMAC, recv_window, ordenação de parâmetros
├── trade-paper/          crate nova — o adaptador
│   ├── src/executor.rs   OrderExecutor contra a testnet
│   └── src/reconcile.rs  posição local × reportada
├── trade-session/        crate nova — o laço contínuo
│   ├── src/laco.rs       prazo de 72h, virada de dia, parada limpa
│   └── src/relogio.rs    o relógio do mundo
└── trade-cli/            compõe conforme o modo
```

`trade-paper` depende de `trade-ports`, `trade-domain` e `trade-bybit`. **Não**
depende de `trade-strategy` nem de `trade-risk`: quem compõe é a CLI.

`trade-session` depende de `trade-ports`, `trade-domain` e `trade-risk`, e
**não** depende de corretora, de rede nem de `trade-backtest`. Está na lista
`ISOLATED` de `tests/architecture.rs`.

> **Emenda de 2026-09-21 — decisão 032 do Jev** (`crate_nova_isolada`, 0,95 ·
> confiança 0,93; emendar o plano, `noul` 0,89).
>
> Até aqui este plano dizia duas coisas que não podem valer juntas: que o laço
> vive em `trade-paper/src/loop.rs`, e que `trade-paper` não depende de
> `trade-risk`. A T029 manda o laço encaminhar pelo `RiskGuard`, e nomear o
> `RiskGuard` exige declarar `trade-risk`.
>
> O que decidiu a escolha não foi a contradição, foi o custo dela.
> `trade-paper` declara `trade-bybit`, porque é ela que fala com a corretora.
> Pôr o laço lá dentro deixaria, na mesma crate, o cliente da corretora ao lado
> do guard — e "nenhuma ordem alcança a corretora fora da camada de risco"
> passaria de garantia de compilação a cuidado de quem escreve. `trade-session`
> repete a forma que `trade-backtest` já tem: conhece o risco, não conhece o
> mercado.
>
> `loop` também é palavra reservada em Rust, e um módulo não pode se chamar
> assim. O arquivo é `laco.rs`.

## Ordem de implementação

Em fatias que fecham sozinhas, cada uma verificável sem a seguinte:

1. **Modo `paper` existe e é recusado sem credencial.** Muda `ExecutionMode`,
   a leitura do ambiente e a verificação de permissão. Verificável sem rede.
2. **Assinatura.** HMAC sobre vetores conhecidos da documentação. Verificável
   sem rede.
3. **Adaptador.** `OrderExecutor` contra um duplo HTTP. Verificável sem rede.
4. **Reconciliação.** Comparação e classificação. Verificável sem rede.
5. **Laço contínuo.** Relógio real, prazo de 72h, reinício. Verificável com
   duplo.
5b. **A composição.** Fonte de velas ao vivo, reconciliação na abertura e o
   comando que liga tudo. Verificável com duplo.
6. **Primeira ordem real na testnet.** Exige credencial.
7. **Os 30 dias.** Exige credencial e tempo.

As fatias 1 a 5b não precisam de credencial e podem ser feitas agora. As 6 e 7
são o que só o mantenedor destrava.

> **Emenda de 2026-09-21 — decisão 034 do Jev** (`fatia_nova_antes_da_6`,
> 0,99 · confiança 0,98). A fatia 5b não estava aqui, e as peças das fatias 3,
> 4 e 5 terminaram sem chamador nenhum. As tarefas da fatia 6 são atos —
> criar a chave, emitir a primeira ordem, medir a divergência —, e nenhuma
> delas é escrever o código que liga as peças. A sessão passa também a
> **reconciliar ao abrir**, e o veredito ganha destino no registro
> (`reconciliar_e_dar_destino_ao_veredito`, 0,89 · confiança 0,84).

## Post-Design Constitution Re-Check

Uma dependência nova preocupa: `trade-paper` conhece a Bybit. O teste de
arquitetura passa a cobrar que ela não seja declarada por estratégia, risco ou
backtest — a mesma trava que já existe para `trade-bybit`, estendida.

## Complexity Tracking

| Decisão | Alternativa mais simples | Por que não |
|---|---|---|
| Crate nova `trade-paper` | pôr o executor em `trade-bybit` | `trade-bybit` é coleta de histórico público e não tem credencial; misturar as duas faria o coletor carregar segredo sem precisar |
| Estado no `runs.db` | arquivo de estado à parte | um segundo lugar de verdade sobre posição é exatamente o que o Princípio II não tolera |
| REST em vez de WebSocket | — | WebSocket é o hot path do `MD BOT/01`, e a Porta 2 não o exige. Uma ordem por hora não precisa de microssegundos |
