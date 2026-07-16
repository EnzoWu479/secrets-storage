//! `SessionManager`: estado gerenciado e orquestração dos comandos de sessão.
//!
//! Segue o princípio C-10: a WebView é não confiável e **todo** método revalida
//! estado (app desbloqueado, existência, modo, força) no Rust. Nenhuma chave ou
//! material derivado cruza o IPC — só entram senhas em create/unlock/change, e
//! nenhuma chave sai. O material de chave em memória (GMK e `root_key` por sessão)
//! vive em [`Key32`] zeroizável (best-effort nesta fatia; hardening em PT-04).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use uuid::Uuid;

use crate::crypto::envelope::{self, SessionContent, UnlockAuth, VaultNonces, WrapAuth};
use crate::crypto::kdf::KdfParams;
use crate::crypto::{keyring, Key32};
use crate::session::error::{Result, SessionError};
use crate::session::model::{normalize_name, AuthMode, LockPolicy, Registry, SessionEntry};
use crate::session::storage::Storage;

/// Comprimento mínimo de senha imposto pelo core (a força fina é do frontend).
const MIN_PASSWORD_LEN: usize = 8;
/// Teto do atraso progressivo entre tentativas, em segundos.
const MAX_BACKOFF_SECS: u64 = 30;

/// Estado do gate global (senha mestra global, D-02).
enum AppLock {
    /// GMP não informada nesta execução (ou keyring ausente = 1º uso).
    Locked,
    /// GMP desbloqueada: GMK em memória, abrindo as sessões `global`.
    Unlocked { gmk: Key32 },
}

/// Uma sessão aberta em memória (material zeroizável + relógio de inatividade).
struct UnlockedSession {
    /// Raiz da sessão (a fatia de segredos derivará `content_key` dela).
    #[allow(dead_code)]
    root_key: Key32,
    /// Última interação intencional, para a política de inatividade.
    last_activity: Instant,
    /// Cópia da política vigente (evita reler o registro no timer).
    lock_policy: LockPolicy,
}

/// Chave do mapa de tentativas: gate global ou uma sessão específica.
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
enum AttemptKey {
    Global,
    Session(Uuid),
}

/// Estado do atraso progressivo de uma origem de autenticação.
struct AttemptState {
    failures: u32,
    next_allowed: Instant,
}

/// Estado mutável protegido por um único mutex (simples e livre de deadlock).
struct Inner {
    app: AppLock,
    registry: Registry,
    unlocked: HashMap<Uuid, UnlockedSession>,
    attempts: HashMap<AttemptKey, AttemptState>,
}

/// Gerenciador de sessões (registrado como `tauri::State`).
pub struct SessionManager {
    storage: Storage,
    kdf_params: KdfParams,
    inner: Mutex<Inner>,
}

/// Estado do app para a tela de entrada (funciona bloqueado).
#[derive(Serialize)]
pub struct AppStatus {
    /// `true` enquanto a GMP não foi desbloqueada nesta execução.
    pub app_locked: bool,
    /// `true` se a GMP já existe (desbloqueio) vs. `false` (1º uso).
    pub keyring_exists: bool,
}

/// Metadados de uma sessão para a UI (nunca inclui chave/senha).
#[derive(Serialize)]
pub struct SessionInfo {
    pub id: String,
    pub name: String,
    pub auth_mode: AuthMode,
    pub locked: bool,
    pub inactivity_secs: Option<u64>,
    pub on_windows_lock: bool,
    pub on_windows_suspend: bool,
    pub has_hint: bool,
    pub created_at_unix: u64,
}

impl SessionManager {
    /// Cria o gerenciador enraizado em `root`, carregando o registro do disco.
    ///
    /// `kdf_params` são os parâmetros do Argon2id usados ao **criar** material novo
    /// (candidatos `⚠️ PT-01` em produção; reduzidos nos testes).
    pub fn new(root: PathBuf, kdf_params: KdfParams) -> Self {
        let storage = Storage::new(root);
        let registry = storage
            .read_registry()
            .unwrap_or_else(|_| Registry::empty());
        Self {
            storage,
            kdf_params,
            inner: Mutex::new(Inner {
                app: AppLock::Locked,
                registry,
                unlocked: HashMap::new(),
                attempts: HashMap::new(),
            }),
        }
    }

