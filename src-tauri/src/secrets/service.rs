use std::fmt::Write as _;

use sha2::{Digest, Sha256};
use thiserror::Error;
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

#[derive(Debug, Error)]
pub enum SecretServiceError {
    #[error("operação de sessão negada")]
    Access(#[source] SessionAccessError),
    #[error("entrada da operação de segredos é inválida")]
    InvalidInput,
    #[error("cursor de segredos está obsoleto")]
    StaleCursor,
}

impl From<SessionAccessError> for SecretServiceError {
    fn from(value: SessionAccessError) -> Self {
        Self::Access(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecretPage {
    pub items: Vec<SecretSummary>,
    pub next_cursor: Option<String>,
    pub total: usize,
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

    pub fn list(
        &self,
        session_id: Uuid,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<SecretPage, SecretServiceError> {
        validate_page_limit(limit)?;
        let authorized = self.access.read_authorized(session_id, |session| {
            let mut items = decode_records(session.content())?
                .into_iter()
                .filter(SecretRecordV1::is_visible)
                .map(|record| SecretSummary::from_record(session_id, session.epoch(), &record))
                .collect::<Vec<_>>();
            sort_summaries(&mut items);
            Ok(items)
        })?;
        let version = cursor_version(&[(session_id, authorized.epoch, authorized.revision)]);
        paginate(authorized.value, cursor, limit, version)
    }

    pub fn search(
        &self,
        query: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<SecretPage, SecretServiceError> {
        validate_page_limit(limit)?;
        if query.len() > crate::secrets::model::MAX_SEARCH_QUERY_BYTES || query.contains('\0') {
            return Err(SecretServiceError::InvalidInput);
        }
        let normalized_query = normalize_search(query.trim());
        let authorized = self.access.read_all_authorized(|session_id, session| {
            Ok(decode_records(session.content())?
                .into_iter()
                .filter(SecretRecordV1::is_visible)
                .filter(|record| record_matches(record, &normalized_query))
                .map(|record| SecretSummary::from_record(session_id, session.epoch(), &record))
                .collect::<Vec<_>>())
        })?;

        let stamps = authorized
            .iter()
            .map(|result| (result.session_id, result.epoch, result.revision))
            .collect::<Vec<_>>();
        let mut items = authorized
            .into_iter()
            .flat_map(|result| result.value)
            .collect::<Vec<_>>();
        sort_summaries(&mut items);
        paginate(items, cursor, limit, cursor_version(&stamps))
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

fn validate_page_limit(limit: usize) -> Result<(), SecretServiceError> {
    if !(1..=crate::secrets::model::MAX_PAGE_SIZE).contains(&limit) {
        return Err(SecretServiceError::InvalidInput);
    }
    Ok(())
}

fn sort_summaries(items: &mut [SecretSummary]) {
    items.sort_by(|left, right| {
        normalize_search(&left.name)
            .cmp(&normalize_search(&right.name))
            .then_with(|| left.session_id.cmp(&right.session_id))
            .then_with(|| left.id.cmp(&right.id))
    });
}

fn paginate(
    items: Vec<SecretSummary>,
    cursor: Option<&str>,
    limit: usize,
    version: u64,
) -> Result<SecretPage, SecretServiceError> {
    let offset = match cursor {
        Some(cursor) => parse_cursor(cursor, version)?,
        None => 0,
    };
    if offset > items.len() {
        return Err(SecretServiceError::StaleCursor);
    }
    let total = items.len();
    let end = offset.saturating_add(limit).min(total);
    let next_cursor = (end < total).then(|| encode_cursor(version, end));
    Ok(SecretPage {
        items: items.into_iter().skip(offset).take(limit).collect(),
        next_cursor,
        total,
    })
}

fn encode_cursor(version: u64, offset: usize) -> String {
    format!("v1-{version:016x}-{offset:016x}")
}

fn parse_cursor(cursor: &str, expected_version: u64) -> Result<usize, SecretServiceError> {
    let mut parts = cursor.split('-');
    if parts.next() != Some("v1") {
        return Err(SecretServiceError::StaleCursor);
    }
    let version = parts
        .next()
        .and_then(|part| u64::from_str_radix(part, 16).ok())
        .ok_or(SecretServiceError::StaleCursor)?;
    let offset = parts
        .next()
        .and_then(|part| u64::from_str_radix(part, 16).ok())
        .and_then(|value| usize::try_from(value).ok())
        .ok_or(SecretServiceError::StaleCursor)?;
    if parts.next().is_some() || version != expected_version {
        return Err(SecretServiceError::StaleCursor);
    }
    Ok(offset)
}

fn cursor_version(stamps: &[(Uuid, u64, u64)]) -> u64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;

    stamps
        .iter()
        .flat_map(|(id, epoch, revision)| {
            id.as_bytes()
                .iter()
                .copied()
                .chain(epoch.to_le_bytes())
                .chain(revision.to_le_bytes())
        })
        .fold(OFFSET, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(PRIME)
        })
}

fn record_matches(record: &SecretRecordV1, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    if normalized_contains(&record.name, query)
        || normalized_contains(kind_name(record.kind()), query)
    {
        return true;
    }

    match &record.data {
        SecretDataV1::Password { username, url, .. } => {
            normalized_contains(username, query)
                || url
                    .as_deref()
                    .and_then(url_host)
                    .is_some_and(|host| normalized_contains(&host, query))
        }
        SecretDataV1::ApiKey {
            environment,
            scopes,
            ..
        } => {
            environment
                .as_deref()
                .is_some_and(|value| normalized_contains(value, query))
                || scopes.iter().any(|scope| normalized_contains(scope, query))
        }
        SecretDataV1::Token { expires_at, .. } => expires_at
            .as_deref()
            .is_some_and(|value| normalized_contains(value, query)),
        SecretDataV1::SecureNote { .. } => false,
        SecretDataV1::SshKey { public_key, .. } => public_key.as_deref().is_some_and(|key| {
            key.split_whitespace()
                .nth(2)
                .is_some_and(|comment| normalized_contains(comment, query))
                || ssh_public_fingerprint(key)
                    .is_some_and(|fingerprint| normalized_contains(&fingerprint, query))
        }),
    }
}

fn kind_name(kind: SecretKind) -> &'static str {
    match kind {
        SecretKind::Password => "password",
        SecretKind::ApiKey => "api-key api key",
        SecretKind::Token => "token",
        SecretKind::SecureNote => "secure-note secure note",
        SecretKind::SshKey => "ssh-key ssh key",
    }
}

fn normalized_contains(value: &str, normalized_query: &str) -> bool {
    normalize_search(value).contains(normalized_query)
}

fn normalize_search(value: &str) -> String {
    value
        .chars()
        .flat_map(char::to_lowercase)
        .map(|character| match character {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            other => other,
        })
        .collect()
}

fn ssh_public_fingerprint(public_key: &str) -> Option<String> {
    let mut fields = public_key.split_whitespace();
    let kind = fields.next()?;
    let encoded_key = fields.next()?;
    let mut digest = Sha256::new();
    digest.update(kind.as_bytes());
    digest.update([0]);
    digest.update(encoded_key.as_bytes());
    let mut fingerprint = String::with_capacity(7 + 64);
    fingerprint.push_str("SHA256:");
    for byte in digest.finalize() {
        write!(&mut fingerprint, "{byte:02x}").expect("escrever em String");
    }
    Some(fingerprint)
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

#[cfg(test)]
mod search_and_pagination_tests {
    use std::collections::VecDeque;
    use std::sync::Mutex;

    use super::*;
    use crate::crypto::envelope::SessionContent;
    use crate::secrets::model::{SecretDataInput, MAX_SEARCH_QUERY_BYTES};
    use crate::secrets::move_state::MoveState;
    use crate::secrets::session_access::FakeSessionAccess;

    const PASSWORD_CANARY: &str = "password-value-must-never-match";
    const NOTES_CANARY: &str = "notes-value-must-never-match";
    const API_KEY_CANARY: &str = "api-key-value-must-never-match";
    const TOKEN_CANARY: &str = "token-value-must-never-match";
    const NOTE_CANARY: &str = "secure-note-value-must-never-match";
    const PRIVATE_KEY_CANARY: &str = "private-key-value-must-never-match";
    const PASSPHRASE_CANARY: &str = "passphrase-value-must-never-match";
    const SSH_PUBLIC_KEY: &str =
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIE5vdEFRealKey comment-allowlisted";

    #[derive(Clone, Copy)]
    struct FixedClock;

    impl Clock for FixedClock {
        fn now_ms(&self) -> i64 {
            1_800_000_000_000
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
                .expect("fila de UUID não envenenada")
                .pop_front()
                .expect("fixture fornece UUID suficiente")
        }
    }

    fn session_id() -> Uuid {
        Uuid::from_u128(0x9100)
    }

    fn second_session_id() -> Uuid {
        Uuid::from_u128(0x9200)
    }

    fn secret_id(index: u128) -> Uuid {
        Uuid::from_u128(0xa000 + index)
    }

    fn empty_content() -> SessionContent {
        SessionContent {
            content_format: 1,
            secrets: Vec::new(),
        }
    }

    fn service_with_ids(
        access: &FakeSessionAccess,
        ids: impl IntoIterator<Item = Uuid>,
    ) -> SecretService<'_, FakeSessionAccess, FixedClock, SequenceIds> {
        SecretService::new(access, FixedClock, SequenceIds::from(ids))
    }

    fn input(name: &str, data: SecretDataInput) -> CreateSecretInput {
        CreateSecretInput {
            name: name.into(),
            data,
        }
    }

    fn password(name: &str, username: &str, url: Option<&str>) -> CreateSecretInput {
        input(
            name,
            SecretDataInput::Password {
                username: username.into(),
                password: PASSWORD_CANARY.into(),
                url: url.map(str::to_owned),
                notes: Some(NOTES_CANARY.into()),
            },
        )
    }

    fn api_key(name: &str) -> CreateSecretInput {
        input(
            name,
            SecretDataInput::ApiKey {
                key: API_KEY_CANARY.into(),
                environment: Some("staging-allowlisted".into()),
                scopes: vec!["billing-read".into(), "users-write".into()],
            },
        )
    }

    fn token(name: &str) -> CreateSecretInput {
        input(
            name,
            SecretDataInput::Token {
                value: TOKEN_CANARY.into(),
                expires_at: Some("2028-03-14T15:09:26Z".into()),
                notes: Some(NOTES_CANARY.into()),
            },
        )
    }

    fn secure_note(name: &str) -> CreateSecretInput {
        input(
            name,
            SecretDataInput::SecureNote {
                text: NOTE_CANARY.into(),
            },
        )
    }

    fn ssh_key(name: &str) -> CreateSecretInput {
        input(
            name,
            SecretDataInput::SshKey {
                public_key: Some(SSH_PUBLIC_KEY.into()),
                private_key: format!(
                    "-----BEGIN OPENSSH PRIVATE KEY-----\n{PRIVATE_KEY_CANARY}\n-----END OPENSSH PRIVATE KEY-----"
                ),
                passphrase: Some(PASSPHRASE_CANARY.into()),
            },
        )
    }

    fn seed(access: &FakeSessionAccess, target_session: Uuid, inputs: Vec<CreateSecretInput>) {
        access.install_unlocked(target_session, empty_content());
        let ids = (0..inputs.len()).map(|index| secret_id(index as u128));
        let service = service_with_ids(access, ids);
        for input in inputs {
            service
                .create(target_session, input)
                .expect("fixture válida");
        }
    }

    #[test]
    fn search_encontra_name_e_type_normalizados() {
        let access = FakeSessionAccess::default();
        seed(
            &access,
            session_id(),
            vec![password("Conta Árvore", "usuario", None)],
        );
        let service = service_with_ids(&access, []);

        let by_name = service
            .search("arvore", None, 100)
            .expect("busca autorizada");
        let by_type = service
            .search("password", None, 100)
            .expect("busca autorizada");

        assert_eq!(by_name.items.len(), 1);
        assert_eq!(by_type.items.len(), 1);
    }

    #[test]
    fn search_password_encontra_username_e_host_da_url() {
        let access = FakeSessionAccess::default();
        seed(
            &access,
            session_id(),
            vec![password(
                "Conta",
                "alice-allowlisted",
                Some("https://vault.example.test/path/secret"),
            )],
        );
        let service = service_with_ids(&access, []);

        assert_eq!(
            service
                .search("alice-allowlisted", None, 100)
                .unwrap()
                .items
                .len(),
            1
        );
        assert_eq!(
            service
                .search("vault.example.test", None, 100)
                .unwrap()
                .items
                .len(),
            1
        );
        assert!(service
            .search("/path/secret", None, 100)
            .unwrap()
            .items
            .is_empty());
    }

    #[test]
    fn search_api_key_encontra_environment_e_scopes() {
        let access = FakeSessionAccess::default();
        seed(&access, session_id(), vec![api_key("Chave")]);
        let service = service_with_ids(&access, []);

        assert_eq!(
            service
                .search("staging-allowlisted", None, 100)
                .unwrap()
                .items
                .len(),
            1
        );
        assert_eq!(
            service
                .search("billing-read", None, 100)
                .unwrap()
                .items
                .len(),
            1
        );
    }

    #[test]
    fn search_token_encontra_somente_expiracao_publica() {
        let access = FakeSessionAccess::default();
        seed(&access, session_id(), vec![token("Token")]);
        let service = service_with_ids(&access, []);

        assert_eq!(
            service.search("2028-03-14", None, 100).unwrap().items.len(),
            1
        );
        assert!(service
            .search(TOKEN_CANARY, None, 100)
            .unwrap()
            .items
            .is_empty());
    }

    #[test]
    fn search_ssh_encontra_comment_e_fingerprint_derivado() {
        let access = FakeSessionAccess::default();
        seed(&access, session_id(), vec![ssh_key("SSH")]);
        let service = service_with_ids(&access, []);
        let fingerprint =
            ssh_public_fingerprint(SSH_PUBLIC_KEY).expect("chave pública estruturada");

        assert_eq!(
            service
                .search("comment-allowlisted", None, 100)
                .unwrap()
                .items
                .len(),
            1
        );
        assert_eq!(
            service.search(&fingerprint, None, 100).unwrap().items.len(),
            1
        );
    }

    #[test]
    fn search_secure_note_considera_somente_name() {
        let access = FakeSessionAccess::default();
        seed(
            &access,
            session_id(),
            vec![secure_note("Caderno allowlisted")],
        );
        let service = service_with_ids(&access, []);

        assert_eq!(service.search("caderno", None, 100).unwrap().items.len(), 1);
        assert!(service
            .search(NOTE_CANARY, None, 100)
            .unwrap()
            .items
            .is_empty());
    }

    #[test]
    fn search_nunca_compara_valores_notas_tokens_ou_chaves_privadas() {
        let access = FakeSessionAccess::default();
        seed(
            &access,
            session_id(),
            vec![
                password("P", "public-user", None),
                api_key("A"),
                token("T"),
                secure_note("N"),
                ssh_key("S"),
            ],
        );
        let service = service_with_ids(&access, []);

        for forbidden in [
            PASSWORD_CANARY,
            NOTES_CANARY,
            API_KEY_CANARY,
            TOKEN_CANARY,
            NOTE_CANARY,
            PRIVATE_KEY_CANARY,
            PASSPHRASE_CANARY,
        ] {
            assert!(
                service
                    .search(forbidden, None, 100)
                    .unwrap()
                    .items
                    .is_empty(),
                "valor sensível participou da busca"
            );
        }
    }

    #[test]
    fn query_vazia_retorna_listagem_paginada_permitida() {
        let access = FakeSessionAccess::default();
        seed(
            &access,
            session_id(),
            vec![secure_note("Beta"), password("Alpha", "usuario", None)],
        );
        let service = service_with_ids(&access, []);

        let page = service.search("", None, 1).expect("query vazia é listagem");

        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].name, "Alpha");
        assert!(page.next_cursor.is_some());
    }

    #[test]
    fn query_acima_do_limite_e_rejeitada_sem_consultar_sessoes() {
        let access = FakeSessionAccess::default();
        let service = service_with_ids(&access, []);
        let oversized = "x".repeat(MAX_SEARCH_QUERY_BYTES + 1);

        let result = service.search(&oversized, None, 100);

        assert!(result.is_err());
    }

    #[test]
    fn staged_e_invisivel_em_list_e_search() {
        let access = FakeSessionAccess::default();
        seed(
            &access,
            session_id(),
            vec![secure_note("Staged allowlisted")],
        );
        let mut content = access.content(session_id()).unwrap();
        let mut records = decode_records(&content).unwrap();
        records[0].revision = 1;
        records[0].move_state = Some(MoveState::Staged {
            move_id: Uuid::from_u128(0xb001),
            source_session_id: second_session_id(),
            original_revision: 0,
        });
        content.secrets = encode_records(&records).unwrap();
        access.install_unlocked(session_id(), content);
        let service = service_with_ids(&access, []);

        assert!(service
            .list(session_id(), None, 100)
            .unwrap()
            .items
            .is_empty());
        assert!(service
            .search("staged", None, 100)
            .unwrap()
            .items
            .is_empty());
    }

    #[test]
    fn search_omite_sessao_bloqueada_sem_vazar_contagem() {
        let access = FakeSessionAccess::default();
        seed(
            &access,
            session_id(),
            vec![secure_note("Visível allowlisted")],
        );
        seed(
            &access,
            second_session_id(),
            vec![secure_note("Bloqueada allowlisted")],
        );
        access.lock(second_session_id());
        let service = service_with_ids(&access, []);

        let page = service
            .search("allowlisted", None, 100)
            .expect("sessão bloqueada é omitida");

        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].session_id, session_id());
    }

