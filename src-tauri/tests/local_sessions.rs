//! Integração serial do `local-sessions`: persistência do registry e (a partir
//! de L07) o `SessionManager` sobre vault/registry temporários.

#![cfg(windows)]

use std::fs;
use std::path::PathBuf;

use secrets_storage_lib::sessions::model::{AuthMode, LockPolicy, Registry, SessionEntry};
use secrets_storage_lib::sessions::registry::{self, to_json_bytes};
use secrets_storage_lib::storage::atomic_vault::AtomicVaultWriter;
use uuid::Uuid;

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn create() -> Self {
        for _ in 0..32 {
            let path = std::env::temp_dir().join(format!("secrets-sessions-{}", Uuid::new_v4()));
            match fs::create_dir(&path) {
                Ok(()) => return Self { path },
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("falha ao criar diretório temporário: {error}"),
            }
        }
        panic!("não foi possível reservar diretório temporário exclusivo");
    }

    fn registry_path(&self) -> PathBuf {
        self.path.join("registry.json")
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn entry(name: &str) -> SessionEntry {
    SessionEntry::new(
        Uuid::new_v4(),
        name,
        AuthMode::Global,
        None,
        LockPolicy::default(),
        "2026-07-21T12:00:00Z".to_owned(),
    )
    .expect("entrada válida")
}

fn registry_with(names: &[&str]) -> Registry {
    let mut reg = Registry::new();
    for name in names {
        reg.insert(entry(name)).unwrap();
    }
    reg
}

#[test]
fn load_de_diretorio_vazio_retorna_registro_vazio() {
    let dir = TestDirectory::create();
    let loaded = registry::load(&dir.registry_path()).expect("primeiro uso");
    assert!(loaded.sessions.is_empty());
}

#[test]
fn save_depois_load_faz_roundtrip() {
    let dir = TestDirectory::create();
    let path = dir.registry_path();
    let reg = registry_with(&["Trabalho", "Pessoal"]);
    registry::save(&path, &reg).expect("primeiro commit");
    assert_eq!(registry::load(&path).unwrap(), reg);
}

#[test]
fn save_substitui_o_registro_anterior() {
    let dir = TestDirectory::create();
    let path = dir.registry_path();
    registry::save(&path, &registry_with(&["Trabalho"])).unwrap();
    let updated = registry_with(&["Trabalho", "Pessoal", "Projetos"]);
    registry::save(&path, &updated).expect("replace");
    assert_eq!(registry::load(&path).unwrap().sessions.len(), 3);
    // Nenhum backup órfão permanece após um commit bem-sucedido.
    assert!(!AtomicVaultWriter::backup_path(&path).try_exists().unwrap());
}

#[test]
fn load_recupera_de_backup_apos_commit_interrompido() {
    let dir = TestDirectory::create();
    let path = dir.registry_path();
    let good = registry_with(&["Trabalho"]);
    registry::save(&path, &good).unwrap();

    // Simula uma queda no meio do replace: destino corrompido + backup válido.
    let backup = AtomicVaultWriter::backup_path(&path);
    fs::copy(&path, &backup).unwrap();
    fs::write(&path, b"lixo corrompido").unwrap();

    let recovered = registry::load(&path).expect("recuperação do backup");
    assert_eq!(recovered, good);
}

#[test]
fn destino_corrompido_sem_backup_falha_fechado() {
    let dir = TestDirectory::create();
    let path = dir.registry_path();
    fs::write(&path, b"nao e json").unwrap();
    assert!(registry::load(&path).is_err());
    // O arquivo é preservado (fail-closed, sem apagar).
    assert!(path.try_exists().unwrap());
}

#[test]
fn versao_futura_no_disco_falha_fechado() {
    let dir = TestDirectory::create();
    let path = dir.registry_path();
    let reg = registry_with(&["Trabalho"]);
    let mut value: serde_json::Value =
        serde_json::from_slice(&to_json_bytes(&reg).unwrap()).unwrap();
    value["version"] = serde_json::json!(999);
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(registry::load(&path).is_err());
    assert!(path.try_exists().unwrap());
}
