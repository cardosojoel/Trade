# Trade

Robô de day trade automatizado de Bitcoin. **Mercado spot, apenas comprado** —
sem alavancagem e sem venda a descoberto.

> **Estado atual:** núcleo funcionando em modo backtest. Paper trading e
> execução com capital real **não existem** e são recusados explicitamente.
> Ver [CURRENT_STATE.md](CURRENT_STATE.md).

---

## O que ele faz hoje

```bash
# 1. Buscar o histórico público da Bybit
trade collect --symbol BTCUSDT --interval 1m --from 2026-09-01 --to 2026-09-19

# 2. Avaliar uma estratégia sobre ele
trade backtest --mode backtest --symbol BTCUSDT --interval 1m \
  --from 2026-09-01 --to 2026-09-19 --capital 10000 \
  --strategy sma-cross --strategy-params fast=9,slow=21 \
  --limits examples/limits.toml --fees examples/fees.toml
```

```
Execução 01M2YPAXW5NENBX62BW07Y8H0V  ·  modo=backtest

  Capital inicial            10000
  Resultado líquido         -97.59     -1.0%
  Taxas                      87.63
  Slippage                   44.31

  Operações                     44
  Profit factor               0.25
  Drawdown máximo            97.59     +1.0%

  Ordens recusadas               0
  Velas percorridas           1440
```

A estratégia `sma-cross` é **referência de teste, não recomendação de
investimento**. Ela existe para exercitar o ciclo completo com comportamento
previsível o bastante para ser conferido à mão — e o resultado acima, negativo
depois dos custos, é o achado clássico de quem finalmente modela taxa e
slippage.

---

## Instalação

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

git clone https://github.com/cardosojoel/Trade && cd Trade
cargo build --release
cp examples/limits.toml examples/fees.toml .    # e ajuste os valores
```

Nada mais é necessário: o SQLite vem embutido no binário e não há serviço a
subir. A toolchain é fixada em `rust-toolchain.toml` — build determinístico é
requisito, não preferência.

---

## O desenho, em uma ideia

**A estratégia não envia ordens.** Ela recebe o estado do mercado e devolve um
sinal. Quem converte sinal em ordem é o motor, e o caminho do motor até o
mercado atravessa obrigatoriamente a camada de risco, que **possui** o executor.

Não existe "caminho alternativo" a proibir por disciplina, porque não existe
tipo que o permita: `trade-strategy` não declara `trade-ports` como dependência
e por isso não consegue sequer **nomear** `OrderExecutor`.

```
crates/
├── trade-domain/      tipos puros; nenhuma E/S
├── trade-ports/       as traits de fronteira
├── trade-risk/        a camada de risco; possui o executor
├── trade-strategy/    estratégias; depende só do domínio
├── trade-backtest/    o motor
├── trade-storage/     SQLite
├── trade-bybit/       coleta; única crate com HTTP
└── trade-cli/         binário `trade`; o ponto de composição
```

### Três garantias que o build cobra

Não são convenção. Verifiquei as três violando de propósito:

| Garantia | O que acontece se for violada |
|---|---|
| Estratégia, risco e backtest não alcançam a corretora nem a rede | `tests/architecture.rs` falha, CI barra o merge |
| Nenhum `f32`/`f64` em caminho monetário | `tests/no_float.rs` falha, CI barra o merge |
| Obter o executor de dentro do `RiskGuard` | **Não compila** |

---

## Segurança

- **Nenhuma credencial em lugar nenhum.** O endpoint de histórico da Bybit é
  público, e o comando `collect` não expõe parâmetro de chave — a forma mais
  simples de garantir isso é não haver o que passar.
- **O modo de execução é obrigatório e sem padrão.** Rodar sem `--mode` falha.
  `paper` e `live` são recusados com mensagem própria, distinta de "valor
  inválido": a diferença entre "ainda não implementado" e "erro de digitação"
  importa.
- **Os limites de risco vivem em `limits.toml`**, nunca no código. Uma ordem que
  passe de qualquer um deles é **recusada**, nunca ajustada em silêncio para
  caber.
- **Kill switch por arquivo sentinela**, acionável de qualquer terminal com
  efeito sobre um processo em operação — e que **não liquida** posição aberta.

```bash
trade kill              # aciona
trade kill --release    # libera; ato humano explícito
```

---

## Verificação

```bash
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
```

O CI roda as travas de arquitetura **primeiro e isoladas**: se uma falha, o
problema é arquitetural e não adianta saber se o resto passou.

Toda decisão do robô é reconstituível em SQL, sem reexecutar nada — ver
[docs/auditoria.md](docs/auditoria.md).

---

## Documentos

| Arquivo | O que é |
|---|---|
| [CURRENT_STATE.md](CURRENT_STATE.md) | Onde o projeto está agora |
| [CLAUDE.md](CLAUDE.md) | Diretrizes de trabalho e invariantes |
| `.specify/memory/constitution.md` | Governa o projeto; prevalece sobre tudo |
| `specs/001-nucleo-execucao/` | Spec, plano, pesquisa, contratos e tarefas |
| [docs/auditoria.md](docs/auditoria.md) | Consultas de reconstituição e conferência |

---

## Aviso

Software de negociação automatizada opera com dinheiro real e pode causar
perdas. Este projeto está em desenvolvimento, **nunca operou com capital real**,
e a constitution exige backtest, paper trading e liberação humana registrada
antes que isso mude. Os limiares de risco distribuídos em `examples/` são
valores de partida provisórios, **não calibrados** — revise-os contra o seu
capital antes de confiar em qualquer resultado.
