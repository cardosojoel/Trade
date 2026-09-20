# 35 — Limites do desenho

**Status:** normativo · **Versão:** 1.0 · **Atualizado em:** 2026-09-20  
**Domínio de requisitos:** `REQ-LIMITE-*`  
**Conformidade:** conforme  
**Vocabulário:** [`00_GLOSSARIO.md`](00_GLOSSARIO.md)

---

## Para que serve

Os limites que cercam este sistema estavam espalhados por seis documentos. Quem
começa um desenho novo não tem como saber, sem lê-los todos, o que é negociável
e o que não é.

Esta é a folha única do envelope. **Um desenho que não cabe aqui não é uma
proposta ousada: é um desenho inválido**, e descobrir isso no fim custa o
trabalho inteiro.

## Três naturezas, três consequências

| Natureza | O que é | Como muda |
|---|---|---|
| **Duro** | imposto pela corretora ou pela aritmética | não muda por decisão nossa |
| **Medido** | derivado de dado observado | muda com medição nova, registrada |
| **De autoridade** | fixado pela constitution ou por invariante | muda por ADR aceita |

**REQ-LIMITE-001** Uma proposta que viole limite **duro** MUST ser recusada na
origem: não há ADR que a torne possível.

**REQ-LIMITE-002** Um limite **medido** MAY ser revisto, e apenas por medição
nova com método reproduzível, conforme `REQ-CFG-006`. Argumento sem medição
MUST NOT alterar número desta folha.

**REQ-LIMITE-003** Um limite **de autoridade** MUST passar por
[`34_ADR_EMENDAS.md`](34_ADR_EMENDAS.md). Nenhum documento deste conjunto MAY
contorná-lo declarando-se exceção.

---

## 1. Limites duros — a corretora

Lidos da interface pública da Bybit em **2026-09-20**, spot BTCUSDT.
`REQ-BYBIT-006` obriga a releitura a cada sessão: estes valores são revisados
nos dias **3 e 17 de cada mês, às 08h00 UTC+8**.

| Limite | Valor | Consequência no desenho |
|---|---:|---|
| Valor mínimo da ordem | **5 USDT** | define o chão de operação e a banca mínima viável |
| Passo de quantidade | **0,000001 BTC** | ≈ US$ 0,08; origem do resíduo da moeda base |
| Passo de preço | **0,1 USDT** | irrelevante nos stops usados (≈ 975 USDT) |
| Quantidade máxima por ordem | 230 BTC | inalcançável nas bancas em estudo |
| Quantidade máxima a mercado | 120 BTC | idem |
| Valor máximo por ordem | 8.000.000 USDT | idem |
| Banda de preço de ordem limitada | 0,5% e 1% | mais estreita que o stop de 2% — ver `REQ-EXEC-010` |
| Ordens abertas por símbolo | 30 TP/SL + 30 condicionais | 15× o que o perfil usa |
| Criação de ordens | 20/s | o perfil usa 2 **por dia** |
| Consultas | 50/s | — |
| Punição por excesso | erro 403, **≥ 10 min** de bloqueio de IP | risco na coleta de histórico, não na operação |
| Taxa | **0,1% por perna**, maker e taker, não-VIP | 80% do custo total |
| Moeda da taxa na compra | **moeda base (BTC)** | a quantidade recebida não é múltiplo do passo |
| `min_order_qty` | **deprecado** | MUST NOT ser usado para validar — `REQ-BYBIT-008` |

**REQ-LIMITE-004** Esta tabela MUST declarar a data da leitura. Tabela sem data
é valor embutido em código com outro nome.

## 2. Limites duros — aritmética

Não dependem de mercado nem de corretora. São identidades.

| Identidade | Consequência |
|---|---|
| `PositionNotional = AllowedTradeRisk / stop_pct` | com stop mais curto que o risco, o sizing pede posição maior que o caixa |
| `risco_por_operação = teto_de_posição × stop` e `teto ≤ 100%` | **em spot, o risco por operação nunca excede a distância do stop** |
| `PositionNotional ≤ caixa disponível` | `REQ-SIZING-003` — em spot não há alavancagem que preencha a diferença |
| `chão_de_operação = min_order_amt / teto` | abaixo disso a conta existe mas não opera |

## 3. Limites medidos — o mercado

Sobre 525.600 velas de 1 min, de 20/09/2025 a 19/09/2026.

