# Quickstart — Validação do Núcleo de Execução e Risco

**Feature**: 001-nucleo-execucao | **Date**: 2026-09-20

Roteiro para provar, de ponta a ponta, que a feature faz o que a spec promete. Cada
cenário aponta para o critério de sucesso que valida. Não contém código de
implementação — isso é `tasks.md`.

---

## Pré-requisitos

**O toolchain Rust ainda não está instalado nesta máquina.** É o primeiro passo:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustc --version      # registrar em rust-toolchain.toml
```

Nada mais é necessário: o SQLite vem embutido no binário via `rusqlite` com a feature
`bundled`, e não há serviço externo a subir.

Opcional, para inspeção manual: DBeaver (já instalado) apontado para `data/market.db`
e `data/runs.db`.

---

## Compilar e testar

```bash
cargo build --workspace
cargo test  --workspace
```

### Cenário A — a fronteira é real, não combinada

O argumento central do desenho é que estratégia e risco não alcançam a corretora. Isso
se confere sem ler uma linha de código:

```bash
cargo tree -p trade-strategy | grep -i bybit    # não deve retornar nada
cargo tree -p trade-risk     | grep -i bybit    # não deve retornar nada
cargo tree -p trade-backtest | grep -i ureq     # não deve retornar nada
```

**Esperado**: nenhuma saída nos três. Se algum retornar, o Princípio V foi violado e a
violação é estrutural — não adianta corrigir por revisão, o `Cargo.toml` precisa mudar.

> Valida SC-006 e o gate do Princípio V. Vale rodar em CI: é o teste mais barato do
> projeto e o que protege a propriedade mais cara.

---

## Coletar histórico

```bash
trade collect --symbol BTCUSDT --interval 1m \
              --from 2025-09-01 --to 2026-09-01
```

### Cenário B — retomada não duplica

```bash
# interrompa com Ctrl-C no meio da coleta, então repita o mesmo comando
trade collect --symbol BTCUSDT --interval 1m --from 2025-09-01 --to 2026-09-01
```

**Esperado**: a segunda execução reporta as velas já existentes e coleta só o que
falta. Conferência direta:

```sql
-- em data/market.db
SELECT COUNT(*), COUNT(DISTINCT open_ms) FROM candle
WHERE symbol='BTCUSDT' AND interval='1m';
```

Os dois números são iguais — a chave primária torna duplicata impossível.

> Valida FR-014 e FR-015.

### Cenário C — a vela em formação não entra

```sql
SELECT MAX(open_ms) FROM candle WHERE symbol='BTCUSDT' AND interval='1m';
```

**Esperado**: o maior `open_ms` corresponde a uma vela cujo intervalo **já terminou**
no instante da coleta. Nunca à vela corrente.

> Valida R-005. É a armadilha mais silenciosa desta feature: gravar a vela aberta
> quebraria o determinismo (FR-029) sem produzir nenhum erro visível.

---

## Executar um backtest

```bash
trade backtest --mode backtest \
               --symbol BTCUSDT --interval 1m \
               --from 2025-09-01 --to 2026-09-01 \
               --capital 10000 \
               --strategy sma-cross --strategy-params fast=9,slow=21 \
               --limits limits.toml --fees fees.toml
```

### Cenário D — o modo não tem padrão

```bash
trade backtest --symbol BTCUSDT ...          # sem --mode
trade backtest --mode live --symbol BTCUSDT ...
```

**Esperado**: ambos falham com código 2. O primeiro diz que `--mode` é obrigatório e
não tem padrão; o segundo diz que `live` ainda não está implementado — não que é valor
inválido.

> Valida FR-001, FR-002, FR-003 e SC-007.

### Cenário E — sem rede

```bash
sudo ip link set <interface> down     # ou desconecte o cabo/Wi-Fi
trade backtest --mode backtest ...    # mesmo comando acima
```

**Esperado**: executa normalmente até o fim.

> Valida SC-011. Não é disciplina: as crates do motor não têm cliente HTTP entre suas
> dependências, como o Cenário A demonstra.

### Cenário F — determinismo

```bash
trade backtest ... --runs-db /tmp/a.db
trade backtest ... --runs-db /tmp/b.db
```

Comparar as métricas das duas execuções:

```sql
SELECT profit_factor, max_drawdown, trade_count, net_result FROM metrics;
```

**Esperado**: idênticas, dígito a dígito.

> Valida FR-029 e SC-004.

### Cenário G — a conta fecha

```sql
-- em data/runs.db
SELECT (SELECT net_result FROM metrics WHERE run_id = :run) AS reportado,
       (SELECT SUM(CAST(pnl AS TEXT)) FROM trade WHERE run_id = :run) AS extrato;
