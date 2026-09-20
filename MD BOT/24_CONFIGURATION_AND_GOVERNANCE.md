# 24 — Configuration & Governance — v3

## Objetivo
Separar código, modelo, estratégia, política de risco e parâmetros operacionais, eliminando múltiplas fontes de verdade.

## Autoridade dos parâmetros
Os MDs definem **regras e invariantes**. Valores operacionais versionados devem existir no `Configuration Registry` (`27_CONFIGURATION_REGISTRY.md`).

Nenhum MD pode ser tratado como fonte concorrente de valores concretos.

## Categorias
`CODE`, `MODEL`, `STRATEGY_CONFIG`, `RISK_POLICY`, `EXCHANGE_CONFIG`, `OBSERVABILITY`.

## Registro
Cada parâmetro possui:
`parameter_id`, `value`, `type`, `unit`, `scope`, `min`, `max`, `version`, `effective_from`, `status`, `source`.

## Sessão
Parâmetros estruturais de risco são congelados no início da sessão.

## Governança
Alterações registram actor, timestamp, reason, previous version e new version. Alterações críticas exigem nova versão de configuração e validação.

## Segurança
Segredos nunca ficam em Markdown, SQLite em texto puro ou configuração versionada.

## Invariantes
Configuration pode parametrizar comportamento, mas nunca pode contornar invariantes do Risk Engine ou permissões de segurança.
