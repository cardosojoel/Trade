//! Captura, na compilação, a versão do código que vai ser gravada no registro.
//!
//! **Por que isto existe.** Duas das nove execuções gravadas têm símbolo,
//! período, estratégia, parâmetros, limites e taxas iguais às outras seis, e
//! resultado diferente — divergindo em 14.299 das 14.308 operações. A causa foi
//! um commit que mudou o arredondamento da quantidade, e ela estava fora do
//! registro: as execuções eram indistinguíveis. É a pendência P6/P9, a decisão
//! 009 escolheu corrigi-la, e a 043 fixou como.
//!
//! **Por que o commit, e não a versão do pacote.** A versão do `Cargo.toml` é a
//! mesma nos dois binários que divergiram. Ela não distingue o caso que motivou
//! a correção, então não serve.
//!
//! **Por que a marca de árvore suja.** Um binário construído a partir de árvore
//! com alteração não commitada afirmaria um commit que não descreve o que
//! rodou — que é a mesma classe de mentira que a ausência da coluna produzia.

use std::process::Command;

fn main() {
    // Recompila quando o commit muda. Sem isto, o valor embutido envelhece em
    // silêncio e o registro passa a mentir com precisão.
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/index");
    println!("cargo:rustc-env=TRADE_CODE_VERSION={}", versao());
}

fn versao() -> String {
    let Some(commit) = git(&["rev-parse", "--short=12", "HEAD"]) else {
        // Sem git — árvore exportada, pacote vendorizado. Devolver um valor
        // inventado seria pior que admitir: nulo no registro significa "não
        // sei", e a interface é obrigada a declarar isso.
        return String::new();
    };

    // `--porcelain` vazio significa árvore limpa. Qualquer linha é alteração
    // não commitada, rastreada ou não.
    let sujo = git(&["status", "--porcelain"]).is_some_and(|s| !s.trim().is_empty());

    if sujo {
        format!("{commit}-sujo")
    } else {
        commit
    }
}

fn git(args: &[&str]) -> Option<String> {
    let saida = Command::new("git").args(args).output().ok()?;
    if !saida.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&saida.stdout).trim().to_string())
}
