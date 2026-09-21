//! `trade porta2` — o relatório da revisão que precede a Porta 3.
//!
//! Lê o registro e põe os seis critérios diante de quem decide, com o
//! observado ao lado do exigido. **Não promove nada**: a promoção entre
//! portões é ato humano registrado (Princípio I), e um comando que a fizesse
//! sozinho seria exatamente o que o Princípio I proíbe.

use crate::cli::Porta2Args;
use crate::cmd_backtest::ExitCode;
use rust_decimal::Decimal;
use std::collections::BTreeSet;
use trade_domain::{Money, ObservadoPorta2, RelatorioPorta2, relatorio_porta2, slippage};
use trade_storage::open_runs;
use trade_storage::runs_repo::RunsRepository;

pub fn run(args: &Porta2Args) -> (ExitCode, String) {
    match executar(args) {
        Ok(v) | Err(v) => v,
    }
}

fn executar(args: &Porta2Args) -> Result<(ExitCode, String), (ExitCode, String)> {
    let conn = open_runs(&args.runs_db).map_err(|e| (ExitCode::Uso, e.to_string()))?;
    let repo = RunsRepository::new(conn);

    let paper: Vec<_> = repo
        .listar_execucoes()
        .map_err(|e| (ExitCode::Uso, e.to_string()))?
        .into_iter()
        .filter(|e| e.mode == "paper")
        .collect();

    if paper.is_empty() {
        return Err((
            ExitCode::HistoricoAusente,
            format!(
                "nenhuma execução em modo `paper` em {}.\n\
                 A Porta 2 se mede sobre trinta dias na testnet, e eles ainda não\n\
                 correram. Antes disso: criar a chave de testnet sem permissão de saque,\n\
                 conferir com `trade paper verificar`, e abrir a sessão com\n\
                 `trade paper rodar --mode paper`.",
                args.runs_db.display()
            ),
        ));
    }

    // A execução mais longa é a que responde pela Porta 2: o critério é
    // operação **ininterrupta**, e somar períodos separados contaria como
    // contínuo o que não foi.
    let alvo = paper
        .iter()
        .max_by_key(|e| {
            e.ended_at
                .unwrap_or(e.started_at)
                .signed_duration_since(e.started_at)
                .num_seconds()
        })
        .expect("há pelo menos uma");

    let eventos = repo
        .linha_do_tempo(&alvo.run_id)
        .map_err(|e| (ExitCode::Uso, e.to_string()))?;

    // Slippage de cada preenchimento. Vem do registro, e não de uma conta
    // paralela: é o mesmo número que o executor gravou.
    let observados: Vec<Money> = eventos
        .iter()
        .filter(|e| e.kind == "fill")
        .filter_map(|e| e.payload["slippage"].as_str())
        .filter_map(|s| s.parse::<Decimal>().ok())
        .collect();

    // O identificador de cliente é `order_link_id(run_id, order_id)`, e o
    // `order_id` está na carga. Repetido dentro da execução significa ordem
    // duplicada (SC-104).
    let mut vistos = BTreeSet::new();
    let mut duplicadas = 0usize;
    for e in eventos.iter().filter(|e| e.kind == "order") {
        if let Some(id) = e.payload["order_id"].as_u64()
            && !vistos.insert(id)
        {
            duplicadas += 1;
        }
    }

    // Reinícios: cada reconciliação na abertura de sessão. Reconciliada é a
    // que deu `sincronizado` — as outras impediram a sessão de começar.
    let reconciliacoes: Vec<_> = eventos
        .iter()
        .filter(|e| e.kind == "reconciliation")
        .collect();
    let reinicios = reconciliacoes.len();
    let reinicios_reconciliados = reconciliacoes
        .iter()
        .filter(|e| e.payload["veredito"] == "sincronizado")
        .count();

    // Divergência de posição que ocorreu sem aparecer classificada: um `halt`
    // por integridade sem a reconciliação correspondente no registro.
    let divergencias_nao_detectadas = eventos
        .iter()
        .filter(|e| e.kind == "halt")
        .filter(|e| e.payload["cause"] == "PositionDivergence")
        .filter(|h| {
            !reconciliacoes
                .iter()
                .any(|r| r.seq < h.seq && r.payload["veredito"] == "divergente")
        })
        .count();

    let dias = duracao_em_dias(alvo.started_at, alvo.ended_at);
    let seq_sem_buraco = sem_buraco(&eventos);

    let o = ObservadoPorta2 {
        dias_corridos: dias,
        seq_sem_buraco,
        slippage: slippage(&observados, args.amostra_minima),
        divergencias_nao_detectadas,
        ordens_duplicadas: duplicadas,
        reinicios,
        reinicios_reconciliados,
        // Exige um backtest do mesmo período para comparar, e compará-los é
        // ato de quem revisa: escolher **qual** backtest é a comparação vale
        // não é decisão que um comando toma sozinho.
        divergencia: None,
    };

    let r = relatorio_porta2(&o, Decimal::from(args.dias_exigidos), args.amostra_minima);
    let code = if r.passou {
        ExitCode::Ok
    } else {
        ExitCode::Interrompido
    };
    Ok((code, render(&alvo.run_id, &r)))
}

/// O `seq` não tem buraco: de 1 ao maior, sem faltar nenhum.
///
/// É o que o `SC-101` exige junto dos trinta dias. Trinta dias cujo registro
/// tem buraco não são trinta dias de registro — o que falta pode ser
/// justamente o que importa.
fn sem_buraco(eventos: &[trade_storage::runs_repo::EventoLido]) -> bool {
    eventos
        .iter()
        .enumerate()
        .all(|(i, e)| e.seq == u64::try_from(i).unwrap_or(0) + 1)
}

fn duracao_em_dias(
    inicio: chrono::DateTime<chrono::Utc>,
    fim: Option<chrono::DateTime<chrono::Utc>>,
) -> Money {
    // Sem `ended_at`, a sessão ainda corre: conta até agora.
    let fim = fim.unwrap_or_else(chrono::Utc::now);
    Decimal::from(fim.signed_duration_since(inicio).num_seconds()) / Decimal::from(86_400)
}

fn render(run_id: &str, r: &RelatorioPorta2) -> String {
    let mut s = format!("\n  Relatório da Porta 2 · execução {run_id}\n\n");
    for c in &r.criterios {
        s.push_str(&format!(
            "  {}  {}\n      exigido    {}\n      observado  {}\n\n",
            if c.passou { "✓" } else { "✗" },
            c.codigo,
            c.exigencia,
            c.observado
        ));
    }
    s.push_str(if r.passou {
        "  Os seis critérios se sustentam sobre o registro.\n\n  \
         Isto **não promove** nada. A promoção para capital real é ato humano\n  \
         registrado, e o que este relatório faz é pôr o registro diante de quem\n  \
         decide.\n"
    } else {
        "  A Porta 2 não se sustenta: os critérios são conjuntivos, e cada um cobre\n  \
         uma forma diferente de o sistema estar errado.\n"
    });
    s
}
