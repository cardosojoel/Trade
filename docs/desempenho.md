# Desempenho e verificação

Medições feitas em 2026-09-20 sobre **dados reais da Bybit**, não sintéticos.
Máquina: Linux x86-64, build `--release`, toolchain 1.98.1.

Refeitas **após** a mudança da perda diária (FR-019b), sobre histórico
recoletado. O resultado da estratégia de referência saiu **idêntico** ao da
regra antiga — ver "O freio de perda diária não dispara neste cenário".

---

## Coleta

| | |
|---|---|
| Período | 2025-09-20 a 2026-09-20, BTCUSDT, velas de 1 minuto |
| Volume | **525.600 velas** em 526 páginas |
| Tempo | **4m46s** |
| Memória máxima | **11,4 MB** |
| Lacunas encontradas | nenhuma |

526 requisições em 4m46 dá cerca de uma a cada 545ms — muito abaixo do limite
documentado de 600 requisições por 5 segundos. A restrição real nunca foi
vazão; o coletor não precisa de paralelismo, precisa de não ser rude.

---

## Backtest (T102)

Meta: **abaixo de 60 segundos** para doze meses de velas de um minuto.

| Período | Velas | Tempo | Memória máxima |
|---|---|---|---|
| 5 dias | 7.200 | 0,06s | 7,8 MB |
| **12 meses** | **525.600** | **2,46s** | **14,6 MB** |

Folga de vinte e quatro vezes sobre a meta.

A medição anterior deste mesmo cenário deu 1,03s. A diferença **não** é o custo
da marcação a mercado introduzida por FR-019b: medindo os dois binários na
mesma máquina e sobre os mesmos dados, o anterior (sem marcação) deu 2,44s,
2,45s e 2,46s, e o atual (com marcação) 2,46s, 2,48s e 2,54s. A marcação por
vela some dentro do ruído; o que mudou foi a máquina, não o código.

### Memória (T103)

73 vezes mais velas custaram **1,9 vez** mais memória. A memória **não** cresce
com o tamanho do período: as velas são percorridas por cursor paginado sobre o
SQLite, mil por vez.

O que cresce é outra coisa, e vale registrar com precisão em vez de alegar
consumo constante: o extrato de operações e o lote de auditoria vivem em
memória durante a execução. Os 6,8 MB a mais correspondem às **14.308 operações**
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

## O freio de perda diária não dispara neste cenário

Depois de FR-019b o contador do dia soma o prejuízo **aberto**, e não só o
fechado. A expectativa registrada antes da medição era que isso barrasse
compras e reduzisse o número de operações. **Não reduziu:** o resultado dos
doze meses saiu idêntico ao da regra antiga — mesmas 14.308 operações, mesmo
−9.871,30, mesmos 143.135 eventos de auditoria.

| | |
|---|---|
| Decisões com o freio armado | **0** em 28.633 |
| Pior resultado diário observado | **−143,80** |
| Limite | −200,00 |
| Recusas por perda diária | **nenhuma** |

O motivo é a forma da perda. A `sma-cross` não perde em quedas diárias
violentas: ela sangra custo de transação em 14 mil operações ao longo do ano,
cerca de −27 por dia em média. Um limite diário de 200 sobre capital de 10.000
nunca é alcançado por esse tipo de perda — nem pelo lado aberto, porque o
tamanho máximo de posição (1.000) limita o prejuízo não realizado a uma queda
de 20% do preço dentro de um mesmo dia.

Duas leituras, e as duas importam:

1. **A mudança está correta e não teve efeito aqui.** O freio é exercitado por
   teste unitário e por teste de motor, que o veem disparar. Este cenário
   apenas não o alcança.
2. **Um limite que nunca dispara não protege nada.** A perda diária de 2% não
   é a cerca que contém esta estratégia — quem a contém, e mal, é o custo de
   transação. Isso é insumo direto para a calibração de limiares que a
   constitution exige antes da Porta 3.

O `daily_pnl` registrado na auditoria mudou de significado e é possível ver
isso no dado: sob a regra antiga o pior valor observado foi −144,38; sob a
nova, −143,80. Números diferentes porque agora a parcela aberta entra na conta.

---

## Integridade da auditoria

Conferências do `docs/auditoria.md` sobre a execução de doze meses
(`01M2ZG1ENV7PMGQS5AK4PQ1M3Z`):

| Conferência | Resultado |
|---|---|
| Toda ordem tem exatamente uma decisão (SC-002) | 28.633 ordens · 28.633 decisões |
| `seq` sem buraco | 143.135 eventos, de 1 a 143.135 |
| Resultado reportado × soma do extrato (SC-009) | −9871.30084728 nos dois — **divergência zero** |
| Retomada automática após parada que exige humano (SC-015) | 0 |
| Determinismo | duas execuções idênticas dígito a dígito |

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