| Limite | Valor | O que impede |
|---|---:|---|
| Custo por ida e volta | **0,25%** | 0,20% taxa + 0,05% slippage e spread |
| Custo ÷ movimento mediano, 5 min | **4,5×** | operar em vela de 1 min é perder por construção |
| Custo ÷ movimento mediano, 1 h | **1,3×** | ainda perdedor |
| Custo ÷ movimento mediano, 4 h | 0,61× | **primeiro horizonte viável** |
| Custo ÷ movimento mediano, 24 h | 0,19× | folga confortável |
| Horizonte mínimo de posição | **4 h** | enquanto o custo for 0,25% |
| ATR de 1 min | 0,061% do preço | stop por múltiplo de ATR de 1 min fica **dentro do custo** |
| Excursão adversa em 4 h | mediana 0,46% · p75 0,90% · p90 1,52% | dimensiona o stop |
| Stop que minimiza o acerto exigido | **2,00%** | mais largo não melhora: sobram posições sem resolver |
| Acerto da entrada aleatória | **46,5%** | a régua |
| Acerto para empatar | **59,4%** | com o resíduo tratado |
| Acerto para empatar sem tratar o resíduo | **63,0%** | em banca de US$ 100 |
| Lacuna a produzir | **12,9 pontos** | é o alvo de desenvolvimento |

## 4. Limites medidos — o capital

| Marco | Valor | O que muda |
|---|---:|---|
| Viabilidade absoluta | **US$ 5,88** | abaixo, a primeira posição não sobrevive ao drawdown |
| Percentuais congelam | **US$ 11,76** | acima disso o perfil derivado é idêntico em qualquer banca |
| Saída parcial possível | **US$ 20** | metade da posição passa do mínimo de ordem |
| Faixa real de dimensionamento | **US$ 100** | dez tamanhos distintos de ordem |
| Piso enquanto o resíduo não for tratado | **US$ 160** | `REQ-CFG-007` |

Capital maior **não** baixa o acerto necessário. Compra granularidade, saída
parcial e folga até o chão.

## 5. Limites de autoridade

| Limite | Origem | Muda por |
|---|---|---|
| Mercado à vista, apenas comprado, sem alavancagem | `CLAUDE.md` — **não está na constitution** | ADR-001, e ver ADR-005 |
| Freio diário sem exceção configurável em tempo de execução | constitution, Princípio II | ADR-002 |
| Retomada automática na virada do período | constitution, Princípio II | ADR-003 |
| Nenhum `f32`/`f64` em caminho monetário | `CLAUDE.md` e `tests/no_float.rs` | ADR-004, e ver ADR-005 |
| Toda ordem atravessa a camada de risco | constitution, Princípio II | emenda — nenhuma ADR aberta |
| Teste escrito e falhando antes da implementação crítica | constitution, Princípio III | emenda — nenhuma ADR aberta |
| Ativo: Bitcoin | constitution | emenda MINOR |
| Modo de execução explícito, sem padrão | constitution, Princípio I | emenda — nenhuma ADR aberta |

## 6. Limites de promoção

Nenhum código chega a capital real sem atravessar as três portas, nesta ordem.

| Porta | Exigência | Situação |
|---|---|---|
| **1. Backtest** | ≥ 12 meses com baixa sustentada e evento de alta volatilidade | **cumprida** — queda de 54,1% em 267 dias e o par −14,02%/+11,92% de fevereiro |
| | ≥ 100 operações, taxas e slippage modelados | pendente |
| | profit factor ≥ 1,3 · drawdown ≤ 15% | pendente — nenhuma estratégia chega perto |
| **2. Paper trading** | ≥ 30 dias ininterruptos em testnet, mesmo código | não iniciada |
| **3. Liberação** | ato humano registrado | não iniciada |

Alteração em estratégia, risco ou execução **reinicia a contagem da Porta 2**.

## 7. Limites do que foi medido

O mais importante desta folha, porque delimita a confiança em todo o resto.

| Limite da medição | Efeito |
|---|---|
| **Um ano, e de baixa** — BTC caiu 29,7%, com drawdown de 54,1% | um robô que só compra foi calibrado contra a correnteza; os números podem não valer em ano de alta |
| **Taxa presumida** — 0,1%/perna, tabela pública VIP0 | é 80% do custo; a taxa real da conta exige credencial |
| **Slippage presumido** — 0,05% por ida e volta | nunca medido contra execução real |
| **Nada com credencial** — taxa efetiva, tipo de conta, permissões da chave, saldo | a auditoria foi feita só com dado público |
| **Um ativo, um regime** | nenhuma medição fora de BTCUSDT |
| **Nenhuma vantagem medida** | tudo aqui diz o que é necessário; nada diz que a estratégia consegue |

**REQ-LIMITE-005** Um número desta folha MUST NOT ser citado sem a limitação
correspondente desta seção quando a limitação for material para o uso. O
exemplo canônico: o acerto necessário de 59,4% foi medido num ano de baixa.

---

## Como usar esta folha

1. Desenhou algo? Confira contra as seções 1 e 2. Não passou, não há recurso.
2. Passou? Confira contra 3 e 4. Não passou, ou o desenho muda, ou é preciso
   medição nova que mova o limite.
3. Passou? Confira contra 5. Não passou, é ADR — o desenho segue válido e para
   antes da implementação.
4. Vai promover? Seção 6.
5. Vai citar um número? Seção 7.
