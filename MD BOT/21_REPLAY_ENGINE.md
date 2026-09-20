# 21 — Replay Engine

`Historical Events → Market State → Features → Regime → Pattern → Probability → EV → Risk → Simulated Execution`

Mesmo conjunto de eventos, versões, configuração, seed e relógio lógico deve produzir a mesma sequência de decisões.

Modos:
- decision replay;
- execution replay;
- full-system replay.

Comparar decision_id, signal, probability, EV, risk result e order intent.

Replay é obrigatório para regressão antes de alterações críticas.
