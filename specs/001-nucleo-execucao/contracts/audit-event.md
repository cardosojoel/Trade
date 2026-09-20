# Contract — Evento de auditoria

**Feature**: 001-nucleo-execucao | **Date**: 2026-09-20

Esquema do que `AuditSink::record` recebe e do que vai para `audit_event.payload_json`.
Satisfaz FR-033 a FR-036 e sustenta SC-005.

---

## Envelope

Todo evento carrega, sem exceção:

| Campo | Tipo | Origem |
|---|---|---|
| `run_id` | string (ULID) | identifica a execução |
| `seq` | inteiro | ordem total dentro da execução, começando em 1 |
| `at` | string ISO-8601 UTC | **instante simulado**, vindo do `Clock`, nunca do relógio da máquina |
| `mode` | string | `backtest` — FR-033 exige o modo em todo registro |
| `kind` | string | discriminante das variantes abaixo |
| `payload` | objeto | conforme `kind` |

`seq` é o que dá ordem total: dois eventos no mesmo milissegundo simulado continuam
ordenados entre si. Sem ele, a cadeia sinal → ordem → decisão → preenchimento perderia
a ordem dentro de uma mesma vela.

---

## Variantes

### `signal`

```json
{
  "signal_id": 41,
  "intent": "Buy",
  "inputs": { "fast_sma": "63412.10", "slow_sma": "63180.55",
              "params": { "fast": 9, "slow": 21 } }
}
```

`inputs` é o campo que torna a auditoria útil (FR-034). Registrar só `intent` responde
"o que o robô fez"; `inputs` responde "por quê" — que é a pergunta que se faz quando
algo dá errado.

### `order`

```json
{ "order_id": 58, "signal_ref": 41, "side": "Buy", "qty": "0.01530000" }
```

### `risk_decision`

```json
{
  "order_ref": 58,
  "verdict": "Rejected",
  "breach": "MaxPositionSize",
  "limits": { "max_position_size": "1000.00", "max_daily_loss": "200.00", "...": "..." },
  "state":  { "daily_pnl": "-84.20", "exposure": "940.00", "orders_in_window": 3 }
}
```

`daily_pnl` é o resultado **do dia**: o realizado desde a virada mais a variação do
não realizado no mesmo intervalo (FR-019b). Não é o resultado acumulado da execução,
nem só o que foi fechado — é exatamente o número contra o qual o freio de perda
diária foi comparado naquele instante, e é isso que torna a recusa reconstituível.

`limits` e `state` são fotografias do momento da avaliação. Sem elas, uma recusa cujo
limite tenha mudado depois vira registro indecifrável — sabe-se que houve recusa, não
se sabe contra o quê.

**Toda ordem gera exatamente um `risk_decision`**, aceita ou recusada. É o que faz
SC-002 ser verificável por contagem: `count(order) == count(risk_decision)` em qualquer
execução.

### `fill`

```json
{ "order_ref": 58, "price": "63420.00", "qty": "0.01530000",
  "fee": "0.97", "slippage": "0.32" }
```

Taxa e slippage discriminados, nunca embutidos no preço (FR-027).

### `halt` / `resume`

```json
{ "reason": "PositionDivergence", "classification": "Integrity",
  "attempts": 5, "requires_human": true }
```

```json
{ "trigger": "DayBoundary", "automatic": true }
```

FR-024c exige causa e classificação em toda parada e toda retomada. O par
`classification` + `requires_human` é o que permite conferir SC-015 por consulta: nenhum
`resume` com `automatic: true` pode seguir um `halt` com `requires_human: true`.

### `anomaly`

Falha transitória que **não** interrompeu a operação — registrada porque uma sequência
de retentativas bem-sucedidas ainda é sinal de que algo vai mal:

```json
{ "cause": "Unavailable", "classification": "Transient",
  "attempt": 2, "max_retries": 5, "recovered": true }
```

### `state_transition`

```json
{ "from": "Flat", "to": "Long", "qty": "0.01530000", "avg_price": "63420.00" }
```

---

## Proibições

- **Nenhuma credencial, chave ou token em nenhum campo** (FR-036). Nesta feature a
  garantia é trivial — o sistema não possui credencial alguma — mas o esquema já a
  declara, porque paper trading e live virão e o esquema não muda.
- **Nenhum campo financeiro como número JSON.** Todos os valores monetários são
  **string**. Número JSON é IEEE-754 double na maioria dos leitores, e serializar
  `Decimal` como número desfaria a exatidão no caminho para o disco — exatamente o erro
  que R-002 existe para evitar.
- **Nenhum instante de relógio de parede.** `at` é o tempo simulado. Se um evento
  precisar do tempo real da máquina — por exemplo, quanto durou a execução — vai em
  `run.started_at` / `run.ended_at`, fora da trilha de auditoria.
