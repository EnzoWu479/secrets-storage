//! Gate global (GMP) sobre `crypto::keyring`.
//!
//! Mantém a GMK desbloqueada em `Key32` (zeroizada no drop). Impõe força mínima
//! de senha e mapeia `CryptoError` para `SessionError` sanitizado, sem revelar
//! se a falha foi senha errada vs. cofre corrompido além do necessário.

use zeroize::Zeroize;

use crate::crypto::kdf::KdfParams;
use crate::crypto::keyring::{self, KeyringEnvelope};
use crate::crypto::{CryptoError, Key32};
use crate::sessions::model::SessionError;

/// Comprimento mínimo (em caracteres) de qualquer senha mestra (GMP ou própria).
pub const MIN_MASTER_PASSWORD_LEN: usize = 8;

/// Fonte de aleatoriedade injetável (CSPRNG em produção, determinística em teste).
pub trait Entropy {
    fn fill(&mut self, buf: &mut [u8]);
}

/// Estado do gate global de desbloqueio.
#[derive(Default)]
pub enum AppLock {
    /// GMP não informada (ou keyring ausente = 1º uso). Nada global acessível.
    #[default]
    Locked,
    /// GMK desbloqueada em memória.
    Unlocked { gmk: Key32 },
}

impl AppLock {
    pub fn is_unlocked(&self) -> bool {
        matches!(self, AppLock::Unlocked { .. })
    }

    /// Referência à GMK, ou `AppLocked` se o gate está fechado.
    pub fn gmk(&self) -> Result<&Key32, SessionError> {
        match self {
            AppLock::Unlocked { gmk } => Ok(gmk),
            AppLock::Locked => Err(SessionError::AppLocked),
        }
    }

    pub fn unlock_with(&mut self, gmk: Key32) {
        *self = AppLock::Unlocked { gmk };
    }

    /// Fecha o gate; a `Key32` anterior é descartada e zeroizada.
    pub fn lock(&mut self) {
        *self = AppLock::Locked;
    }
}

/// Impõe o mínimo de força exigido pelo core (o frontend exibe o indicador).
pub fn validate_password_strength(password: &str) -> Result<(), SessionError> {
    if password.chars().count() < MIN_MASTER_PASSWORD_LEN {
        return Err(SessionError::WeakPassword);
    }
    Ok(())
}

fn map_unlock_error(error: CryptoError) -> SessionError {
    match error {
        CryptoError::Authentication => SessionError::WrongPassword,
        _ => SessionError::IncompatibleVault,
    }
}

fn map_create_error(error: CryptoError) -> SessionError {
    match error {
        CryptoError::InvalidKdfParams => SessionError::InvalidInput,
        _ => SessionError::IncompatibleVault,
    }
}

/// Cria o keyring da GMP no 1º uso e devolve a GMK já desbloqueada.
///
/// `entropy` fornece `salt_global`, a GMK aleatória e o nonce do wrap.
pub fn create_global<E: Entropy>(
    entropy: &mut E,
    password: &str,
    params: KdfParams,
) -> Result<(KeyringEnvelope, Key32), SessionError> {
    validate_password_strength(password)?;

    let mut salt_global = [0u8; 16];
    entropy.fill(&mut salt_global);
    let mut gmk_rand = [0u8; 32];
    entropy.fill(&mut gmk_rand);
    let mut nonce = [0u8; 24];
    entropy.fill(&mut nonce);

    let env = keyring::create_keyring(password.as_bytes(), salt_global, params, &gmk_rand, &nonce)
        .map_err(map_create_error)?;
    gmk_rand.zeroize();

    let gmk = keyring::unwrap_gmk(password.as_bytes(), &env).map_err(map_unlock_error)?;
    Ok((env, gmk))
}

/// Desbloqueia a GMK provando a GMP. GMP errada ⇒ `WrongPassword`.
pub fn unlock_global(password: &str, env: &KeyringEnvelope) -> Result<Key32, SessionError> {
    keyring::unwrap_gmk(password.as_bytes(), env).map_err(map_unlock_error)
}

