# 00 — Fronteira com o repositório Trade

**Status:** normativo · **Versão:** 1.0 · **Atualizado em:** 2026-09-20  
**Domínio de requisitos:** `REQ-FRONTEIRA-*`  
**Conformidade:** conforme  
**Vocabulário:** [`00_GLOSSARIO.md`](00_GLOSSARIO.md)

---

## Por que este documento existe

Este conjunto é trabalho de **desenho de sistema**: ele explora, propõe e
registra. O repositório Trade é trabalho de **implementação**: ele compila, testa
e opera. Os dois precisam evoluir em paralelo sem que um contamine o outro.

O risco concreto não é teórico. Este conjunto já contém quatro premissas que o
Trade não autoriza. Enquanto forem **proposta registrada**, não custam nada.
No dia em que alguém as tratar como plano aprovado, elas entram no código sem
que ninguém tenha assinado.

Este documento é o contrato que impede isso.

## 1. Autoridade

**REQ-FRONTEIRA-001** A autoridade sobre o que o robô pode fazer é, nesta ordem:

```text
1. .specify/memory/constitution.md      (governa)
2. CLAUDE.md + invariantes do build     (cobrados na compilação e no CI)
3. este conjunto de desenho             (propõe)
```

Este conjunto MUST NOT ser tratado como fonte de autoridade sobre o Trade.
Ele descreve um sistema possível, não o sistema autorizado.

### Âncora de sincronia

| | |
|---|---|
| Documento | `.specify/memory/constitution.md` |
| Versão | **1.2.0**, ratificada em 2026-09-20 |
| SHA-256 | `82b24e3cb309d6f297a9c91b4a40b0b5390669b5b65d806a8107cb02bb8d1c72` |
| Linhas | 220 |

**REQ-FRONTEIRA-002** Se o hash mudar, toda a coluna `Conformidade` deste
conjunto MUST ser reavaliada antes de qualquer proposta nova ser aceita. Hash
diferente do registrado significa que as regras mudaram e que as conformidades
declaradas aqui foram verificadas contra um texto que já não existe.

## 2. Direção única

**REQ-FRONTEIRA-003** Existe uma única saída deste conjunto para o Trade, e ela
é unidirecional:

```text
documento de desenho
      ↓
ADR registrada em 34_ADR_EMENDAS.md
      ↓
ADR aceita pelo mantenedor
      ↓
  ┌───────────────┴───────────────┐
emenda formal à constitution   /speckit.specify de uma feature
  (ou ao CLAUDE.md / invariante)     no repositório Trade
```

**REQ-FRONTEIRA-004** Este conjunto MUST NOT alterar, direta ou indiretamente:
`.specify/memory/constitution.md`, `CLAUDE.md`, `specs/`, `crates/`, `tests/`,
`limits.toml`, `Cargo.toml` ou a configuração de CI. Uma decisão de desenho que
exija qualquer dessas mudanças MUST parar em uma ADR e esperar assinatura.

O caminho inverso não existe: o Trade nunca precisa consultar este conjunto para
compilar, testar ou operar.

## 3. Conformidade declarada por documento

**REQ-FRONTEIRA-005** Todo documento normativo deste conjunto MUST declarar no
cabeçalho:

```text
Conformidade: conforme
Conformidade: exige emenda (ADR-001)
```

Um documento é `conforme` quando nada nele **depende** de premissa que a
constitution ou os invariantes do build não autorizem. Menção incidental não
muda o estado — mas menção incidental MUST NOT virar requisito.

Um documento marcado `exige emenda` continua válido como desenho. O que ele não
pode é gerar tarefa de implementação enquanto a ADR não for aceita.

## 4. Duas fronteiras, não uma

A verificação de 2026-09-20 mostrou que as quatro divergências conhecidas não
são todas do mesmo tipo, e tratá-las como se fossem enfraquece as duas:

| Divergência | Onde a regra contrária vive | Como se resolve |
|---|---|---|
| Recovery depois do freio diário | **constitution**, Princípio II | emenda formal à constitution |
| Retomada por confirmação humana, não automática | **constitution**, Princípio II | emenda formal à constitution |
| Derivativos, alavancagem, funding | `CLAUDE.md` — **não está na constitution** | ver ADR-005 |
| `f64` em caminho monetário | `CLAUDE.md` e `tests/no_float.rs` — **não está na constitution** | ver ADR-005 |

**REQ-FRONTEIRA-006** Uma proposta MUST nomear qual documento ela contraria.
"Conflita com a constitution" MUST NOT ser usado como fórmula genérica: das
quatro divergências conhecidas, duas contrariam a constitution e duas contrariam
uma diretriz de trabalho e um teste de build.

A distinção importa porque o peso do ato é diferente. Emendar a constitution é
ato formal com racional escrito e aprovação registrada. Mudar o `CLAUDE.md` é
decisão de rotina — e é exatamente por isso que **"mercado à vista, apenas
comprado, sem alavancagem" estar apenas ali é, em si, um risco**: é a restrição
que impede a perda de exceder o depósito, e hoje qualquer sessão pode reescrevê-la
sem cerimônia. Ver ADR-005.

## 4.1 O envelope

A fronteira diz **quem decide**. [`35_LIMITES.md`](35_LIMITES.md) diz **o que
cabe**: os limites duros da corretora e da aritmética, os medidos do mercado e do
capital, e os de autoridade. Um desenho que não passa naquela folha não chega a
precisar de ADR.

## 5. Portabilidade

Este conjunto é quase autossuficiente. A verificação encontrou **oito**
referências dele para fora, em 45 documentos:

| Referência | Documentos |
|---|---|
| `CURRENT_STATE.md` | `00_MASTER_INDEX_FINAL`, `29` |
| `market.db` | `27` (duas vezes) |
| `.specify/memory/constitution.md` | `32` |
| `tests/no_float.rs` | `29` |
| `specs/001-nucleo-execucao/` | `26` |
| `limits.toml` | `33` |

**REQ-FRONTEIRA-007** Mover este conjunto para outro repositório MUST converter
essas oito referências em citação com origem declarada — repositório, caminho e
data da leitura — nunca em caminho relativo que só resolve de um lado.

Nada mais precisa mudar. Não há inclusão de arquivo, script compartilhado ou
dependência de build. A mudança de endereço é `git mv` mais esta lista; a
decisão de mover é do mantenedor e não altera nenhuma regra deste documento.

Ao ser movido, a âncora da seção 1 passa a acompanhar uma cópia somente leitura
em `constraints/constitution-trade.md`, e o hash registrado é o que detecta
divergência.

## 6. Colisão de sigla

`SDD` designa neste conjunto o **System Design Document**. O fluxo de
desenvolvimento do repositório Trade chama-se **Spec Kit** e MUST NOT ser
chamado de SDD, ainda que "Spec-Driven Development" produza a mesma sigla. Ver
[`00_GLOSSARIO.md`](00_GLOSSARIO.md).
