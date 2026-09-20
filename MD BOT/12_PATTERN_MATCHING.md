# 12 — Historical Pattern Matching

`Current Pattern → Normalize → Candidate Filter → Distance → Top-K → Weighted Neighbors`

O padrão é um vetor versionado de features normalizadas, com timestamp, regime e horizonte.

A distância deve ser versionada; baseline: distância euclidiana ponderada. Extensões como Mahalanobis só entram após validação.

K é hiperparâmetro e não pode ser otimizado no mesmo período usado para avaliação.

Filtrar por regime, timeframe, horizonte e compatibilidade de features.

Sem amostra mínima ou qualidade suficiente: `INSUFFICIENT_EVIDENCE`.

O histórico não deve ser consultado diretamente no hot path; usar índice/estrutura em memória.
