//! FR-018 como erro de compilação.
//!
//! A camada de risco garante que nenhuma ordem alcança o mercado por fora
//! dela. Essa garantia vale o que valer a impossibilidade de obter o executor
//! — e isso se prova compilando algo que tenta obtê-lo e verificando que o
//! compilador recusa.
//!
//! Se um dia alguém adicionar um `pub fn inner()` por conveniência, este teste
//! falha: o arquivo passaria a compilar, e a garantia teria sido desfeita sem
//! que ninguém percebesse.

#[test]
fn executor_nao_escapa_do_guard() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
