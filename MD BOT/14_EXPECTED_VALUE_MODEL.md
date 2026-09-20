# 14 — Expected Value Model — Canonical v3

## 1. Objetivo
Transformar a distribuição de resultados prevista pelo modelo em vantagem econômica líquida, incorporando custos e cenários conservadores.

## 2. Modelo canônico
O modelo oficial é multiestado/ternário. Para um horizonte H:

```text
P_up + P_neutral + P_down = 1
```

Cada estado possui retorno monetário esperado líquido antes dos custos específicos:

```text
R_up
R_neutral
R_down
```

O Expected Value bruto é:

```text
EV_gross = P_up*R_up + P_neutral*R_neutral + P_down*R_down
```

O Expected Value líquido é:

```text
EV_net = EV_gross - C_total
```

onde `C_total` inclui, conforme aplicável:
- trading fees;
- spread;
- slippage esperado;
- funding;
- impacto de mercado;
- custo esperado de execução/latência;
- demais custos determinísticos ou estatísticos da operação.

## 3. Compatibilidade binária
O modelo binário legado:

```text
EV = P(win)*E[profit|win] - (1-P(win))*E[loss|loss] - Costs
```

é apenas uma projeção do modelo canônico quando não existe estado `neutral` relevante. Não deve coexistir como fórmula independente.

## 4. Critérios de decisão
Uma operação somente pode ser elegível se:

```text
P values valid
EV_net > EV_min
EV_conservative > EV_min_conservative
Risk Engine = APPROVED
```

Probability alta não é suficiente para operar.

## 5. Cenário conservador
O sistema deve calcular pelo menos:

```text
Nominal EV
Conservative EV
```

O cenário conservador deve degradar, conforme política versionada:
- probability;
- retorno favorável;
- slippage;
- fees;
- fill probability.

Se o EV conservador não atingir o threshold, retornar `NO_TRADE`.

## 6. Unidades
Todos os termos devem ser calculados na mesma unidade econômica. O sistema deve também produzir uma versão normalizada por capital da sessão para comparação.

## 7. Persistência
A decisão deve registrar:
- `p_up`, `p_neutral`, `p_down`;
- retornos por estado;
- costs por categoria;
- `ev_gross`;
- `ev_net`;
- `ev_conservative`;
- thresholds;
- `model_version`;
- `feature_version`;
- `decision_id`.

## 8. Invariantes
- `0 <= P_state <= 1`;
- soma das probabilidades deve ser 1 dentro da tolerância numérica definida;
- EV deve incluir custos;
- EV não autoriza ordem por si só;
- Risk Engine permanece autoridade final.
