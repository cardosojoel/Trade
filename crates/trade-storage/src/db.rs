//! Abertura de banco e aplicação idempotente do esquema.

use rusqlite::Connection;
use std::path::Path;
use trade_ports::StorageError;

const SCHEMA_MARKET: &str = include_str!("schema_market.sql");
const SCHEMA_RUNS: &str = include_str!("schema_runs.sql");

fn open_with(path: &Path, schema: &str) -> Result<Connection, StorageError> {
    if let Some(dir) = path.parent()
        && !dir.as_os_str().is_empty()
    {
        std::fs::create_dir_all(dir).map_err(|e| StorageError::Open {
            path: dir.display().to_string(),
            cause: e.to_string(),
        })?;
    }

    let conn = Connection::open(path).map_err(|e| StorageError::Open {
        path: path.display().to_string(),
        cause: e.to_string(),
    })?;

    // WAL melhora a escrita concorrente; foreign_keys por higiene; a aplicação
    // do esquema é idempotente (CREATE TABLE IF NOT EXISTS), de modo que abrir
    // um banco existente não é distinguível de criar um novo.
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| StorageError::Query(e.to_string()))?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| StorageError::Query(e.to_string()))?;
    conn.execute_batch(schema)
        .map_err(|e| StorageError::Write(e.to_string()))?;

    Ok(conn)
}

/// Abre o banco do histórico, criando o esquema se necessário.
pub fn open_market(path: impl AsRef<Path>) -> Result<Connection, StorageError> {
    open_with(path.as_ref(), SCHEMA_MARKET)
}

/// Abre o banco de execuções e auditoria, criando o esquema se necessário.
pub fn open_runs(path: impl AsRef<Path>) -> Result<Connection, StorageError> {
    let conn = open_with(path.as_ref(), SCHEMA_RUNS)?;
    migrar_runs(&conn)?;
    Ok(conn)
}

/// Alcança bancos que já existiam antes de uma coluna ser acrescentada.
///
/// `CREATE TABLE IF NOT EXISTS` não altera tabela existente: um banco gravado
/// sob esquema antigo continuaria sem a coluna, e a primeira gravação falharia.
/// O `runs.db` deste repositório tem nove execuções nessas condições, e o
/// registro é **insubstituível** — uma migração que o quebrasse destruiria
/// auditoria que não se reconstrói de fonte nenhuma.
///
/// Só acrescenta coluna anulável. Nunca remove, nunca reescreve linha.
fn migrar_runs(conn: &Connection) -> Result<(), StorageError> {
    const NOVAS: &[(&str, &str)] = &[("code_version", "TEXT")];

    for (coluna, tipo) in NOVAS {
        let existe: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('run') WHERE name = ?1",
                [coluna],
                |r| r.get(0),
            )
            .map_err(|e| StorageError::Query(e.to_string()))?;
        if existe == 0 {
            conn.execute_batch(&format!("ALTER TABLE run ADD COLUMN {coluna} {tipo}"))
                .map_err(|e| StorageError::Write(e.to_string()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn tabelas(conn: &Connection) -> Vec<String> {
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap();
        let v: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        v
    }

    #[test]
    fn market_cria_as_tres_tabelas() {
        let dir = tempdir().unwrap();
        let conn = open_market(dir.path().join("market.db")).unwrap();
        assert_eq!(tabelas(&conn), vec!["candle", "dataset", "gap"]);
    }

    #[test]
    fn runs_cria_as_quatro_tabelas() {
        let dir = tempdir().unwrap();
        let conn = open_runs(dir.path().join("runs.db")).unwrap();
        assert_eq!(
            tabelas(&conn),
            vec!["audit_event", "metrics", "run", "trade"]
        );
    }

    #[test]
    fn abrir_duas_vezes_e_idempotente() {
        let dir = tempdir().unwrap();
        let caminho = dir.path().join("market.db");
        let _ = open_market(&caminho).unwrap();
        let conn = open_market(&caminho).unwrap();
        assert_eq!(tabelas(&conn), vec!["candle", "dataset", "gap"]);
    }

    #[test]
    fn cria_o_diretorio_se_nao_existir() {
        let dir = tempdir().unwrap();
        let caminho = dir.path().join("data").join("aninhado").join("market.db");
        assert!(open_market(&caminho).is_ok());
        assert!(caminho.exists());
    }

    #[test]
    fn os_dois_bancos_sao_arquivos_separados() {
        // Apagar o cache não pode ser a mesma operação que destruir a auditoria.
        let dir = tempdir().unwrap();
        let m = dir.path().join("market.db");
        let r = dir.path().join("runs.db");
        let _ = open_market(&m).unwrap();
        let _ = open_runs(&r).unwrap();
        assert!(m.exists() && r.exists());
        std::fs::remove_file(&m).unwrap();
        assert!(r.exists(), "runs.db sobrevive à remoção de market.db");
    }
}
