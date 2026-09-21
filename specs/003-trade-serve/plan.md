# Implementation Plan: Servidor local de leitura do registro (`trade serve`)

**Branch**: `003-trade-serve` · **Spec**: [spec.md](spec.md)

**Constitution**: v2.0.0

## Summary

Uma crate nova, `trade-serve`, que serve onze rotas HTTP em `127.0.0.1` sobre o
`runs.db` e o `market.db`. Dez são de leitura. A décima primeira — `POST /runs`
— é a **única** superfície de escrita, e não emite ordem: inicia uma execução
pela mesma composição que o `trade backtest` já usa.

A escolha estruturante é **de onde o servidor não alcança**. Ele não declara a
corretora, não declara a camada de risco e não declara o `trade-cli`: entra na
lista `ISOLATED` de `tests/architecture.rs`, ao lado de estratégia, risco,
backtest e sessão. Iniciar execução chega por um ponto de ligação que a CLI
fornece, e é o que faz `SC-008` ser conferível pelo build em vez de por revisão.

## Technical Context

**Linguagem**: Rust estável 1.98.1, edition 2024.

**HTTP**: `tiny_http` 0.12 — síncrona, sem runtime assíncrono, quatro
dependências transitivas pequenas (`ascii`, `chunked_transfer`, `httpdate`,
`log`). **Sem async**, pela mesma razão que a feature 002 recusou: a interface
tem um usuário na própria máquina, e um backtest de doze meses roda em cerca de
um segundo. Decisão 037 do Jev (`biblioteca_sincrona_pequena`, 0,73 · confiança
0,63).

> A alternativa de escrever o servidor à mão sobre `TcpListener` recebeu
> **0,27** — não é desprezível, e o repositório já tem um servidor de teste
> assim. O que pesou contra: aquele responde corpo fixo, e este precisa de
> rotas com parâmetros, cabeçalhos, códigos de status e concorrência. A pilha
> assíncrona recebeu **0,00**.

**Concorrência**: um punhado de threads atendendo conexões, e **uma thread por
trabalho iniciado**. `POST /runs` devolve o identificador imediatamente e o
progresso sai por consulta a `GET /jobs/{id}` (`FR-017`, `FR-018`). Não há
conexão mantida aberta empurrando evento.

**Serialização**: `serde_json`, já no workspace. Todo valor monetário, de
quantidade e de preço sai como **string** (`FR-002`). Contagem e identificador
seguem número. É o mesmo invariante que `tests/no_float.rs` cobra no código e
que o `audit.rs` já aplica no payload dos eventos.

**Leitura**: `trade-storage`, que já abre os dois bancos e conhece o esquema.
Nenhuma consulta SQL nova vive na crate do servidor — o que é acesso a banco
fica onde o acesso a banco mora.

**Rede**: escuta exclusivamente em `127.0.0.1`, com porta explícita e sem padrão
que exponha a máquina (`FR-003`). Sem TLS: é um socket local de um dono só.

## Constitution Check

| Princípio | Como esta feature o respeita |
|---|---|
| I — validação antes de capital real | `POST /runs` exige o modo declarado, e `live` **MUST NOT** ser iniciável pelo servidor (`FR-004`). A recusa vem do mesmo `ExecutionMode::from_str` que a CLI usa, não de uma segunda lista. |
| II — limites invioláveis | O servidor não declara `trade-risk` e não tem como nomear o `RiskGuard`. Iniciar execução atravessa a composição da CLI, que é onde a cerca é montada. Não existe rota que altere limite ou taxa (`FR-005`). |
| III — test-first na lógica crítica | A recusa de `live`, a recusa sem modo, a exigência de token e a verificação de origem são lógica crítica: teste escrito e **falhando** antes. O mesmo para a serialização monetária, que `SC-002` cobra varrendo toda resposta. |
| IV — auditabilidade | O servidor **lê** o registro e não escreve nele. A execução que ele inicia grava pela composição de sempre, com o mesmo `AuditRecorder`. Nenhum caminho novo até a auditoria. |
| V — independência de corretora | `trade-serve` entra na lista `ISOLATED`: não declara `trade-bybit`, `trade-paper`, `trade-cli` nem cliente de rede. O build falha se declarar. |
| VI — segurança de credenciais | Nenhuma rota aceita, guarda ou devolve credencial. O token local do `FR-019` não é credencial de corretora e **MUST NOT** ser gravado no registro (`FR-020`). |

**Sobre o que a feature admite não saber.** A versão do código na tabela `run`
**não existe** — é a pendência P6/P9. A spec não espera por ela: o `FR-006`
exige o campo presente e nulo em toda resposta que descreva execução, e o
`FR-007` obriga a comparação entre execuções a se declarar **não confiável**
enquanto for assim, ainda que limites e taxas sejam idênticos. A decisão 036 do
Jev confirmou que a interface pode nascer assim (`noul` 0,31): ausência
declarada é resposta honesta, não defeito. O mesmo padrão vale para o
`fechado_por` de episódios antigos, gravados antes de o campo existir.

