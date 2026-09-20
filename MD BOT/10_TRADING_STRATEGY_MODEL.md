# 10 — Trading Strategy Model

## Objetivo
Formalizar a cadeia de decisão sem misturar sinal, risco e execução.

`Market Data → Features → Regime → Pattern Matching → Probability → EV → Position Sizing → Risk → Execution`

### Regras
- Strategy Engine pode propor uma operação.
- Risk Engine pode rejeitá-la.
- Execution Engine nunca pode transformar uma rejeição em ordem válida.
- `NO_TRADE` é resultado válido.
- Mesmos inputs + versões + configuração devem produzir a mesma decisão, salvo componentes estocásticos explicitamente versionados.

### Pré-condições
Dados válidos, freshness aceitável, regime válido/explicitamente `UNKNOWN`, evidência estatística mínima, Probability e EV válidos, sizing permitido e Risk aprovado.

### Versionamento
Registrar `strategy_version`, `feature_version`, `model_version`, `risk_policy_version` e `config_version` em cada decisão.
