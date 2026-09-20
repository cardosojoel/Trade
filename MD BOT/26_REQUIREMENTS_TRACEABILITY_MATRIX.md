# 26 — Requirements Traceability Matrix

## Objetivo
Criar rastreabilidade verificável entre requisito, especificação, implementação Rust, teste, benchmark e métrica de produção.

## ID obrigatório
Cada requisito recebe um ID estável:
`REQ-<DOMÍNIO>-<NÚMERO>`.

## Domínios
`RISK`, `RECOVERY`, `STRATEGY`, `FEATURE`, `PATTERN`, `PROB`, `EV`, `SIZING`, `REGIME`, `EXEC`, `RECON`, `BYBIT`, `DATA`, `PERF`, `DB`, `EVENT`, `REPLAY`, `TEST`, `SEC`, `GOV`.

## Estrutura
| Requirement | Specification | Rust Module | Test | Benchmark/Backtest | Production Metric | Status |
|---|---|---|---|---|---|---|
| REQ-RISK-001 | 03 | risk/session | property test | replay | risk_limit_breach | REQUIRED |
| REQ-RECOVERY-001 | 03/22 | recovery | state-machine test | fault injection | recovery_success_rate | REQUIRED |
| REQ-EV-001 | 14 | expected_value | math/property | backtest | ev_realized | REQUIRED |
| REQ-PATTERN-001 | 12 | pattern | deterministic test | latency benchmark | pattern_p99_us | REQUIRED |
| REQ-BYBIT-001 | 01/28 | bybit_adapter | integration | E2E benchmark | order_ack_latency | REQUIRED |
| REQ-PERF-001 | 09 | hot_path | benchmark | load/stress | decision_p99_us | REQUIRED |

## Critério de completude
Um requisito só pode ser `VERIFIED` quando possuir:
1. especificação normativa;
2. implementação identificada;
3. teste automatizado;
4. benchmark/backtest quando aplicável;
5. métrica operacional quando aplicável.

## Estados
`DRAFT`, `SPECIFIED`, `IMPLEMENTED`, `TESTED`, `BENCHMARKED`, `OBSERVED`, `VERIFIED`, `BLOCKED`.

## Regra
Nenhum requisito crítico de Risk, Execution, Reconciliation, Security ou Performance pode chegar a `VERIFIED` apenas por revisão documental.

## Cobertura
A matriz deve ser gerada/atualizada como artefato de release e comparada com o código Rust.
