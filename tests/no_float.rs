//! Guarda contra ponto flutuante em caminho monetário.
//!
//! Erro de arredondamento em `f64` não levanta exceção: ele se acumula e
//! reaparece como divergência entre a posição calculada e a reportada pela
//! corretora — exatamente a anomalia de integridade que o Princípio II manda
//! tratar como parada. Usar `f64` seria construir a causa da parada dentro do
//! detector dela. Todo valor monetário é `rust_decimal::Decimal`.

use std::fs;
use std::path::{Path, PathBuf};

const MONETARY_CRATES: &[&str] = &[
    "trade-domain",
    "trade-risk",
    "trade-backtest",
    // O adaptador de paper carrega preço, quantidade e taxa vindos da
    // corretora: é caminho monetário como qualquer outro.
    "trade-paper",
];

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Procura `f32`/`f64` como token isolado, ignorando linhas de comentário.
/// Comentário é ignorado de propósito: explicar *por que* não se usa float é
/// justamente o que se espera encontrar nestes arquivos.
fn float_hits(src: &str) -> Vec<(usize, String)> {
    let mut hits = Vec::new();
    for (i, line) in src.lines().enumerate() {
        let t = line.trim_start();
        if t.starts_with("//") || t.starts_with("*") {
            continue;
        }
        for tipo in ["f32", "f64"] {
            let mut from = 0;
            while let Some(pos) = line[from..].find(tipo) {
                let abs = from + pos;
                let antes = line[..abs].chars().next_back();
                let depois = line[abs + tipo.len()..].chars().next();
                let limite = |c: Option<char>| !c.is_some_and(|c| c.is_alphanumeric() || c == '_');
                if limite(antes) && limite(depois) {
                    hits.push((i + 1, line.trim().to_string()));
                    break;
                }
                from = abs + tipo.len();
            }
        }
    }
    hits
}

#[test]
fn nenhum_ponto_flutuante_em_caminho_monetario() {
    let mut violacoes = Vec::new();

    for &c in MONETARY_CRATES {
        let mut files = Vec::new();
        rs_files(Path::new(&format!("crates/{c}/src")), &mut files);
        files.sort();
        for f in files {
            let src = fs::read_to_string(&f).unwrap_or_default();
            for (linha, texto) in float_hits(&src) {
                violacoes.push(format!("{}:{linha}\n      {texto}", f.display()));
            }
        }
    }

    assert!(
        violacoes.is_empty(),
        "Ponto flutuante encontrado em caminho monetário:\n  {}\n\n\
         Todo preço, quantidade, saldo e P&L é `rust_decimal::Decimal`. \
         Ver research.md R-002.",
        violacoes.join("\n  ")
    );
}
