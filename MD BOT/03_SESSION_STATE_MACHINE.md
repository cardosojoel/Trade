# 03 — Session State Machine / Risk & Recovery

**Status:** índice — **não normativo** · **Versão:** 1.0 · **Atualizado em:** 2026-09-20  
**Fonte de verdade:** [`trading_risk_recovery_mathematical_spec.md`](trading_risk_recovery_mathematical_spec.md)  
**Vocabulário:** [`00_GLOSSARIO.md`](00_GLOSSARIO.md)

Sessão, orçamento de risco e episódios de recuperação.

Este documento não enuncia regra própria. Ele existe para localizar, na
especificação normativa, o assunto que antes era reenunciado aqui — e para que
uma correção precise ser feita **uma vez só**.

| Assunto | Onde está |
|---|---|
| Variáveis da sessão (`D`, `L`, `P`, `R`, `T`, `RT`, `Emax`, `Amax`) | §3 |
| Limite de perda do depósito e CapitalFloor | §4–5 |
| Peak equity e lucro realizado | §6–7 |
| Lucro protegido e lucro elegível a Recovery | §8–9 |
| RecoveryBudgetSession: criação, significado, consumo | §10–12 |
| Condição de entrada em Recovery | §13 |
| RecoveryTarget, progresso, sucesso e falha | §14–19 |
| Risco por operação, normal e em Recovery | §20–22 |
| Regra anti-Martingale e regra anti-loop | §23, §29 |
| Position size e risco total | §24–26 |
| Novo episódio de Recovery | §28 |
| Confirmação pré-sessão e imutabilidade da configuração | §30, §32 |
| WorstCaseSessionExposure | §31 |
| Invariantes matemáticos e operacionais | §34, §40 |
| Estados da sessão e estrutura de episódios | §39 |
