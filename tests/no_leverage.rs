//! Guarda contra alavancagem, derivativos e venda a descoberto.
//!
//! A emenda 1.3.0 da constitution fixou em *Restrições Operacionais e de
//! Segurança* → **Mercado**: exclusivamente à vista, apenas comprado, sem
//! alavancagem, sem venda a descoberto, e ordem que nunca excede o caixa.
//!
//! Essa é a restrição que garante que o pior caso seja perder o capital
//! depositado. Com alavancagem, o pior caso passa a depender da liquidação da
//! corretora — e o Princípio II perde a sua garantia mais forte.
//!
//! Até esta data a regra não tinha nenhuma trava executável: o
//! `tests/no_float.rs` barrava ponto flutuante no CI, e nada barrava
//! alavancagem. Este arquivo fecha essa lacuna pelo mesmo caminho barato dos
//! outros guardas — lendo o código antes que ele chegue ao merge.

use std::fs;
use std::path::{Path, PathBuf};

const CRATES: &[&str] = &[
    "trade-domain",
    "trade-ports",
    "trade-risk",
    "trade-strategy",
    "trade-backtest",
    "trade-storage",
    "trade-bybit",
    "trade-paper",
    "trade-cli",
];

/// Vocabulário que só existe para operar alavancado, vendido ou em derivativo.
///
/// A lista é deliberadamente restrita a termos sem uso legítimo neste domínio.
/// `borrow` ficou de fora: é `RefCell`, não empréstimo. `futuro` e `liquidar`
/// também: aparecem em português corrente no código atual, um falando de datas
/// e outro de uma decisão do operador.
const PROIBIDOS: &[&str] = &[
    "leverage",
    "alavancagem",
    "alavancado",
    "margin",
    "margem",
    "reduce_only",
    "reduceOnly",
    "position_idx",
    "positionIdx",
    "funding_rate",
    "fundingRate",
    "perpetual",
    "perpetuo",
    "short_sell",
    "sell_short",
    "descoberto",
    "cross_margin",
    "isolated_margin",
    "borrowed",
    "emprestimo",
];

/// Categorias da Bybit que designam derivativo. `spot` é a única admitida.
const CATEGORIAS_PROIBIDAS: &[&str] = &["\"linear\"", "\"inverse\"", "\"option\""];

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

/// Devolve a linha sem comentário, ou `None` se a linha inteira for comentário.
///
/// Comentário é ignorado de propósito, pela mesma razão do `no_float.rs`:
/// explicar *por que* não há alavancagem é exatamente o que se espera encontrar
/// nestes arquivos, e a explicação não pode disparar o guarda. O `://` é
/// preservado para não truncar URL.
fn codigo(linha: &str) -> Option<&str> {
    let t = linha.trim_start();
    if t.starts_with("//") || t.starts_with("*") || t.starts_with("/*") {
        return None;
    }
    let mut corte = linha.len();
    let bytes = linha.as_bytes();
    for i in 0..linha.len().saturating_sub(1) {
        if bytes[i] == b'/' && bytes[i + 1] == b'/' && (i == 0 || bytes[i - 1] != b':') {
            corte = i;
            break;
        }
    }
    Some(&linha[..corte])
}

fn fontes() -> Vec<PathBuf> {
    let mut arquivos = Vec::new();
    for c in CRATES {
        rs_files(Path::new(&format!("crates/{c}/src")), &mut arquivos);
    }
    arquivos.sort();
    arquivos
}

#[test]
fn vocabulario_de_alavancagem_nao_entra_no_codigo() {
    let mut violacoes = Vec::new();

    for f in fontes() {
        let src = fs::read_to_string(&f).unwrap_or_default();
        for (n, linha) in src.lines().enumerate() {
            let Some(codigo) = codigo(linha) else {
                continue;
            };
            let baixa = codigo.to_lowercase();
            for termo in PROIBIDOS {
                if baixa.contains(&termo.to_lowercase()) {
                    violacoes.push(format!("{}:{} — '{termo}'", f.display(), n + 1));
                }
            }
        }
    }

    assert!(
        violacoes.is_empty(),
        "Vocabulário de alavancagem, derivativo ou venda a descoberto no código:\n  {}\n\n\
         A constitution 1.3.0, em Restrições Operacionais e de Segurança → Mercado, \
         determina mercado exclusivamente à vista, apenas comprado, sem alavancagem e \
         sem venda a descoberto. Com alavancagem a perda deixa de ser limitada ao \
         depósito, e é essa limitação que sustenta o Princípio II.\n\n\
         Se a mudança for mesmo desejada, o que muda primeiro é a constitution — por \
         emenda escrita e aprovada, registrada em MD BOT/34_ADR_EMENDAS.md — e só \
         depois este teste.",
        violacoes.join("\n  ")
    );
}

