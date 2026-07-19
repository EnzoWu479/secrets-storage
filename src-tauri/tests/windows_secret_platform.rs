#![cfg(windows)]

use std::cell::Cell;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::{Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

use secrets_storage_lib::crypto::envelope::{
    create_vault, unlock, SessionContent, UnlockAuth, VaultEnvelope, VaultNonces, WrapAuth,
};
use secrets_storage_lib::crypto::Key32;
use secrets_storage_lib::platform::windows::clipboard::WindowsClipboard;
use secrets_storage_lib::secrets::clipboard::{
    ClipboardClearResult, ClipboardCoordinator, ClipboardPort, ClipboardTimeout,
};
use secrets_storage_lib::secrets::model::{
    validate_new, CreateSecretInput, SecretDataInput, SecretDataV1, SecretText,
};
use secrets_storage_lib::storage::atomic_vault::{
    AtomicVaultError, AtomicVaultWriter, RecoverySource,
};
use uuid::Uuid;
use windows::core::w;
use windows::Win32::Foundation::{GlobalFree, HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, WINDOW_EX_STYLE, WINDOW_STYLE,
};

const CF_UNICODETEXT_FORMAT: u32 = 13;
static CLIPBOARD_TEST_LOCK: Mutex<()> = Mutex::new(());

struct ClipboardFixture {
    _lock: MutexGuard<'static, ()>,
    original_text: Option<String>,
}

impl ClipboardFixture {
    fn acquire() -> Self {
        let lock = CLIPBOARD_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let original_text = read_clipboard_text();
        Self {
            _lock: lock,
            original_text,
        }
    }
}

impl Drop for ClipboardFixture {
    fn drop(&mut self) {
        match self.original_text.as_deref() {
            Some(text) => write_clipboard_text(text),
            None => clear_clipboard_raw(),
        }
    }
}

struct OpenClipboardGuard;

impl OpenClipboardGuard {
    fn open() -> Self {
        Self::open_owned(None)
    }

    fn open_owned(owner: Option<HWND>) -> Self {
        unsafe { OpenClipboard(owner) }.expect("abrir clipboard do teste");
        Self
    }
}

impl Drop for OpenClipboardGuard {
    fn drop(&mut self) {
        let _ = unsafe { CloseClipboard() };
    }
}

fn write_clipboard_text(text: &str) {
    let utf16: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let bytes = utf16.len() * size_of::<u16>();
    let memory = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes) }.expect("alocar clipboard");
    let pointer = unsafe { GlobalLock(memory) }.cast::<u16>();
    assert!(!pointer.is_null(), "travar memória global");
    unsafe { std::ptr::copy_nonoverlapping(utf16.as_ptr(), pointer, utf16.len()) };
    let _ = unsafe { GlobalUnlock(memory) };

    let _clipboard = OpenClipboardGuard::open();
    unsafe { EmptyClipboard() }.expect("esvaziar clipboard");
    let transferred = unsafe {
        SetClipboardData(
            CF_UNICODETEXT_FORMAT,
            Some(windows::Win32::Foundation::HANDLE(memory.0)),
        )
    };
    if transferred.is_err() {
        let _ = unsafe { GlobalFree(Some(memory)) };
    }
    transferred.expect("publicar texto no clipboard");
}

fn read_clipboard_text() -> Option<String> {
    let _clipboard = OpenClipboardGuard::open();
    let handle = unsafe { GetClipboardData(CF_UNICODETEXT_FORMAT) }.ok()?;
    let memory = HGLOBAL(handle.0);
    let pointer = unsafe { GlobalLock(memory) }.cast::<u16>();
    if pointer.is_null() {
        return None;
    }
    let mut length = 0usize;
    while unsafe { *pointer.add(length) } != 0 {
        length += 1;
    }
    let text = String::from_utf16(unsafe { std::slice::from_raw_parts(pointer, length) }).ok();
    let _ = unsafe { GlobalUnlock(memory) };
    text
}

fn clear_clipboard_raw() {
    let _clipboard = OpenClipboardGuard::open();
    unsafe { EmptyClipboard() }.expect("limpar clipboard do teste");
}

fn sensitive_text(value: &str) -> SecretText {
    let record = validate_new(CreateSecretInput {
        name: "Clipboard".into(),
        data: SecretDataInput::SecureNote { text: value.into() },
    })
    .expect("fixture sensível válida");
    match record.data {
        SecretDataV1::SecureNote { text } => text,
        _ => unreachable!(),
    }
}

struct BusyClipboard {
    child: Child,
    _stdout: BufReader<ChildStdout>,
}

impl BusyClipboard {
    fn wait(&mut self) -> std::io::Result<std::process::ExitStatus> {
        self.child.wait()
    }
}

impl Drop for BusyClipboard {
    fn drop(&mut self) {
        match self.child.try_wait() {
            Ok(Some(_)) => {}
            _ => {
                let _ = self.child.kill();
                let _ = self.child.wait();
            }
        }
    }
}

