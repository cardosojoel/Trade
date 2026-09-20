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
| Versão | **1.3.0**, emendada em 2026-09-20 |
| SHA-256 | `38814ec18cab41b6059aaab3133b96670c49fffb795d307b173d1f7a8ea90114` |
| Linhas | 251 |
| Âncora anterior | 1.2.0 · `82b24e3c…` · 220 linhas — substituída pela emenda da ADR-005 |

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

## 4. Uma fronteira só — desde a emenda 1.3.0

A verificação de 2026-09-20 encontrou que as quatro divergências conhecidas não
contrariavam a mesma autoridade: duas eram com a constitution e duas com uma
diretriz de trabalho. A **ADR-005 foi aceita no mesmo dia** e a emenda 1.3.0
fechou a lacuna, trazendo as duas restrições para o corpo normativo.

| Divergência | Contraria hoje | Contrariava antes da emenda |
|---|---|---|
| Recovery depois do freio diário | constitution, Princípio II | o mesmo |
| Retomada por confirmação humana | constitution, Princípio II | o mesmo |
| Derivativos, alavancagem, funding | **constitution**, *Restrições Operacionais* → Mercado | `CLAUDE.md` |
| `f64` em caminho monetário | **constitution**, *Restrições Operacionais* → Representação de valores | `CLAUDE.md` e `tests/no_float.rs` |

As quatro agora exigem emenda formal. O ato ficou **mais pesado**, não menos, e
essa era a intenção: as duas restrições que limitam a perda máxima possível
deixaram de poder ser alteradas por edição de arquivo.

**REQ-FRONTEIRA-006** Uma proposta MUST nomear qual documento ela contraria.
"Conflita com a constitution" MUST NOT ser usado como fórmula genérica — hoje
ela é verdadeira para as quatro divergências conhecidas, mas a fórmula continua
proibida, porque foi justamente a verificação documento a documento que revelou
a lacuna que a emenda fechou.

### Reavaliação exigida pela troca de âncora

`REQ-FRONTEIRA-002` obriga a reavaliar toda a coluna `Conformidade` quando o
hash muda. Feita em 2026-09-20, com este resultado:

- **nenhum documento mudou de estado.** Os seis marcados `exige emenda`
  continuam marcados; os demais continuam `conforme`;
- o que mudou foi **a autoridade contrariada** por ADR-001 e ADR-004, que subiu
  de diretriz de trabalho para constitution;
- a restrição nova de nocional — *ordem nunca excede o caixa disponível* — já era
  observada por `REQ-SIZING-003`, e não criou não-conformidade.

## 4.1 O envelope

A fronteira diz **quem decide**. [`35_LIMITES.md`](35_LIMITES.md) diz **o que
cabe**: os limites duros da corretora e da aritmética, os medidos do mercado e do
capital, e os de autoridade. Um desenho que não passa naquela folha não chega a
precisar de ADR.

## 5. Portabilidade

Este conjunto não tem dependência de build: nenhuma inclusão de arquivo, nenhum
script compartilhado, nenhum caminho que precise resolver em tempo de
compilação. Mover é `git mv` mais a conversão das referências abaixo.

**Verificação refeita em 2026-09-20**, depois da emenda 1.3.0, do
`tests/no_leverage.rs` e dos documentos `00_FRONTEIRA`, `34` e `35`:
**43 referências para fora, em 8 arquivos-alvo e 10 dos 47 documentos** — não
contado o inventário desta seção, que é a lista e não uma referência. A
contagem anterior, de oito, media o conjunto antes de a camada de governança
existir.

**REQ-FRONTEIRA-008** Este inventário MUST ser refeito sempre que a âncora da
seção 1 mudar. Foi por não ser refeito que ele passou a afirmar duas coisas
falsas: que `00_MASTER_INDEX_FINAL` citava o `CURRENT_STATE.md`, quando a
citação já tinha sido removida, e que `32_NAO_OBJETIVOS` tinha uma referência,
quando tem três.

As referências têm duas naturezas, e só uma delas dá trabalho ao mover.

### Apoio factual — precisa virar citação

Um documento de desenho apoiando-se num arquivo do Trade como fonte. São
**quatro ocorrências em três arquivos-alvo**, e são estas que se quebram ao
mudar de endereço.

| Alvo no Trade | Documento | Ocorrências | Para que serve ali |
|---|---|---:|---|
| `data/market.db` | `27` | 2 | origem das 525.600 velas de onde saíram os limites medidos |
| `specs/001-nucleo-execucao/` | `26` | 1 | de onde o código existente nasceu, e não desta SDD |
| `limits.toml` | `33` | 1 | os limiares que hoje governam sem racional registrado |

**REQ-FRONTEIRA-007** Mover este conjunto para outro repositório MUST converter
cada referência de apoio factual em citação com origem declarada — repositório,
caminho e data da leitura — nunca em caminho relativo que só resolve de um lado.

### Nomeação de autoridade — muda de endereço, não de natureza

As outras **39** são a camada de governança nomeando a regra que a governa:
`00_FRONTEIRA`, `34` e `35` ao fixar o que decide, e `00_GLOSSARIO`,
`00_MASTER_INDEX_FINAL`, `29`, `32` e `33` ao registrar o que contrariam.
Nomear a autoridade é a função desses trechos — a referência não desaparece com
a mudança de endereço, muda de destino.

| Alvo no Trade | Documentos | Ocorrências |
|---|---|---:|
| `CLAUDE.md` | `00_FRONTEIRA`, `00_GLOSSARIO`, `00_MASTER_INDEX_FINAL`, `32`, `33`, `34` | 19 |
| `tests/no_float.rs` | `00_FRONTEIRA`, `29`, `32`, `34`, `35` | 9 |
| `.specify/memory/constitution.md` | `00_FRONTEIRA`, `32`, `34` | 5 |
| `tests/no_leverage.rs` | `32`, `33`, `34`, `35` | 4 |
| `limits.toml` | `00_FRONTEIRA` | 1 |
| `Cargo.toml` | `00_FRONTEIRA` | 1 |

Ao ser movido, a âncora da seção 1 passa a acompanhar uma cópia somente leitura
em `constraints/constitution-trade.md`, e essas referências passam a apontar
para `constraints/` — não para o repositório Trade. O hash registrado continua
sendo o que detecta divergência.

A decisão de mover é do mantenedor e não altera nenhuma regra deste documento.

## 6. Colisão de sigla

`SDD` designa neste conjunto o **System Design Document**. O fluxo de
desenvolvimento do repositório Trade chama-se **Spec Kit** e MUST NOT ser
chamado de SDD, ainda que "Spec-Driven Development" produza a mesma sigla. Ver
[`00_GLOSSARIO.md`](00_GLOSSARIO.md).
