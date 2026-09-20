# Bybit Calibration — Revision

Esta revisão atualiza as Frentes 1, 2, 4 e 9 com base na documentação atual da Bybit.

Principais alterações:

1. WebSocket Order Entry como caminho preferencial para create/amend/cancel.
2. `execution.fast` como caminho rápido de detecção de fills.
3. Stream `order`/`execution` como estado completo.
4. Separação entre ACK e execução.
5. Data quality baseada em snapshot/delta, sequência e timestamps.
6. Benchmark com bursts reais, incluindo mensagens de trades que podem conter até 1024 trades.
7. Rate limits observados dinamicamente por headers.
8. SLO E2E não fixa arbitrariamente um tempo de fill antes de medir o ambiente real.
9. Sub-millisecond permanece como SLO do hot path interno.
10. Emergency Exit permanece como caminho crítico independente de SQLite, learning e analytics.

Fontes oficiais consultadas:

- https://bybit-exchange.github.io/docs/v5/websocket/trade/guideline
- https://bybit-exchange.github.io/docs/v5/websocket/private/fast-execution
- https://bybit-exchange.github.io/docs/v5/websocket/private/execution
- https://bybit-exchange.github.io/docs/v5/websocket/public/orderbook
- https://bybit-exchange.github.io/docs/v5/websocket/public/trade
- https://bybit-exchange.github.io/docs/v5/ws/connect
