//! Persistência atômica do `registry.json` (metadado não secreto, legível com o
//! app bloqueado — AD-013). Reusa `storage::atomic_vault::AtomicVaultWriter`
//! para temp exclusivo + flush + replace, com verificação por reparse.

use crate::sessions::model::{Registry, SessionError, REGISTRY_VERSION};

/// Serializa o registro em JSON canônico.
pub fn to_json_bytes(registry: &Registry) -> Result<Vec<u8>, SessionError> {
    serde_json::to_vec_pretty(registry).map_err(|_| SessionError::Storage)
}

/// Desserializa e valida a versão. Falha fechada em JSON malformado ou versão
/// superior à corrente (fail-closed, sem interpretar).
pub fn from_json_bytes(bytes: &[u8]) -> Result<Registry, SessionError> {
    let registry: Registry =
        serde_json::from_slice(bytes).map_err(|_| SessionError::IncompatibleRegistry)?;
    if registry.version != REGISTRY_VERSION {
        return Err(SessionError::IncompatibleRegistry);
    }
    Ok(registry)
}

#[cfg(windows)]
mod persistence {
    use std::path::Path;

    use super::{from_json_bytes, to_json_bytes};
    use crate::sessions::model::{Registry, SessionError};
    use crate::storage::atomic_vault::{AtomicVaultError, AtomicVaultWriter};

    fn verifier(bytes: &[u8]) -> bool {
        from_json_bytes(bytes).is_ok()
    }

    /// Grava o registro de forma atômica (temp exclusivo + flush + replace no
    /// mesmo diretório), verificando o destino após o replace.
    pub fn save(path: &Path, registry: &Registry) -> Result<(), SessionError> {
        let bytes = to_json_bytes(registry)?;
        AtomicVaultWriter::commit(path, &bytes, verifier).map_err(map_storage_error)
    }

    /// Carrega o registro. Retorna vazio no 1º uso (nenhum arquivo/backup),
    /// recupera de um commit interrompido e falha fechada em corrupção/versão
    /// futura, preservando os arquivos.
    pub fn load(path: &Path) -> Result<Registry, SessionError> {
        let backup = AtomicVaultWriter::backup_path(path);
        let dest_exists = path.try_exists().map_err(|_| SessionError::Storage)?;
        let backup_exists = backup.try_exists().map_err(|_| SessionError::Storage)?;
        if !dest_exists && !backup_exists {
            return Ok(Registry::new());
        }
        let recovered = AtomicVaultWriter::recover(path, verifier).map_err(map_storage_error)?;
        from_json_bytes(&recovered.bytes)
    }

    fn map_storage_error(error: AtomicVaultError) -> SessionError {
        match error {
            // Nenhuma cópia autenticada ou destino que não reparseia: incompatível.
            AtomicVaultError::NoAuthenticatedVault | AtomicVaultError::AuthenticationFailed => {
                SessionError::IncompatibleRegistry
            }
            AtomicVaultError::CommitFailed | AtomicVaultError::Io(_) => SessionError::Storage,
        }
    }
}

#[cfg(windows)]
pub use persistence::{load, save};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sessions::model::{AuthMode, LockPolicy, SessionEntry};
    use uuid::Uuid;

    fn sample() -> Registry {
        let mut reg = Registry::new();
        reg.insert(
            SessionEntry::new(
                Uuid::new_v4(),
                "Trabalho",
                AuthMode::Global,
                None,
                LockPolicy::default(),
                "2026-07-21T12:00:00Z".to_owned(),
            )
            .unwrap(),
        )
        .unwrap();
        reg
    }

    #[test]
    fn roundtrip_json_preserva_o_registro() {
        let reg = sample();
        let bytes = to_json_bytes(&reg).unwrap();
        assert_eq!(from_json_bytes(&bytes).unwrap(), reg);
    }

    #[test]
    fn json_malformado_e_incompativel() {
        assert_eq!(
            from_json_bytes(b"{ not json").unwrap_err(),
            SessionError::IncompatibleRegistry
        );
    }

    #[test]
    fn versao_futura_e_fail_closed() {
        let reg = sample();
        let mut value: serde_json::Value =
            serde_json::from_slice(&to_json_bytes(&reg).unwrap()).unwrap();
        value["version"] = serde_json::json!(REGISTRY_VERSION + 1);
        let bytes = serde_json::to_vec(&value).unwrap();
        assert_eq!(
            from_json_bytes(&bytes).unwrap_err(),
            SessionError::IncompatibleRegistry
        );
    }
}
