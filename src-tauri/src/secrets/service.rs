use uuid::Uuid;

use crate::secrets::codec::{decode_records, encode_records};
use crate::secrets::model::{
    apply_patch, validate_new, CreateSecretInput, SecretError, SecretPatchInput, SecretRecordV1,
    MAX_RECORDS_PER_SESSION, SECRET_RECORD_VERSION,
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