    #[test]
    fn search_descarta_resultados_de_epoch_invalidada_antes_da_resposta() {
        let access = FakeSessionAccess::default();
        seed(
            &access,
            session_id(),
            vec![secure_note("Descartar allowlisted")],
        );
        access.invalidate_before_next_commit(session_id());
        let service = service_with_ids(&access, []);

        let page = service
            .search("allowlisted", None, 100)
            .expect("resultado stale é descartado");

        assert!(page.items.is_empty());
        assert_eq!(access.content(session_id()), None);
    }

    #[test]
    fn cursor_e_rejeitado_quando_epoch_da_sessao_muda() {
        let access = FakeSessionAccess::default();
        seed(
            &access,
            session_id(),
            vec![
                secure_note("Alpha"),
                secure_note("Beta"),
                secure_note("Gamma"),
            ],
        );
        let service = service_with_ids(&access, []);
        let first = service.search("", None, 1).unwrap();
        let cursor = first.next_cursor.expect("há próxima página");
        let same_content = access.content(session_id()).unwrap();
        access.install_unlocked(session_id(), same_content);

        let result = service.search("", Some(&cursor), 1);

        assert!(matches!(result, Err(SecretServiceError::StaleCursor)));
    }

    #[test]
    fn cursor_estavel_retorna_pagina_seguinte_sem_duplicar_item() {
        let access = FakeSessionAccess::default();
        seed(
            &access,
            session_id(),
            vec![
                secure_note("Alpha"),
                secure_note("Beta"),
                secure_note("Gamma"),
            ],
        );
        let service = service_with_ids(&access, []);

        let first = service.list(session_id(), None, 1).unwrap();
        let cursor = first.next_cursor.expect("há próxima página");
        let second = service.list(session_id(), Some(&cursor), 1).unwrap();

        assert_eq!(first.items.len(), 1);
        assert_eq!(second.items.len(), 1);
        assert_ne!(first.items[0].id, second.items[0].id);
    }

