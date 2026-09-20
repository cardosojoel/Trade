# 17 — Model Validation

Pipeline:
`Train → Validation → Out-of-Sample → Walk-Forward → Stress → Approval`

Não usar teste para selecionar hiperparâmetros.

Avaliar retorno líquido, drawdown, Sharpe, Sortino, CVaR, hit rate, EV, Brier/log loss, turnover, custos e estabilidade por regime.

Stressar fees, slippage, latency, probability e parâmetros.

Nenhum modelo é promovido somente por retorno absoluto.

Registrar dataset, seed, código, configuração e versões para reprodutibilidade.
