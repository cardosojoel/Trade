# 03 — Session State Machine / Risk & Recovery — v3

## Objetivo
Formalizar sessão, orçamento de risco e episódios de recuperação sem permitir que Recovery altere silenciosamente a política de risco.

## Estados
`CREATED`, `CONFIRMING`, `ACTIVE`, `PROFIT_PROTECTED`, `LOSS_LIMIT_REACHED`, `RECOVERY`, `RECOVERY_SUCCESS`, `RECOVERY_FAILED`, `STOPPED`, `ERROR`, `RECONCILIATION_REQUIRED`, `EMERGENCY_STOP`.

## Parâmetros congelados na sessão
```text
D0  = depósito inicial
L   = perda máxima percentual do depósito
P   = percentual do lucro realizado elegível para Recovery
R   = teto estrutural de Recovery relativo ao depósito
T   = risco máximo por operação normal
RT  = risco máximo por operação em Recovery
Cmax = máximo de episódios de Recovery
Amax = máximo de tentativas por episódio
```

## Limite primário
```text
MaxLossDeposit = D0 * L
CapitalFloor = D0 - MaxLossDeposit
```

`MaxLossDeposit` é o limite de perda do capital inicial e não deve ser confundido com a exposição adicional de Recovery.

## Lucro elegível
```text
RealizedProfit = max(0, RealizedEquity - D0)
RecoveryEligibleProfit = RealizedProfit * P
RecoveryMaxSession = D0 * R
```

Lucro não realizado nunca cria Recovery Budget.

## Recovery Budget da sessão
O orçamento total de Recovery é limitado por:

```text
RecoveryBudgetSession = min(RecoveryEligibleProfit_at_authorization, RecoveryMaxSession)
```

O orçamento é consumível. Cada episódio reduz `RecoveryBudgetRemaining` pelo valor efetivamente comprometido/consumido conforme a política de execução.

## Episódio de Recovery
Um `RecoveryEpisode` é uma unidade operacional independente dentro da sessão.

```text
Session
 ├─ Episode #1
 │    ├─ attempts
 │    └─ SUCCESS | FAILED
 ├─ Episode #2
 │    └─ ...
 └─ Episode #N
```

O sucesso de um episódio **não cria orçamento novo**. Ele apenas encerra aquele episódio e retorna a sessão ao estado permitido pela política.

Uma nova deterioração pode iniciar outro episódio somente se ainda houver `RecoveryBudgetRemaining` e `Cmax` não tiver sido atingido.

## Regras de entrada em Recovery
A entrada ocorre quando a condição de perda da sessão definida pela política for atingida e houver Recovery elegível.

Se não houver budget disponível, se `Cmax` tiver sido atingido ou se a política não permitir Recovery, a sessão vai para `STOPPED`/`RECOVERY_FAILED` conforme o motivo.

## Tentativas
Cada episódio possui `attempt_count <= Amax`. Falha final de uma tentativa não pode ser tratada como sucesso parcial.

## Worst Case Session Exposure
Devem ser exibidos separadamente:

```text
MaxLossDeposit
RecoveryMaxSession
WorstCaseSessionExposure
```

A definição canônica é:

```text
WorstCaseSessionExposure = MaxLossDeposit + RecoveryMaxSession
```

Isto representa a exposição máxima teórica de caixa autorizada pela política, não uma previsão de perda. Se a implementação utilizar uma política de netting diferente, deve declarar fórmula própria e impedir dupla contagem.

## Confirmação pré-sessão
Mostrar:
- depósito;
- perda máxima do depósito;
- capital floor;
- % de lucro elegível;
- Recovery máximo;
- Recovery Budget inicial;
- risco por operação normal;
- risco por operação em Recovery;
- máximo de episódios;
- máximo de tentativas;
- Worst Case Session Exposure.

O usuário deve confirmar explicitamente. Após confirmação, parâmetros estruturais ficam imutáveis até o encerramento da sessão.

## Invariantes
- Risk Engine é autoridade final.
- Recovery nunca aumenta `MaxLossDeposit`.
- Recovery nunca cria crédito ilimitado.
- Recovery Budget é global à sessão e consumível.
- episódio é diferente de budget.
- lucro não realizado não é elegível.
- falha de reconciliação bloqueia novas entradas.
- `UNKNOWN` relevante bloqueia novas entradas.
- modelo quantitativo não pode alterar política estrutural de risco.