    // ---------------------------------------------------------------- gate global

    /// Estado do app (funciona bloqueado): 1º uso vs. desbloqueio.
    pub fn app_status(&self) -> AppStatus {
        let inner = self.lock();
        AppStatus {
            app_locked: matches!(inner.app, AppLock::Locked),
            keyring_exists: self.storage.keyring_exists(),
        }
    }

    /// Cria a senha mestra global no 1º uso e já deixa o app desbloqueado.
    pub fn create_global_password(&self, gmp: &[u8]) -> Result<()> {
        check_strength(gmp)?;
        if self.storage.keyring_exists() {
            return Err(SessionError::AlreadyInitialized);
        }
        let salt = rand_array::<16>()?;
        let gmk_rand = rand_array::<32>()?;
        let nonce = rand_array::<24>()?;

        let env = keyring::create_keyring(gmp, salt, self.kdf_params, &gmk_rand, &nonce)?;
        self.storage.write_keyring(&env)?;

        let mut inner = self.lock();
        inner.app = AppLock::Unlocked {
            gmk: Key32::from_bytes(gmk_rand),
        };
        Ok(())
    }

    /// Desbloqueia a GMP e abre **todas** as sessões `global` de uma vez (D-03).
    pub fn unlock_app(&self, gmp: &[u8]) -> Result<()> {
        if !self.storage.keyring_exists() {
            return Err(SessionError::NotInitialized);
        }
        let env = self.storage.read_keyring()?;

        let mut inner = self.lock();
        let now = Instant::now();
        gate(&mut inner.attempts, AttemptKey::Global, now)?;

        let gmk = match keyring::unwrap_gmk(gmp, &env) {
            Ok(gmk) => gmk,
            Err(e) => {
                record_failure(&mut inner.attempts, AttemptKey::Global, now);
                return Err(e.into());
            }
        };
        inner.attempts.remove(&AttemptKey::Global);

        // Abre as sessões `global` (best-effort: uma sessão adulterada fica bloqueada).
        let globals: Vec<SessionEntry> = inner
            .registry
            .sessions
            .iter()
            .filter(|s| s.auth_mode == AuthMode::Global)
            .cloned()
            .collect();
        for entry in globals {
            match self.open_global(&entry, &gmk) {
                Ok(session) => {
                    inner.unlocked.insert(entry.id, session);
                }
                Err(_) => {
                    eprintln!("aviso: sessão global {} não abriu (ignorada)", entry.id);
                }
            }
        }

        inner.app = AppLock::Unlocked { gmk };
        Ok(())
    }

    /// Bloqueia o app: zeroiza a GMK e todas as sessões (fail-closed; também no exit).
    pub fn lock_app(&self) {
        let mut inner = self.lock();
        inner.app = AppLock::Locked;
        inner.unlocked.clear();
    }

    /// Alias de [`Self::lock_app`] para o botão "bloquear tudo" (VAULT-01 AC7).
    pub fn lock_all(&self) {
        self.lock_app();
    }

    /// Troca a GMP reenvolvendo a **mesma** GMK; as sessões não mudam (ROT-01).
    pub fn change_global_password(&self, current: &[u8], new: &[u8]) -> Result<()> {
        check_strength(new)?;
        if !self.storage.keyring_exists() {
            return Err(SessionError::NotInitialized);
        }
        let env = self.storage.read_keyring()?;

        let mut inner = self.lock();
        let now = Instant::now();
        gate(&mut inner.attempts, AttemptKey::Global, now)?;

        let new_salt = rand_array::<16>()?;
        let nonce = rand_array::<24>()?;
        let env2 = match keyring::change_gmp(current, new, new_salt, self.kdf_params, &nonce, &env)
        {
            Ok(env2) => env2,
            Err(e) => {
                record_failure(&mut inner.attempts, AttemptKey::Global, now);
                return Err(e.into());
            }
        };
        inner.attempts.remove(&AttemptKey::Global);
        self.storage.write_keyring(&env2)?;
        Ok(())
    }