```

**Esperado**: divergência zero entre o resultado reportado e a soma do extrato.

> Valida SC-009. É a conferência que pega erro de arredondamento — e o motivo de todo
> valor monetário ser `Decimal` e ser gravado como texto.

---

## Provar a camada de risco

### Cenário H — a estratégia imprudente

Roda-se a estratégia de teste `reckless`, que tenta posição acima do teto, insiste
depois de estourar a perda diária e tenta alterar os próprios limites:

```bash
trade backtest --mode backtest --strategy reckless ... --limits limits-tight.toml
```

**Esperado**: nenhum limite ultrapassado, e cada tentativa recusada e registrada:

```sql
SELECT json_extract(payload_json,'$.breach') AS limite, COUNT(*)
FROM audit_event
WHERE run_id = :run AND kind = 'risk_decision'
  AND json_extract(payload_json,'$.verdict') = 'Rejected'
GROUP BY limite;
```

> Valida SC-003 e FR-018 a FR-021.

### Cenário I — toda ordem tem decisão

```sql
SELECT
  (SELECT COUNT(*) FROM audit_event WHERE run_id=:run AND kind='order')         AS ordens,
  (SELECT COUNT(*) FROM audit_event WHERE run_id=:run AND kind='risk_decision') AS decisoes;
```

**Esperado**: os dois números são iguais, em qualquer execução.

> Valida SC-002. É consequência do tipo de retorno de `RiskGuard::submit`, não de
> cuidado ao programar.

### Cenário J — a autonomia funciona nos dois sentidos

Perda diária estourada no meio do período, com o backtest cruzando a meia-noite UTC:

```sql
SELECT seq, kind, json_extract(payload_json,'$.trigger') AS gatilho,
       json_extract(payload_json,'$.automatic') AS automatico
FROM audit_event
WHERE run_id=:run AND kind IN ('halt','resume')
ORDER BY seq;
```

**Esperado**: o bloqueio por perda diária aparece, e a retomada seguinte tem
`automatic: true` com gatilho `DayBoundary` — **sem nenhum ato humano**. Já uma parada
com `requires_human: true` nunca é seguida de retomada automática.

> Valida SC-013, SC-014 e SC-015. É a verificação de que o robô opera sozinho dentro
> da cerca e só trava onde deve travar.

---

## Reconstituir uma operação

O teste final do Princípio IV. Escolha uma operação qualquer do extrato e refaça o
caminho até o dado que a originou, só com SQL:

```sql
SELECT seq, kind, payload_json
FROM audit_event
WHERE run_id = :run
  AND seq BETWEEN :seq_do_sinal AND :seq_do_fill
ORDER BY seq;
```

**Esperado**: quatro eventos encadeados — `signal` (com `inputs`), `order` (com
`signal_ref`), `risk_decision` (com `limits` e `state` do momento) e `fill` (com taxa e
slippage discriminados). Nada precisa ser reexecutado, nada depende de memória.

> Valida SC-005 e FR-033 a FR-035.

---

## Resumo da cobertura

| Critério | Cenário |
|---|---|
| SC-001 | execução completa do backtest |
| SC-002 | I |
| SC-003 | H |
| SC-004 | F |
| SC-005 | reconstituição |
| SC-006 | A |
| SC-007 | D |
| SC-008 | `cargo test` sem rede |
| SC-009 | G |
| SC-010 | teste de propriedade em `trade-domain` |
| SC-011 | E |
| SC-012 | coleta de 12 meses em um comando |
| SC-013 a SC-015 | J |
