# P0 — Audit Remediation v3

## Status
P0 aplicado à baseline.

## Correções aplicadas
1. EV ternário tornou-se canônico.
2. Recovery Episode foi separado de Recovery Budget da sessão.
3. WorstCaseSessionExposure foi formalizado.
4. Configuration Registry tornou-se autoridade dos valores operacionais.
5. Bybit Instrument Registry foi criado.
6. Contratos arquiteturais Rust foram formalizados.
7. Requirements Traceability Matrix foi criada.
8. Database Schema incorporou recovery episodes, configuration parameters e instrument specs.
9. Failure Recovery ganhou protocolo explícito de reconnect/resubscribe/reconcile.
10. Testing Strategy ganhou invariantes correspondentes às correções.

## Resultado
A baseline deixa de possuir múltiplas fontes de verdade para valores de configuração e passa a distinguir claramente:
- risco estrutural da sessão;
- episódios de Recovery;
- budget de Recovery;
- exposição máxima teórica;
- EV multiestado;
- constraints do instrumento;
- contratos de software.