    // ------------------------------------------------------------ ciclo de sessão

    /// Lista as sessões conhecidas (funciona bloqueado — AD-013).
    pub fn list_sessions(&self) -> Vec<SessionInfo> {
        let inner = self.lock();
        inner
            .registry
            .sessions
            .iter()
            .map(|e| info(e, !inner.unlocked.contains_key(&e.id)))
            .collect()
    }

    /// Cria uma sessão. `global` exige app desbloqueado; `own` exige senha própria.
    pub fn create_session(
        &self,
        name: &str,
        auth_mode: AuthMode,
        password: Option<&[u8]>,
        hint: Option<String>,
        lock_policy: LockPolicy,
    ) -> Result<SessionInfo> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(SessionError::DuplicateName);
        }
        let normalized = normalize_name(trimmed);

        let id = Uuid::new_v4();
        let uuid_bytes = *id.as_bytes();
        let root_rand = rand_array::<32>()?;
        let nonces = VaultNonces {
            key_wrap: rand_array::<24>()?,
            payload: rand_array::<24>()?,
        };
        let content = SessionContent {
            content_format: 1,
            secrets: vec![],
        };

        let mut inner = self.lock();
        if inner.registry.name_taken(&normalized, None) {
            return Err(SessionError::DuplicateName);
        }

        // Monta o cofre conforme o modo (a `root_key` vem de `root_rand`).
        let env = match auth_mode {
            AuthMode::Global => {
                let gmk = match &inner.app {
                    AppLock::Unlocked { gmk } => gmk,
                    AppLock::Locked => return Err(SessionError::AppLocked),
                };
                envelope::create_vault(
                    WrapAuth::Global { gmk },
                    uuid_bytes,
                    trimmed,
                    &root_rand,
                    0,
                    nonces,
                    &content,
                )?
            }
            AuthMode::Own => {
                let password = password.ok_or(SessionError::WeakPassword)?;
                check_strength(password)?;
                let salt = rand_array::<16>()?;
                envelope::create_vault(
                    WrapAuth::Own {
                        password,
                        salt,
                        params: self.kdf_params,
                    },
                    uuid_bytes,
                    trimmed,
                    &root_rand,
                    0,
                    nonces,
                    &content,
                )?
            }
        };

        self.storage.write_vault(&id, &env)?;
        let entry = SessionEntry {
            id,
            name: trimmed.to_owned(),
            name_normalized: normalized,
            auth_mode,
            hint,
            lock_policy,
            created_at_unix: now_unix(),
        };
        inner.registry.sessions.push(entry.clone());
        self.storage.write_registry(&inner.registry)?;

        // Sessão `global` nasce aberta (o app está desbloqueado); `own` nasce bloqueada.
        let locked = if auth_mode == AuthMode::Global {
            inner.unlocked.insert(
                id,
                UnlockedSession {
                    root_key: Key32::from_bytes(root_rand),
                    last_activity: Instant::now(),
                    lock_policy,
                },
            );
            false
        } else {
            true
        };

        Ok(info(&entry, locked))
    }

    /// Desbloqueia uma sessão `own` (as `global` abrem via [`Self::unlock_app`]).
    pub fn unlock_session(&self, id: &Uuid, password: &[u8]) -> Result<()> {
        let env = self.storage.read_vault(id)?;

        let mut inner = self.lock();
        let entry = inner.registry.find(id).ok_or(SessionError::NotFound)?;
        if entry.auth_mode != AuthMode::Own {
            return Err(SessionError::InvalidAuthMode);
        }
        let policy = entry.lock_policy;
        let expected_name = entry.name.clone();

        let now = Instant::now();
        gate(&mut inner.attempts, AttemptKey::Session(*id), now)?;

        let unlocked = match envelope::unlock(UnlockAuth::Own { password }, &env) {
            Ok(u) => u,
            Err(e) => {
                record_failure(&mut inner.attempts, AttemptKey::Session(*id), now);
                return Err(e.into());
            }
        };
        // Nome autenticado deve bater com o registro (detecta adulteração do registry).
        if unlocked.session_name != expected_name {
            return Err(SessionError::CorruptOrIncompatible);
        }
        inner.attempts.remove(&AttemptKey::Session(*id));
        inner.unlocked.insert(
            *id,
            UnlockedSession {
                root_key: unlocked.root_key,
                last_activity: now,
                lock_policy: policy,
            },
        );
        Ok(())
    }

    /// Bloqueia uma sessão (zeroiza e remove de memória). No-op se já bloqueada.
    pub fn lock_session(&self, id: &Uuid) {
        self.lock().unlocked.remove(id);
    }

    /// Troca a senha própria de uma sessão `own` (rewrap own→own; ROT-01).
    pub fn change_master_password(&self, id: &Uuid, current: &[u8], new: &[u8]) -> Result<()> {
        check_strength(new)?;
        let env = self.storage.read_vault(id)?;

        let mut inner = self.lock();
        let entry = inner.registry.find(id).ok_or(SessionError::NotFound)?;
        if entry.auth_mode != AuthMode::Own {
            return Err(SessionError::InvalidAuthMode);
        }

        let now = Instant::now();
        gate(&mut inner.attempts, AttemptKey::Session(*id), now)?;

        let new_salt = rand_array::<16>()?;
        let nonces = VaultNonces {
            key_wrap: rand_array::<24>()?,
            payload: rand_array::<24>()?,
        };
        let env2 = match envelope::rewrap(
            UnlockAuth::Own { password: current },
            WrapAuth::Own {
                password: new,
                salt: new_salt,
                params: self.kdf_params,
            },
            nonces,
            &env,
        ) {
            Ok(env2) => env2,
            Err(e) => {
                record_failure(&mut inner.attempts, AttemptKey::Session(*id), now);
                return Err(e.into());
            }
        };
        inner.attempts.remove(&AttemptKey::Session(*id));
        self.storage.write_vault(id, &env2)?;
        Ok(())
    }

    /// Alterna o `auth_mode` de uma sessão (rewrap own↔global). Exige app desbloqueado.
    ///
    /// `secret` é a senha própria **atual** (saindo de `own`) ou a **nova** senha
    /// própria (entrando em `own`). O conteúdo não muda.
    pub fn set_session_auth_mode(
        &self,
        id: &Uuid,
        new_mode: AuthMode,
        secret: &[u8],
    ) -> Result<()> {
        let env = self.storage.read_vault(id)?;

        let mut inner = self.lock();
        let current_mode = inner
            .registry
            .find(id)
            .ok_or(SessionError::NotFound)?
            .auth_mode;
        if current_mode == new_mode {
            return Ok(());
        }

        let nonces = VaultNonces {
            key_wrap: rand_array::<24>()?,
            payload: rand_array::<24>()?,
        };
        // A GMK é necessária nas duas direções (desbloquear e/ou reenvolver global).
        let gmk = match &inner.app {
            AppLock::Unlocked { gmk } => Key32::from_bytes(*gmk.as_bytes()),
            AppLock::Locked => return Err(SessionError::AppLocked),
        };

        let env2 = match (current_mode, new_mode) {
            (AuthMode::Own, AuthMode::Global) => {
                let now = Instant::now();
                gate(&mut inner.attempts, AttemptKey::Session(*id), now)?;
                match envelope::rewrap(
                    UnlockAuth::Own { password: secret },
                    WrapAuth::Global { gmk: &gmk },
                    nonces,
                    &env,
                ) {
                    Ok(env2) => {
                        inner.attempts.remove(&AttemptKey::Session(*id));
                        env2
                    }
                    Err(e) => {
                        record_failure(&mut inner.attempts, AttemptKey::Session(*id), now);
                        return Err(e.into());
                    }
                }
            }
            (AuthMode::Global, AuthMode::Own) => {
                check_strength(secret)?;
                let salt = rand_array::<16>()?;
                envelope::rewrap(
                    UnlockAuth::Global { gmk: &gmk },
                    WrapAuth::Own {
                        password: secret,
                        salt,
                        params: self.kdf_params,
                    },
                    nonces,
                    &env,
                )?
            }
            // As combinações iguais já retornaram acima.
            _ => unreachable!("transição de modo idêntica já tratada"),
        };

        self.storage.write_vault(id, &env2)?;
        if let Some(entry) = inner.registry.find_mut(id) {
            entry.auth_mode = new_mode;
        }
        self.storage.write_registry(&inner.registry)?;

        // Ajusta o estado em memória para manter a invariante das sessões global.
        match new_mode {
            AuthMode::Global => {
                if let Ok(unlocked) = envelope::unlock(UnlockAuth::Global { gmk: &gmk }, &env2) {
                    let policy = inner
                        .registry
                        .find(id)
                        .map(|e| e.lock_policy)
                        .unwrap_or_default();
                    inner.unlocked.insert(
                        *id,
                        UnlockedSession {
                            root_key: unlocked.root_key,
                            last_activity: Instant::now(),
                            lock_policy: policy,
                        },
                    );
                }
            }
            // Ao virar `own`, a sessão deixa de abrir junto com a GMP: bloqueia.
            AuthMode::Own => {
                inner.unlocked.remove(id);
            }
        }
        Ok(())
    }

    /// Atualiza a política de bloqueio (o core aceita 60s…∞; "nunca" = `None`).
    pub fn set_lock_policy(&self, id: &Uuid, policy: LockPolicy) -> Result<()> {
        let mut inner = self.lock();
        let entry = inner.registry.find_mut(id).ok_or(SessionError::NotFound)?;
        entry.lock_policy = policy;
        self.storage.write_registry(&inner.registry)?;
        if let Some(session) = inner.unlocked.get_mut(id) {
            session.lock_policy = policy;
        }
        Ok(())
    }

    /// Registra interação intencional, adiando o bloqueio por inatividade. No-op se bloqueada.
    pub fn touch_session(&self, id: &Uuid) -> Result<()> {
        let mut inner = self.lock();
        if let Some(session) = inner.unlocked.get_mut(id) {
            session.last_activity = Instant::now();
        }
        Ok(())
    }

    /// Revela a dica de uma sessão (metadado não secreto — VAULT-04 AC4).
    pub fn reveal_hint(&self, id: &Uuid) -> Result<Option<String>> {
        let inner = self.lock();
        let entry = inner.registry.find(id).ok_or(SessionError::NotFound)?;
        Ok(entry.hint.clone())
    }

    /// Exclui uma sessão após provar a senha da própria sessão (VAULT-01 AC10).
    ///
    /// Em `own`, prova a senha própria; em `global`, prova a GMP.
    pub fn delete_session(&self, id: &Uuid, password: &[u8]) -> Result<()> {
        let env = self.storage.read_vault(id)?;

        let mut inner = self.lock();
        let mode = inner
            .registry
            .find(id)
            .ok_or(SessionError::NotFound)?
            .auth_mode;

        let now = Instant::now();
        gate(&mut inner.attempts, AttemptKey::Session(*id), now)?;

        let proven = match mode {
            AuthMode::Own => envelope::unlock(UnlockAuth::Own { password }, &env).map(|_| ()),
            AuthMode::Global => {
                let keyring_env = self.storage.read_keyring()?;
                keyring::unwrap_gmk(password, &keyring_env).map(|_| ())
            }
        };
        if let Err(e) = proven {
            record_failure(&mut inner.attempts, AttemptKey::Session(*id), now);
            return Err(e.into());
        }
        inner.attempts.remove(&AttemptKey::Session(*id));

        self.storage.delete_vault(id)?;
        inner.unlocked.remove(id);
        inner.registry.sessions.retain(|s| &s.id != id);
        self.storage.write_registry(&inner.registry)?;
        Ok(())
    }

    // ------------------------------------------------------------------- inatividade

    /// Percorrido pelo relógio (~1 s): bloqueia sessões inativas há tempo demais.
    pub fn sweep_locks(&self) {
        self.sweep_locks_at(Instant::now());
    }

    /// Núcleo testável de [`Self::sweep_locks`] com o instante `now` injetado.
    pub fn sweep_locks_at(&self, now: Instant) {
        let mut inner = self.lock();
        inner
            .unlocked
            .retain(|_, s| match s.lock_policy.inactivity_secs {
                Some(secs) => {
                    now.saturating_duration_since(s.last_activity) < Duration::from_secs(secs)
                }
                None => true, // "nunca"
            });
    }

    // ------------------------------------------------------------------- auxiliares

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        // Mutex envenenado só se um titular anterior entrou em pânico; recuperamos
        // o estado assim mesmo (fail-safe para não travar o app inteiro).
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Abre uma sessão `global` a partir da GMK (usado no desbloqueio do app).
    fn open_global(&self, entry: &SessionEntry, gmk: &Key32) -> Result<UnlockedSession> {
        let env = self.storage.read_vault(&entry.id)?;
        let unlocked = envelope::unlock(UnlockAuth::Global { gmk }, &env)?;
        if unlocked.session_name != entry.name {
            return Err(SessionError::CorruptOrIncompatible);
        }
        Ok(UnlockedSession {
            root_key: unlocked.root_key,
            last_activity: Instant::now(),
            lock_policy: entry.lock_policy,
        })
    }
}

