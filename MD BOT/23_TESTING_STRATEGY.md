# 23 — Testing Strategy — v3

## Camadas
Unit, integration, property-based, regression, replay, backtest, fault injection, load, stress, performance e security.

## Invariantes matemáticos
- Probability ∈ [0,1];
- soma das probabilidades = 1 dentro da tolerância;
- EV ternário inclui custos;
- EV conservador aplica stress;
- sizing respeita caps;
- risco não excede limites;
- Recovery Budget consumido não excede budget da sessão;
- WorstCaseSessionExposure segue fórmula canônica.

## Risk / Recovery
Testar múltiplos episódios, sucesso seguido de nova deterioração, budget esgotado, Cmax, Amax e falha de reconciliação.

## Bybit
Testar ACK vs fill, duplicate prevention, partial fill, reconnect, resubscribe, sequence gap, unknown order, instrument constraints e reconciliation.

## Performance
Medir p50/p95/p99/p99.9/max e jitter por estágio e end-to-end interno.

## Gate
Nenhum build de produção sem testes críticos, replay/regression e benchmarks mínimos definidos no Registry.
