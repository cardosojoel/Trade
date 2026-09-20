# Reconstituir o que o robô fez

Toda decisão é explicável a partir do registro, sem reexecutar nada e sem
recorrer à memória de quem estava presente. Este documento traz as consultas
prontas para colar no DBeaver, apontado para `data/runs.db`.

O Princípio IV da constitution é o que exige isso. Registrar só "houve compra"
responde *o que* o robô fez; a pergunta que se faz quando algo dá errado é
*por quê*.

---

## O encadeamento

```
signal  ──signal_id──▶  order  ──order_id──▶  risk_decision
                          │
                          └────order_id────▶  fill
```

Cada evento tem `seq`, que dá **ordem total** dentro da execução — inclusive
entre dois eventos do mesmo instante simulado. Sem ele, a cadeia teria um elo
ambíguo dentro da mesma vela.

---

## Listar as execuções

```sql
SELECT run_id, mode, symbol, interval, strategy, outcome,
       datetime(started_at/1000, 'unixepoch') AS iniciada
FROM run
ORDER BY run_id DESC;   -- ULID é ordenável por tempo
```

## A cerca sob a qual a execução correu

Comparar duas execuções exige saber sob que condições cada uma foi feita.

```sql
SELECT json_extract(limits_json, '$.max_daily_loss')     AS perda_diaria,
       json_extract(limits_json, '$.max_position_size')  AS tamanho_max,
       json_extract(fees_json,   '$.taker_fee_rate')     AS taxa,
       json_extract(fees_json,   '$.slippage_rate')      AS slippage
FROM run WHERE run_id = :run;
```

## Reconstituir uma operação, do fim ao começo

Escolha um preenchimento e siga a cadeia. Quatro saltos, tudo em SQL.

```sql
WITH f AS (
  SELECT seq, payload_json,
         json_extract(payload_json, '$.order_ref') AS order_ref
  FROM audit_event WHERE run_id = :run AND kind = 'fill' AND seq = :seq_do_fill
),
o AS (
  SELECT a.payload_json,
         json_extract(a.payload_json, '$.signal_ref') AS signal_ref
  FROM audit_event a, f
  WHERE a.run_id = :run AND a.kind = 'order'
    AND json_extract(a.payload_json, '$.order_id') = f.order_ref
),
s AS (
  SELECT a.payload_json
  FROM audit_event a, o
  WHERE a.run_id = :run AND a.kind = 'signal'
    AND json_extract(a.payload_json, '$.signal_id') = o.signal_ref
)
SELECT
  json_extract((SELECT payload_json FROM s), '$.inputs.values') AS dados_de_entrada,
  json_extract((SELECT payload_json FROM s), '$.inputs.params') AS parametros,
  json_extract((SELECT payload_json FROM o), '$.qty')           AS quantidade,
  json_extract((SELECT payload_json FROM f), '$.price')         AS preco,
  json_extract((SELECT payload_json FROM f), '$.fee')           AS taxa,
  json_extract((SELECT payload_json FROM f), '$.slippage')      AS slippage;
```

---

## Conferências

### Toda ordem tem exatamente uma decisão de risco (SC-002)

Se os números divergirem, houve ordem sem veredito — e a garantia central do
Princípio II deixou de valer.

```sql
SELECT
  (SELECT COUNT(*) FROM audit_event WHERE run_id=:run AND kind='order')         AS ordens,
  (SELECT COUNT(*) FROM audit_event WHERE run_id=:run AND kind='risk_decision') AS decisoes;
```

### O resultado reportado bate com a soma do extrato (SC-009)

A divergência tem de ser **zero**, não aproximadamente zero. Todos os valores
são truncados à precisão do satoshi justamente para que esta soma possa ser
refeita aqui, com outra aritmética.

```sql
SELECT (SELECT net_result FROM metrics WHERE run_id = :run) AS reportado,
       (SELECT SUM(CAST(pnl AS REAL)) FROM trade WHERE run_id = :run) AS extrato;
```

> `CAST(... AS REAL)` aqui é só para somar no SQLite. Os valores estão
> gravados como **texto**, e é assim que a exatidão é preservada no disco.

### Nenhuma retomada automática após parada que exige humano (SC-015)

Precisa devolver zero.

```sql
SELECT COUNT(*) FROM audit_event r
WHERE r.run_id = :run AND r.kind = 'resume'
  AND json_extract(r.payload_json, '$.automatic') = 1
  AND EXISTS (
    SELECT 1 FROM audit_event h
    WHERE h.run_id = r.run_id AND h.kind = 'halt' AND h.seq < r.seq
      AND json_extract(h.payload_json, '$.requires_human') = 1
  );
```

### Por que as ordens foram recusadas

```sql
SELECT json_extract(payload_json, '$.breach') AS limite, COUNT(*) AS vezes
FROM audit_event
WHERE run_id = :run AND kind = 'risk_decision'
  AND json_extract(payload_json, '$.verdict') = 'Rejected'
GROUP BY limite ORDER BY vezes DESC;
```

### Paradas e retomadas

```sql
SELECT seq, datetime(at_ms/1000, 'unixepoch') AS quando, kind,
       json_extract(payload_json, '$.classification')  AS classificacao,
       json_extract(payload_json, '$.requires_human')  AS exige_humano,
       json_extract(payload_json, '$.trigger')         AS gatilho,
       json_extract(payload_json, '$.reason')          AS motivo
FROM audit_event
WHERE run_id = :run AND kind IN ('halt', 'resume', 'anomaly')
ORDER BY seq;
```

---

## Sobre o histórico

`data/market.db` é **cache**: pode ser reconstruído da fonte a qualquer momento.
`data/runs.db` é **insubstituível**. Por isso são arquivos separados — apagar um
não pode ser a mesma operação que destruir o outro.

### O que está coletado, e de onde veio

```sql
-- em market.db
SELECT symbol, interval, source, COUNT(*) AS velas,
       datetime(MIN(open_ms)/1000, 'unixepoch') AS primeira,
       datetime(MAX(open_ms)/1000, 'unixepoch') AS ultima
FROM candle JOIN dataset USING (symbol, interval)
GROUP BY symbol, interval, source;
```

### Lacunas conhecidas

Registradas, nunca interpoladas: interpolar inventa preço que não existiu, e a
estratégia decidiria sobre um mercado imaginário.

```sql
SELECT datetime(from_ms/1000, 'unixepoch') AS de,
       datetime(to_ms/1000, 'unixepoch')   AS ate
FROM gap ORDER BY from_ms;
```
