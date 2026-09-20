//! Kill switch.
//!
//! Acionável **sem acesso ao código** e com efeito sobre um processo já em
//! operação (FR-023): é um arquivo sentinela cuja existência é verificada a
//! cada ordem. Criar um arquivo é algo que se faz de qualquer terminal, sem
//! recompilar, sem reiniciar e sem anexar um depurador.
//!
//! Acionar **não liquida posição aberta** (FR-023a). Vender sob pânico em
//! mercado desordenado pode custar mais que a posição mantida, e essa é uma
//! decisão do mantenedor, não do robô.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum KillSwitch {
    /// Acionado pela presença de um arquivo.
    Sentinel(PathBuf),
    /// Estado fixo — usado em teste e quando não há sentinela configurada.
    Fixed(bool),
}

impl KillSwitch {
    pub fn sentinel(path: impl AsRef<Path>) -> Self {
        KillSwitch::Sentinel(path.as_ref().to_path_buf())
    }

    pub const fn disarmed() -> Self {
        KillSwitch::Fixed(false)
    }

    pub const fn engaged() -> Self {
        KillSwitch::Fixed(true)
    }

    pub fn is_engaged(&self) -> bool {
        match self {
            KillSwitch::Sentinel(p) => p.exists(),
            KillSwitch::Fixed(v) => *v,
        }
    }

    /// Aciona, criando o arquivo sentinela.
    pub fn engage(&self) -> std::io::Result<()> {
        match self {
            KillSwitch::Sentinel(p) => {
                if let Some(d) = p.parent()
                    && !d.as_os_str().is_empty()
                {
                    std::fs::create_dir_all(d)?;
                }
                std::fs::write(p, b"engaged\n")
            }
            KillSwitch::Fixed(_) => Ok(()),
        }
    }

    /// Libera. **Ato humano explícito** — nenhuma rotina do sistema chama isto.
    pub fn release(&self) -> std::io::Result<()> {
        match self {
            KillSwitch::Sentinel(p) if p.exists() => std::fs::remove_file(p),
            _ => Ok(()),
        }
    }
}
