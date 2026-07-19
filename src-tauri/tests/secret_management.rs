use std::collections::VecDeque;
use std::sync::Mutex;

use secrets_storage_lib::crypto::envelope::SessionContent;
use secrets_storage_lib::secrets::codec::decode_records;
use secrets_storage_lib::secrets::model::{
    CreateSecretInput, SecretDataInput, SecretDataV1, SecretKind, SecretPatchInput,
};
use secrets_storage_lib::secrets::service::{Clock, RandomSource, SecretService};
use secrets_storage_lib::secrets::session_access::FakeSessionAccess;
use uuid::Uuid;

const NOW_MS: i64 = 1_800_000_000_000;

#[derive(Clone, Copy)]
struct FixedClock;

impl Clock for FixedClock {
    fn now_ms(&self) -> i64 {
        NOW_MS
    }
}

struct SequenceIds(Mutex<VecDeque<Uuid>>);

impl SequenceIds {
    fn from(ids: impl IntoIterator<Item = Uuid>) -> Self {
        Self(Mutex::new(ids.into_iter().collect()))
    }
}

impl RandomSource for SequenceIds {
    fn next_uuid(&self) -> Uuid {
        self.0
            .lock()
            .expect("fila de UUIDs não envenenada")
            .pop_front()
            .expect("fixture fornece um UUID para cada criação")
    }
}

fn session_id() -> Uuid {
    Uuid::from_u128(0x100)
}

fn secret_id() -> Uuid {
    Uuid::from_u128(0x200)
}

fn empty_content() -> SessionContent {
    SessionContent {
        content_format: 1,
        secrets: Vec::new(),
    }
}

fn service(
    access: &FakeSessionAccess,
) -> SecretService<'_, FakeSessionAccess, FixedClock, SequenceIds> {
    SecretService::new(access, FixedClock, SequenceIds::from([secret_id()]))
}

fn input(name: &str, data: SecretDataInput) -> CreateSecretInput {
    CreateSecretInput {
        name: name.into(),
        data,
    }
}

fn password(name: &str) -> CreateSecretInput {
    input(
        name,
        SecretDataInput::Password {
            username: "enzo".into(),
            password: "password-canary".into(),
            url: Some("https://example.test".into()),
            notes: Some("password-notes-canary".into()),
        },
    )
}

fn api_key(name: &str) -> CreateSecretInput {
    input(
        name,
        SecretDataInput::ApiKey {
            key: "api-key-canary".into(),
            environment: Some("test".into()),
            scopes: vec!["read".into(), "write".into()],
        },
    )
}

fn token(name: &str) -> CreateSecretInput {
    input(
        name,
        SecretDataInput::Token {
            value: "token-canary".into(),
            expires_at: Some("2027-07-19T12:00:00Z".into()),
            notes: Some("token-notes-canary".into()),
        },
    )
}

fn secure_note(name: &str) -> CreateSecretInput {
    input(
        name,
        SecretDataInput::SecureNote {
            text: "secure-note-canary".into(),
        },
    )
}

fn ssh_key(name: &str) -> CreateSecretInput {
    input(
        name,
        SecretDataInput::SshKey {
            public_key: Some("ssh-ed25519 AAAATEST test@example".into()),
            private_key:
                "-----BEGIN OPENSSH PRIVATE KEY-----\nSSH-CANARY\n-----END OPENSSH PRIVATE KEY-----"
                    .into(),
            passphrase: Some("ssh-passphrase-canary".into()),
        },
    )
}

fn rename(name: &str) -> SecretPatchInput {
    SecretPatchInput {
        name: Some(name.into()),
        data: None,
    }
}

fn records(access: &FakeSessionAccess) -> Vec<secrets_storage_lib::secrets::model::SecretRecordV1> {
    let content = access
        .content(session_id())
        .expect("sessão permanece desbloqueada");
    decode_records(&content).expect("payload confirmado continua válido")
}

fn seed(access: &FakeSessionAccess, create: CreateSecretInput) {
    access.install_unlocked(session_id(), empty_content());
    service(access)
        .create(session_id(), create)
        .expect("fixture inicial é válida");
}

fn assert_crud(create: CreateSecretInput, expected_kind: SecretKind) {
    let access = FakeSessionAccess::default();
    access.install_unlocked(session_id(), empty_content());
    let service = service(&access);

    let created = service
        .create(session_id(), create)
        .expect("criação autorizada");
    assert_eq!(created.id, secret_id());
    assert_eq!(created.revision, 0);
    assert_eq!(created.session_revision, 1);

    let detail = service
        .detail_for_update(session_id(), secret_id())
        .expect("detalhe autorizado");
    assert_eq!(detail.kind(), expected_kind);
    assert_eq!(detail.name, "Original");
    assert_eq!(detail.revision, 0);

    let updated = service
        .update(session_id(), secret_id(), 0, rename("Alterado"))
        .expect("edição autorizada");
    assert_eq!(updated.id, secret_id());
    assert_eq!(updated.revision, 1);
    assert_eq!(updated.session_revision, 2);

    let detail = service
        .detail_for_update(session_id(), secret_id())
        .expect("detalhe atualizado");
    assert_eq!(detail.name, "Alterado");
    assert_eq!(detail.revision, 1);

    let deleted = service
        .delete(session_id(), secret_id(), 1)
        .expect("exclusão autorizada");
    assert_eq!(deleted.id, secret_id());
    assert_eq!(deleted.revision, 2);
    assert_eq!(deleted.session_revision, 3);
    assert!(records(&access).is_empty());
}