/// Troca a GMP reenrolando a mesma GMK sob nova gKEK/salt. GMP atual errada ⇒
/// `WrongPassword`; a nova senha precisa atender à força mínima.
pub fn change_global<E: Entropy>(
    entropy: &mut E,
    old_password: &str,
    new_password: &str,
    params: KdfParams,
    env: &KeyringEnvelope,
) -> Result<KeyringEnvelope, SessionError> {
    validate_password_strength(new_password)?;

    let mut new_salt = [0u8; 16];
    entropy.fill(&mut new_salt);
    let mut nonce = [0u8; 24];
    entropy.fill(&mut nonce);

    keyring::change_gmp(
        old_password.as_bytes(),
        new_password.as_bytes(),
        new_salt,
        params,
        &nonce,
        env,
    )
    .map_err(map_unlock_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::kdf::{self, KdfParams};

    const TEST_PARAMS: KdfParams = KdfParams {
        mem_kib: kdf::MIN_MEM_KIB,
        iters: 1,
        parallelism: 1,
    };
    const STRONG: &str = "senha-bem-forte";

    /// Entropia determinística: bytes crescentes a partir de uma semente.
    struct SeqEntropy(u8);
    impl Entropy for SeqEntropy {
        fn fill(&mut self, buf: &mut [u8]) {
            for byte in buf {
                *byte = self.0;
                self.0 = self.0.wrapping_add(1);
            }
        }
    }

    #[test]
    fn create_e_unlock_recuperam_a_mesma_gmk() {
        let mut entropy = SeqEntropy(1);
        let (env, gmk) = create_global(&mut entropy, STRONG, TEST_PARAMS).unwrap();
        let reopened = unlock_global(STRONG, &env).unwrap();
        assert_eq!(gmk.as_bytes(), reopened.as_bytes());
    }

    #[test]
    fn create_rejeita_senha_fraca() {
        let mut entropy = SeqEntropy(1);
        assert!(matches!(
            create_global(&mut entropy, "curta", TEST_PARAMS),
            Err(SessionError::WeakPassword)
        ));
    }

    #[test]
    fn unlock_com_senha_errada_e_wrong_password() {
        let mut entropy = SeqEntropy(1);
        let (env, _gmk) = create_global(&mut entropy, STRONG, TEST_PARAMS).unwrap();
        assert!(matches!(
            unlock_global("outra-senha-forte", &env),
            Err(SessionError::WrongPassword)
        ));
    }

    #[test]
    fn change_troca_a_gmp_preservando_a_gmk() {
        let mut entropy = SeqEntropy(1);
        let (env, gmk) = create_global(&mut entropy, STRONG, TEST_PARAMS).unwrap();
        let env2 =
            change_global(&mut entropy, STRONG, "nova-senha-forte", TEST_PARAMS, &env).unwrap();

        let reopened = unlock_global("nova-senha-forte", &env2).unwrap();
        assert_eq!(gmk.as_bytes(), reopened.as_bytes());
        assert!(matches!(
            unlock_global(STRONG, &env2),
            Err(SessionError::WrongPassword)
        ));
    }

    #[test]
    fn change_com_gmp_antiga_errada_e_wrong_password() {
        let mut entropy = SeqEntropy(1);
        let (env, _gmk) = create_global(&mut entropy, STRONG, TEST_PARAMS).unwrap();
        assert_eq!(
            change_global(
                &mut entropy,
                "errada-forte",
                "nova-senha-forte",
                TEST_PARAMS,
                &env
            )
            .unwrap_err(),
            SessionError::WrongPassword
        );
    }

    #[test]
    fn change_rejeita_nova_senha_fraca() {
        let mut entropy = SeqEntropy(1);
        let (env, _gmk) = create_global(&mut entropy, STRONG, TEST_PARAMS).unwrap();
        assert_eq!(
            change_global(&mut entropy, STRONG, "curta", TEST_PARAMS, &env).unwrap_err(),
            SessionError::WeakPassword
        );
    }

    #[test]
    fn app_lock_gate_transiciona() {
        let mut entropy = SeqEntropy(1);
        let (_env, gmk) = create_global(&mut entropy, STRONG, TEST_PARAMS).unwrap();

        let mut lock = AppLock::default();
        assert!(!lock.is_unlocked());
        assert!(matches!(lock.gmk(), Err(SessionError::AppLocked)));

        lock.unlock_with(gmk);
        assert!(lock.is_unlocked());
        assert!(lock.gmk().is_ok());

        lock.lock();
        assert!(matches!(lock.gmk(), Err(SessionError::AppLocked)));
    }

    #[test]
    fn mapeamento_de_erro_e_fail_closed() {
        assert_eq!(
            map_unlock_error(CryptoError::Authentication),
            SessionError::WrongPassword
        );
        assert_eq!(
            map_unlock_error(CryptoError::UnsupportedVersion),
            SessionError::IncompatibleVault
        );
        assert_eq!(
            map_unlock_error(CryptoError::InvalidMagic),
            SessionError::IncompatibleVault
        );
        assert_eq!(
            map_create_error(CryptoError::InvalidKdfParams),
            SessionError::InvalidInput
        );
    }
}
