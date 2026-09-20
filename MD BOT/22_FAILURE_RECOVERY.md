# 22 — Failure Recovery — v3

## Padrão
`Detect → Classify → Freeze/Continue → Recover → Reconcile → Resume`

## Bybit reconnect
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

Novas entradas permanecem bloqueadas até a reconciliação obrigatória terminar.

## Cenários
WebSocket disconnect, stale data, sequence gap, Bybit indisponível, order UNKNOWN, partial fill, position mismatch, SQLite indisponível, crash, reboot, CPU/memory pressure e clock anomaly.

## UNKNOWN order
Nunca repetir cegamente uma ordem cujo resultado seja desconhecido. Primeiro reconciliar estado remoto.

## Safe state
Qualquer incerteza material sobre posição, ordem, saldo ou risco bloqueia novas entradas.
