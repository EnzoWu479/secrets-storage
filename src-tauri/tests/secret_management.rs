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

mod move_integration_tests {
    use super::*;
    use secrets_storage_lib::secrets::codec::encode_records;
    use secrets_storage_lib::secrets::model::SecretRecordV1;
    use secrets_storage_lib::secrets::move_state::{begin_move, staged_copy};
    use secrets_storage_lib::secrets::service::MoveCompletion;

    fn source_id() -> Uuid {
        session_id()
    }

    fn target_id() -> Uuid {
        Uuid::from_u128(0x300)
    }

    fn move_id() -> Uuid {
        Uuid::from_u128(0x400)
    }

    fn move_service(
        access: &FakeSessionAccess,
    ) -> SecretService<'_, FakeSessionAccess, FixedClock, SequenceIds> {
        SecretService::new(access, FixedClock, SequenceIds::from([move_id()]))
    }

    fn records_for(access: &FakeSessionAccess, session: Uuid) -> Vec<SecretRecordV1> {
        let content = access.content(session).expect("sessão desbloqueada");
        decode_records(&content).expect("payload válido")
    }

    fn install_records(access: &FakeSessionAccess, session: Uuid, records: &[SecretRecordV1]) {
        access.install_unlocked(
            session,
            SessionContent {
                content_format: 1,
                secrets: encode_records(records).expect("records de fixture válidos"),
            },
        );
    }

    fn seed_source(access: &FakeSessionAccess, create: CreateSecretInput) {
        access.install_unlocked(source_id(), empty_content());
        access.install_unlocked(target_id(), empty_content());
        service(access)
            .create(source_id(), create)
            .expect("segredo de origem criado");
    }

    fn visible_count(access: &FakeSessionAccess) -> usize {
        [source_id(), target_id()]
            .into_iter()
            .filter_map(|session| access.content(session))
            .flat_map(|content| decode_records(&content).expect("payload válido"))
            .filter(SecretRecordV1::is_visible)
            .count()
    }

    fn assert_committed_in_target(access: &FakeSessionAccess, kind: SecretKind) {
        assert!(records_for(access, source_id()).is_empty());
        let target = records_for(access, target_id());
        assert_eq!(target.len(), 1);
        assert_eq!(target[0].id, secret_id());
        assert_eq!(target[0].kind(), kind);
        assert_eq!(target[0].revision, 1);
        assert!(target[0].move_state.is_none());
        assert_eq!(visible_count(access), 1);
    }

    fn assert_move(create: CreateSecretInput, kind: SecretKind) {
        let access = FakeSessionAccess::default();
        seed_source(&access, create);

        let result = move_service(&access)
            .move_secret(source_id(), target_id(), secret_id(), 0)
            .expect("movimentação confirmada");

        assert_eq!(result.id, secret_id());
        assert_eq!(result.revision, 1);
        assert_eq!(result.state, MoveCompletion::Committed);
        assert_committed_in_target(&access, kind);
    }

    fn pending_record(access: &FakeSessionAccess) -> SecretRecordV1 {
        let mut source = records_for(access, source_id())
            .into_iter()
            .next()
            .expect("origem contém segredo");
        let revision = source.revision;
        begin_move(&mut source, source_id(), target_id(), move_id(), revision)
            .expect("marker pending válido");
        source
    }

    fn recover_and_assert_single_visible(access: &FakeSessionAccess) {
        move_service(access)
            .recover_moves(source_id(), target_id())
            .expect("recovery determinístico");
        assert_eq!(visible_count(access), 1);
        let all = [source_id(), target_id()]
            .into_iter()
            .flat_map(|session| records_for(access, session))
            .collect::<Vec<_>>();
        assert_eq!(all.iter().filter(|record| record.is_visible()).count(), 1);
        assert_eq!(
            all.iter()
                .filter(|record| record.move_state.is_none())
                .count(),
            1
        );
    }

    #[test]
    fn move_password_preserva_tipo_e_confirma_destino() {
        assert_move(password("Mover password"), SecretKind::Password);
    }

    #[test]
    fn move_api_key_preserva_tipo_e_confirma_destino() {
        assert_move(api_key("Mover api key"), SecretKind::ApiKey);
    }

    #[test]
    fn move_token_preserva_tipo_e_confirma_destino() {
        assert_move(token("Mover token"), SecretKind::Token);
    }

    #[test]
    fn move_secure_note_preserva_tipo_e_confirma_destino() {
        assert_move(secure_note("Mover nota"), SecretKind::SecureNote);
    }

    #[test]
    fn move_ssh_key_preserva_tipo_e_confirma_destino() {
        assert_move(ssh_key("Mover ssh"), SecretKind::SshKey);
    }

    #[test]
    fn move_rejeita_source_igual_target_sem_alterar_estado() {
        let access = FakeSessionAccess::default();
        seed_source(&access, password("Original"));
        let before = access.content(source_id()).unwrap();
        let before_revision = access.revision(source_id());

        let result = move_service(&access).move_secret(source_id(), source_id(), secret_id(), 0);

        assert!(result.is_err());
        assert_eq!(access.content(source_id()).unwrap(), before);
        assert_eq!(access.revision(source_id()), before_revision);
    }

    #[test]
    fn move_rejeita_revisao_obsoleta_sem_criar_copia() {
        let access = FakeSessionAccess::default();
        seed_source(&access, password("Original"));

        let result = move_service(&access).move_secret(source_id(), target_id(), secret_id(), 9);

        assert!(result.is_err());
        assert_eq!(visible_count(&access), 1);
        assert!(records_for(&access, target_id()).is_empty());
        assert!(records_for(&access, source_id())[0].move_state.is_none());
    }

    #[test]
    fn move_rejeita_source_bloqueada() {
        let access = FakeSessionAccess::default();
        seed_source(&access, password("Original"));
        access.lock(source_id());

        let result = move_service(&access).move_secret(source_id(), target_id(), secret_id(), 0);

        assert!(result.is_err());
        assert!(records_for(&access, target_id()).is_empty());
    }

    #[test]
    fn move_rejeita_target_bloqueada_e_preserva_source() {
        let access = FakeSessionAccess::default();
        seed_source(&access, password("Original"));
        access.lock(target_id());

        let result = move_service(&access).move_secret(source_id(), target_id(), secret_id(), 0);

        assert!(result.is_err());
        assert_eq!(records_for(&access, source_id()).len(), 1);
    }

    #[test]
    fn move_descarta_operacao_quando_epoch_muda() {
        let access = FakeSessionAccess::default();
        seed_source(&access, password("Original"));
        access.invalidate_before_next_commit(target_id());

        let result = move_service(&access).move_secret(source_id(), target_id(), secret_id(), 0);

        assert!(result.is_err());
        assert!(records_for(&access, source_id())
            .iter()
            .any(SecretRecordV1::is_visible));
    }

    #[test]
    fn move_adquire_sessoes_em_ordem_crescente_de_uuid() {
        let access = FakeSessionAccess::default();
        let low = Uuid::from_u128(1);
        let high = Uuid::from_u128(2);
        access.install_unlocked(high, empty_content());
        access.install_unlocked(low, empty_content());
        let service = SecretService::new(
            &access,
            FixedClock,
            SequenceIds::from([secret_id(), move_id()]),
        );
        service
            .create(high, password("Original"))
            .expect("segredo criado");

        service
            .move_secret(high, low, secret_id(), 0)
            .expect("movimentação confirmada");

        assert_eq!(access.last_lock_order(), vec![low, high]);
    }

    #[test]
    fn sucesso_so_retorna_depois_do_quarto_commit_committed() {
        let access = FakeSessionAccess::default();
        seed_source(&access, password("Original"));

        let result = move_service(&access)
            .move_secret(source_id(), target_id(), secret_id(), 0)
            .expect("movimentação confirmada");

        assert_eq!(result.state, MoveCompletion::Committed);
        assert_committed_in_target(&access, SecretKind::Password);
    }

    #[test]
    fn falha_antes_da_fronteira_1_preserva_origem_e_recovery_e_idempotente() {
        assert_failure_boundary_recovers(1);
    }

    #[test]
    fn falha_antes_da_fronteira_2_preserva_pending_e_recovery_e_idempotente() {
        assert_failure_boundary_recovers(2);
    }

    #[test]
    fn falha_antes_da_fronteira_3_preserva_pending_staged_e_recovery_e_idempotente() {
        assert_failure_boundary_recovers(3);
    }

    #[test]
    fn falha_antes_da_fronteira_4_preserva_staged_e_recovery_e_idempotente() {
        assert_failure_boundary_recovers(4);
    }

    fn assert_failure_boundary_recovers(boundary: usize) {
        let access = FakeSessionAccess::default();
        seed_source(&access, password("Original"));
        access.fail_before_commit_number(boundary);

        let result = move_service(&access).move_secret(source_id(), target_id(), secret_id(), 0);

        assert!(result.is_err());
        move_service(&access)
            .recover_moves(source_id(), target_id())
            .expect("primeiro recovery");
        let after_first = (
            access.content(source_id()).unwrap(),
            access.content(target_id()).unwrap(),
        );
        move_service(&access)
            .recover_moves(source_id(), target_id())
            .expect("segundo recovery idempotente");
        assert_eq!(access.content(source_id()).unwrap(), after_first.0);
        assert_eq!(access.content(target_id()).unwrap(), after_first.1);
        assert_eq!(visible_count(&access), 1);
    }

    #[test]
    fn restart_recovery_pending_only_reverte_para_source_visivel() {
        let access = FakeSessionAccess::default();
        seed_source(&access, password("Original"));
        let pending = pending_record(&access);
        install_records(&access, source_id(), &[pending]);

        recover_and_assert_single_visible(&access);

        assert_eq!(records_for(&access, source_id()).len(), 1);
        assert!(records_for(&access, target_id()).is_empty());
    }

    #[test]
    fn restart_recovery_pending_e_staged_completa_no_target() {
        let access = FakeSessionAccess::default();
        seed_source(&access, password("Original"));
        let pending = pending_record(&access);
        let staged = staged_copy(&pending, source_id()).expect("staged válido");
        install_records(&access, source_id(), &[pending]);
        install_records(&access, target_id(), &[staged]);

        recover_and_assert_single_visible(&access);

        assert!(records_for(&access, source_id()).is_empty());
        assert_committed_in_target(&access, SecretKind::Password);
    }

    #[test]
    fn restart_recovery_staged_only_confirma_target() {
        let access = FakeSessionAccess::default();
        seed_source(&access, password("Original"));
        let pending = pending_record(&access);
        let staged = staged_copy(&pending, source_id()).expect("staged válido");
        install_records(&access, source_id(), &[]);
        install_records(&access, target_id(), &[staged]);

        recover_and_assert_single_visible(&access);

        assert!(records_for(&access, source_id()).is_empty());
        assert_committed_in_target(&access, SecretKind::Password);
    }

    #[test]
    fn recovery_nao_promove_staged_se_origem_normal_ainda_existe() {
        let access = FakeSessionAccess::default();
        seed_source(&access, password("Original"));
        let original = records_for(&access, source_id())
            .into_iter()
            .next()
            .expect("origem");
        let mut pending = original.clone();
        begin_move(
            &mut pending,
            source_id(),
            target_id(),
            move_id(),
            original.revision,
        )
        .expect("pending");
        let staged = staged_copy(&pending, source_id()).expect("staged");
        install_records(&access, source_id(), &[original]);
        install_records(&access, target_id(), &[staged]);

        let result = move_service(&access).recover_moves(source_id(), target_id());

        assert!(result.is_err());
        assert_eq!(visible_count(&access), 1);
        assert!(records_for(&access, target_id())[0].move_state.is_some());
    }
}
