# 21 — Replay Engine

**Status:** normativo · **Versão:** 1.1 · **Atualizado em:** 2026-09-20  
**Domínio de requisitos:** `REQ-REPLAY-*`  
**Vocabulário:** [`00_GLOSSARIO.md`](00_GLOSSARIO.md)

---

`Historical Events → Market State → Features → Regime → Pattern → Probability → EV → Risk → Simulated Execution`

**REQ-REPLAY-001** Mesmo conjunto de eventos, versões, configuração, seed e
relógio lógico MUST produzir a mesma sequência de decisões.

Modos:
- decision replay;
- execution replay;
- full-system replay.

**REQ-REPLAY-002** A comparação MUST cobrir `decision_id`, signal, probability,
EV, resultado de risco e order intent.

**REQ-REPLAY-003** Replay de regressão MUST preceder qualquer alteração
crítica.
