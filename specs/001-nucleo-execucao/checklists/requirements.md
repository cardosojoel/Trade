# Specification Quality Checklist: Núcleo de Execução e Risco

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-09-20
**Feature**: [spec.md](../spec.md)
**Constitution**: v1.1.0

## Content Quality

- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [x] Written for non-technical stakeholders
- [x] All mandatory sections completed

## Requirement Completeness

- [x] No [NEEDS CLARIFICATION] markers remain
- [x] Requirements are testable and unambiguous
- [x] Success criteria are measurable
- [x] Success criteria are technology-agnostic (no implementation details)
- [x] All acceptance scenarios are defined
- [x] Edge cases are identified
- [x] Scope is clearly bounded
- [x] Dependencies and assumptions identified

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria
- [x] User scenarios cover primary flows
- [x] Feature meets measurable outcomes defined in Success Criteria
- [x] No implementation details leak into specification

## Notes

**Iteração 1 — reprovações corrigidas antes de fechar o checklist:**

1. *Requirements are testable and unambiguous* — dois casos de borda descreviam o
   comportamento como "deve ser explícito" sem dizer qual é. Resolvidos: **FR-019a**
   (valor exato de fronteira de cada limite) e **FR-023a** (kill switch não liquida
   posição aberta).

**Iteração 2 — lacuna apontada pelo mantenedor:**

2. *Requirements are testable and unambiguous* — a spec definia **quando o robô para**
   e nunca **quando ele volta**. Sem isso, "operação autônoma" não era verificável.
   Resolvido: **FR-018a** (nenhuma aprovação humana por ordem dentro dos limites),
   **FR-020a** (recusa de ordem não interrompe o ciclo), **FR-022a** (bloqueio de perda
   diária cai sozinho na virada do dia), **FR-024a/b/c** (anomalia transitória retenta
   sozinha; anomalia de integridade exige ato humano; toda parada e retomada
   registrada). Motivou emenda da constitution para **v1.1.0**.

**Decisões tomadas na especificação, registradas para o plano:**

- Mercado **spot, apenas comprado** — sem alavancagem, sem venda a descoberto.
  Elimina liquidação forçada e taxa de financiamento do modelo de risco desta feature.
- Histórico **coletado dos dados públicos da Bybit** e armazenado localmente. A coleta
  é ação separada da execução: o backtest nunca acessa a rede.

**Observações sobre itens que passaram com ressalva:**

- *No implementation details* — a Bybit é nomeada como fonte do histórico e como
  corretora alvo. É decisão de negócio, herdada da constitution, não escolha técnica.
  A spec não presume nenhum formato, protocolo ou biblioteca de acesso.
- *Success criteria are technology-agnostic* — **SC-008** ("nenhum teste de lógica
  crítica depende de rede") é critério de processo de desenvolvimento, não de produto.
  Mantido porque deriva do Princípio III da constitution, que é não-negociável e
  precisa ser verificável já nesta feature.

**Rastreabilidade com a constitution:**

| Princípio | Onde aparece na spec |
|---|---|
| I. Validação antes de capital real | FR-001 a FR-003; US1 |
| II. Limites de risco invioláveis | FR-018 a FR-025 e FR-018a/019a/020a/022a/023a/024a/024b/024c; US2 |
| III. Test-first na lógica crítica | SC-008; assumption "Dependência de constitution" |
| IV. Auditabilidade reconstituível | FR-033 a FR-036; US4 |
| V. Independência de corretora | FR-007 a FR-010; US5 |
| VI. Segurança de credenciais | FR-012, FR-036 |

**Status**: aprovado. Pronto para `/speckit-plan`.
