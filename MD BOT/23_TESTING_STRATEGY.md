# 23 — Testing Strategy — v3

**Status:** normativo · **Versão:** 1.1 · **Atualizado em:** 2026-09-20  
**Domínio de requisitos:** `REQ-TEST-*`  
**Conformidade:** conforme  
**Vocabulário:** [`00_GLOSSARIO.md`](00_GLOSSARIO.md)

---

## Camadas
Unit, integration, property-based, regression, replay, backtest, fault injection, load, stress, performance e security.

## Invariantes matemáticos
**REQ-TEST-001** MUST existir teste automatizado para cada invariante:

- Probability ∈ [0,1];
- soma das probabilidades = 1 dentro da tolerância;
- EV ternário inclui custos;
- EV conservador aplica stress;
- sizing respeita caps;
- risco não excede limites;
- Recovery Budget consumido não excede budget da sessão;
- WorstCaseSessionExposure segue fórmula canônica.

## Risk / Recovery
**REQ-TEST-002** MUST ser testado: múltiplos episódios, sucesso seguido de nova deterioração, budget esgotado, Emax, Amax e falha de reconciliação.

## Bybit
**REQ-TEST-003** MUST ser testado: ACK vs fill, duplicate prevention, partial fill, reconnect, resubscribe, sequence gap, unknown order, instrument constraints e reconciliation.

## Instrumento e resíduo

**REQ-TEST-006** MUST ser testado: resíduo da moeda base somado à ordem
seguinte; quantidade recebida diferente da solicitada por conta da taxa;
releitura do instrumento invalidando o perfil derivado; `min_order_amt` alterado
entre sessões; stop recusado antes e depois da abertura da posição.

## Performance
**REQ-TEST-004** MUST ser medido p50/p95/p99/p99.9/max e jitter, por estágio e
end-to-end interno.

## Gate
**REQ-TEST-005** Nenhum build de produção MAY ser promovido sem testes críticos,
replay/regressão e os benchmarks mínimos definidos no Registry.
