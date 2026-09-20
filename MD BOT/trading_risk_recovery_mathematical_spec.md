# Especificação Matemática Final — Controle de Risco e Recovery

**Versão:** 1.0  
**Status:** Final  
**Escopo:** Controle matemático de risco e recuperação de sessão  
**Aplicação:** Bot de trading

---

## 1. Objetivo

Implementar exclusivamente a camada matemática de controle de risco da sessão de trading.

O módulo deve determinar:

- capital inicial da sessão;
- perda máxima do capital depositado;
- lucro protegido;
- lucro autorizado para Recovery;
- Recovery Budget máximo;
- risco máximo por operação;
- risco máximo durante Recovery;
- valor ainda disponível para Recovery;
- progresso da recuperação;
- limite de ciclos e tentativas;
- condição de encerramento da sessão.

Não fazem parte deste módulo:

- estratégia;
- sinais;
- indicadores;
- seleção de ativos;
- execução de ordens;
- integração com corretora.

---

# 2. Princípio fundamental

O sistema deve manter matematicamente separados:

1. **Capital original**
2. **Lucro protegido**
3. **Recovery Budget**
4. **Recovery Progress**

Esses quatro valores nunca devem ser tratados como um único saldo de risco.

---

# 3. Variáveis da sessão

Definir:

```text
D  = depósito inicial
L  = limite de perda do depósito (%)
P  = percentual do lucro autorizado para Recovery (%)
R  = percentual máximo do depósito que pode ser utilizado em Recovery (%)
T  = risco máximo por operação normal (%)
RT = risco máximo por operação durante Recovery (%)
C  = número máximo de ciclos de Recovery
A  = número máximo de tentativas por ciclo
```

Exemplo:

```text
D  = R$ 50,00
L  = 20%
P  = 30%
R  = 10%
T  = 2%
RT = 25%
C  = 3
A  = 4
```

---

# 4. Limite de perda do depósito

O limite máximo de perda do capital originalmente depositado é:

```text
LOSS_LIMIT = D × L
```

Exemplo:

```text
D = R$ 50
L = 20%

LOSS_LIMIT = 50 × 0,20
LOSS_LIMIT = R$ 10
```

Portanto:

```text
MAX_DEPOSIT_LOSS = R$ 10
```

---

# 5. Capital Floor

O Capital Floor representa o menor valor permitido para o capital original antes de atingir o limite de perda:

```text
CAPITAL_FLOOR = D - LOSS_LIMIT
```

Exemplo:

```text
D = R$ 50
LOSS_LIMIT = R$ 10

CAPITAL_FLOOR = R$ 40
```

---

# 6. Peak Equity

Durante toda a sessão:

```text
PEAK_EQUITY =
MAX(todas as equities registradas)
```

O Peak Equity nunca deve diminuir.

---

# 7. Lucro realizado

Somente lucro **realizado** poderá gerar Recovery Budget.

Definir:

```text
REALIZED_PROFIT =
MAX(0, REALIZED_EQUITY - D)
```

O lucro não realizado não participa desse cálculo.

### Regra obrigatória

Lucro não realizado:

```text
não gera Recovery Budget
não aumenta Recovery Budget
não pode ser utilizado para Recovery
```

Uma posição aberta e temporariamente lucrativa não libera capital de Recovery.

---

# 8. Lucro protegido

O lucro realizado será dividido entre:

```text
PROTECTED_PROFIT
+
RECOVERY_ELIGIBLE_PROFIT
```

O percentual autorizado pelo usuário determina quanto do lucro pode ser utilizado para Recovery:

```text
RECOVERY_ELIGIBLE_PROFIT =
REALIZED_PROFIT × P
```

O restante permanece protegido:

```text
PROTECTED_PROFIT =
REALIZED_PROFIT - RECOVERY_ELIGIBLE_PROFIT
```

Exemplo:

```text
REALIZED_PROFIT = R$ 20
P = 30%

RECOVERY_ELIGIBLE_PROFIT =
20 × 0,30
= R$ 6

PROTECTED_PROFIT =
20 - 6
= R$ 14
```

---

# 9. Limite absoluto de Recovery

Além do percentual do lucro, existe um limite máximo baseado no depósito:

```text
RECOVERY_MAX =
D × R
```

Exemplo:

```text
D = R$ 50
R = 10%

RECOVERY_MAX =
50 × 0,10
= R$ 5
```

