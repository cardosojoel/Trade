//! O domínio de produção existe em **um único lugar** (emenda 2.1.0).
//!
//! Antes da troca para Demo Trading, apontar para produção por engano não
//! produziria ordem alguma: a chave de testnet não existe lá. Depois dela,
//! produziria — a chave de demo é emitida pela conta de produção, e
//! `api-demo.bybit.com` difere de `api.bybit.com` por um prefixo.
//!
//! Este teste é o que torna o engano impossível em vez de improvável. Um
//! domínio escrito em dois lugares pode divergir num deles, e divergir aqui é
//! enviar ordem para a corretora errada.

use std::fs;
use std::path::{Path, PathBuf};

/// Onde o literal pode existir. Um lugar, e é o braço do enum.
const AUTORIZADO: &str = "crates/trade-bybit/src/credencial.rs";

/// O domínio de produção, partido para que **este arquivo** não conte como
/// ocorrência de si mesmo.
fn producao() -> String {
    format!("https://api.{}.com", "bybit")
}

fn crates_do_workspace() -> Vec<String> {
    let raiz = fs::read_to_string("Cargo.toml").expect("lendo Cargo.toml da raiz");
    let doc: toml::Table = raiz.parse().expect("parse do Cargo.toml da raiz");
    doc.get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
        .expect("workspace.members")
        .iter()
        .filter_map(|m| m.as_str())
        .filter_map(|m| m.strip_prefix("crates/"))
        .map(str::to_string)
        .collect()
}

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

#[test]
fn o_dominio_de_producao_so_existe_no_enum_de_ambiente() {
    let alvo = producao();
    let mut violacoes = Vec::new();

    for c in crates_do_workspace() {
        let mut arquivos = Vec::new();
        rs_files(Path::new(&format!("crates/{c}/src")), &mut arquivos);
        arquivos.sort();
        for f in arquivos {
            if f.to_string_lossy().replace('\\', "/") == AUTORIZADO {
                continue;
            }
            let src = fs::read_to_string(&f).unwrap_or_default();
            if src.contains(&alvo) {
                violacoes.push(f.display().to_string());
            }
        }
    }

    assert!(
        violacoes.is_empty(),
        "o domínio de produção aparece fora de `{AUTORIZADO}`:\n  {}\n\n\
         Ele existe num lugar só de propósito. Escrito em dois, pode divergir \
         num deles — e divergir aqui é enviar ordem para a corretora errada. \
         Quem precisa do domínio chama `Ambiente::base_url()`.",
        violacoes.join("\n  ")
    );
}

#[test]
fn o_braco_autorizado_realmente_contem_o_dominio() {
    // Sem isto, renomear o arquivo faria o teste acima passar varrendo nada —
    // e um invariante que passa por não encontrar o que procura é pior que
    // nenhum.
    let src = fs::read_to_string(AUTORIZADO).unwrap_or_else(|e| panic!("lendo {AUTORIZADO}: {e}"));
    assert!(
        src.contains(&producao()),
        "`{AUTORIZADO}` devia conter o domínio de produção, e não contém: \
         o teste acima estaria varrendo o vazio"
    );
}
