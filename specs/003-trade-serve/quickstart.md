# Como verificar que o `trade serve` funciona

Sem rede, sem credencial e sem nada que toque dinheiro — o servidor lê dois
arquivos SQLite e serve em `127.0.0.1`.

## Antes

```
cargo build --release --workspace
```

Na raiz não basta: `cargo build --release` sozinho compila a lib da raiz, diz
`Finished` e deixa um `target/release/trade` velho no lugar.

É preciso ter o que ler. Se `data/runs.db` estiver vazio:

```
./target/release/trade collect --symbol BTCUSDT --interval 1m \
  --from 2026-01-01 --to 2026-01-08
./target/release/trade backtest --mode backtest --symbol BTCUSDT \
  --from 2026-01-01 --to 2026-01-08 --capital 10000
```

## As recusas, que são o que mais importa

Estas quatro valem mais que as dez rotas de leitura: cada uma é um caminho por
onde algo indevido poderia passar.

```
# 1. Sem modo declarado — recusado, e nenhuma execução começa (FR-004, SC-003)
curl -s -X POST 127.0.0.1:7878/runs -H "X-Trade-Token: $TOKEN" \
  -d '{"symbol":"BTCUSDT","from":"2026-01-01","to":"2026-01-08"}'

# 2. Pedindo `live` — recusado, com mensagem DIFERENTE da anterior (SC-004)
curl -s -X POST 127.0.0.1:7878/runs -H "X-Trade-Token: $TOKEN" \
  -d '{"mode":"live","symbol":"BTCUSDT"}'

# 3. Sem token — recusado antes de qualquer trabalho começar (SC-005)
curl -s -X POST 127.0.0.1:7878/runs -d '{"mode":"backtest"}'

# 4. Com token válido e `Origin` de outro lugar — recusado mesmo assim (SC-006)
curl -s -X POST 127.0.0.1:7878/runs -H "X-Trade-Token: $TOKEN" \
  -H "Origin: https://exemplo.invalido" -d '{"mode":"backtest"}'
```

As quatro devem devolver o **mesmo formato** de corpo, com código próprio — é
o `FR-023`, e existe para que a tela distinga uma recusa da outra sem
interpretar texto.

## Que nenhum dinheiro virou número

O `SC-002` diz **zero** valores monetários como número JSON, varrendo toda
resposta. Dá para conferir de fora:

```
for rota in /runs /runs/$RUN /runs/$RUN/daily /runs/$RUN/episodes; do
  curl -s "127.0.0.1:7878$rota" \
    | python3 -c '
import json,sys
MONEY={"qty","price","fee","pnl","net_result","exposure","daily_pnl",
       "avg_price","entry_price","exit_price","fees","slippage","local",
       "remota","diferenca","initial_capital","max_drawdown"}
def anda(v,c=""):
    if isinstance(v,dict):
        for k,x in v.items():
            if k in MONEY and isinstance(x,(int,float)):
                print("NÚMERO onde devia ser string:",c+"."+k)
            anda(x,c+"."+k)
    elif isinstance(v,list):
        for i,x in enumerate(v): anda(x,f"{c}[{i}]")
anda(json.load(sys.stdin))'
done
```

Saída vazia é o resultado certo.

## Que o servidor não escuta fora da máquina

```
ss -ltnp | grep 7878
```

Deve mostrar `127.0.0.1:7878`, nunca `0.0.0.0:7878` nem `*:7878`.

Numa máquina com outra interface de rede, o `SC-007` se confere de verdade
tentando de lá — e a tentativa tem de falhar por conexão recusada.

## Que a comparação diz a verdade sobre si mesma

Enquanto a tabela `run` não gravar a versão do código, **toda** comparação
entre execuções tem de se declarar não confiável (`FR-007`, `SC-010`):

```
curl -s "127.0.0.1:7878/runs/compare?a=$RUN_A&b=$RUN_B" \
  | python3 -m json.tool | grep -i confia
```

Duas execuções com a mesma cerca e resultados diferentes **não** podem ser
apresentadas como comparáveis. Essa é a resposta certa, não um defeito.
