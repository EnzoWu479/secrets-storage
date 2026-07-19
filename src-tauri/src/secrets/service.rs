use uuid::Uuid;

use crate::secrets::codec::{decode_records, encode_records};
use crate::secrets::model::{
    apply_patch, validate_new, CreateSecretInput, SecretDataV1, SecretError, SecretKind,
    SecretPatchInput, SecretRecordV1, SecretText, MAX_RECORDS_PER_SESSION, SECRET_RECORD_VERSION,
};
use crate::secrets::session_access::{SessionAccess, SessionAccessError};

pub trait Clock {
    fn now_ms(&self) -> i64;
}

pub trait RandomSource {
    fn next_uuid(&self) -> Uuid;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SecretMutationResult {
    pub id: Uuid,
    pub revision: u64,
    pub session_revision: u64,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SecretField {
    Username,
    Password,
    Url,
    Notes,
    ApiKey,
    Environment,
    Scopes,
    Token,
    ExpiresAt,
    SecureNote,
    PublicKey,
    PrivateKey,
    Passphrase,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SensitiveFieldDescriptor {
    pub field: SecretField,
    pub present: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicField {
    pub field: SecretField,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecretSummary {
    pub session_id: Uuid,
    pub epoch: u64,
    pub id: Uuid,
    pub kind: SecretKind,
    pub name: String,
    pub subtitle: Option<String>,
    pub revision: u64,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

impl SecretSummary {
    pub fn from_record(session_id: Uuid, epoch: u64, record: &SecretRecordV1) -> Self {
        Self {
            session_id,
            epoch,
            id: record.id,
            kind: record.kind(),
            name: record.name.clone(),
            subtitle: summary_subtitle(&record.data),
            revision: record.revision,
            created_at_ms: record.created_at_ms,
            updated_at_ms: record.updated_at_ms,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecretDetail {
    pub session_id: Uuid,
    pub epoch: u64,
    pub id: Uuid,
    pub kind: SecretKind,
    pub name: String,
    pub revision: u64,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub public_fields: Vec<PublicField>,
    pub sensitive_fields: Vec<SensitiveFieldDescriptor>,
}

impl SecretDetail {
    pub fn from_record(session_id: Uuid, epoch: u64, record: &SecretRecordV1) -> Self {
        Self {
            session_id,
            epoch,
            id: record.id,
            kind: record.kind(),
            name: record.name.clone(),
            revision: record.revision,
            created_at_ms: record.created_at_ms,
            updated_at_ms: record.updated_at_ms,
            public_fields: public_fields(&record.data),
            sensitive_fields: sensitive_fields(&record.data),
        }
    }
}

pub struct SensitiveValue {
    pub field: SecretField,
    pub revision: u64,
    pub value: SecretText,
}

pub struct SecretService<'a, A, C, R> {
    access: &'a A,
    clock: C,
    random: R,
}

impl<'a, A, C, R> SecretService<'a, A, C, R>
where
    A: SessionAccess,
    C: Clock,
    R: RandomSource,
{
    pub fn new(access: &'a A, clock: C, random: R) -> Self {
        Self {
            access,
            clock,
            random,
        }
    }

    pub fn create(
        &self,
        session_id: Uuid,
        input: CreateSecretInput,
    ) -> Result<SecretMutationResult, SessionAccessError> {
        let committed = self.access.write_authorized(session_id, |mut session| {
            let validated = validate_new(input)?;
            let mut records = decode_records(session.content())?;
            if records.len() >= MAX_RECORDS_PER_SESSION {
                return Err(SecretError::CapacityExceeded);
            }

            let id = self.random.next_uuid();
            if records.iter().any(|record| record.id == id) {
                return Err(SecretError::IdCollision);
            }
            let now_ms = self.validated_now()?;
            records.push(SecretRecordV1 {
                version: SECRET_RECORD_VERSION,
                id,
                revision: 0,
                name: validated.name,
                created_at_ms: now_ms,
                updated_at_ms: now_ms,
                move_state: None,
                data: validated.data,
            });
            session.content_mut().secrets = encode_records(&records)?;
            Ok((id, 0))
        })?;

        Ok(SecretMutationResult {
            id: committed.value.0,
            revision: committed.value.1,
            session_revision: committed.revision,
        })
    }

    pub fn detail_for_update(
        &self,
        session_id: Uuid,
        secret_id: Uuid,
    ) -> Result<SecretRecordV1, SessionAccessError> {
        self.access
            .read_authorized(session_id, |session| {
                decode_records(session.content())?
                    .into_iter()
                    .find(|record| record.id == secret_id && record.is_visible())
                    .ok_or(SecretError::NotFound)
            })
            .map(|authorized| authorized.value)
    }

    pub fn reveal(
        &self,
        session_id: Uuid,
        secret_id: Uuid,
        field: SecretField,
        expected_revision: u64,
    ) -> Result<SensitiveValue, SessionAccessError> {
        self.access
            .read_authorized(session_id, |session| {
                let records = decode_records(session.content())?;
                let record = records
                    .iter()
                    .find(|record| record.id == secret_id && record.is_visible())
                    .ok_or(SecretError::NotFound)?;
                if record.revision != expected_revision {
                    return Err(SecretError::RevisionConflict);
                }
                let value = reveal_value(&record.data, field)?;
                Ok(SensitiveValue {
                    field,
                    revision: record.revision,
                    value,
                })
            })
            .map(|authorized| authorized.value)
    }

    pub fn update(
        &self,
        session_id: Uuid,
        secret_id: Uuid,
        expected_revision: u64,
        patch: SecretPatchInput,
    ) -> Result<SecretMutationResult, SessionAccessError> {
        let committed = self.access.write_authorized(session_id, |mut session| {
            let mut records = decode_records(session.content())?;
            let record = records
                .iter_mut()
                .find(|record| record.id == secret_id && record.is_visible())
                .ok_or(SecretError::NotFound)?;
            apply_patch(record, expected_revision, patch)?;
            record.updated_at_ms = self.validated_now()?;
            let revision = record.revision;
            session.content_mut().secrets = encode_records(&records)?;
            Ok(revision)
        })?;

        Ok(SecretMutationResult {
            id: secret_id,
            revision: committed.value,
            session_revision: committed.revision,
        })
    }

    pub fn delete(
        &self,
        session_id: Uuid,
        secret_id: Uuid,
        expected_revision: u64,
    ) -> Result<SecretMutationResult, SessionAccessError> {
        let committed = self.access.write_authorized(session_id, |mut session| {
            let mut records = decode_records(session.content())?;
            let index = records
                .iter()
                .position(|record| record.id == secret_id && record.is_visible())
                .ok_or(SecretError::NotFound)?;
            if records[index].revision != expected_revision {
                return Err(SecretError::RevisionConflict);
            }
            let revision = expected_revision
                .checked_add(1)
                .ok_or(SecretError::InvalidInput)?;
            records.remove(index);
            session.content_mut().secrets = encode_records(&records)?;
            Ok(revision)
        })?;

        Ok(SecretMutationResult {
            id: secret_id,
            revision: committed.value,
            session_revision: committed.revision,
        })
    }

    fn validated_now(&self) -> Result<i64, SecretError> {
        let now_ms = self.clock.now_ms();
        if now_ms < 0 {
            return Err(SecretError::InvalidInput);
        }
        Ok(now_ms)
    }
}

fn summary_subtitle(data: &SecretDataV1) -> Option<String> {
    match data {
        SecretDataV1::Password { username, url, .. } => {
            non_empty(username).or_else(|| url.as_deref().and_then(url_host))
        }
        SecretDataV1::ApiKey { environment, .. } => environment.as_deref().and_then(non_empty),
        SecretDataV1::Token { expires_at, .. } => expires_at.as_deref().and_then(non_empty),
        SecretDataV1::SecureNote { .. } => None,
        SecretDataV1::SshKey { public_key, .. } => public_key
            .as_deref()
            .and_then(|key| key.split_whitespace().nth(2))
            .and_then(non_empty),
    }
}

fn non_empty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

fn url_host(value: &str) -> Option<String> {
    let (_, remainder) = value.split_once("://")?;
    let authority = remainder
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .rsplit('@')
        .next()
        .unwrap_or_default();
    let host = if authority.starts_with('[') {
        authority
            .find(']')
            .map(|end| &authority[..=end])
            .unwrap_or_default()
    } else {
        authority.split(':').next().unwrap_or_default()
    };
    non_empty(host)
}

fn public_fields(data: &SecretDataV1) -> Vec<PublicField> {
    let mut fields = Vec::new();
    match data {
        SecretDataV1::Password { username, url, .. } => {
            fields.push(public_field(SecretField::Username, username));
            if let Some(url) = url {
                fields.push(public_field(SecretField::Url, url));
            }
        }
        SecretDataV1::ApiKey {
            environment,
            scopes,
            ..
        } => {
            if let Some(environment) = environment {
                fields.push(public_field(SecretField::Environment, environment));
            }
            if !scopes.is_empty() {
                fields.push(public_field(SecretField::Scopes, &scopes.join(", ")));
            }
        }
        SecretDataV1::Token { expires_at, .. } => {
            if let Some(expires_at) = expires_at {
                fields.push(public_field(SecretField::ExpiresAt, expires_at));
            }
        }
        SecretDataV1::SecureNote { .. } => {}
        SecretDataV1::SshKey { public_key, .. } => {
            if let Some(public_key) = public_key {
                fields.push(public_field(SecretField::PublicKey, public_key));
            }
        }
    }
    fields
}

fn public_field(field: SecretField, value: &str) -> PublicField {
    PublicField {
        field,
        value: value.to_owned(),
    }
}

fn sensitive_fields(data: &SecretDataV1) -> Vec<SensitiveFieldDescriptor> {
    let descriptor = |field, present| SensitiveFieldDescriptor { field, present };
    match data {
        SecretDataV1::Password {
            password, notes, ..
        } => vec![
            descriptor(SecretField::Password, !password.as_str().is_empty()),
            descriptor(SecretField::Notes, notes.is_some()),
        ],
        SecretDataV1::ApiKey { key, .. } => {
            vec![descriptor(SecretField::ApiKey, !key.as_str().is_empty())]
        }
        SecretDataV1::Token { value, notes, .. } => vec![
            descriptor(SecretField::Token, !value.as_str().is_empty()),
            descriptor(SecretField::Notes, notes.is_some()),
        ],
        SecretDataV1::SecureNote { text } => {
            vec![descriptor(
                SecretField::SecureNote,
                !text.as_str().is_empty(),
            )]
        }
        SecretDataV1::SshKey {
            private_key,
            passphrase,
            ..
        } => vec![
            descriptor(SecretField::PrivateKey, !private_key.as_str().is_empty()),
            descriptor(SecretField::Passphrase, passphrase.is_some()),
        ],
    }
}

fn reveal_value(data: &SecretDataV1, field: SecretField) -> Result<SecretText, SecretError> {
    match (data, field) {
        (SecretDataV1::Password { password, .. }, SecretField::Password) => Ok(password.clone()),
        (SecretDataV1::Password { notes, .. }, SecretField::Notes)
        | (SecretDataV1::Token { notes, .. }, SecretField::Notes) => {
            notes.clone().ok_or(SecretError::NotFound)
        }
        (SecretDataV1::ApiKey { key, .. }, SecretField::ApiKey) => Ok(key.clone()),
        (SecretDataV1::Token { value, .. }, SecretField::Token) => Ok(value.clone()),
        (SecretDataV1::SecureNote { text }, SecretField::SecureNote) => Ok(text.clone()),
        (SecretDataV1::SshKey { private_key, .. }, SecretField::PrivateKey) => {
            Ok(private_key.clone())
        }
        (SecretDataV1::SshKey { passphrase, .. }, SecretField::Passphrase) => {
            passphrase.clone().ok_or(SecretError::NotFound)
        }
        _ => Err(SecretError::InvalidInput),
    }
}

#[cfg(test)]
mod read_projection_tests {
    use std::collections::VecDeque;
    use std::sync::Mutex;

    use super::*;
    use crate::crypto::envelope::SessionContent;
    use crate::secrets::model::{SecretDataInput, SecretKind};
    use crate::secrets::session_access::FakeSessionAccess;

    const NOW_MS: i64 = 1_800_000_000_000;
    const PASSWORD_CANARY: &str = "password-canary-t08";
    const NOTES_CANARY: &str = "notes-canary-t08";
    const API_KEY_CANARY: &str = "api-key-canary-t08";
    const TOKEN_CANARY: &str = "token-canary-t08";
    const NOTE_CANARY: &str = "secure-note-canary-t08";
    const PRIVATE_KEY_CANARY: &str = "PRIVATE-KEY-CANARY-T08";
    const PASSPHRASE_CANARY: &str = "passphrase-canary-t08";

    #[derive(Clone, Copy)]
    struct FixedClock;

    impl Clock for FixedClock {
        fn now_ms(&self) -> i64 {
            NOW_MS
        }
    }

    struct SequenceIds(Mutex<VecDeque<Uuid>>);

    impl SequenceIds {
        fn one(id: Uuid) -> Self {
            Self(Mutex::new(VecDeque::from([id])))
        }
    }

    impl RandomSource for SequenceIds {
        fn next_uuid(&self) -> Uuid {
            self.0
                .lock()
                .expect("fila de UUID não envenenada")
                .pop_front()
                .expect("fixture fornece UUID suficiente")
        }
    }

    fn session_id() -> Uuid {
        Uuid::from_u128(0x8100)
    }

    fn secret_id() -> Uuid {
        Uuid::from_u128(0x8200)
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
        SecretService::new(access, FixedClock, SequenceIds::one(secret_id()))
    }

    fn create_input(data: SecretDataInput) -> CreateSecretInput {
        CreateSecretInput {
            name: "Registro público".into(),
            data,
        }
    }

    fn password_input(notes: Option<&str>) -> CreateSecretInput {
        create_input(SecretDataInput::Password {
            username: "usuario-publico".into(),
            password: PASSWORD_CANARY.into(),
            url: Some("https://public.example".into()),
            notes: notes.map(str::to_owned),
        })
    }

    fn seed(access: &FakeSessionAccess, input: CreateSecretInput) {
        access.install_unlocked(session_id(), empty_content());
        service(access)
            .create(session_id(), input)
            .expect("fixture válida");
    }

    fn record(access: &FakeSessionAccess) -> SecretRecordV1 {
        let content = access.content(session_id()).expect("sessão aberta");
        decode_records(&content)
            .expect("payload válido")
            .into_iter()
            .next()
            .expect("fixture contém um registro")
    }

    fn assert_absent(rendered: &str, canaries: &[&str]) {
        for canary in canaries {
            assert!(
                !rendered.contains(canary),
                "DTO ou erro expôs canário sensível"
            );
        }
    }

    #[test]
    fn summary_debug_omite_todos_os_canarios_sensiveis() {
        let access = FakeSessionAccess::default();
        seed(&access, password_input(Some(NOTES_CANARY)));

        let summary = SecretSummary::from_record(session_id(), 1, &record(&access));
        let rendered = format!("{summary:?}");

        assert_absent(&rendered, &[PASSWORD_CANARY, NOTES_CANARY]);
        assert_eq!(summary.session_id, session_id());
        assert_eq!(summary.id, secret_id());
        assert_eq!(summary.kind, SecretKind::Password);
        assert_eq!(summary.name, "Registro público");
    }

    #[test]
    fn summary_de_password_reduz_url_ao_host() {
        let access = FakeSessionAccess::default();
        seed(
            &access,
            create_input(SecretDataInput::Password {
                username: String::new(),
                password: PASSWORD_CANARY.into(),
                url: Some("https://public.example/private/path?token=query-canary".into()),
                notes: None,
            }),
        );

        let summary = SecretSummary::from_record(session_id(), 1, &record(&access));

        assert_eq!(summary.subtitle.as_deref(), Some("public.example"));
        assert!(!format!("{summary:?}").contains("query-canary"));
    }

    #[test]
    fn detail_debug_omite_todos_os_canarios_sensiveis() {
        let access = FakeSessionAccess::default();
        seed(&access, password_input(Some(NOTES_CANARY)));

        let detail = SecretDetail::from_record(session_id(), 1, &record(&access));
        let rendered = format!("{detail:?}");

        assert_absent(&rendered, &[PASSWORD_CANARY, NOTES_CANARY]);
    }

    #[test]
    fn erros_de_reveal_nao_incluem_canarios() {
        let access = FakeSessionAccess::default();
        seed(&access, password_input(Some(NOTES_CANARY)));

        let result = service(&access).reveal(session_id(), secret_id(), SecretField::Password, 99);
        let error = match result {
            Err(error) => error,
            Ok(_) => panic!("revisão obsoleta deveria ser negada"),
        };
        let rendered = format!("{error:?}");

        assert_absent(&rendered, &[PASSWORD_CANARY, NOTES_CANARY]);
    }

    #[test]
    fn detail_descreve_campos_sensiveis_presentes_e_ausentes() {
        let access = FakeSessionAccess::default();
        seed(&access, password_input(None));

        let detail = SecretDetail::from_record(session_id(), 1, &record(&access));

        assert!(detail.sensitive_fields.contains(&SensitiveFieldDescriptor {
            field: SecretField::Password,
            present: true,
        }));
        assert!(detail.sensitive_fields.contains(&SensitiveFieldDescriptor {
            field: SecretField::Notes,
            present: false,
        }));
    }

    #[test]
    fn detail_contem_somente_campos_publicos_e_descriptors() {
        let access = FakeSessionAccess::default();
        seed(&access, password_input(Some(NOTES_CANARY)));

        let detail = SecretDetail::from_record(session_id(), 1, &record(&access));

        assert!(detail.public_fields.contains(&PublicField {
            field: SecretField::Username,
            value: "usuario-publico".into(),
        }));
        assert!(detail.public_fields.contains(&PublicField {
            field: SecretField::Url,
            value: "https://public.example".into(),
        }));
        let rendered = format!("{detail:?}");
        assert_absent(&rendered, &[PASSWORD_CANARY, NOTES_CANARY]);
    }

    #[test]
    fn reveal_retorna_exatamente_o_password_solicitado() {
        let access = FakeSessionAccess::default();
        seed(&access, password_input(Some(NOTES_CANARY)));

        let revealed = service(&access)
            .reveal(session_id(), secret_id(), SecretField::Password, 0)
            .expect("reveal autorizado");
        let SensitiveValue {
            field,
            revision,
            value,
        } = revealed;

        assert_eq!(field, SecretField::Password);
        assert_eq!(revision, 0);
        assert_eq!(value.as_str(), PASSWORD_CANARY);
        assert_ne!(value.as_str(), NOTES_CANARY);
    }

    #[test]
    fn reveal_retorna_somente_notes_quando_notes_e_solicitado() {
        let access = FakeSessionAccess::default();
        seed(&access, password_input(Some(NOTES_CANARY)));

        let revealed = service(&access)
            .reveal(session_id(), secret_id(), SecretField::Notes, 0)
            .expect("reveal autorizado");

        assert_eq!(revealed.field, SecretField::Notes);
        assert_eq!(revealed.value.as_str(), NOTES_CANARY);
        assert_ne!(revealed.value.as_str(), PASSWORD_CANARY);
    }

    #[test]
    fn reveal_rejeita_campo_publico() {
        let access = FakeSessionAccess::default();
        seed(&access, password_input(Some(NOTES_CANARY)));

        let result = service(&access).reveal(session_id(), secret_id(), SecretField::Username, 0);

        assert!(result.is_err());
    }

    #[test]
    fn reveal_rejeita_campo_sensivel_ausente() {
        let access = FakeSessionAccess::default();
        seed(&access, password_input(None));

        let result = service(&access).reveal(session_id(), secret_id(), SecretField::Notes, 0);

        assert!(result.is_err());
    }

    #[test]
    fn reveal_rejeita_revisao_obsoleta() {
        let access = FakeSessionAccess::default();
        seed(&access, password_input(Some(NOTES_CANARY)));

        let result = service(&access).reveal(session_id(), secret_id(), SecretField::Password, 1);

        assert!(result.is_err());
    }

    #[test]
    fn reveal_rejeita_epoch_invalidada_antes_da_resposta() {
        let access = FakeSessionAccess::default();
        seed(&access, password_input(Some(NOTES_CANARY)));
        access.invalidate_before_next_commit(session_id());

        let result = service(&access).reveal(session_id(), secret_id(), SecretField::Password, 0);

        assert!(result.is_err());
        assert_eq!(access.content(session_id()), None);
    }

    #[test]
    fn reveal_api_key_retorna_somente_a_key() {
        let access = FakeSessionAccess::default();
        seed(
            &access,
            create_input(SecretDataInput::ApiKey {
                key: API_KEY_CANARY.into(),
                environment: Some("test".into()),
                scopes: vec!["read".into()],
            }),
        );

        let revealed = service(&access)
            .reveal(session_id(), secret_id(), SecretField::ApiKey, 0)
            .expect("reveal autorizado");

        assert_eq!(revealed.value.as_str(), API_KEY_CANARY);
    }

    #[test]
    fn reveal_token_retorna_somente_o_value() {
        let access = FakeSessionAccess::default();
        seed(
            &access,
            create_input(SecretDataInput::Token {
                value: TOKEN_CANARY.into(),
                expires_at: None,
                notes: Some(NOTES_CANARY.into()),
            }),
        );

        let revealed = service(&access)
            .reveal(session_id(), secret_id(), SecretField::Token, 0)
            .expect("reveal autorizado");

        assert_eq!(revealed.value.as_str(), TOKEN_CANARY);
        assert_ne!(revealed.value.as_str(), NOTES_CANARY);
    }

    #[test]
    fn reveal_secure_note_retorna_somente_o_text() {
        let access = FakeSessionAccess::default();
        seed(
            &access,
            create_input(SecretDataInput::SecureNote {
                text: NOTE_CANARY.into(),
            }),
        );

        let revealed = service(&access)
            .reveal(session_id(), secret_id(), SecretField::SecureNote, 0)
            .expect("reveal autorizado");

        assert_eq!(revealed.value.as_str(), NOTE_CANARY);
    }

    #[test]
    fn reveal_ssh_private_key_nao_retorna_passphrase() {
        let access = FakeSessionAccess::default();
        seed(
            &access,
            create_input(SecretDataInput::SshKey {
                public_key: Some("ssh-ed25519 AAAATEST test@example".into()),
                private_key: format!(
                    "-----BEGIN OPENSSH PRIVATE KEY-----\n{PRIVATE_KEY_CANARY}\n-----END OPENSSH PRIVATE KEY-----"
                ),
                passphrase: Some(PASSPHRASE_CANARY.into()),
            }),
        );

        let revealed = service(&access)
            .reveal(session_id(), secret_id(), SecretField::PrivateKey, 0)
            .expect("reveal autorizado");

        assert!(revealed.value.as_str().contains(PRIVATE_KEY_CANARY));
        assert!(!revealed.value.as_str().contains(PASSPHRASE_CANARY));
    }
}
