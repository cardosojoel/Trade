# Desempenho e verificação

Medições feitas em 2026-09-20 sobre **dados reais da Bybit**, não sintéticos.
Máquina: Linux x86-64, build `--release`, toolchain 1.98.1.

---

## Coleta

| | |
|---|---|
| Período | 2025-09-20 a 2026-09-20, BTCUSDT, velas de 1 minuto |
| Volume | **525.600 velas** em 526 páginas |
| Tempo | **4m48s** |
| Memória máxima | **12 MB** |
| Lacunas encontradas | nenhuma |

526 requisições em 4m48 dá cerca de uma a cada 550ms — muito abaixo do limite
documentado de 600 requisições por 5 segundos. A restrição real nunca foi
vazão; o coletor não precisa de paralelismo, precisa de não ser rude.

---

## Backtest (T102)

Meta: **abaixo de 60 segundos** para doze meses de velas de um minuto.

| Período | Velas | Tempo | Memória máxima |
|---|---|---|---|
| 5 dias | 7.200 | 0,01s | 8 MB |
| **12 meses** | **525.600** | **1,03s** | **15 MB** |

Folga de quase sessenta vezes sobre a meta.

### Memória (T103)

73 vezes mais velas custaram **1,8 vez** mais memória. A memória **não** cresce
com o tamanho do período: as velas são percorridas por cursor paginado sobre o
SQLite, mil por vez.

O que cresce é outra coisa, e vale registrar com precisão em vez de alegar
consumo constante: o extrato de operações e o lote de auditoria vivem em
memória durante a execução. Os 7 MB a mais correspondem às **14.308 operações**
do período longo, não às 525.600 velas. Memória constante em relação ao
**período**, linear em relação ao número de **operações**.

---

## Cenários do quickstart (T101)

Executados sobre o histórico real de doze meses.

| Cenário | O que verifica | Resultado |
|---|---|---|
| **A** | Estratégia, risco e backtest não alcançam corretora nem rede | ✓ zero ocorrências em `cargo tree` |
| **B** | Retomada não duplica | ✓ coberto por teste de integração |
| **C** | Vela em formação descartada | ✓ última vela gravada já fechada |
| **D** | `--mode` obrigatório, `paper`/`live` recusados | ✓ coberto por teste de CLI |
| **E** | Backtest sem rede | ✓ por construção — ver nota |
| **F** | Determinismo | ✓ duas execuções idênticas dígito a dígito |
| **G** | Resultado bate com a soma do extrato | ✓ **divergência zero** sobre 14.308 operações |
| **H** | Estratégia imprudente contida | ✓ coberto por teste de integração |
| **I** | Toda ordem tem decisão de risco | ✓ 28.633 ordens, 28.633 decisões |
| **J** | Autonomia e retomada | ✓ coberto por teste de integração |

**Nota sobre E:** a execução com a rede desligada não pôde ser feita neste
ambiente (`unshare` indisponível). A garantia, porém, não depende de desligar a
rede: as crates do motor **não têm cliente HTTP entre suas dependências**, o que
o cenário A demonstra e a trava em `tests/architecture.rs` impede de mudar. É
uma propriedade do grafo de dependências, não do ambiente de execução.

---

## O que a medição revelou sobre a estratégia de referência

Sobre doze meses reais, a `sma-cross` em velas de um minuto perdeu **98,7% do
capital**:

```
  Resultado líquido       -9871.30    -98.7%
  Taxas                    6443.14
  Slippage                 3221.58
  Operações                  14308
  Profit factor               0.07
```

Taxas e slippage somaram **9.664** sobre um capital de 10.000. O custo de
transação sozinho consumiu praticamente tudo.

Isso não é defeito do motor — é o motor funcionando. Uma estratégia que opera
14 mil vezes por ano em velas de um minuto paga custo de transação em cada
entrada e cada saída, e nenhuma vantagem estatística modesta sobrevive a isso.
Um backtest que não modelasse taxa e slippage mostraria um resultado bem
diferente e igualmente falso.

A `sma-cross` é **estratégia de referência, não recomendação de investimento**
(FR-038). Ela existe para exercitar o ciclo. Que ela perca dinheiro é o
resultado esperado, e a clareza com que perde é a evidência de que a
modelagem de custos está fazendo seu trabalho.
