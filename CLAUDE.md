# CLAUDE.md

Diretrizes de comportamento para reduzir erros comuns de LLM em código.

**Tradeoff:** estas diretrizes privilegiam cautela sobre velocidade. Para tarefas
triviais, use bom senso.

## 1. Pensar antes de codar

Não presuma. Não esconda confusão. Exponha tradeoffs.

Antes de implementar:

- Declare as premissas explicitamente. Se houver incerteza, pergunte.
- Se existem várias interpretações, apresente-as — não escolha em silêncio.
- Se existe um caminho mais simples, diga. Discorde quando for o caso.
- Se algo não está claro, pare. Nomeie o que confunde. Pergunte.

## 2. Simplicidade primeiro

O mínimo de código que resolve o problema. Nada especulativo.

- Nenhuma funcionalidade além do que foi pedido.
- Nenhuma abstração para código de uso único.
- Nenhuma "flexibilidade" ou "configurabilidade" que não foi solicitada.
- Nenhum tratamento de erro para cenários impossíveis.
- Se escreveu 200 linhas e dava para 50, reescreva.

Pergunte-se: "um engenheiro sênior diria que isto está complicado demais?" Se
sim, simplifique.

## 3. Mudanças cirúrgicas

Toque apenas no que precisa. Limpe apenas a própria bagunça.

Ao editar código existente:

- Não "melhore" código, comentários ou formatação adjacentes.
- Não refatore o que não está quebrado.
- Siga o estilo existente, mesmo que você faria diferente.
- Se notar código morto não relacionado, **mencione** — não apague.

Quando suas mudanças criam órfãos:

- Remova imports, variáveis e funções que **as suas** mudanças tornaram inúteis.
- Não remova código morto preexistente sem que peçam.

O teste: toda linha alterada deve rastrear diretamente ao pedido do usuário.

## 4. Execução orientada a objetivo

Defina critérios de sucesso. Itere até verificar.

Transforme tarefas em objetivos verificáveis:

- "Adicionar validação" → "escrever testes para entradas inválidas e fazê-los passar"
- "Corrigir o bug" → "escrever um teste que o reproduz e fazê-lo passar"
- "Refatorar X" → "garantir que os testes passam antes e depois"

Para tarefas de vários passos, declare um plano curto:

```
1. [passo] → verificar: [checagem]
2. [passo] → verificar: [checagem]
3. [passo] → verificar: [checagem]
```

Critérios fortes permitem iterar sozinho. Critérios fracos ("faça funcionar")
exigem esclarecimento constante.

---

# Específico deste projeto

## Início de sessão: comece pelo CURRENT_STATE.md

**Toda sessão nova neste projeto começa lendo `CURRENT_STATE.md`**, e é a partir
dele que se decide por onde continuar. O arquivo traz o que funciona, o que
falta por fase, as pendências e as decisões em aberto.

Um hook `SessionStart` em `.claude/settings.json` injeta o conteúdo
automaticamente. Se por algum motivo ele não rodar, leia o arquivo antes de
propor qualquer trabalho — decidir o próximo passo sem saber o estado atual é
como escolher rota sem saber onde se está.

Ao concluir um bloco de trabalho relevante — uma fase, uma correção de defeito,
uma decisão de arquitetura — **atualize o `CURRENT_STATE.md`**. Um arquivo de
estado desatualizado é pior que nenhum: ele faz a sessão seguinte decidir com
base em algo que já não é verdade.

Robô de day trade automatizado de Bitcoin. **Mercado spot, apenas comprado** —
sem alavancagem e sem venda a descoberto.

## A constitution prevalece

`.specify/memory/constitution.md` governa o projeto e tem precedência sobre
qualquer outra prática. Leia antes de alterar estratégia, risco ou execução.

Três princípios são **NÃO-NEGOCIÁVEIS** e só deixam de valer por emenda formal,
nunca por prazo ou exceção pontual:

- **I. Validação antes de capital real** — o modo de execução é obrigatório e
  sem padrão; a promoção para `live` é ato humano registrado.
- **II. Limites de risco invioláveis** — toda ordem atravessa o `RiskGuard`; a
  estratégia não pode elevar nem contornar limite algum. Dentro da cerca, o robô
  opera sozinho: parar para pedir permissão é violação tanto quanto ultrapassar.
- **III. Test-first na lógica crítica** — ordens, risco e cálculo de posição e
  P&L exigem teste escrito e **falhando** antes da implementação. Demais
  módulos: teste convencional.

## Invariantes que o build cobra

Não são convenção — falham a compilação ou o CI:

- `tests/architecture.rs` — `trade-strategy`, `trade-risk` e `trade-backtest`
  não declaram a corretora nem cliente de rede. `trade-strategy` não declara
  `trade-ports` e portanto não consegue nomear `OrderExecutor`.
- `tests/no_float.rs` — nenhum `f32`/`f64` em caminho monetário. Todo valor é
  `rust_decimal::Decimal`.
- `crates/trade-risk/tests/compile_fail/` — obter o executor de dentro do
  `RiskGuard` **não compila**.

Se precisar violar uma destas, o que muda primeiro é a constitution, não o teste.

## Convenções

- Comentários e documentação em português; identificadores em português quando
  o domínio é português.
- Valores monetários em arquivos de configuração são **string**, nunca número:
  número em TOML passa por ponto flutuante.
- Limiares de risco vivem em `limits.toml`, nunca embutidos no código.
- Fluxo Spec Kit: `specs/001-nucleo-execucao/` tem spec, plano e tarefas.

## Comandos

```
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
```

O binário `trade` vive em `crates/trade-cli`. A raiz também é um pacote — só
para hospedar os testes de arquitetura — e por isso **`cargo build --release`
na raiz não recompila o binário**: compila a lib da raiz, diz `Finished` e
deixa um `target/release/trade` velho no lugar. Para medir ou executar, use:

```
cargo build --release --workspace
```