---

# 10. Recovery Budget

O orçamento máximo de Recovery é definido pelo menor dos dois limites:

```text
INITIAL_RECOVERY_BUDGET =
MIN(
    RECOVERY_ELIGIBLE_PROFIT,
    RECOVERY_MAX
)
```

Exemplo:

```text
REALIZED_PROFIT = R$ 20
P = 30%

RECOVERY_ELIGIBLE_PROFIT = R$ 6

RECOVERY_MAX = R$ 5

INITIAL_RECOVERY_BUDGET =
MIN(6,5)

= R$ 5
```

Portanto:

```text
Recovery Budget = R$ 5
```

---

# 11. Significado do Recovery Budget

O Recovery Budget representa exclusivamente:

> O máximo de lucro previamente realizado que o sistema está autorizado a colocar novamente em risco durante a recuperação.

Ele não representa:

- saldo adicional;
- capital operacional;
- dinheiro adicional depositado;
- lucro livre;
- capital normal de trading.

---

# 12. Recovery Budget não é reposto

O Recovery Budget é um orçamento **consumível**.

Se:

```text
INITIAL_RECOVERY_BUDGET = R$ 5
```

e ocorrer:

```text
LOSS = R$ 1
```

então:

```text
RECOVERY_BUDGET_REMAINING =
5 - 1
= R$ 4
```

Se posteriormente houver:

```text
PROFIT = R$ 2
```

o orçamento não retorna para R$ 5.

Ele permanece:

```text
RECOVERY_BUDGET_REMAINING = R$ 4
```

O lucro aumenta o Recovery Progress, mas não regenera o orçamento.

---

# 13. Condição para entrar em Recovery

O Recovery somente poderá ser ativado quando:

```text
EQUITY <= CAPITAL_FLOOR
```

e:

```text
INITIAL_RECOVERY_BUDGET > 0
```

e:

```text
REALIZED_PROFIT > 0
```

Caso não exista Recovery Budget disponível:

```text
SESSION_STOPPED
```

---

# 14. Recovery Target

Quando o limite de perda do depósito for atingido:

```text
LOSS_TO_RECOVER =
LOSS_LIMIT
```

O alvo inicial será:

```text
RECOVERY_TARGET =
LOSS_TO_RECOVER
```

Exemplo:

```text
LOSS_LIMIT = R$ 10

RECOVERY_TARGET = R$ 10
```

---

# 15. Recovery Progress

O sistema deve separar:

```text
RECOVERY_BUDGET
```

de:

```text
RECOVERY_PROGRESS
```

Recovery Budget representa:

```text
quanto ainda pode ser perdido
```

Recovery Progress representa:

```text
quanto da perda original já foi recuperado
```

---

# 16. Cálculo do Recovery Progress

Definir:

```text
RECOVERED_AMOUNT =
soma líquida dos resultados das operações de Recovery
```

Exemplo:

```text
Trade 1 = +R$ 2
Trade 2 = -R$ 1
Trade 3 = +R$ 3

RECOVERED_AMOUNT =
2 - 1 + 3

= R$ 4
```

---

# 17. Recovery Progress %

```text
RECOVERY_PROGRESS =
RECOVERED_AMOUNT / RECOVERY_TARGET
```

Exemplo:

```text
RECOVERED_AMOUNT = R$ 4
RECOVERY_TARGET = R$ 10

RECOVERY_PROGRESS =
4 / 10
= 40%
```

Limitar matematicamente:

```text
0 <= RECOVERY_PROGRESS <= 1
```

---

# 18. Recovery Success

A recuperação será considerada concluída quando:

```text
RECOVERED_AMOUNT >= RECOVERY_TARGET
```

Resultado:

```text
RECOVERY_STATUS = SUCCESS
```

Nenhum novo Recovery Budget deve ser criado automaticamente.

---

# 19. Recovery Failure

A recuperação falhará quando qualquer uma das condições abaixo ocorrer:

```text
RECOVERY_BUDGET_REMAINING <= 0
```

ou:

```text
RECOVERY_ATTEMPT >= MAX_RECOVERY_ATTEMPTS
```

ou:

```text
RECOVERY_CYCLE >= MAX_RECOVERY_CYCLES
```

Resultado:

```text
SESSION_STATUS = STOPPED
```

Nenhum novo Recovery automático será iniciado.

---

