# Audit Action Plan — P0

**Status:** registro histórico — **não normativo**, preservado como está  
**Vocabulário:** [`00_GLOSSARIO.md`](00_GLOSSARIO.md)

---

1. Canonicalizar EV ternário.
2. Separar Recovery Episode de Recovery Budget global.
3. Formalizar WorstCaseSessionLoss.
4. Criar Configuration Registry.
5. Criar Bybit Instrument Registry.
6. Definir tipos monetários/timestamps/errors/State Transition em Rust.
7. Criar Requirements Traceability Matrix (MD 26).
8. Definir PatternIndex.
9. Definir ProbabilityCalibration.
10. Evoluir Execution Simulator com microestrutura e fills reais.

## Gate

Nenhum novo componente crítico deve ser promovido para produção antes de possuir:

Specification → Rust implementation → Unit/Integration test → Replay/Backtest → Benchmark → Observable production metric.
