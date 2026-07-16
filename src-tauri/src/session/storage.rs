//! Armazenamento local dos artefatos de sessão.
//!
//! Layout em `<root>/` (em produção `%APPDATA%/secrets-storage/`):
//! - `registry.json` — metadados não secretos (JSON), legível bloqueado (AD-013);
//! - `keyring.vault` — envelope da GMK (CBOR);
//! - `vaults/<uuid>.vault` — cofre por sessão (CBOR).
//!
//! Toda escrita é **atômica** (grava em `.tmp` e renomeia) para nunca deixar um
//! arquivo meio-escrito após uma falha. Falhas viram [`SessionError::Storage`];
//! CBOR malformado vira [`SessionError::CorruptOrIncompatible`].

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::crypto::envelope::VaultEnvelope;
use crate::crypto::keyring::KeyringEnvelope;
use crate::session::error::{Result, SessionError};
use crate::session::model::Registry;

/// Acesso ao diretório de dados do app.
pub struct Storage {
    root: PathBuf,
}

impl Storage {
    /// Cria o acesso enraizado em `root` (não toca o disco até uma operação).
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn registry_path(&self) -> PathBuf {
        self.root.join("registry.json")
    }

    fn keyring_path(&self) -> PathBuf {
        self.root.join("keyring.vault")
    }

    fn vaults_dir(&self) -> PathBuf {
        self.root.join("vaults")
    }

    fn vault_path(&self, id: &Uuid) -> PathBuf {
        self.vaults_dir().join(format!("{id}.vault"))
    }

    /// Indica se a GMP já foi criada (keyring presente): 1º uso vs. desbloqueio.
    pub fn keyring_exists(&self) -> bool {
        self.keyring_path().is_file()
    }

    // --- Registro (JSON) ---

    /// Lê o registro; ausente ⇒ registro vazio (1º uso).
    pub fn read_registry(&self) -> Result<Registry> {
        let path = self.registry_path();
        match fs::read(&path) {
            Ok(bytes) => {
                serde_json::from_slice(&bytes).map_err(|_| SessionError::CorruptOrIncompatible)
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Registry::empty()),
            Err(_) => Err(SessionError::Storage),
        }
    }

    /// Grava o registro de forma atômica.
    pub fn write_registry(&self, reg: &Registry) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(reg).map_err(|_| SessionError::Storage)?;
        self.atomic_write(&self.registry_path(), &bytes)
    }

    // --- Keyring (CBOR) ---

    /// Lê o envelope do keyring global.
    pub fn read_keyring(&self) -> Result<KeyringEnvelope> {
        let bytes = self.read_file(&self.keyring_path())?;
        ciborium::from_reader(bytes.as_slice()).map_err(|_| SessionError::CorruptOrIncompatible)
    }

    /// Grava o envelope do keyring global de forma atômica.
    pub fn write_keyring(&self, env: &KeyringEnvelope) -> Result<()> {
        let mut bytes = Vec::new();
        ciborium::into_writer(env, &mut bytes).map_err(|_| SessionError::Storage)?;
        self.atomic_write(&self.keyring_path(), &bytes)
    }

    // --- Cofres por sessão (CBOR) ---

    /// Lê o cofre de uma sessão.
    pub fn read_vault(&self, id: &Uuid) -> Result<VaultEnvelope> {
        let bytes = self.read_file(&self.vault_path(id))?;
        ciborium::from_reader(bytes.as_slice()).map_err(|_| SessionError::CorruptOrIncompatible)
    }

    /// Grava o cofre de uma sessão de forma atômica (cria `vaults/` se preciso).
    pub fn write_vault(&self, id: &Uuid, env: &VaultEnvelope) -> Result<()> {
        fs::create_dir_all(self.vaults_dir()).map_err(|_| SessionError::Storage)?;
        let mut bytes = Vec::new();
        ciborium::into_writer(env, &mut bytes).map_err(|_| SessionError::Storage)?;
        self.atomic_write(&self.vault_path(id), &bytes)
    }

    /// Apaga o cofre de uma sessão (idempotente: ausência não é erro).
    pub fn delete_vault(&self, id: &Uuid) -> Result<()> {
        match fs::remove_file(self.vault_path(id)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(SessionError::Storage),
        }
    }

    // --- Auxiliares ---

    fn read_file(&self, path: &Path) -> Result<Vec<u8>> {
        fs::read(path).map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => SessionError::NotFound,
            _ => SessionError::Storage,
        })
    }

    /// Escrita atômica: grava num `.tmp` irmão e renomeia por cima do alvo.
    fn atomic_write(&self, target: &Path, bytes: &[u8]) -> Result<()> {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|_| SessionError::Storage)?;
        }
        let tmp = target.with_extension("tmp");
        fs::write(&tmp, bytes).map_err(|_| SessionError::Storage)?;
        fs::rename(&tmp, target).map_err(|_| SessionError::Storage)?;
        Ok(())
    }
}
