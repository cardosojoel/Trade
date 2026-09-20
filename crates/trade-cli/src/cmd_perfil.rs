//! `trade perfil` — deriva o perfil de operação a partir do depósito.
//!
//! Quem opera informa só o depósito. O comando devolve teto, stop, risco,
//! limite diário e chão de operação, e imprime o `limits.toml` correspondente.
//!
//! **Não grava o arquivo, e é de propósito.** A constitution exige que os
//! limiares sejam revistos contra o capital real antes da liberação para
//! dinheiro de verdade; um comando que sobrescrevesse a cerca sozinho tiraria
//! essa revisão do caminho. Derivar é automático; adotar é ato de quem opera.

use crate::cli::PerfilArgs;
use crate::config::{load_instrumento, load_parametros};
use trade_domain::Perfil;

pub fn run(args: &PerfilArgs) -> Result<String, (u8, String)> {
    let instrumento = load_instrumento(&args.instrumento).map_err(|e| (2, e.to_string()))?;
    let params = load_parametros(&args.parametros).map_err(|e| (2, e.to_string()))?;

    let p = Perfil::derivar(args.capital, &instrumento, &params).map_err(|e| (2, e.to_string()))?;

    let pct = |v: trade_domain::Money| (v * rust_decimal::Decimal::ONE_HUNDRED).round_dp(2);
    let abs = |v: trade_domain::Money| (args.capital * v).round_dp(2);

    Ok(format!(
        "\n\
         \x20 Depósito                 {}\n\
         \x20 Instrumento              passo {} · ordem mínima {}\n\
         \n\
         \x20 Teto de posição          {}%   {}\n\
         \x20 Stop                     {}%   (distância de preço)\n\
         \x20 Risco por operação       {}%   {}\n\
         \x20 Limite diário            {}%   {}\n\
         \x20 Prazo máximo             {} min\n\
         \x20 Chão de operação         {}\n\
         \n\
         \x20 Abaixo do chão a conta existe e não opera: o teto de posição\n\
         \x20 deixa de pagar a ordem mínima da corretora.\n\
         \n\
         \x20 limits.toml correspondente — revise antes de adotar:\n\
         \n\
         max_daily_loss        = \"{}\"\n\
         max_position_size     = \"{}\"\n\
         max_total_exposure    = \"{}\"\n",
        args.capital,
        instrumento.passo_qty(),
        instrumento.valor_minimo_ordem(),
        pct(p.teto_de_posicao),
        abs(p.teto_de_posicao),
        pct(p.stop),
        pct(p.risco_por_operacao),
        abs(p.risco_por_operacao),
        pct(p.limite_diario),
        abs(p.limite_diario),
        p.prazo_maximo_min,
        p.chao_de_operacao.round_dp(2),
        abs(p.limite_diario),
        abs(p.teto_de_posicao),
        abs(p.teto_de_posicao),
    ))
}