# 20. Risco máximo por operação normal

O risco máximo por operação normal será:

```text
NORMAL_TRADE_RISK =
D × T
```

Exemplo:

```text
D = R$ 50
T = 2%

NORMAL_TRADE_RISK =
50 × 0,02

= R$ 1
```

---

# 21. Risco máximo por operação durante Recovery

Durante Recovery, o risco deve ser calculado sobre o orçamento restante:

```text
RECOVERY_TRADE_RISK =
RECOVERY_BUDGET_REMAINING × RT
```

Exemplo:

```text
RECOVERY_BUDGET = R$ 5
RT = 25%

RECOVERY_TRADE_RISK =
5 × 0,25

= R$ 1,25
```

Após uma perda de R$ 1:

```text
RECOVERY_BUDGET_REMAINING = R$ 4

RECOVERY_TRADE_RISK =
4 × 25%

= R$ 1
```

O risco diminui automaticamente.

---

# 22. Limite global de risco

O risco efetivamente permitido para uma operação será:

```text
ALLOWED_TRADE_RISK =
MIN(
    CONFIGURED_TRADE_RISK,
    REMAINING_SESSION_RISK,
    REMAINING_RECOVERY_RISK,
    REMAINING_EXPOSURE_RISK
)
```

A estratégia nunca poderá determinar sozinha o risco final.

---

# 23. Regra anti-Martingale

Uma perda nunca poderá aumentar automaticamente:

```text
NORMAL_TRADE_RISK
RECOVERY_TRADE_RISK
RECOVERY_BUDGET
LOSS_LIMIT
```

É proibido aumentar o risco em função de:

- quantidade de perdas consecutivas;
- valor da última perda;
- sequência de perdas;
- duração do Recovery;
- distância até o Recovery Target.

Não implementar Martingale.

---

# 24. Position Size

Dado:

```text
R = risco máximo permitido
E = preço de entrada
S = preço do Stop Loss
M = multiplicador do ativo
```

Calcular:

```text
UNIT_RISK =
ABS(E - S) × M
```

Então:

```text
POSITION_SIZE =
FLOOR(
    R / UNIT_RISK
)
```

Sempre arredondar para baixo.

Nunca arredondar para cima.

---

# 25. Risco total da posição

A perda máxima estimada da posição será:

```text
POSITION_RISK =
POSITION_SIZE × UNIT_RISK
```

A posição somente poderá ser autorizada quando:

```text
POSITION_RISK <= ALLOWED_TRADE_RISK
```

---

# 26. Fees e Slippage

O cálculo deve considerar custos estimados:

```text
TOTAL_RISK =
POSITION_RISK
+
ESTIMATED_FEES
+
ESTIMATED_SLIPPAGE
```

A operação somente poderá ser autorizada quando:

```text
TOTAL_RISK <= ALLOWED_TRADE_RISK
```

---

# 27. Exemplo completo

Configuração:

```text
Depósito = R$ 50
Loss Limit = 20%
Lucro autorizado para Recovery = 30%
Recovery Max = 10%
Risco normal = 2%
Risco Recovery = 25%
```

## 27.1 Início

```text
Capital inicial = R$ 50
Loss Limit = R$ 10
Capital Floor = R$ 40
```

## 27.2 Geração de lucro

A sessão obtém:

```text
REALIZED_PROFIT = R$ 20
```

Então:

```text
RECOVERY_ELIGIBLE_PROFIT =
20 × 30%
= R$ 6
```

Limite absoluto:

```text
RECOVERY_MAX =
50 × 10%
= R$ 5
```

Recovery Budget:

```text
MIN(6,5)
= R$ 5
```

Lucro protegido:

```text
PROTECTED_PROFIT =
20 - 6
= R$ 14
```

## 27.3 Perda posterior

A Equity cai para:

```text
R$ 40
```

O limite de perda do depósito foi atingido:

```text
LOSS = R$ 10
```

Recovery pode ser ativado:

```text
Recovery Target = R$ 10
Recovery Budget = R$ 5
```

## 27.4 Recovery

Trade 1:

```text
Risk = R$ 1,25
Result = +R$ 2
```

Então:

```text
Recovered = R$ 2
Progress = 20%
Budget = R$ 5
```

Trade 2:

```text
Result = -R$ 1
```

Então:

```text
Recovered = R$ 1
Budget = R$ 4
```

Novo risco máximo:

