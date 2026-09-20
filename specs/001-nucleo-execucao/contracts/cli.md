# Contract — Linha de comando (`trade`)

**Feature**: 001-nucleo-execucao | **Date**: 2026-09-20

O binário `trade` é a única interface do sistema nesta feature. Saídas em `stdout`,
erros em `stderr`, código de saída significativo.

---

## `trade collect` — coletar histórico

```
trade collect --symbol BTCUSDT --interval 1m --from 2025-09-01 --to 2026-09-01
              [--db data/market.db] [--max-retries 5]
```

| Argumento | Obrigatório | Nota |
|---|---|---|
| `--symbol` | sim | par; `BTCUSDT` nesta feature |
| `--interval` | sim | `1m`, `5m`, `15m`, `1h`, `4h`, `1d` |
| `--from`, `--to` | sim | datas UTC, inclusivo/exclusivo |
| `--db` | não | padrão `data/market.db` |
| `--max-retries` | não | tentativas de falha transitória (FR-024a) |

**Não aceita credencial.** Não há `--api-key` nem leitura de variável de ambiente de
segredo: o endpoint é público (R-005), e não oferecer o parâmetro é a forma mais
simples de garantir FR-012.

Comportamento: pagina para frente em blocos de 1000, inverte cada página (a Bybit
devolve do mais recente para o mais antigo), **descarta a vela ainda em formação**,
grava página a página em transação e registra as lacunas encontradas.

Saída:

```
Coletando BTCUSDT 1m de 2025-09-01 a 2026-09-01
  526 páginas · 525.600 velas · 12.480 já existentes · 3 lacunas
Gravado em data/market.db
```

Repetir o mesmo comando após interrupção continua de onde parou e não duplica nada.

---

## `trade backtest` — executar um backtest

```
trade backtest --mode backtest
               --symbol BTCUSDT --interval 1m
               --from 2025-09-01 --to 2026-09-01
               --capital 10000
               --strategy sma-cross --strategy-params fast=9,slow=21
               --limits limits.toml
               [--fees fees.toml] [--market-db data/market.db] [--runs-db data/runs.db]
```

### `--mode` é obrigatório e não tem padrão

Este é o ponto onde o Princípio I vira comportamento observável:

| Invocação | Resultado |
|---|---|
| sem `--mode` | **erro**, código de saída 2: `--mode é obrigatório e não possui valor padrão. Modos disponíveis: backtest` |
| `--mode backtest` | executa |
| `--mode paper` | **erro**, código 2: `modo 'paper' ainda não implementado — apenas 'backtest' está disponível nesta versão` |
| `--mode live` | **erro**, código 2: mesma forma, para `live` |

`paper` e `live` são recusados com mensagem própria, e não como "valor inválido"
(FR-003). A diferença importa: "valor inválido" sugere erro de digitação e convida a
tentar de novo; "ainda não implementado" informa o estado real do sistema.

### Saída

```
Execução 01K5R8...  modo=backtest  BTCUSDT 1m  2025-09-01 → 2026-09-01

  Capital inicial     10.000,00
  Resultado líquido    1.243,50   (+12,4%)
  Taxas                  186,20
  Slippage                94,80

  Operações                  87
  Profit factor            1,42
  Drawdown máximo          8,3%

  Ordens recusadas           14   (tamanho 9 · exposição 5)
  Paradas                     0

Registro: data/runs.db · run_id 01K5R8...
```

Quando não houver operação perdedora, a linha do profit factor mostra `indefinido`,
nunca `∞` — ver data-model.

### Códigos de saída

| Código | Significado |
|---|---|
| 0 | execução completa |
| 2 | erro de uso: modo ausente, modo não implementado, argumento inválido |
| 3 | histórico ausente ou insuficiente para o período pedido |
| 4 | execução interrompida por falha de integridade (FR-024b) |
| 5 | capital esgotado durante o período (FR-032) |

Códigos 4 e 5 são distintos de 0 porque ambos terminam a execução **sem** completar o
período. Um script que apenas verificasse "terminou" trataria os três como sucesso; os
códigos separados impedem isso.

---

## `trade kill` — acionar o kill switch

```
trade kill [--runs-db data/runs.db]
trade kill --release
```

Aciona ou libera o kill switch sem alteração de código (FR-023). Uma vez acionado, a
liberação é ato humano explícito — `--release` não é invocado por nenhuma rotina
automática do sistema.

Acionar **não liquida posição aberta** (FR-023a): a execução para de aceitar ordens,
encerra de forma controlada e reporta o que ficou em aberto.

---

## Formato dos arquivos de configuração

`limits.toml` — a cerca (FR-025). Sem valores embutidos no código:

```toml
max_daily_loss       = "200.00"   # strings: parse direto para Decimal
max_position_size    = "1000.00"
max_total_exposure   = "2000.00"
max_orders_per_window = 10
window_minutes        = 60
max_transient_retries = 5
```

`fees.toml` — custo de transação (FR-027):

```toml
taker_fee_rate = "0.001"
slippage_rate  = "0.0005"
```

Valores como string, pelo mesmo motivo do esquema do banco: passar por float na
desserialização desfaria a exatidão antes mesmo do primeiro cálculo.
