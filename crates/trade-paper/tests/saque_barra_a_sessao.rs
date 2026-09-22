//! Emenda 2.1.0, salvaguarda 2 — saque é **condição de partida**.
//!
//! Antes da troca para Demo Trading, a permissão de saque era precaução
//! abstrata: a chave de testnet pertencia a conta sem fundo algum. Depois dela,
//! a chave é emitida pela conta de **produção**, e ali permissão de saque é
//! acesso a dinheiro de verdade.
//!
//! O que estes testes afirmam não é que a checagem devolve erro — `permissao.rs`
//! já cobre isso. É que **nenhuma ordem alcança o transporte** quando ela
//! falha. Código de saída certo com ordem enviada seria a pior combinação
//! possível: o registro diria que parou, e a corretora teria recebido.

use trade_paper::ordem::CAMINHO_CRIAR;
use trade_paper::transporte::testing::TransporteFalso;
use trade_paper::verificar_sem_saque;

fn permissoes(wallet: &str, spot: &str) -> String {
    format!(
        r#"{{"retCode":0,"retMsg":"OK","result":{{"permissions":{{"Wallet":[{wallet}],"Spot":[{spot}]}}}}}}"#
    )
}

/// A sequência que `paper rodar` executa: confere a chave **antes** de montar
/// o executor.
///
/// O executor não é construído aqui de propósito. Ele toma o transporte por
/// valor — é o que faz FR-018 ser propriedade do tipo —, e construí-lo tiraria
/// do teste justamente o objeto que ele precisa interrogar depois. O que se
/// afirma é o que dá para afirmar de fora e é o que importa: quantas
/// requisições saíram, e quais.
fn partida(t: &TransporteFalso) -> bool {
    verificar_sem_saque(t).is_ok()
}

fn caminhos(t: &TransporteFalso) -> Vec<String> {
    t.chamadas.borrow().iter().map(|(c, _)| c.clone()).collect()
}

#[test]
fn chave_com_saque_nao_deixa_ordem_alcancar_o_transporte() {
    for permissao in [
        r#""Withdraw""#,
        r#""AccountTransfer""#,
        r#""SubMemberTransfer""#,
    ] {
        let t = TransporteFalso::responde(&permissoes(permissao, r#""SpotTrade""#));
        assert!(!partida(&t), "permissão {permissao} devia barrar a partida");

        let enviados = caminhos(&t);
        assert_eq!(
            enviados.len(),
            1,
            "só a consulta de permissão devia ter saído: {enviados:?}"
        );
        assert!(
            !enviados.iter().any(|c| c == CAMINHO_CRIAR),
            "uma ordem alcançou o transporte: {enviados:?}"
        );
    }
}

#[test]
fn chave_sem_negociacao_tambem_barra() {
    // Sem permissão de spot não há o que operar. Começar para descobrir isso
    // na primeira ordem custa uma ida à corretora e um evento no registro que
    // não descreve nada.
    let t = TransporteFalso::responde(&permissoes("", ""));
    assert!(!partida(&t));
    assert!(!caminhos(&t).iter().any(|c| c == CAMINHO_CRIAR));
}

#[test]
fn chave_que_negocia_e_nao_saca_deixa_a_partida_seguir() {
    let t = TransporteFalso::responde(&permissoes(r#""ContractTrade""#, r#""SpotTrade""#));
    assert!(
        partida(&t),
        "negocia e não saca: é exatamente a chave que se quer"
    );
}

#[test]
fn permissao_desconhecida_que_mova_fundo_e_recusada() {
    // A lista é de **negação**, não de aceitação: a Bybit acrescenta nomes, e
    // um nome novo que dê saque MUST ser recusado por não estar previsto. Uma
    // lista de aceitação o deixaria passar justamente por ser novo.
    let t = TransporteFalso::responde(&permissoes(r#""WithdrawV2""#, r#""SpotTrade""#));
    assert!(!partida(&t), "nome novo com 'withdraw' devia ser recusado");
}
