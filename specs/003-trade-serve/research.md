# Fase 0 — o que foi conferido antes de planejar

**Escrita em:** 2026-09-21 · commit `1442d6e`

Cada item abaixo foi verificado contra o repositório ou contra a árvore de
dependências, não suposto. Três mudaram o plano.

---

## 1. O lado de **leitura** do registro não existe

**Conferido.** Busca por `SELECT` em `crates/trade-storage/src/`, fora de
testes: o `candle_repo` e o `market_source` leem velas; a `recuperacao` lê
`audit_event`; o `runs_repo` lê uma linha só — a última execução `paper` em
aberto. **Nada lê `run` para listar, nada lê `trade`, nada lê `metrics`.**

A crate de persistência, até hoje, só **escreveu** execução, extrato e
métricas. Quem quis ler, leu por SQL à mão — é o que o `docs/auditoria.md`
documenta, com consultas para o operador rodar no `sqlite3`.

**Decisão.** O lado de leitura é trabalho desta feature, e vive em
`trade-storage`, não na crate do servidor. Consulta a banco fica onde o acesso
a banco mora; o servidor recebe estruturas e as serializa.

**Alternativa recusada.** Pôr as consultas em `trade-serve`. Espalharia
conhecimento do esquema por duas crates, e a próxima mudança de esquema
precisaria lembrar das duas.

**Consequência para o tamanho da feature.** As fatias 2, 3 e 4 do plano são
maiores do que "servir JSON": cada uma traz consulta nova em `trade-storage`,
com teste próprio. É melhor saber disso agora do que ao abrir a fatia.

---

## 2. `Decimal` não serializa sozinho, e isso é bom

**Conferido.** O `Cargo.toml` da raiz declara
`rust_decimal = { version = "1.43.0", features = ["macros"] }` — **sem** a
feature `serde`. Um `Decimal` não tem `Serialize`, e um `json!` que o receba
não compila.

**Decisão.** Manter assim, e seguir a convenção que o `audit.rs` já usa: uma
função de uma linha que devolve `v.to_string()`, aplicada em cada campo
monetário.

**Por que não ligar a feature.** Ligar `serde` faria `Decimal` serializar —
como **número JSON**, por padrão. Isso é exatamente o que o `FR-002` proíbe e o
`SC-002` cobra varrendo toda resposta. Existe `serde-with-str` para forçar
string, mas aí a garantia passa a depender de cada campo ter o atributo certo.

**O que isso compra.** Sem a feature, esquecer a conversão **não compila**.
`SC-002` deixa de depender de alguém ter lembrado, que é a mesma forma de
garantia que `tests/no_float.rs` já dá ao código.

---

## 3. `tiny_http` 0.12: o que entrega e o que não

**Conferido** pela árvore de dependências: `tiny_http v0.12.0` traz `ascii`,
`chunked_transfer`, `httpdate` e `log`. Nenhum runtime assíncrono, nenhum TLS,
nenhuma macro.

**Entrega**: aceitar conexão, ler método, caminho e cabeçalhos, e responder com
status e corpo. Bloqueante, um `Request` por vez, e a mesma instância pode ser
lida de várias threads.

**Não entrega**, e é trabalho desta feature:

- **Roteamento.** Não há tabela de rotas; o caminho chega como texto e o
  despacho é nosso.
- **Parâmetros de consulta.** `?a={id}&b={id}` chega dentro do caminho, sem
  parsing.
- **Corpo JSON.** Chega como leitor de bytes.

**Decisão.** Um despacho escrito à mão sobre onze caminhos conhecidos. Não é
framework: é um `match` sobre segmentos, e a spec fixa os onze de antemão.

**A alternativa que quase ganhou.** Escrever o servidor inteiro sobre
`TcpListener` recebeu **0,27** do Jev, contra 0,73. O repositório já tem um
servidor assim, em `crates/trade-bybit/tests/common/mod.rs`, com o comentário
"em vez de trazer uma dependência — precisa responder JSON fixo e contar
requisições, nada além disso". A distância entre aquilo e um servidor de
verdade é `Content-Length`, `chunked`, conexões persistentes, cabeçalhos
malformados e clientes que desistem no meio. É pouco código e muitos casos.

---

## 4. Como se confere que o servidor **não** escuta fora do `127.0.0.1`

O `SC-007` pede que isso seja conferível "por tentativa a partir de outra
interface de rede", e uma máquina de CI pode não ter outra interface.

**Decisão.** Duas verificações, e não uma:

1. **De tipo**, no código: o endereço de escuta é construído a partir de
   `Ipv4Addr::LOCALHOST`, e não de texto de configuração. Não há caminho em que
   um endereço venha de fora e vire o que se escuta.
2. **De comportamento**, no teste: o servidor sobe, e o teste confere que o
   socket ligou em `127.0.0.1` e não em `0.0.0.0` — lendo o endereço que o
   próprio sistema devolve, não o que pedimos.

A tentativa a partir de outra interface fica para a verificação manual do
`quickstart.md`, onde há uma máquina de verdade.

---

## 5. O que fica sem resposta, e por quê

**Os limiares de desempenho.** `SC-013` e `SC-014` nascem como configuração de
partida, e a própria spec diz que **nenhum foi medido**. Medi-los exige o
servidor existir. Ficam como estão e são revisados quando houver o que medir.

**A versão do código na tabela `run`.** Não existe — pendência P6/P9. A spec
foi escrita contando com isso: campo presente e nulo, e comparação declarada
não confiável. A decisão 036 do Jev confirmou que a interface nasce assim
(`noul` 0,31), e não é trabalho desta feature resolver.
