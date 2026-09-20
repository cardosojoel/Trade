# 22 — Failure Recovery — v3

**Status:** normativo · **Versão:** 1.1 · **Atualizado em:** 2026-09-20  
**Domínio de requisitos:** `REQ-FAIL-*`  
**Conformidade:** conforme  
**Vocabulário:** [`00_GLOSSARIO.md`](00_GLOSSARIO.md)

---

## Padrão
`Detect → Classify → Freeze/Continue → Recover → Reconcile → Resume`

## Bybit reconnect
**REQ-FAIL-001** A reconexão MUST seguir a sequência completa, sem pular etapa:

```text
DISCONNECTED
→ reconnect transport
→ authenticate
→ resubscribe
→ restore market/private streams
→ validate sequence/snapshot
→ query/reconcile authoritative state
→ persist reconciliation
→ RESUME
```

**REQ-FAIL-002** Novas entradas MUST permanecer bloqueadas até a reconciliação
obrigatória terminar.

## Cenários
WebSocket disconnect, stale data, sequence gap, Bybit indisponível, order UNKNOWN, partial fill, position mismatch, SQLite indisponível, crash, reboot, CPU/memory pressure e clock anomaly.

## UNKNOWN order
**REQ-FAIL-003** Ordem de resultado desconhecido MUST NOT ser repetida antes da
reconciliação do estado remoto.

## Safe state
**REQ-FAIL-004** Incerteza material sobre posição, ordem, saldo ou risco MUST
bloquear novas entradas.