#[test]
fn o_adaptador_so_negocia_na_categoria_spot() {
    let mut violacoes = Vec::new();

    for f in fontes() {
        let src = fs::read_to_string(&f).unwrap_or_default();
        for (n, linha) in src.lines().enumerate() {
            let Some(codigo) = codigo(linha) else {
                continue;
            };

            for cat in CATEGORIAS_PROIBIDAS {
                if codigo.contains(cat) {
                    violacoes.push(format!("{}:{} — literal {cat}", f.display(), n + 1));
                }
            }

            // Toda linha que nomeia `category` tem de fixá-la em spot. A API da
            // Bybit assume `linear` quando o parâmetro é omitido, então o valor
            // certo por omissão é o errado para este projeto.
            if codigo.contains("category") && !codigo.contains("\"spot\"") {
                violacoes.push(format!(
                    "{}:{} — `category` sem \"spot\" na mesma linha",
                    f.display(),
                    n + 1
                ));
            }
        }
    }

    assert!(
        violacoes.is_empty(),
        "Categoria de derivativo no adaptador:\n  {}\n\n\
         `category=spot` MUST ser enviado explicitamente em toda chamada. A \
         documentação da Bybit assume `linear` — perpétuo — quando o parâmetro é \
         omitido, de modo que esquecer o parâmetro não dá erro: dá derivativo.",
        violacoes.join("\n  ")
    );
}

/// Extrai os nomes das variantes de um `enum` declarado no arquivo.
fn variantes(src: &str, nome: &str) -> Vec<String> {
    let inicio = src
        .find(&format!("pub enum {nome} {{"))
        .unwrap_or_else(|| panic!("`pub enum {nome}` não encontrado"));
    let corpo = &src[inicio..];
    let fim = corpo.find('}').expect("enum sem fechamento");
    corpo[..fim]
        .lines()
        .skip(1)
        .filter_map(|l| {
            let t = l.trim().trim_end_matches(',');
            if t.is_empty() || t.starts_with("//") || t.starts_with('#') {
                None
            } else {
                Some(t.to_string())
            }
        })
        .collect()
}

#[test]
fn o_lado_da_ordem_admite_apenas_compra_e_venda() {
    let src = fs::read_to_string("crates/trade-domain/src/types.rs")
        .expect("lendo crates/trade-domain/src/types.rs");

    assert_eq!(
        variantes(&src, "Side"),
        vec!["Buy", "Sell"],
        "`Side` deixou de admitir apenas compra e venda.\n\n\
         Em spot comprado não existe lado vendido a descoberto: vender é reduzir \
         o que se detém. Uma variante nova aqui — `Short`, `Long`, o que for — \
         representa posição que a constitution 1.3.0 não autoriza."
    );

    assert_eq!(
        variantes(&src, "Intent"),
        vec!["Buy", "Sell", "Hold"],
        "`Intent` mudou de forma. A estratégia propõe comprar, vender ou não \
         fazer nada; qualquer outra intenção precisa de emenda antes de existir."
    );
}

#[test]
fn o_dominio_nao_representa_posicao_vendida() {
    let src = fs::read_to_string("crates/trade-domain/src/position.rs")
        .expect("lendo crates/trade-domain/src/position.rs");

    assert!(
        src.contains("SellExceedsHoldings"),
        "`PositionError::SellExceedsHoldings` desapareceu do domínio.\n\n\
         É esse erro que impede a quantidade detida de ficar negativa (SC-010). \
         Sem ele, uma venda acima do detido deixa de ser erro e passa a ser \
         posição vendida — venda a descoberto por omissão, que é como ela \
         costuma entrar."
    );
}
