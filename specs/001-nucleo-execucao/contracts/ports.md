# Contract — Portas (`trade-ports`)

**Feature**: 001-nucleo-execucao | **Date**: 2026-09-20

As traits abaixo são a fronteira do sistema. Tudo que é substituível — corretora,
banco, relógio, destino da auditoria — entra por aqui. Assinaturas são normativas: o
contrato é o que se testa, e alterá-lo é mudança de contrato, não refinamento.

Nenhuma trait menciona Bybit, SQLite ou HTTP. É o que sustenta o Princípio V.

---

## `MarketDataSource` — origem das velas

```rust
pub trait MarketDataSource {
    /// Percorre as velas do período em ordem cronológica crescente.
    /// O iterador é preguiçoso: memória constante em relação ao período.
    fn candles(
        &self,
        symbol: &Symbol,
        interval: Interval,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Box<dyn Iterator<Item = Result<Candle, MarketError>> + '_>, MarketError>;
}
```

O `Item` é `Result` e não `Candle` porque uma vela inválida no meio do percurso é
informação, não motivo para abortar a leitura inteira em silêncio. `Box<dyn Iterator>`
em vez de tipo associado mantém a trait utilizável como objeto (`dyn MarketDataSource`),
necessária para trocar provedor em tempo de execução (US5).

---

## `OrderExecutor` — destino das ordens

```rust
pub trait OrderExecutor {
    fn execute(&mut self, order: &Order) -> Result<Fill, ExecError>;
}
```

**Quem obtém um `OrderExecutor` obtém acesso ao mercado.** Por isso ele não é
acessível à estratégia, nem por empréstimo: na composição, o executor concreto é
**movido** para dentro do `RiskGuard` (ver abaixo). Não sobra referência viva fora
dele. É assim que FR-018 deixa de ser regra e vira propriedade do tipo.

---

## `AccountView` — saldo e posição

```rust
pub trait AccountView {
    fn balance(&self) -> Decimal;
    fn position(&self) -> Position;
}
```

Somente leitura. A posição é alterada pela aplicação de um `Fill`, nunca por atribuição
direta — é o que impede que uma quantidade detida negativa (SC-010) seja construída por
engano.

---

## `Clock` — tempo

```rust
pub trait Clock {
    fn now(&self) -> DateTime<Utc>;
}
```

Existe para que nada no sistema chame o relógio do sistema. No backtest, o `Clock`
devolve o **instante simulado** da vela corrente — o que torna impossível, por
construção, datar um evento com a hora real da máquina e quebrar o determinismo de
FR-029.

---

## `AuditSink` — destino do registro

```rust
pub trait AuditSink {
    fn record(&mut self, event: AuditEvent) -> Result<(), AuditError>;
    fn flush(&mut self) -> Result<(), AuditError>;
}
```

Porta obrigatória do motor: o motor não é construível sem um `AuditSink`. Não há
configuração que desligue a auditoria, porque o Princípio IV não admite execução sem
registro. Ver [audit-event.md](./audit-event.md) para o esquema do evento.

---

## `CandleRepository` — persistência do histórico

```rust
pub trait CandleRepository {
    fn upsert_page(&mut self, symbol: &Symbol, interval: Interval, candles: &[Candle])
        -> Result<usize, StorageError>;
    fn record_gaps(&mut self, symbol: &Symbol, interval: Interval, gaps: &[Gap])
        -> Result<(), StorageError>;
    fn coverage(&self, symbol: &Symbol, interval: Interval)
        -> Result<Option<Coverage>, StorageError>;
}
```

`upsert_page` grava uma página inteira **em uma transação** e devolve quantas velas
eram novas. Transação por página é o que garante FR-015: uma coleta interrompida no
meio deixa páginas inteiras gravadas, nunca meia página. `coverage` informa o que já
existe, e é sobre ela que a retomada de FR-014 decide por onde continuar.

---

## `Strategy` — a fronteira que mais importa