```text
4 × 25%
= R$ 1
```

Trade 3:

```text
Result = +R$ 3
```

Então:

```text
Recovered = R$ 4
Progress = 40%
Budget = R$ 4
```

Trade 4:

```text
Result = +R$ 6
```

Então:

```text
Recovered = R$ 10
Progress = 100%
```

Resultado:

```text
RECOVERY_SUCCESS
```

O Recovery Budget restante permanece:

```text
R$ 4
```

Ele não é convertido automaticamente em novo orçamento de risco.

---

# 28. Novo ciclo de Recovery

Um novo ciclo somente poderá ser iniciado se:

```text
RECOVERY_BUDGET_REMAINING > 0
```

e:

```text
RECOVERY_TARGET_NOT_REACHED
```

e:

```text
CURRENT_CYCLE < MAX_RECOVERY_CYCLES
```

O novo ciclo utiliza somente:

```text
RECOVERY_BUDGET_REMAINING
```

Nunca recriar:

```text
INITIAL_RECOVERY_BUDGET
```

---

# 29. Regra anti-loop

É proibido:

```text
Recovery
→ perda
→ novo Recovery Budget
→ Recovery
→ perda
→ novo Recovery Budget
→ ...
```

O orçamento é criado uma única vez com base no lucro realizado elegível.

Depois disso:

```text
INITIAL_RECOVERY_BUDGET
        ↓
CONSUMPTION
        ↓
REMAINING_BUDGET
```

Não existe regeneração automática.

---

# 30. Visualização antes da confirmação

Antes do início da sessão, o sistema deverá apresentar ao usuário:

```text
════════════════════════════════════
       CONFIRMAÇÃO DE RISCO
════════════════════════════════════

DEPÓSITO
R$ 50,00

LIMITE DE PERDA
20%

PERDA MÁXIMA DO DEPÓSITO
R$ 10,00

CAPITAL FLOOR
R$ 40,00

────────────────────────────────────

LUCRO PARA RECOVERY
30%

LIMITE MÁXIMO RECOVERY
10% DO DEPÓSITO

RECOVERY BUDGET MÁXIMO
R$ 5,00

LUCRO PROTEGIDO
R$ 14,00

────────────────────────────────────

RISCO NORMAL / TRADE
R$ 1,00

RISCO RECOVERY / TRADE
R$ 1,25

────────────────────────────────────

MÁXIMA PERDA DO CAPITAL
R$ 10,00

MÁXIMO DE LUCRO COLOCADO EM RISCO
R$ 5,00

────────────────────────────────────

CICLOS MÁXIMOS
3

TENTATIVAS POR CICLO
4

════════════════════════════════════
```

O usuário deve confirmar explicitamente os parâmetros antes da sessão se tornar ativa.

---

# 31. Máxima perda da sessão

O sistema deve apresentar separadamente:

```text
MAX_DEPOSIT_LOSS
```

e:

```text
MAX_PROFIT_AT_RISK
```

Exemplo:

```text
MAX_DEPOSIT_LOSS = R$ 10
MAX_PROFIT_AT_RISK = R$ 5
```

Não representar esses valores como uma única perda de R$ 15.

São fontes de capital diferentes.

---

# 32. Imutabilidade da configuração

Após a confirmação da sessão:

```text
RiskConfiguration
```

não poderá ser alterada.

Para alterar os parâmetros:

```text
STOP SESSION
+
CREATE NEW SESSION
```

---

# 33. Precisão numérica

Todos os cálculos financeiros devem ser determinísticos.

É proibido utilizar:

```text
f32
f64
float
double
```

para representar dinheiro.

Utilizar representação inteira ou decimal exata.

Percentuais devem utilizar representação determinística, preferencialmente basis points.

Exemplos:

```text
2%  = 200 bps
10% = 1000 bps
20% = 2000 bps
25% = 2500 bps
30% = 3000 bps
```

---

# 34. Invariantes matemáticos

As seguintes condições nunca podem ser violadas:

```text
D > 0

LOSS_LIMIT >= 0

LOSS_LIMIT <= D

RECOVERY_MAX >= 0

RECOVERY_BUDGET >= 0

RECOVERY_BUDGET <= RECOVERY_MAX

RECOVERY_BUDGET <= RECOVERY_ELIGIBLE_PROFIT

RECOVERED_AMOUNT >= 0

RECOVERY_PROGRESS >= 0

RECOVERY_PROGRESS <= 1

TRADE_RISK >= 0

TRADE_RISK <= ALLOWED_TRADE_RISK

POSITION_RISK <= ALLOWED_TRADE_RISK
```

