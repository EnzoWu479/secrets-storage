#![cfg(windows)]

use std::cell::Cell;
use std::fs::{self, OpenOptions};
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use secrets_storage_lib::crypto::envelope::{
    create_vault, unlock, SessionContent, UnlockAuth, VaultEnvelope, VaultNonces, WrapAuth,
};
use secrets_storage_lib::crypto::Key32;
use secrets_storage_lib::storage::atomic_vault::{
    AtomicVaultError, AtomicVaultWriter, RecoverySource,
};
use uuid::Uuid;

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn create() -> Self {
        for _ in 0..32 {
            let path = std::env::temp_dir().join(format!("secrets-storage-{}", Uuid::new_v4()));
            match fs::create_dir(&path) {
                Ok(()) => return Self { path },
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("falha ao criar diretório temporário exclusivo: {error}"),
            }
        }
        panic!("não foi possível reservar diretório temporário exclusivo");
    }

    fn vault_path(&self) -> PathBuf {
        self.path.join("session.vault")
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn authenticated_bytes(marker: i64, nonce: u8) -> Vec<u8> {
    let gmk = Key32::from_bytes([0xC0; 32]);
    let content = SessionContent {
        content_format: 1,
        secrets: vec![ciborium::value::Value::Integer(marker.into())],
    };
    let envelope = create_vault(
        WrapAuth::Global { gmk: &gmk },
        [0xAB; 16],
        "session",
        &[marker as u8; 32],
        marker as u32,
        VaultNonces {
            key_wrap: [nonce; 24],
            payload: [nonce.wrapping_add(1); 24],
        },
        &content,
    )
    .expect("fixture cifrada válida");
    let mut bytes = Vec::new();
    ciborium::into_writer(&envelope, &mut bytes).expect("serializar fixture cifrada");
    bytes
}

fn authenticates(bytes: &[u8]) -> bool {
    let Ok(envelope) = ciborium::from_reader::<VaultEnvelope, _>(bytes) else {
        return false;
    };
    let gmk = Key32::from_bytes([0xC0; 32]);
    unlock(UnlockAuth::Global { gmk: &gmk }, &envelope).is_ok()
}

fn read(path: &Path) -> Vec<u8> {
    fs::read(path).expect("ler vault do teste")
}

#[test]
fn first_commit_moves_an_authenticated_temp_to_destination() {
    let directory = TestDirectory::create();
    let destination = directory.vault_path();
    let candidate = authenticated_bytes(1, 1);

    AtomicVaultWriter::commit(&destination, &candidate, authenticates).expect("primeiro commit");

    assert_eq!(read(&destination), candidate);
    assert!(!AtomicVaultWriter::backup_path(&destination).exists());
}

#[test]
fn replacing_an_existing_destination_commits_the_new_envelope() {
    let directory = TestDirectory::create();
    let destination = directory.vault_path();
    let previous = authenticated_bytes(1, 3);
    let candidate = authenticated_bytes(2, 5);
    AtomicVaultWriter::commit(&destination, &previous, authenticates).unwrap();

    AtomicVaultWriter::commit(&destination, &candidate, authenticates).expect("replace atômico");

    assert_eq!(read(&destination), candidate);
    assert!(!AtomicVaultWriter::backup_path(&destination).exists());
}

#[test]
fn recovery_prefers_an_authenticated_destination_over_a_valid_backup() {
    let directory = TestDirectory::create();
    let destination = directory.vault_path();
    let current = authenticated_bytes(2, 7);
    let backup = authenticated_bytes(1, 9);
    fs::write(&destination, &current).unwrap();
    fs::write(AtomicVaultWriter::backup_path(&destination), backup).unwrap();

    let recovered = AtomicVaultWriter::recover(&destination, authenticates).expect("recovery");

    assert_eq!(recovered.source, RecoverySource::Destination);
    assert_eq!(recovered.bytes, current);
    assert!(!AtomicVaultWriter::backup_path(&destination).exists());
}

#[test]
fn recovery_replaces_a_corrupted_destination_with_authenticated_backup() {
    let directory = TestDirectory::create();
    let destination = directory.vault_path();
    let backup = authenticated_bytes(1, 11);
    fs::write(&destination, b"corrupted ciphertext").unwrap();
    fs::write(AtomicVaultWriter::backup_path(&destination), &backup).unwrap();

    let recovered = AtomicVaultWriter::recover(&destination, authenticates).expect("recovery");

    assert_eq!(recovered.source, RecoverySource::Backup);
    assert_eq!(recovered.bytes, backup);
    assert_eq!(read(&destination), recovered.bytes);
}

#[test]
fn recovery_never_promotes_an_orphan_temp() {
    let directory = TestDirectory::create();
    let destination = directory.vault_path();
    let orphan = directory.path.join(".session.vault-orphan.tmp");
    let candidate = authenticated_bytes(1, 13);
    fs::write(&orphan, &candidate).unwrap();

    let result = AtomicVaultWriter::recover(&destination, authenticates);

    assert!(matches!(
        result,
        Err(AtomicVaultError::NoAuthenticatedVault)
    ));
    assert!(!destination.exists());
    assert_eq!(read(&orphan), candidate);
}

#[test]
fn rejected_candidate_preserves_the_previous_destination() {
    let directory = TestDirectory::create();
    let destination = directory.vault_path();
    let previous = authenticated_bytes(1, 15);
    AtomicVaultWriter::commit(&destination, &previous, authenticates).unwrap();

    let result = AtomicVaultWriter::commit(&destination, b"invalid candidate", authenticates);

    assert!(matches!(
        result,
        Err(AtomicVaultError::AuthenticationFailed)
    ));
    assert_eq!(read(&destination), previous);
}

#[test]
fn replacefile_failure_preserves_the_previous_destination() {
    let directory = TestDirectory::create();
    let destination = directory.vault_path();
    let previous = authenticated_bytes(1, 17);
    let candidate = authenticated_bytes(2, 19);
    AtomicVaultWriter::commit(&destination, &previous, authenticates).unwrap();
    let lock = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&destination)
        .expect("abrir destino sem compartilhamento");

    let result = AtomicVaultWriter::commit(&destination, &candidate, authenticates);

    assert!(matches!(result, Err(AtomicVaultError::CommitFailed)));
    drop(lock);
    assert_eq!(read(&destination), previous);
}

#[test]
fn failed_post_commit_verification_restores_the_authenticated_backup() {
    let directory = TestDirectory::create();
    let destination = directory.vault_path();
    let previous = authenticated_bytes(1, 21);
    let candidate = authenticated_bytes(2, 23);
    AtomicVaultWriter::commit(&destination, &previous, authenticates).unwrap();
    let calls = Cell::new(0usize);

    let result = AtomicVaultWriter::commit(&destination, &candidate, |bytes| {
        let call = calls.get();
        calls.set(call + 1);
        call != 1 && authenticates(bytes)
    });

    assert!(matches!(
        result,
        Err(AtomicVaultError::AuthenticationFailed)
    ));
    assert_eq!(read(&destination), previous);
    assert!(!AtomicVaultWriter::backup_path(&destination).exists());
}