    #[test]
    fn limites_de_pagina_aceitam_um_e_cem_e_rejeitam_zero_e_101() {
        let access = FakeSessionAccess::default();
        seed(&access, session_id(), vec![secure_note("Alpha")]);
        let service = service_with_ids(&access, []);

        assert!(service.list(session_id(), None, 1).is_ok());
        assert!(service.list(session_id(), None, 100).is_ok());
        assert!(service.list(session_id(), None, 0).is_err());
        assert!(service.list(session_id(), None, 101).is_err());
    }

    #[test]
    fn cursor_e_opaco_e_nao_contem_query_nome_uuid_ou_segredo() {
        let access = FakeSessionAccess::default();
        seed(
            &access,
            session_id(),
            vec![
                password("Cursor Alpha", "public-user", None),
                password("Cursor Beta", "public-user", None),
            ],
        );
        let service = service_with_ids(&access, []);

        let first = service.search("cursor", None, 1).expect("primeira página");
        let cursor = first.next_cursor.expect("há próxima página");
        let normalized = cursor.to_lowercase();

        for forbidden in [
            "cursor",
            "alpha",
            "beta",
            PASSWORD_CANARY,
            &secret_id(0).to_string(),
        ] {
            assert!(!normalized.contains(&forbidden.to_lowercase()));
        }
    }
}