fn hold_clipboard_for(duration: Duration) -> BusyClipboard {
    let mut child = Command::new(std::env::current_exe().expect("test binary"))
        .args([
            "--ignored",
            "--exact",
            "clipboard_lock_holder_helper",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(
            "SECRETS_STORAGE_HOLD_CLIPBOARD_MS",
            duration.as_millis().to_string(),
        )
        .stdout(Stdio::piped())
        .spawn()
        .expect("iniciar processo ocupante");
    let stdout = child.stdout.take().expect("stdout do ocupante");
    let mut busy = BusyClipboard {
        child,
        _stdout: BufReader::new(stdout),
    };
    let mut line = String::new();
    loop {
        line.clear();
        let bytes = busy
            ._stdout
            .read_line(&mut line)
            .expect("ler sinal do ocupante");
        assert_ne!(bytes, 0, "ocupante terminou antes de abrir o clipboard");
        if line.contains("CLIPBOARD_LOCKED") {
            return busy;
        }
    }
}

#[test]
#[ignore = "subprocess helper; invoked explicitly by busy-clipboard tests"]
fn clipboard_lock_holder_helper() {
    let Some(duration_ms) = std::env::var_os("SECRETS_STORAGE_HOLD_CLIPBOARD_MS") else {
        return;
    };
    let duration_ms = duration_ms
        .to_string_lossy()
        .parse::<u64>()
        .expect("duração do helper");
    let window = TestWindow::create();
    let _clipboard = OpenClipboardGuard::open_owned(Some(window.handle));
    println!("CLIPBOARD_LOCKED");
    std::io::stdout().flush().expect("publicar sinal");
    thread::sleep(Duration::from_millis(duration_ms));
}

struct TestWindow {
    handle: HWND,
}

impl TestWindow {
    fn create() -> Self {
        let handle = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("clipboard-test-owner"),
                WINDOW_STYLE::default(),
                0,
                0,
                0,
                0,
                None,
                None,
                None,
                None,
            )
        }
        .expect("criar janela proprietária do clipboard");
        Self { handle }
    }
}

impl Drop for TestWindow {
    fn drop(&mut self) {
        let _ = unsafe { DestroyWindow(self.handle) };
    }
}

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

#[test]
fn clipboard_copy_publishes_unicode_and_returns_the_owned_sequence() {
    let _fixture = ClipboardFixture::acquire();
    let clipboard = WindowsClipboard;

    let sequence = clipboard
        .copy_text(&sensitive_text("segredo ç漢字🔐"))
        .expect("copiar Unicode");

    assert_eq!(read_clipboard_text().as_deref(), Some("segredo ç漢字🔐"));
    assert_eq!(sequence, clipboard.sequence_number().expect("sequence"));
}

#[test]
fn clipboard_sequence_changes_when_later_content_replaces_the_copy() {
    let _fixture = ClipboardFixture::acquire();
    let clipboard = WindowsClipboard;
    let owned_sequence = clipboard
        .copy_text(&sensitive_text("primeiro"))
        .expect("copiar");

    write_clipboard_text("conteúdo posterior");

    assert_ne!(
        clipboard.sequence_number().expect("sequence posterior"),
        owned_sequence
    );
}

#[test]
fn coordinator_never_clears_content_that_replaced_its_owned_copy() {
    let _fixture = ClipboardFixture::acquire();
    let mut coordinator = ClipboardCoordinator::new(WindowsClipboard);
    coordinator
        .copy(
            Uuid::from_u128(1),
            &sensitive_text("segredo original"),
            ClipboardTimeout::ThirtySeconds,
            0,
        )
        .expect("copiar");
    write_clipboard_text("conteúdo posterior");

    assert_eq!(coordinator.clear_now(), ClipboardClearResult::NotOwned);
    assert_eq!(read_clipboard_text().as_deref(), Some("conteúdo posterior"));
}

#[test]
fn clipboard_copy_retries_a_short_lived_busy_clipboard() {
    let _fixture = ClipboardFixture::acquire();
    let mut busy = hold_clipboard_for(Duration::from_millis(25));
    let clipboard = WindowsClipboard;

    let result = clipboard.copy_text(&sensitive_text("após retry"));
    assert!(busy.wait().expect("liberar clipboard").success());

    assert!(result.is_ok());
    assert_eq!(read_clipboard_text().as_deref(), Some("após retry"));
}

#[test]
fn clipboard_clear_empties_the_owned_unicode_value() {
    let _fixture = ClipboardFixture::acquire();
    let clipboard = WindowsClipboard;
    clipboard
        .copy_text(&sensitive_text("limpar"))
        .expect("copiar");

    clipboard.clear().expect("limpar");

    assert_eq!(read_clipboard_text(), None);
}

#[test]
fn coordinator_reports_inconclusive_when_busy_clipboard_cannot_be_cleared() {
    let _fixture = ClipboardFixture::acquire();
    let mut coordinator = ClipboardCoordinator::new(WindowsClipboard);
    coordinator
        .copy(
            Uuid::from_u128(1),
            &sensitive_text("permanece"),
            ClipboardTimeout::ThirtySeconds,
            0,
        )
        .expect("copiar");
    let mut busy = hold_clipboard_for(Duration::from_millis(250));

    let result = coordinator.clear_now();
    assert!(busy.wait().expect("liberar clipboard").success());

    assert_eq!(result, ClipboardClearResult::Inconclusive);
    assert_eq!(read_clipboard_text().as_deref(), Some("permanece"));
}
