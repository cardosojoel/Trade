//! Guarda de fronteira entre crates.
//!
//! Os Princípios II e V da constitution exigem que estratégia e risco não
//! alcancem a corretora. Este teste verifica isso lendo os manifestos: se uma
//! dependência proibida for declarada, o build falha antes de qualquer revisão
//! humana. É o teste mais barato do projeto e o que protege a propriedade mais
//! cara — e por isso roda no CI como portão de merge.

use std::collections::BTreeSet;
use std::fs;

/// Crates que não podem, em nenhuma hipótese, alcançar a rede ou a corretora.
const ISOLATED: &[&str] = &["trade-strategy", "trade-risk", "trade-backtest"];

/// Adaptador de corretora e clientes de rede.
const FORBIDDEN: &[&str] = &[
    "trade-bybit",
    "trade-paper",
    "trade-storage",
    "ureq",
    "reqwest",
    "hyper",
    "curl",
    "tokio",
    "async-std",
];

/// Clientes de rede também são proibidos em dev-dependencies: um teste que
/// alcance a rede corrompe a garantia tanto quanto o código de produção.
const FORBIDDEN_IN_DEV: &[&str] = &["ureq", "reqwest", "hyper", "curl"];

fn deps_of(crate_name: &str, table: &str) -> BTreeSet<String> {
    let path = format!("crates/{crate_name}/Cargo.toml");
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| panic!("lendo {path}: {e}"));
    let doc: toml::Table = raw
        .parse()
        .unwrap_or_else(|e| panic!("parse de {path}: {e}"));
    doc.get(table)
        .and_then(|t| t.as_table())
        .map(|t| t.keys().cloned().collect())
        .unwrap_or_default()
}

#[test]
fn crates_isoladas_nao_dependem_da_corretora_nem_da_rede() {
    let mut violacoes = Vec::new();

    for &c in ISOLATED {
        for dep in deps_of(c, "dependencies") {
            if FORBIDDEN.contains(&dep.as_str()) {
                violacoes.push(format!("{c} declara `{dep}` em [dependencies]"));
            }
        }
        for dep in deps_of(c, "dev-dependencies") {
            if FORBIDDEN_IN_DEV.contains(&dep.as_str()) {
                violacoes.push(format!("{c} declara `{dep}` em [dev-dependencies]"));
            }
        }
    }

    assert!(
        violacoes.is_empty(),
        "Fronteira violada — Princípio V da constitution:\n  {}\n\n\
         Estratégia, risco e backtest não podem alcançar a corretora nem a rede. \
         Quem precisa de uma corretora recebe uma implementação de `trade-ports` \
         no ponto de composição (trade-cli), nunca por dependência direta.",
        violacoes.join("\n  ")
    );
}

#[test]
fn estrategia_nao_consegue_nomear_o_executor_de_ordens() {
    let deps = deps_of("trade-strategy", "dependencies");
    assert!(
        !deps.contains("trade-ports"),
        "trade-strategy declara `trade-ports` e passa a poder nomear `OrderExecutor`.\n\n\
         FR-018 e FR-021 dependem de a estratégia não ter a quem emitir ordem e não \
         ter o que afrouxar. A estratégia devolve `Signal`; o motor converte em ordem \
         e encaminha pelo `RiskGuard`. Se esta dependência for realmente necessária, \
         é a constitution que precisa mudar primeiro, não este teste."
    );
}

#[test]
fn o_ponto_de_composicao_e_unico() {
    // trade-cli é a única crate autorizada a conhecer as duas pontas.
    let cli = deps_of("trade-cli", "dependencies");
    assert!(cli.contains("trade-bybit") && cli.contains("trade-storage"));

    for &c in ISOLATED {
        let deps = deps_of(c, "dependencies");
        assert!(
            !deps.contains("trade-cli"),
            "{c} depende de trade-cli — o ponto de composição não pode ser dependência \
             de quem ele compõe, ou o grafo deixa de ser acíclico e a garantia se perde."
        );
    }
}

/// Nomes de corretora que não podem aparecer no código de estratégia e risco.
const CORRETORAS: &[&str] = &["bybit", "binance", "coinbase", "kraken", "okx"];

#[test]
fn estrategia_e_risco_nao_mencionam_corretora_alguma() {
    // A dependência já é barrada pelo grafo. Este teste pega o passo anterior:
    // uma constante com URL, um comentário que assume o comportamento de uma
    // corretora específica, um campo batizado com o nome dela. Nada disso
    // quebra o build, e tudo isso é conhecimento vazando para onde não devia.
    let mut violacoes = Vec::new();

    for c in ["trade-strategy", "trade-risk"] {
        let mut arquivos = Vec::new();
        rs_files(Path::new(&format!("crates/{c}/src")), &mut arquivos);
        arquivos.sort();

        for f in arquivos {
            let src = fs::read_to_string(&f).unwrap_or_default().to_lowercase();
            for nome in CORRETORAS {
                if src.contains(nome) {
                    violacoes.push(format!("{} menciona '{nome}'", f.display()));
                }
            }
        }
    }

    assert!(
        violacoes.is_empty(),
        "Conhecimento de corretora vazou para estratégia ou risco:\n  {}\n\n\
         Estratégia e risco operam sobre as traits de `trade-ports`. Quem \
         conhece a corretora é o adaptador, e só ele.",
        violacoes.join("\n  ")
    );
}

use std::path::Path;

fn rs_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
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
