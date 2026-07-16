//! Testes de integração do `SessionManager` sobre armazenamento em disco (tempdir).
//!
//! Usam parâmetros de KDF reduzidos (mínimo seguro, 1 iteração) para manter os
//! testes rápidos, exercitando de ponta a ponta o núcleo cripto real + storage.

use std::time::{Duration, Instant};

use uuid::Uuid;

use crate::crypto::kdf::{self, KdfParams};
use crate::session::error::SessionError;
use crate::session::manager::SessionManager;
use crate::session::model::{AuthMode, LockPolicy};

const GMP: &[u8] = b"senha-global-forte";

fn test_params() -> KdfParams {
    KdfParams {
        mem_kib: kdf::MIN_MEM_KIB,
        iters: 1,
        parallelism: 1,
    }
}

/// Diretório temporário exclusivo por teste (limpo no `Drop`).
struct TempRoot(std::path::PathBuf);

impl TempRoot {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("ss-session-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        Self(root)
    }
    fn manager(&self) -> SessionManager {
        SessionManager::new(self.0.clone(), test_params())
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn default_policy() -> LockPolicy {
    LockPolicy {
        inactivity_secs: Some(900),
        on_windows_lock: true,
        on_windows_suspend: true,
    }
}

fn id_of(m: &SessionManager, name: &str) -> Uuid {
    let s = m
        .list_sessions()
        .into_iter()
        .find(|s| s.name == name)
        .expect("sessão existe");
    Uuid::parse_str(&s.id).unwrap()
}

#[test]
fn status_reflete_primeiro_uso_e_desbloqueio() {
    let root = TempRoot::new();
    let m = root.manager();

    let s0 = m.app_status();
    assert!(s0.app_locked);
    assert!(!s0.keyring_exists);

    m.create_global_password(GMP).unwrap();
    let s1 = m.app_status();
    assert!(!s1.app_locked, "criar a GMP já desbloqueia");
    assert!(s1.keyring_exists);
}

#[test]
fn criar_gmp_duas_vezes_falha() {
    let root = TempRoot::new();
    let m = root.manager();
    m.create_global_password(GMP).unwrap();
    assert!(matches!(
        m.create_global_password(GMP),
        Err(SessionError::AlreadyInitialized)
    ));
}

#[test]
fn gmp_curta_e_recusada() {
    let root = TempRoot::new();
    let m = root.manager();
    assert!(matches!(
        m.create_global_password(b"curta"),
        Err(SessionError::WeakPassword)
    ));
}

#[test]
fn desbloqueio_com_gmp_errada_falha_e_ativa_atraso() {
    let root = TempRoot::new();
    let m = root.manager();
    m.create_global_password(GMP).unwrap();
    m.lock_app();

    assert!(matches!(
        m.unlock_app(b"gmp-errada!!"),
        Err(SessionError::Auth)
    ));
    // Segunda tentativa (mesmo correta) é barrada pelo atraso progressivo.
    assert!(matches!(
        m.unlock_app(GMP),
        Err(SessionError::TooManyAttempts { .. })
    ));
}

#[test]
fn sessao_global_abre_junto_com_o_app() {
    let root = TempRoot::new();
    let m = root.manager();
    m.create_global_password(GMP).unwrap();

    let info = m
        .create_session("Trabalho", AuthMode::Global, None, None, default_policy())
        .unwrap();
    assert!(!info.locked, "global nasce aberta com o app desbloqueado");

    m.lock_app();
    let listed = m.list_sessions();
    assert!(
        listed.iter().all(|s| s.locked),
        "lock_app bloqueia as globais"
    );

    m.unlock_app(GMP).unwrap();
    let reaberta = &m.list_sessions()[0];
    assert!(!reaberta.locked, "unlock_app reabre as globais");
}

#[test]
fn sessao_own_exige_senha_e_abre_isolada() {
    let root = TempRoot::new();
    let m = root.manager();
    m.create_global_password(GMP).unwrap();

    // Sem senha → recusada.
    assert!(matches!(
        m.create_session("Cofre", AuthMode::Own, None, None, default_policy()),
        Err(SessionError::WeakPassword)
    ));

    let info = m
        .create_session(
            "Cofre",
            AuthMode::Own,
            Some(b"senha-propria-1"),
            Some("dica".into()),
            default_policy(),
        )
        .unwrap();
    assert!(info.locked, "own nasce bloqueada");
    let id = id_of(&m, "Cofre");

    assert!(matches!(
        m.unlock_session(&id, b"errada-errada"),
        Err(SessionError::Auth)
    ));
    // Nova tentativa é barrada pelo atraso; a prova de senha correta vem depois.
    assert!(matches!(
        m.unlock_session(&id, b"senha-propria-1"),
        Err(SessionError::TooManyAttempts { .. })
    ));
}

#[test]
fn unlock_session_em_global_e_invalido() {
    let root = TempRoot::new();
    let m = root.manager();
    m.create_global_password(GMP).unwrap();
    m.create_session("Trabalho", AuthMode::Global, None, None, default_policy())
        .unwrap();
    let id = id_of(&m, "Trabalho");
    assert!(matches!(
        m.unlock_session(&id, b"qualquer-coisa"),
        Err(SessionError::InvalidAuthMode)
    ));
}

#[test]
fn nome_duplicado_normalizado_e_rejeitado() {
    let root = TempRoot::new();
    let m = root.manager();
    m.create_global_password(GMP).unwrap();
    m.create_session("Trabalho", AuthMode::Global, None, None, default_policy())
        .unwrap();
    assert!(matches!(
        m.create_session(
            "  trabalho ",
            AuthMode::Global,
            None,
            None,
            default_policy()
        ),
        Err(SessionError::DuplicateName)
    ));
}

#[test]
fn troca_de_senha_propria() {
    let root = TempRoot::new();
    let m = root.manager();
    m.create_global_password(GMP).unwrap();
    m.create_session(
        "Cofre",
        AuthMode::Own,
        Some(b"senha-antiga-1"),
        None,
        default_policy(),
    )
    .unwrap();
    let id = id_of(&m, "Cofre");

    m.change_master_password(&id, b"senha-antiga-1", b"senha-nova-22")
        .unwrap();
    // Nova abre; antiga não.
    m.unlock_session(&id, b"senha-nova-22").unwrap();
    m.lock_session(&id);
    assert!(matches!(
        m.unlock_session(&id, b"senha-antiga-1"),
        Err(SessionError::Auth)
    ));
}

#[test]
fn alternar_auth_mode_own_global_own() {
    let root = TempRoot::new();
    let m = root.manager();
    m.create_global_password(GMP).unwrap();
    m.create_session(
        "Projeto",
        AuthMode::Own,
        Some(b"senha-propria-x"),
        None,
        default_policy(),
    )
    .unwrap();
    let id = id_of(&m, "Projeto");

    // own → global (secret = senha própria atual)
    m.set_session_auth_mode(&id, AuthMode::Global, b"senha-propria-x")
        .unwrap();
    let s = &m.list_sessions()[0];
    assert_eq!(s.auth_mode, AuthMode::Global);
    assert!(!s.locked, "virou global e abriu com o app desbloqueado");

    // global → own (secret = nova senha própria)
    m.set_session_auth_mode(&id, AuthMode::Own, b"outra-senha-9")
        .unwrap();
    let s = &m.list_sessions()[0];
    assert_eq!(s.auth_mode, AuthMode::Own);
    assert!(s.locked, "virou own e passou a exigir desbloqueio próprio");
    m.unlock_session(&id, b"outra-senha-9").unwrap();
}

#[test]
fn excluir_sessao_own_exige_senha() {
    let root = TempRoot::new();
    let m = root.manager();
    m.create_global_password(GMP).unwrap();
    // Duas sessões: o atraso progressivo é por sessão, então uma tentativa errada
    // numa não barra a exclusão correta da outra.
    m.create_session(
        "Alvo",
        AuthMode::Own,
        Some(b"senha-propria-d"),
        None,
        default_policy(),
    )
    .unwrap();
    m.create_session(
        "Outra",
        AuthMode::Own,
        Some(b"outra-senha-z"),
        None,
        default_policy(),
    )
    .unwrap();

    let alvo = id_of(&m, "Alvo");
    assert!(matches!(
        m.delete_session(&alvo, b"senha-errada!"),
        Err(SessionError::Auth)
    ));

    let outra = id_of(&m, "Outra");
    m.delete_session(&outra, b"outra-senha-z").unwrap();
    let restantes = m.list_sessions();
    assert_eq!(restantes.len(), 1);
    assert_eq!(restantes[0].name, "Alvo");
}

#[test]
fn excluir_sessao_global_exige_gmp() {
    let root = TempRoot::new();
    let m = root.manager();
    m.create_global_password(GMP).unwrap();
    m.create_session("Trabalho", AuthMode::Global, None, None, default_policy())
        .unwrap();
    m.create_session("Pessoal", AuthMode::Global, None, None, default_policy())
        .unwrap();

    let trabalho = id_of(&m, "Trabalho");
    assert!(matches!(
        m.delete_session(&trabalho, b"gmp-errada!!"),
        Err(SessionError::Auth)
    ));

    let pessoal = id_of(&m, "Pessoal");
    m.delete_session(&pessoal, GMP).unwrap();
    let restantes = m.list_sessions();
    assert_eq!(restantes.len(), 1);
    assert_eq!(restantes[0].name, "Trabalho");
}

#[test]
fn dica_e_politica_persistem_e_sao_lidas() {
    let root = TempRoot::new();
    let m = root.manager();
    m.create_global_password(GMP).unwrap();
    m.create_session(
        "ComDica",
        AuthMode::Own,
        Some(b"senha-propria-c"),
        Some("meu esquema".into()),
        default_policy(),
    )
    .unwrap();
    let id = id_of(&m, "ComDica");

    assert_eq!(m.reveal_hint(&id).unwrap().as_deref(), Some("meu esquema"));

    let nova = LockPolicy {
        inactivity_secs: None,
        on_windows_lock: false,
        on_windows_suspend: false,
    };
    m.set_lock_policy(&id, nova).unwrap();
    let s = &m.list_sessions()[0];
    assert_eq!(s.inactivity_secs, None);
    assert!(!s.on_windows_lock);
}

#[test]
fn estado_persiste_entre_instancias() {
    let root = TempRoot::new();
    {
        let m = root.manager();
        m.create_global_password(GMP).unwrap();
        m.create_session("Trabalho", AuthMode::Global, None, None, default_policy())
            .unwrap();
    }
    // Nova instância sobre a mesma raiz relê o registro do disco.
    let m2 = root.manager();
    let listed = m2.list_sessions();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, "Trabalho");
    assert!(listed[0].locked, "recomeça bloqueada até desbloquear a GMP");

    m2.unlock_app(GMP).unwrap();
    assert!(!m2.list_sessions()[0].locked);
}

#[test]
fn troca_de_gmp_mantem_acesso_as_sessoes() {
    let root = TempRoot::new();
    let m = root.manager();
    m.create_global_password(GMP).unwrap();
    m.create_session("Trabalho", AuthMode::Global, None, None, default_policy())
        .unwrap();

    m.change_global_password(GMP, b"nova-gmp-melhor").unwrap();
    m.lock_app();
    // A GMP nova reabre; a GMK (e portanto as sessões) foi preservada.
    m.unlock_app(b"nova-gmp-melhor").unwrap();
    assert!(!m.list_sessions()[0].locked);
    // A GMP antiga não abre mais (verificado após o sucesso, pois um erro ativa o atraso).
    m.lock_app();
    assert!(matches!(m.unlock_app(GMP), Err(SessionError::Auth)));
}

#[test]
fn varredura_bloqueia_sessao_ociosa() {
    let root = TempRoot::new();
    let m = root.manager();
    m.create_global_password(GMP).unwrap();
    m.create_session(
        "Ativa",
        AuthMode::Own,
        Some(b"senha-propria-a"),
        None,
        LockPolicy {
            inactivity_secs: Some(60),
            on_windows_lock: true,
            on_windows_suspend: true,
        },
    )
    .unwrap();
    let id = id_of(&m, "Ativa");
    m.unlock_session(&id, b"senha-propria-a").unwrap();

    let now = Instant::now();
    // Antes do limite de inatividade: continua aberta.
    m.sweep_locks_at(now);
    assert!(!m.list_sessions()[0].locked);
    // Passado o limite: bloqueia.
    m.sweep_locks_at(now + Duration::from_secs(61));
    assert!(m.list_sessions()[0].locked);
}

#[test]
fn touch_adia_o_bloqueio() {
    let root = TempRoot::new();
    let m = root.manager();
    m.create_global_password(GMP).unwrap();
    m.create_session(
        "Nunca",
        AuthMode::Own,
        Some(b"senha-propria-n"),
        None,
        LockPolicy {
            inactivity_secs: None,
            on_windows_lock: false,
            on_windows_suspend: false,
        },
    )
    .unwrap();
    let id = id_of(&m, "Nunca");
    m.unlock_session(&id, b"senha-propria-n").unwrap();

    // Política "nunca": mesmo bem no futuro, a varredura não bloqueia.
    m.sweep_locks_at(Instant::now() + Duration::from_secs(100_000));
    assert!(!m.list_sessions()[0].locked);
    // touch não deve derrubar a sessão aberta.
    m.touch_session(&id).unwrap();
    assert!(!m.list_sessions()[0].locked);
}