---

# 35. Regra de falha segura

Se qualquer cálculo necessário para autorizar uma operação não puder ser determinado com segurança:

```text
DENY
```

Exemplos:

```text
saldo desconhecido
equity desconhecida
Recovery Budget inconsistente
risco desconhecido
preço inválido
Stop Loss inválido
configuração inválida
dados financeiros inconsistentes
```

Nunca utilizar fallback que resulte em autorização.

---

# 36. Modelo conceitual final

```text
                    DEPÓSITO INICIAL
                           │
                           ▼
                 ┌───────────────────┐
                 │  LIMITE DE PERDA  │
                 └─────────┬─────────┘
                           │
                           ▼
                     CAPITAL FLOOR


                    LUCRO REALIZADO
                           │
                           ▼
                 ┌───────────────────┐
                 │ % AUTORIZADO (P)  │
                 └─────────┬─────────┘
                           │
                           ▼
             RECOVERY ELIGIBLE PROFIT
                           │
                           │ MIN
                           ▼
                 ┌───────────────────┐
                 │ RECOVERY MAX      │
                 │ D × R             │
                 └─────────┬─────────┘
                           │
                           ▼
                 INITIAL RECOVERY
                      BUDGET
                           │
                  ┌────────┴────────┐
                  │                 │
                PERDA             GANHO
                  │                 │
                  ▼                 ▼
             BUDGET ↓          PROGRESS ↑
                  │                 │
                  ▼                 │
              RISCO ↓               │
                  │                 │
                  └────────┬────────┘
                           ▼
                     RECOVERY
                     DECISION
                           │
                 ┌─────────┴─────────┐
                 │                   │
               SUCCESS              FAILURE
                 │                   │
                 ▼                   ▼
             REASSESS             STOP
```

---

# 37. Regra definitiva

O sistema deve obedecer às seguintes regras:

### Capital

O depósito define o limite máximo de perda do capital original.

### Lucro

Somente lucro realizado pode ser utilizado como fonte de Recovery.

### Lucro não realizado

Nunca libera Recovery Budget.

### Lucro protegido

Não pode ser utilizado para Recovery.

### Recovery Budget

É limitado por:

```text
MIN(
    % do lucro realizado,
    % máximo do depósito
)
```

### Consumo

Perdas reduzem o Recovery Budget.

### Ganhos

Ganhos aumentam o Recovery Progress, mas não regeneram o Recovery Budget.

### Risco

O risco máximo diminui conforme o Recovery Budget é consumido.

### Martingale

É proibido.

### Falha

Se o Recovery Budget acabar ou os limites de ciclos/tentativas forem atingidos, a sessão é encerrada.

### Nova sessão

Um novo Recovery Budget somente poderá ser criado em uma nova sessão com novos parâmetros e/ou novo depósito.

---

# 38. Fórmulas principais

```text
LOSS_LIMIT =
D × L
```

```text
CAPITAL_FLOOR =
D - LOSS_LIMIT
```

```text
REALIZED_PROFIT =
MAX(0, REALIZED_EQUITY - D)
```

```text
RECOVERY_ELIGIBLE_PROFIT =
REALIZED_PROFIT × P
```

```text
PROTECTED_PROFIT =
REALIZED_PROFIT - RECOVERY_ELIGIBLE_PROFIT
```

```text
RECOVERY_MAX =
D × R
```

```text
INITIAL_RECOVERY_BUDGET =
MIN(
    RECOVERY_ELIGIBLE_PROFIT,
    RECOVERY_MAX
)
```

```text
RECOVERY_TRADE_RISK =
RECOVERY_BUDGET_REMAINING × RT
```

```text
RECOVERY_PROGRESS =
RECOVERED_AMOUNT / RECOVERY_TARGET
```

```text
UNIT_RISK =
ABS(E - S) × M
```

```text
POSITION_SIZE =
FLOOR(
    ALLOWED_TRADE_RISK / UNIT_RISK
)
```

```text
TOTAL_RISK =
POSITION_RISK
+
ESTIMATED_FEES
+
ESTIMATED_SLIPPAGE
```

---

# FIM DA ESPECIFICAÇÃO
