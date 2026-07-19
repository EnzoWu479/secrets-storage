use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use thiserror::Error;
use uuid::Uuid;
use windows::core::PCWSTR;
use windows::Win32::Storage::FileSystem::{
    MoveFileExW, ReplaceFileW, MOVEFILE_WRITE_THROUGH, REPLACEFILE_WRITE_THROUGH,
};

#[derive(Debug, Error)]
pub enum AtomicVaultError {
    #[error("a autenticação do envelope falhou")]
    AuthenticationFailed,
    #[error("o commit atômico do vault falhou")]
    CommitFailed,
    #[error("nenhum vault autenticado está disponível")]
    NoAuthenticatedVault,
    #[error("a operação de armazenamento falhou")]
    Io(#[source] io::Error),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoverySource {
    Destination,
    Backup,
}

#[derive(Debug, PartialEq, Eq)]
pub struct RecoveredVault {
    pub bytes: Vec<u8>,
    pub source: RecoverySource,
}

pub struct AtomicVaultWriter;

impl AtomicVaultWriter {
    pub fn backup_path(destination: &Path) -> PathBuf {
        let mut name = destination
            .file_name()
            .map_or_else(OsString::new, OsString::from);
        name.push(".backup");
        destination.with_file_name(name)
    }

    pub fn commit(
        destination: &Path,
        encrypted_envelope: &[u8],
        mut verifier: impl FnMut(&[u8]) -> bool,
    ) -> Result<(), AtomicVaultError> {
        let backup = Self::backup_path(destination);
        if backup.try_exists().map_err(AtomicVaultError::Io)? {
            return Err(AtomicVaultError::CommitFailed);
        }

        let (mut temp, mut temp_file) = ExclusiveTemp::create(destination)?;
        temp_file
            .write_all(encrypted_envelope)
            .map_err(AtomicVaultError::Io)?;
        temp_file.sync_all().map_err(AtomicVaultError::Io)?;
        drop(temp_file);

        let temp_bytes = fs::read(temp.path()).map_err(AtomicVaultError::Io)?;
        if !verifier(&temp_bytes) {
            return Err(AtomicVaultError::AuthenticationFailed);
        }

        if destination.try_exists().map_err(AtomicVaultError::Io)? {
            if replace_file(destination, temp.path(), Some(&backup)).is_err() {
                return Err(AtomicVaultError::CommitFailed);
            }
            temp.disarm();

            let destination_bytes = fs::read(destination).map_err(AtomicVaultError::Io)?;
            if verifier(&destination_bytes) {
                fs::remove_file(&backup).map_err(AtomicVaultError::Io)?;
                return Ok(());
            }

            Self::restore_backup(destination, &backup, &mut verifier)?;
            return Err(AtomicVaultError::AuthenticationFailed);
        }

        if move_file(temp.path(), destination).is_err() {
            return Err(AtomicVaultError::CommitFailed);
        }
        temp.disarm();

        let destination_bytes = fs::read(destination).map_err(AtomicVaultError::Io)?;
        if verifier(&destination_bytes) {
            Ok(())
        } else {
            let _ = fs::remove_file(destination);
            Err(AtomicVaultError::AuthenticationFailed)
        }
    }

    pub fn recover(
        destination: &Path,
        mut verifier: impl FnMut(&[u8]) -> bool,
    ) -> Result<RecoveredVault, AtomicVaultError> {
        let backup = Self::backup_path(destination);

        if let Ok(bytes) = fs::read(destination) {
            if verifier(&bytes) {
                if backup.try_exists().map_err(AtomicVaultError::Io)? {
                    fs::remove_file(&backup).map_err(AtomicVaultError::Io)?;
                }
                return Ok(RecoveredVault {
                    bytes,
                    source: RecoverySource::Destination,
                });
            }
        }

        let backup_bytes = match fs::read(&backup) {
            Ok(bytes) if verifier(&bytes) => bytes,
            _ => return Err(AtomicVaultError::NoAuthenticatedVault),
        };

        let destination_exists = destination.try_exists().map_err(AtomicVaultError::Io)?;
        let promotion = if destination_exists {
            replace_file(destination, &backup, None)
        } else {
            move_file(&backup, destination)
        };
        if promotion.is_err() {
            return Err(AtomicVaultError::CommitFailed);
        }

        let bytes = fs::read(destination).map_err(AtomicVaultError::Io)?;
        if !verifier(&bytes) {
            return Err(AtomicVaultError::NoAuthenticatedVault);
        }

        debug_assert_eq!(bytes, backup_bytes);
        Ok(RecoveredVault {
            bytes,
            source: RecoverySource::Backup,
        })
    }

    fn restore_backup(
        destination: &Path,
        backup: &Path,
        verifier: &mut impl FnMut(&[u8]) -> bool,
    ) -> Result<(), AtomicVaultError> {
        let backup_bytes = fs::read(backup).map_err(AtomicVaultError::Io)?;
        if !verifier(&backup_bytes) {
            return Err(AtomicVaultError::NoAuthenticatedVault);
        }
        replace_file(destination, backup, None).map_err(|_| AtomicVaultError::CommitFailed)?;
        let restored = fs::read(destination).map_err(AtomicVaultError::Io)?;
        if verifier(&restored) {
            Ok(())
        } else {
            Err(AtomicVaultError::NoAuthenticatedVault)
        }
    }
}

struct ExclusiveTemp {
    path: PathBuf,
    armed: bool,
}

impl ExclusiveTemp {
    fn create(destination: &Path) -> Result<(Self, File), AtomicVaultError> {
        let parent = destination.parent().unwrap_or_else(|| Path::new("."));
        let file_name = destination
            .file_name()
            .ok_or(AtomicVaultError::CommitFailed)?;

        for _ in 0..32 {
            let mut temp_name = OsString::from(".");
            temp_name.push(file_name);
            temp_name.push(".");
            temp_name.push(Uuid::new_v4().to_string());
            temp_name.push(".tmp");
            let path = parent.join(temp_name);
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => return Ok((Self { path, armed: true }, file)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(AtomicVaultError::Io(error)),
            }
        }
        Err(AtomicVaultError::CommitFailed)
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for ExclusiveTemp {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn replace_file(
    destination: &Path,
    replacement: &Path,
    backup: Option<&Path>,
) -> windows::core::Result<()> {
    let destination = wide_path(destination);
    let replacement = wide_path(replacement);
    let backup = backup.map(wide_path);
    let backup_ptr = backup
        .as_ref()
        .map_or(PCWSTR::null(), |value| PCWSTR(value.as_ptr()));

    unsafe {
        ReplaceFileW(
            PCWSTR(destination.as_ptr()),
            PCWSTR(replacement.as_ptr()),
            backup_ptr,
            REPLACEFILE_WRITE_THROUGH,
            None,
            None,
        )
    }
}

fn move_file(source: &Path, destination: &Path) -> windows::core::Result<()> {
    let source = wide_path(source);
    let destination = wide_path(destination);
    unsafe {
        MoveFileExW(
            PCWSTR(source.as_ptr()),
            PCWSTR(destination.as_ptr()),
            MOVEFILE_WRITE_THROUGH,
        )
    }
}

fn wide_path(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}
