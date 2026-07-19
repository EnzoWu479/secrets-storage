use crate::crypto::envelope::SessionContent;
use crate::secrets::model::SecretError;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SessionAccessError {
    #[error("sessão bloqueada")]
    Locked,
    #[error("autorização da sessão ficou obsoleta")]
    StaleAuthorization,
    #[error("operação de sessão rejeitada")]
    Operation(#[source] SecretError),
    #[error("falha de commit injetada")]
    InjectedCommitFailure,
    #[error("origem e destino devem ser diferentes")]
    SameSession,
    #[error("revisão da sessão esgotada")]
    RevisionOverflow,
}

pub struct SessionRead<'a> {
    content: &'a SessionContent,
    epoch: u64,
    revision: u64,
}

impl SessionRead<'_> {
    pub fn content(&self) -> &SessionContent {
        self.content
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }
}

pub struct SessionWrite<'a> {
    content: &'a mut SessionContent,
    epoch: u64,
    revision: u64,
}

impl SessionWrite<'_> {
    pub fn content(&self) -> &SessionContent {
        self.content
    }

    pub fn content_mut(&mut self) -> &mut SessionContent {
        self.content
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionStamp {
    pub epoch: u64,
    pub revision: u64,
}

pub struct AuthorizedResult<T> {
    pub value: T,
    pub epoch: u64,
    pub revision: u64,
}

pub struct AuthorizedSessionResult<T> {
    pub session_id: Uuid,
    pub value: T,
    pub epoch: u64,
    pub revision: u64,
}

pub struct TwoSessionResult<T> {
    pub value: T,
    pub first: SessionStamp,
    pub second: SessionStamp,
}

pub trait SessionAccess {
    fn read_authorized<T>(
        &self,
        session_id: Uuid,
        operation: impl FnOnce(SessionRead<'_>) -> Result<T, SecretError>,
    ) -> Result<AuthorizedResult<T>, SessionAccessError>;

    fn read_all_authorized<T>(
        &self,
        operation: impl FnMut(Uuid, SessionRead<'_>) -> Result<T, SecretError>,
    ) -> Result<Vec<AuthorizedSessionResult<T>>, SessionAccessError>;

    fn write_authorized<T>(
        &self,
        session_id: Uuid,
        operation: impl FnOnce(SessionWrite<'_>) -> Result<T, SecretError>,
    ) -> Result<AuthorizedResult<T>, SessionAccessError>;

    fn write_two_authorized<T>(
        &self,
        first: Uuid,
        second: Uuid,
        operation: impl FnOnce(SessionWrite<'_>, SessionWrite<'_>) -> Result<T, SecretError>,
    ) -> Result<TwoSessionResult<T>, SessionAccessError>;
}

#[cfg(any(test, debug_assertions))]
mod fake {
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::{Mutex, MutexGuard};

    use super::*;

    #[derive(Clone)]
    struct Snapshot {
        content: SessionContent,
        epoch: u64,
        revision: u64,
    }

    struct FakeSession {
        content: Option<SessionContent>,
        epoch: u64,
        revision: u64,
    }

    #[derive(Default)]
    struct FakeInner {
        sessions: BTreeMap<Uuid, FakeSession>,
        invalidate_before_commit: BTreeSet<Uuid>,
        fail_before_commit: bool,
        last_lock_order: Vec<Uuid>,
    }

    #[derive(Default)]
    pub struct FakeSessionAccess {
        inner: Mutex<FakeInner>,
    }

    impl FakeSessionAccess {
        pub fn install_unlocked(&self, id: Uuid, content: SessionContent) {
            let mut inner = self.acquire_fail_closed();
            let (epoch, revision) = inner.sessions.get(&id).map_or((1, 0), |session| {
                (session.epoch.saturating_add(1), session.revision)
            });
            inner.sessions.insert(
                id,
                FakeSession {
                    content: Some(content),
                    epoch,
                    revision,
                },
            );
        }

        pub fn invalidate_before_next_commit(&self, id: Uuid) {
            self.acquire_fail_closed()
                .invalidate_before_commit
                .insert(id);
        }

        pub fn lock(&self, id: Uuid) {
            let mut inner = self.acquire_fail_closed();
            if let Some(session) = inner.sessions.get_mut(&id) {
                session.content = None;
                session.epoch = session.epoch.saturating_add(1);
            }
        }

        pub fn fail_before_next_commit(&self) {
            self.acquire_fail_closed().fail_before_commit = true;
        }

        pub fn content(&self, id: Uuid) -> Option<SessionContent> {
            self.acquire_fail_closed()
                .sessions
                .get(&id)
                .and_then(|session| session.content.clone())
        }

        pub fn revision(&self, id: Uuid) -> Option<u64> {
            self.acquire_fail_closed()
                .sessions
                .get(&id)
                .and_then(|session| session.content.as_ref().map(|_| session.revision))
        }

        pub fn last_lock_order(&self) -> Vec<Uuid> {
            self.acquire_fail_closed().last_lock_order.clone()
        }

        fn snapshot(inner: &FakeInner, id: Uuid) -> Result<Snapshot, SessionAccessError> {
            let session = inner.sessions.get(&id).ok_or(SessionAccessError::Locked)?;
            let content = session
                .content
                .as_ref()
                .ok_or(SessionAccessError::Locked)?
                .clone();
            Ok(Snapshot {
                content,
                epoch: session.epoch,
                revision: session.revision,
            })
        }

        fn invalidate_requested(inner: &mut FakeInner, id: Uuid) {
            if inner.invalidate_before_commit.remove(&id) {
                if let Some(session) = inner.sessions.get_mut(&id) {
                    session.content = None;
                    session.epoch = session.epoch.saturating_add(1);
                }
            }
        }

        fn revalidate(
            inner: &FakeInner,
            id: Uuid,
            snapshot: &Snapshot,
        ) -> Result<(), SessionAccessError> {
            let session = inner
                .sessions
                .get(&id)
                .ok_or(SessionAccessError::StaleAuthorization)?;
            if session.epoch != snapshot.epoch || session.revision != snapshot.revision {
                return Err(SessionAccessError::StaleAuthorization);
            }
            if session.content.is_none() {
                return Err(SessionAccessError::Locked);
            }
            Ok(())
        }

        fn acquire_fail_closed(&self) -> MutexGuard<'_, FakeInner> {
            match self.inner.lock() {
                Ok(inner) => inner,
                Err(poisoned) => {
                    let mut inner = poisoned.into_inner();
                    for session in inner.sessions.values_mut() {
                        session.content = None;
                        session.epoch = session.epoch.saturating_add(1);
                    }
                    self.inner.clear_poison();
                    inner
                }
            }
        }
    }

    impl SessionAccess for FakeSessionAccess {
        fn read_authorized<T>(
            &self,
            session_id: Uuid,
            operation: impl FnOnce(SessionRead<'_>) -> Result<T, SecretError>,
        ) -> Result<AuthorizedResult<T>, SessionAccessError> {
            let snapshot = {
                let inner = self.acquire_fail_closed();
                Self::snapshot(&inner, session_id)?
            };
            let value = operation(SessionRead {
                content: &snapshot.content,
                epoch: snapshot.epoch,
                revision: snapshot.revision,
            })
            .map_err(SessionAccessError::Operation)?;

            let mut inner = self.acquire_fail_closed();
            Self::invalidate_requested(&mut inner, session_id);
            Self::revalidate(&inner, session_id, &snapshot)?;
            Ok(AuthorizedResult {
                value,
                epoch: snapshot.epoch,
                revision: snapshot.revision,
            })
        }

        fn read_all_authorized<T>(
            &self,
            mut operation: impl FnMut(Uuid, SessionRead<'_>) -> Result<T, SecretError>,
        ) -> Result<Vec<AuthorizedSessionResult<T>>, SessionAccessError> {
            let snapshots = {
                let inner = self.acquire_fail_closed();
                inner
                    .sessions
                    .iter()
                    .filter_map(|(id, session)| {
                        session.content.as_ref().map(|content| {
                            (
                                *id,
                                Snapshot {
                                    content: content.clone(),
                                    epoch: session.epoch,
                                    revision: session.revision,
                                },
                            )
                        })
                    })
                    .collect::<Vec<_>>()
            };

            let mut projected = Vec::with_capacity(snapshots.len());
            for (id, snapshot) in snapshots {
                let value = operation(
                    id,
                    SessionRead {
                        content: &snapshot.content,
                        epoch: snapshot.epoch,
                        revision: snapshot.revision,
                    },
                )
                .map_err(SessionAccessError::Operation)?;
                projected.push((id, snapshot, value));
            }

            let mut inner = self.acquire_fail_closed();
            for (id, _, _) in &projected {
                Self::invalidate_requested(&mut inner, *id);
            }
            Ok(projected
                .into_iter()
                .filter_map(|(id, snapshot, value)| {
                    Self::revalidate(&inner, id, &snapshot)
                        .ok()
                        .map(|()| AuthorizedSessionResult {
                            session_id: id,
                            value,
                            epoch: snapshot.epoch,
                            revision: snapshot.revision,
                        })
                })
                .collect())
        }

        fn write_authorized<T>(
            &self,
            session_id: Uuid,
            operation: impl FnOnce(SessionWrite<'_>) -> Result<T, SecretError>,
        ) -> Result<AuthorizedResult<T>, SessionAccessError> {
            let snapshot = {
                let inner = self.acquire_fail_closed();
                Self::snapshot(&inner, session_id)?
            };
            let mut candidate = snapshot.content.clone();
            let value = operation(SessionWrite {
                content: &mut candidate,
                epoch: snapshot.epoch,
                revision: snapshot.revision,
            })
            .map_err(SessionAccessError::Operation)?;

            let mut inner = self.acquire_fail_closed();
            if inner.fail_before_commit {
                inner.fail_before_commit = false;
                return Err(SessionAccessError::InjectedCommitFailure);
            }
            Self::invalidate_requested(&mut inner, session_id);
            Self::revalidate(&inner, session_id, &snapshot)?;
            let revision = snapshot
                .revision
                .checked_add(1)
                .ok_or(SessionAccessError::RevisionOverflow)?;
            let session = inner
                .sessions
                .get_mut(&session_id)
                .ok_or(SessionAccessError::StaleAuthorization)?;
            session.content = Some(candidate);
            session.revision = revision;

            Ok(AuthorizedResult {
                value,
                epoch: snapshot.epoch,
                revision,
            })
        }

        fn write_two_authorized<T>(
            &self,
            first: Uuid,
            second: Uuid,
            operation: impl FnOnce(SessionWrite<'_>, SessionWrite<'_>) -> Result<T, SecretError>,
        ) -> Result<TwoSessionResult<T>, SessionAccessError> {
            if first == second {
                return Err(SessionAccessError::SameSession);
            }
            let (first_snapshot, second_snapshot) = {
                let mut inner = self.acquire_fail_closed();
                let mut order = vec![first, second];
                order.sort_unstable();
                inner.last_lock_order = order;
                (
                    Self::snapshot(&inner, first)?,
                    Self::snapshot(&inner, second)?,
                )
            };
            let mut first_candidate = first_snapshot.content.clone();
            let mut second_candidate = second_snapshot.content.clone();
            let value = operation(
                SessionWrite {
                    content: &mut first_candidate,
                    epoch: first_snapshot.epoch,
                    revision: first_snapshot.revision,
                },
                SessionWrite {
                    content: &mut second_candidate,
                    epoch: second_snapshot.epoch,
                    revision: second_snapshot.revision,
                },
            )
            .map_err(SessionAccessError::Operation)?;

            let mut inner = self.acquire_fail_closed();
            if inner.fail_before_commit {
                inner.fail_before_commit = false;
                return Err(SessionAccessError::InjectedCommitFailure);
            }
            Self::invalidate_requested(&mut inner, first);
            Self::invalidate_requested(&mut inner, second);
            Self::revalidate(&inner, first, &first_snapshot)?;
            Self::revalidate(&inner, second, &second_snapshot)?;
            let first_revision = first_snapshot
                .revision
                .checked_add(1)
                .ok_or(SessionAccessError::RevisionOverflow)?;
            let second_revision = second_snapshot
                .revision
                .checked_add(1)
                .ok_or(SessionAccessError::RevisionOverflow)?;

            let first_session = inner
                .sessions
                .get_mut(&first)
                .ok_or(SessionAccessError::StaleAuthorization)?;
            first_session.content = Some(first_candidate);
            first_session.revision = first_revision;
            let second_session = inner
                .sessions
                .get_mut(&second)
                .ok_or(SessionAccessError::StaleAuthorization)?;
            second_session.content = Some(second_candidate);
            second_session.revision = second_revision;

            Ok(TwoSessionResult {
                value,
                first: SessionStamp {
                    epoch: first_snapshot.epoch,
                    revision: first_revision,
                },
                second: SessionStamp {
                    epoch: second_snapshot.epoch,
                    revision: second_revision,
                },
            })
        }
    }
}

#[cfg(any(test, debug_assertions))]
pub use fake::FakeSessionAccess;

#[cfg(test)]
mod tests {
    use super::*;
    use ciborium::value::Value;

    fn content(marker: i64) -> SessionContent {
        SessionContent {
            content_format: 1,
            secrets: vec![Value::Integer(marker.into())],
        }
    }

    #[test]
    fn leitura_de_sessao_bloqueada_e_negada() {
        let access = FakeSessionAccess::default();
        let id = Uuid::from_u128(1);

        let result = access.read_authorized(id, |_| Ok::<_, SecretError>(()));

        assert!(matches!(result, Err(SessionAccessError::Locked)));
    }

    #[test]
    fn escrita_confirma_candidato_e_incrementa_revisao() {
        let access = FakeSessionAccess::default();
        let id = Uuid::from_u128(1);
        access.install_unlocked(id, content(1));

        let result = access
            .write_authorized(id, |mut session| {
                session.content_mut().secrets.push(Value::Integer(2.into()));
                Ok::<_, SecretError>(session.revision())
            })
            .expect("commit autorizado");

        assert_eq!(result.value, 0);
        assert_eq!(result.revision, 1);
        assert_eq!(access.content(id).expect("sessão aberta").secrets.len(), 2);
    }

    #[test]
    fn erro_da_operacao_descarta_candidato() {
        let access = FakeSessionAccess::default();
        let id = Uuid::from_u128(1);
        access.install_unlocked(id, content(1));

        let result = access.write_authorized(id, |mut session| {
            session.content_mut().secrets.clear();
            Err::<(), _>(SecretError::InvalidInput)
        });

        assert!(matches!(result, Err(SessionAccessError::Operation(_))));
        assert_eq!(access.content(id).expect("sessão aberta"), content(1));
        assert_eq!(access.revision(id), Some(0));
    }

    #[test]
    fn epoch_e_revalidada_antes_de_responder_leitura() {
        let access = FakeSessionAccess::default();
        let id = Uuid::from_u128(1);
        access.install_unlocked(id, content(1));
        access.invalidate_before_next_commit(id);

        let result = access.read_authorized(id, |_| Ok::<_, SecretError>("resultado"));

        assert!(matches!(
            result,
            Err(SessionAccessError::StaleAuthorization)
        ));
    }

    #[test]
    fn epoch_obsoleta_descarta_escrita_candidata() {
        let access = FakeSessionAccess::default();
        let id = Uuid::from_u128(1);
        access.install_unlocked(id, content(1));
        access.invalidate_before_next_commit(id);

        let result = access.write_authorized(id, |mut session| {
            session.content_mut().secrets.clear();
            Ok::<_, SecretError>(())
        });

        assert!(matches!(
            result,
            Err(SessionAccessError::StaleAuthorization)
        ));
        assert_eq!(access.revision(id), None);
    }

    #[test]
    fn falha_injetada_antes_do_commit_preserva_estado() {
        let access = FakeSessionAccess::default();
        let id = Uuid::from_u128(1);
        access.install_unlocked(id, content(1));
        access.fail_before_next_commit();

        let result = access.write_authorized(id, |mut session| {
            session.content_mut().secrets.clear();
            Ok::<_, SecretError>(())
        });

        assert!(matches!(
            result,
            Err(SessionAccessError::InjectedCommitFailure)
        ));
        assert_eq!(access.content(id).expect("sessão aberta"), content(1));
        assert_eq!(access.revision(id), Some(0));
    }

    #[test]
    fn duas_sessoes_usam_ordem_crescente_de_uuid() {
        let access = FakeSessionAccess::default();
        let low = Uuid::from_u128(1);
        let high = Uuid::from_u128(2);
        access.install_unlocked(low, content(1));
        access.install_unlocked(high, content(2));

        access
            .write_two_authorized(high, low, |mut first, mut second| {
                first.content_mut().secrets.clear();
                second.content_mut().secrets.clear();
                Ok::<_, SecretError>(())
            })
            .expect("commit duplo");

        assert_eq!(access.last_lock_order(), vec![low, high]);
        assert_eq!(access.revision(low), Some(1));
        assert_eq!(access.revision(high), Some(1));
    }

    #[test]
    fn commit_duplo_e_atomico_quando_uma_epoch_muda() {
        let access = FakeSessionAccess::default();
        let first = Uuid::from_u128(1);
        let second = Uuid::from_u128(2);
        access.install_unlocked(first, content(1));
        access.install_unlocked(second, content(2));
        access.invalidate_before_next_commit(second);

        let result = access.write_two_authorized(first, second, |mut left, mut right| {
            left.content_mut().secrets.clear();
            right.content_mut().secrets.clear();
            Ok::<_, SecretError>(())
        });

        assert!(matches!(
            result,
            Err(SessionAccessError::StaleAuthorization)
        ));
        assert_eq!(access.content(first).expect("primeira aberta"), content(1));
        assert_eq!(access.revision(first), Some(0));
        assert_eq!(access.revision(second), None);
    }

    #[test]
    fn mesma_sessao_na_operacao_dupla_e_rejeitada() {
        let access = FakeSessionAccess::default();
        let id = Uuid::from_u128(1);
        access.install_unlocked(id, content(1));

        let result = access.write_two_authorized(id, id, |_, _| Ok::<_, SecretError>(()));

        assert!(matches!(result, Err(SessionAccessError::SameSession)));
        assert_eq!(access.revision(id), Some(0));
    }

    #[test]
    fn leitura_global_enumera_somente_sessoes_desbloqueadas_em_ordem_estavel() {
        let access = FakeSessionAccess::default();
        let low = Uuid::from_u128(1);
        let locked = Uuid::from_u128(2);
        let high = Uuid::from_u128(3);
        access.install_unlocked(high, content(3));
        access.install_unlocked(low, content(1));
        access.install_unlocked(locked, content(2));
        access.invalidate_before_next_commit(locked);
        let _ = access.read_authorized(locked, |_| Ok::<_, SecretError>(()));

        let results = access
            .read_all_authorized(|id, session| {
                Ok::<_, SecretError>((id, session.content().secrets.len()))
            })
            .expect("leitura global");

        assert_eq!(
            results
                .iter()
                .map(|result| result.session_id)
                .collect::<Vec<_>>(),
            vec![low, high]
        );
    }

    #[test]
    fn leitura_global_descarta_sessao_invalidada_antes_da_resposta() {
        let access = FakeSessionAccess::default();
        let stable = Uuid::from_u128(1);
        let invalidated = Uuid::from_u128(2);
        access.install_unlocked(stable, content(1));
        access.install_unlocked(invalidated, content(2));
        access.invalidate_before_next_commit(invalidated);

        let results = access
            .read_all_authorized(|id, _| Ok::<_, SecretError>(id))
            .expect("leitura global");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, stable);
        assert_eq!(results[0].value, stable);
        assert_eq!(access.content(invalidated), None);
    }

    #[test]
    fn leitura_global_retorna_epoch_e_revisao_revalidadas() {
        let access = FakeSessionAccess::default();
        let id = Uuid::from_u128(1);
        access.install_unlocked(id, content(1));

        let results = access
            .read_all_authorized(|_, session| {
                Ok::<_, SecretError>((session.epoch(), session.revision()))
            })
            .expect("leitura global");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, (1, 0));
        assert_eq!(results[0].epoch, 1);
        assert_eq!(results[0].revision, 0);
    }
}