> **Onde mora**: `trade-domain`, não `trade-ports`. A diferença é o ponto inteiro.
> Se `Strategy` vivesse aqui, `trade-strategy` precisaria declarar `trade-ports`
> como dependência e passaria a **conseguir nomear** `OrderExecutor`. Vivendo em
> `trade-domain`, a crate de estratégias depende só do domínio e o executor é,
> literalmente, um nome que ela não alcança. Verificado por
> `tests/architecture.rs::estrategia_nao_consegue_nomear_o_executor_de_ordens`.

```rust
pub trait Strategy {
    fn on_candle(&mut self, ctx: &MarketContext) -> Option<Signal>;
}

pub struct MarketContext<'a> {
    pub candle: &'a Candle,
    pub position: &'a Position,
    pub balance: Decimal,
}
```

Repare no que **não** está aqui: nenhum `OrderExecutor`, nenhum `RiskLimits` mutável,
nenhuma corretora. A estratégia enxerga o mercado e o próprio estado, e devolve
intenção. Ela não pode emitir ordem porque não tem a quem emitir, e não pode afrouxar
limite porque não tem o que alterar — FR-021 satisfeito pela ausência, que é a única
forma de proibição que não depende de vigilância.

`MarketContext` contém a vela **corrente**, e o preenchimento é simulado na vela
**seguinte** (FR-030). A estratégia não tem meio de olhar adiante: o iterador já
passou, e nada no contexto dá acesso ao futuro.

---

## `RiskGuard` — o funil obrigatório (`trade-risk`)

Não é uma porta; é o componente que as costura. Aparece aqui porque é onde a garantia
estrutural se materializa.

```rust
pub struct RiskGuard<E: OrderExecutor> {
    inner: E,                 // movido para dentro: não há referência externa
    limits: RiskLimits,       // sem setter público
    state: RiskState,
    kill_switch: KillSwitch,
}

impl<E: OrderExecutor> RiskGuard<E> {
    pub fn new(inner: E, limits: RiskLimits, kill: KillSwitch) -> Self;

    /// Único caminho até o executor.
    pub fn submit(&mut self, order: &Order)
        -> Result<(RiskDecision, Option<Fill>), ExecError>;

    pub fn on_day_boundary(&mut self, day: NaiveDate);   // FR-022a
    pub fn classify(&self, err: &ExecError) -> Anomaly;  // FR-024a / FR-024b
}
```

Três propriedades que o tipo impõe:

1. `inner` é privado e foi movido. **Não existe forma de obter o executor de volta** —
   nem `inner()`, nem `Deref`, nem campo público. FR-018 é estrutural.
2. `limits` não tem setter. Alterar limites exige construir outro `RiskGuard`, o que
   só o ponto de composição faz. FR-021 é estrutural.
3. `submit` devolve `RiskDecision` **sempre**, mesmo em recusa. Não há caminho que
   produza um `Fill` sem produzir a decisão que o autorizou — o que faz SC-002 (100%
   das ordens com decisão registrada) ser consequência do tipo de retorno.

`on_day_boundary` é chamado pelo motor ao cruzar a meia-noite UTC e zera a perda do
dia, devolvendo autonomia sem ato humano (FR-022a).

`classify` traduz um erro de execução em `Anomaly::Transient { attempt }` ou
`Anomaly::Integrity(cause)`. Enum exaustivo: acrescentar causa nova quebra a
compilação de quem trata anomalias, que é o comportamento desejado.

---

## Erros

```rust
pub enum MarketError { NotFound, InvalidCandle { .. }, Source { .. } }
pub enum ExecError    { Rejected { .. }, Unavailable { .. }, Timeout, Integrity { .. } }
pub enum StorageError { .. }
pub enum AuditError   { .. }
```

`ExecError` é deliberadamente **agnóstico de corretora** (FR-010): a estratégia e o
motor veem `Unavailable`, não "erro 10006 da Bybit". A tradução de códigos específicos
acontece dentro do adaptador, que é o único lugar que conhece a corretora. É o que
permite que o mesmo tratamento de falha sirva para o simulador de backtest e para a
Bybit ao vivo, sem alteração no motor.