/// Monta o DTO de UI a partir de uma entrada e do estado de bloqueio.
fn info(entry: &SessionEntry, locked: bool) -> SessionInfo {
    SessionInfo {
        id: entry.id.to_string(),
        name: entry.name.clone(),
        auth_mode: entry.auth_mode,
        locked,
        inactivity_secs: entry.lock_policy.inactivity_secs,
        on_windows_lock: entry.lock_policy.on_windows_lock,
        on_windows_suspend: entry.lock_policy.on_windows_suspend,
        has_hint: entry.hint.is_some(),
        created_at_unix: entry.created_at_unix,
    }
}

/// Impõe o comprimento mínimo de senha (a força fina fica no frontend).
fn check_strength(password: &[u8]) -> Result<()> {
    if password.len() < MIN_PASSWORD_LEN {
        Err(SessionError::WeakPassword)
    } else {
        Ok(())
    }
}

/// Segundos desde a época Unix (para `created_at`).
fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Gera `N` bytes do CSPRNG do sistema.
fn rand_array<const N: usize>() -> Result<[u8; N]> {
    let mut buf = [0u8; N];
    getrandom::getrandom(&mut buf).map_err(|_| SessionError::Random)?;
    Ok(buf)
}

/// Backoff do atraso progressivo (VAULT-04 AC2): 1,2,4,8,16,30,30… segundos.
fn backoff_secs(failures: u32) -> u64 {
    let exp = failures.saturating_sub(1).min(6);
    (1u64 << exp).min(MAX_BACKOFF_SECS)
}

/// Bloqueia a tentativa se o atraso progressivo ainda estiver ativo.
fn gate(
    attempts: &mut HashMap<AttemptKey, AttemptState>,
    key: AttemptKey,
    now: Instant,
) -> Result<()> {
    if let Some(state) = attempts.get(&key) {
        if now < state.next_allowed {
            let retry_after_secs = state.next_allowed.saturating_duration_since(now).as_secs() + 1;
            return Err(SessionError::TooManyAttempts { retry_after_secs });
        }
    }
    Ok(())
}

/// Contabiliza uma falha de autenticação e agenda a próxima tentativa permitida.
fn record_failure(attempts: &mut HashMap<AttemptKey, AttemptState>, key: AttemptKey, now: Instant) {
    let state = attempts.entry(key).or_insert(AttemptState {
        failures: 0,
        next_allowed: now,
    });
    state.failures += 1;
    let wait = Duration::from_secs(backoff_secs(state.failures));
    state.next_allowed = now.checked_add(wait).unwrap_or(now);
}
