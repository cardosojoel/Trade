# 24 — Configuration & Governance — v3

**Status:** normativo · **Versão:** 1.1 · **Atualizado em:** 2026-09-20  
**Domínio de requisitos:** `REQ-GOV-*`  
**Conformidade:** conforme  
**Vocabulário:** [`00_GLOSSARIO.md`](00_GLOSSARIO.md)

---

## Objetivo
Separar código, modelo, estratégia, política de risco e parâmetros operacionais, eliminando múltiplas fontes de verdade.

## Autoridade dos parâmetros
Os MDs definem **regras e invariantes**. Valores operacionais versionados devem existir no `Configuration Registry` (`27_CONFIGURATION_REGISTRY.md`).

**REQ-GOV-001** Nenhum documento MAY ser fonte concorrente de valores
concretos: regras vivem nos MDs, valores vivem no Registry.

## Categorias
`CODE`, `MODEL`, `STRATEGY_CONFIG`, `RISK_POLICY`, `EXCHANGE_CONFIG`, `OBSERVABILITY`.

## Registro
**REQ-GOV-002** Cada parâmetro MUST possuir:
`parameter_id`, `value`, `type`, `unit`, `scope`, `min`, `max`, `version`, `effective_from`, `status`, `source`.

## Sessão
**REQ-GOV-003** Parâmetros estruturais de risco MUST ser congelados no início
da sessão.

## Governança
**REQ-GOV-004** Toda alteração MUST registrar actor, timestamp, reason, versão
anterior e versão nova. Alterações críticas exigem nova versão de configuração e validação.

## Segurança
**REQ-GOV-005** Segredos MUST NOT existir em Markdown, em SQLite em texto puro
ou em configuração versionada.

## Invariantes
**REQ-GOV-006** Configuração MAY parametrizar comportamento e MUST NOT
contornar invariante do Risk Engine ou permissão de segurança.