#[test]
fn crud_password_confirma_criar_ler_editar_e_excluir() {
    assert_crud(password("Original"), SecretKind::Password);
}

#[test]
fn crud_api_key_confirma_criar_ler_editar_e_excluir() {
    assert_crud(api_key("Original"), SecretKind::ApiKey);
}

#[test]
fn crud_token_confirma_criar_ler_editar_e_excluir() {
    assert_crud(token("Original"), SecretKind::Token);
}

#[test]
fn crud_secure_note_confirma_criar_ler_editar_e_excluir() {
    assert_crud(secure_note("Original"), SecretKind::SecureNote);
}

#[test]
fn crud_ssh_key_confirma_criar_ler_editar_e_excluir() {
    assert_crud(ssh_key("Original"), SecretKind::SshKey);
}

#[test]
fn create_confirma_payload_e_revisao_da_sessao_antes_do_sucesso() {
    let access = FakeSessionAccess::default();
    access.install_unlocked(session_id(), empty_content());

    let result = service(&access)
        .create(session_id(), password("Original"))
        .expect("commit confirmado");

    assert_eq!(
        result.session_revision,
        access.revision(session_id()).unwrap()
    );
    let confirmed = records(&access);
    assert_eq!(confirmed.len(), 1);
    assert_eq!(confirmed[0].id, result.id);
    assert_eq!(confirmed[0].revision, result.revision);
}

#[test]
fn update_confirma_payload_e_revisao_da_sessao_antes_do_sucesso() {
    let access = FakeSessionAccess::default();
    seed(&access, password("Original"));

    let result = service(&access)
        .update(session_id(), secret_id(), 0, rename("Confirmado"))
        .expect("commit confirmado");

    assert_eq!(
        result.session_revision,
        access.revision(session_id()).unwrap()
    );
    let confirmed = records(&access);
    assert_eq!(confirmed[0].name, "Confirmado");
    assert_eq!(confirmed[0].revision, result.revision);
}

#[test]
fn delete_confirma_payload_e_revisao_da_sessao_antes_do_sucesso() {
    let access = FakeSessionAccess::default();
    seed(&access, password("Original"));

    let result = service(&access)
        .delete(session_id(), secret_id(), 0)
        .expect("commit confirmado");

    assert_eq!(
        result.session_revision,
        access.revision(session_id()).unwrap()
    );
    assert!(records(&access).is_empty());
}

#[test]
fn update_com_revisao_obsoleta_preserva_ultima_versao_confirmada() {
    let access = FakeSessionAccess::default();
    seed(&access, password("Original"));
    service(&access)
        .update(session_id(), secret_id(), 0, rename("Mais recente"))
        .expect("primeira edição");
    let before = access.content(session_id()).unwrap();
    let before_revision = access.revision(session_id());

    let result = service(&access).update(session_id(), secret_id(), 0, rename("Obsoleto"));

    assert!(result.is_err());
    assert_eq!(access.content(session_id()).unwrap(), before);
    assert_eq!(access.revision(session_id()), before_revision);
}

#[test]
fn delete_com_revisao_obsoleta_preserva_ultima_versao_confirmada() {
    let access = FakeSessionAccess::default();
    seed(&access, password("Original"));
    service(&access)
        .update(session_id(), secret_id(), 0, rename("Mais recente"))
        .expect("primeira edição");
    let before = access.content(session_id()).unwrap();
    let before_revision = access.revision(session_id());

    let result = service(&access).delete(session_id(), secret_id(), 0);

    assert!(result.is_err());
    assert_eq!(access.content(session_id()).unwrap(), before);
    assert_eq!(access.revision(session_id()), before_revision);
}

#[test]
fn create_em_sessao_bloqueada_e_negado() {
    let access = FakeSessionAccess::default();

    let result = service(&access).create(session_id(), password("Original"));

    assert!(result.is_err());
    assert_eq!(access.content(session_id()), None);
}

#[test]
fn detail_for_update_em_sessao_bloqueada_e_negado() {
    let access = FakeSessionAccess::default();

    let result = service(&access).detail_for_update(session_id(), secret_id());

    assert!(result.is_err());
}

#[test]
fn update_em_sessao_bloqueada_e_negado() {
    let access = FakeSessionAccess::default();

    let result = service(&access).update(session_id(), secret_id(), 0, rename("Negado"));

    assert!(result.is_err());
}

#[test]
fn delete_em_sessao_bloqueada_e_negado() {
    let access = FakeSessionAccess::default();

    let result = service(&access).delete(session_id(), secret_id(), 0);

    assert!(result.is_err());
}