## Project Structure

```
crates/
├── trade-serve/          crate nova — o servidor
│   ├── src/rotas.rs      as onze rotas e o despacho
│   ├── src/respostas.rs  serialização, com dinheiro como string
│   ├── src/consultas.rs  agregados sobre o que trade-storage devolve
│   ├── src/guarda.rs     token local e verificação de origem (FR-019)
│   ├── src/erros.rs      o corpo único de recusa (FR-023)
│   └── src/trabalhos.rs  trabalho assíncrono e o progresso por consulta
└── trade-cli/            ganha `trade serve`, e fornece o ponto de ligação
```

`trade-serve` depende de `trade-ports`, `trade-domain`, `trade-storage`,
`tiny_http` e `serde_json`. **Não** depende de `trade-bybit`, `trade-paper`,
`trade-risk`, `trade-backtest` nem `trade-cli`.

### O ponto de ligação

O `FR-001` manda iniciar execução **pela mesma composição que o `trade
backtest` usa**, e o `SC-008` manda o servidor não alcançar a corretora. As
duas valem juntas se o servidor não souber iniciar nada: ele recebe, na
construção, algo que sabe — uma trait em `trade-ports` com um método que aceita
os parâmetros da execução e devolve o identificador do trabalho.

Quem a implementa é `trade-cli`, que já é o ponto de composição. O servidor
chama e não sabe o que há do outro lado, exatamente como o laço de sessão
recebe um `RiskGuard` já montado sem saber qual executor está dentro.

**Por que não é abstração especulativa:** sem ela, `trade-serve` precisaria
declarar `trade-cli`, e herdaria pelo grafo tudo que a CLI alcança — inclusive
a corretora. A trait existe para que o teste de arquitetura possa cobrar o que
o `SC-008` promete.

## Ordem de implementação

Em fatias que fecham sozinhas, cada uma verificável sem a seguinte:

1. **O servidor existe e recusa.** Escuta em `127.0.0.1`, porta explícita, e o
   corpo único de erro. Nenhuma rota de dado ainda — só as recusas, que são o
   que mais importa acertar: sem modo, `live`, sem token, origem de outro
   lugar.
2. **Leitura do que já rodou.** `GET /runs`, `/runs/{id}`, `/runs/{id}/daily`.
   Agregados calculados no servidor, em decimal exato, chegando como string.
3. **Reconstituição.** `/runs/{id}/episodes`, as saídas de um episódio, e os
   cinco elos de `/runs/{id}/chain/{seq}`, ordenados por `seq` e nunca por
   `at_ms`.
4. **Comparação e histórico.** `/runs/compare`, `/datasets`, `/runs/match`.
   É aqui que `FR-007` aparece: a resposta que se declara não confiável.
5. **A escrita.** `POST /runs` e `GET /jobs/{id}`, sobre o ponto de ligação.
6. **O comando.** `trade serve` na CLI, compondo tudo.

As seis não precisam de credencial e não alcançam a rede: o servidor é local, e
os dois bancos são arquivos. Diferente da feature 002, **nada aqui espera um
ato do dono do projeto.**

## Sobre os artefatos que esta fase não gerou

O fluxo prevê `data-model.md` e `contracts/`. Nenhum dos dois foi escrito, e
não por esquecimento: **a própria `spec.md` já os carrega**. Ela tem a seção
*Key Entities* e a seção *Contrato das rotas*, com as onze rotas, o que cada
uma devolve e exemplos de JSON para três delas. Copiar isso para dois arquivos
novos criaria duas fontes para a mesma verdade, e a segunda envelheceria.

A feature 002 fez a mesma escolha pelo mesmo motivo. A 001 gerou os dois
porque o contrato dela — o esquema do evento de auditoria — não cabia na spec.

## Complexity Tracking

| Adição | Por que é necessária | Alternativa mais simples, e por que foi recusada |
|---|---|---|
| Dependência `tiny_http` | Rotas com parâmetros, cabeçalhos, códigos de status e concorrência | Escrever à mão sobre `TcpListener`, como o servidor de teste da crate da corretora. Recusada pelo Jev com 0,27 contra 0,73 — aquele responde corpo fixo, e a distância até um servidor de verdade é onde moram os erros de HTTP que ninguém quer depurar |
| Crate nova `trade-serve` | `SC-008` exige que a fronteira seja conferível pelo `tests/architecture.rs`, e o teste só olha crates | Pôr o servidor em `trade-cli`. Recusada com 0,03 contra 0,96: a CLI é a única camada que o teste não protege, porque ela existe para conhecer tudo |
| Trait de ligação em `trade-ports` | Fazer `FR-001` e `SC-008` valerem juntos | `trade-serve` declarar `trade-cli` e chamar a composição direto. Recusada com 0,01: herdaria pelo grafo tudo que a CLI alcança, inclusive a corretora |