#[test]
fn create_descarta_candidato_quando_epoch_muda_antes_do_commit() {
    let access = FakeSessionAccess::default();
    access.install_unlocked(session_id(), empty_content());
    access.invalidate_before_next_commit(session_id());

    let result = service(&access).create(session_id(), password("Original"));

    assert!(result.is_err());
    assert_eq!(access.content(session_id()), None);
    assert_eq!(access.revision(session_id()), None);
}

#[test]
fn detail_for_update_descarta_resultado_quando_epoch_muda() {
    let access = FakeSessionAccess::default();
    seed(&access, password("Original"));
    access.invalidate_before_next_commit(session_id());

    let result = service(&access).detail_for_update(session_id(), secret_id());

    assert!(result.is_err());
    assert_eq!(access.content(session_id()), None);
}

#[test]
fn update_descarta_candidato_quando_epoch_muda_antes_do_commit() {
    let access = FakeSessionAccess::default();
    seed(&access, password("Original"));
    access.invalidate_before_next_commit(session_id());

    let result = service(&access).update(session_id(), secret_id(), 0, rename("Negado"));

    assert!(result.is_err());
    assert_eq!(access.content(session_id()), None);
    assert_eq!(access.revision(session_id()), None);
}

#[test]
fn delete_descarta_candidato_quando_epoch_muda_antes_do_commit() {
    let access = FakeSessionAccess::default();
    seed(&access, password("Original"));
    access.invalidate_before_next_commit(session_id());

    let result = service(&access).delete(session_id(), secret_id(), 0);

    assert!(result.is_err());
    assert_eq!(access.content(session_id()), None);
    assert_eq!(access.revision(session_id()), None);
}

#[test]
fn falha_antes_do_commit_de_create_preserva_payload_confirmado() {
    let access = FakeSessionAccess::default();
    access.install_unlocked(session_id(), empty_content());
    let before = access.content(session_id()).unwrap();
    access.fail_before_next_commit();

    let result = service(&access).create(session_id(), password("Original"));

    assert!(result.is_err());
    assert_eq!(access.content(session_id()).unwrap(), before);
    assert_eq!(access.revision(session_id()), Some(0));
}

#[test]
fn falha_antes_do_commit_de_update_preserva_payload_confirmado() {
    let access = FakeSessionAccess::default();
    seed(&access, password("Original"));
    let before = access.content(session_id()).unwrap();
    let before_revision = access.revision(session_id());
    access.fail_before_next_commit();

    let result = service(&access).update(session_id(), secret_id(), 0, rename("Não confirmado"));

    assert!(result.is_err());
    assert_eq!(access.content(session_id()).unwrap(), before);
    assert_eq!(access.revision(session_id()), before_revision);
}

#[test]
fn falha_antes_do_commit_de_delete_preserva_payload_confirmado() {
    let access = FakeSessionAccess::default();
    seed(&access, password("Original"));
    let before = access.content(session_id()).unwrap();
    let before_revision = access.revision(session_id());
    access.fail_before_next_commit();

    let result = service(&access).delete(session_id(), secret_id(), 0);

    assert!(result.is_err());
    assert_eq!(access.content(session_id()).unwrap(), before);
    assert_eq!(access.revision(session_id()), before_revision);
}

#[test]
fn create_invalido_nao_avanca_memoria_nem_payload_confirmado() {
    let access = FakeSessionAccess::default();
    access.install_unlocked(session_id(), empty_content());
    let before = access.content(session_id()).unwrap();

    let result = service(&access).create(session_id(), password(""));

    assert!(result.is_err());
    assert_eq!(access.content(session_id()).unwrap(), before);
    assert_eq!(access.revision(session_id()), Some(0));
}

#[test]
fn update_invalido_nao_avanca_memoria_nem_payload_confirmado() {
    let access = FakeSessionAccess::default();
    seed(&access, password("Original"));
    let before = access.content(session_id()).unwrap();
    let before_revision = access.revision(session_id());
    let invalid = SecretPatchInput {
        name: Some(String::new()),
        data: None,
    };

    let result = service(&access).update(session_id(), secret_id(), 0, invalid);

    assert!(result.is_err());
    assert_eq!(access.content(session_id()).unwrap(), before);
    assert_eq!(access.revision(session_id()), before_revision);
}

#[test]
fn detalhe_ausente_nao_avanca_revisao_da_sessao() {
    let access = FakeSessionAccess::default();
    access.install_unlocked(session_id(), empty_content());

    let result = service(&access).detail_for_update(session_id(), secret_id());

    assert!(result.is_err());
    assert_eq!(access.revision(session_id()), Some(0));
}

#[test]
fn update_preserva_tipo_quando_patch_altera_somente_nome() {
    let access = FakeSessionAccess::default();
    seed(&access, ssh_key("Original"));

    service(&access)
        .update(session_id(), secret_id(), 0, rename("Renomeada"))
        .expect("edição autorizada");

    let confirmed = records(&access);
    assert!(matches!(confirmed[0].data, SecretDataV1::SshKey { .. }));
}
