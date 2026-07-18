use std::sync::{Mutex, MutexGuard};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockState {
    Locked,
    Unlocked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockReason {
    Startup,
    Manual,
    SessionLocked,
    Suspending,
    Resumed,
    ShuttingDown,
    Exiting,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LockSnapshot {
    pub state: LockState,
    pub epoch: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LockOutcome {
    pub state: LockState,
    pub epoch: u64,
    pub changed: bool,
    pub reason: LockReason,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuthorizationGuard {
    epoch: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AuthorizationError {
    #[error("operation requires unlocked state")]
    Locked,
    #[error("authorization guard is stale")]
    StaleAuthorization,
}

impl AuthorizationError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Locked => "locked",
            Self::StaleAuthorization => "stale_authorization",
        }
    }
}

struct Inner<T> {
    epoch: u64,
    state: Option<T>,
}

pub struct LockCoordinator<T> {
    inner: Mutex<Inner<T>>,
}

impl<T> Default for LockCoordinator<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> LockCoordinator<T> {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                epoch: 0,
                state: None,
            }),
        }
    }

    pub fn snapshot(&self) -> LockSnapshot {
        let inner = self.acquire_fail_closed();
        Self::snapshot_from(&inner)
    }

    pub fn install_unlocked(&self, state: T) -> LockSnapshot {
        let mut inner = self.acquire_fail_closed();
        inner.epoch = inner.epoch.saturating_add(1);
        let previous = inner.state.replace(state);
        let snapshot = Self::snapshot_from(&inner);
        drop(previous);
        snapshot
    }

    pub fn begin_authorized(&self) -> Result<AuthorizationGuard, AuthorizationError> {
        let inner = self.acquire_fail_closed();
        inner
            .state
            .as_ref()
            .map(|_| AuthorizationGuard { epoch: inner.epoch })
            .ok_or(AuthorizationError::Locked)
    }

    pub fn commit_if_current<R, E>(
        &self,
        guard: AuthorizationGuard,
        action: impl FnOnce(&mut T) -> Result<R, E>,
    ) -> Result<R, AuthorizationError>
    where
        E: Into<AuthorizationError>,
    {
        let mut inner = self.acquire_fail_closed();
        if guard.epoch != inner.epoch {
            return Err(AuthorizationError::StaleAuthorization);
        }

        let mut state = inner.state.take().ok_or(AuthorizationError::Locked)?;
        let result = action(&mut state);
        inner.state = Some(state);
        result.map_err(Into::into)
    }

    pub fn lock(&self, reason: LockReason) -> LockOutcome {
        let mut inner = self.acquire_fail_closed();
        let state = inner.state.take();
        let changed = state.is_some();
        if changed {
            inner.epoch = inner.epoch.saturating_add(1);
        }
        let outcome = LockOutcome {
            state: LockState::Locked,
            epoch: inner.epoch,
            changed,
            reason,
        };
        drop(state);
        outcome
    }

    fn acquire_fail_closed(&self) -> MutexGuard<'_, Inner<T>> {
        match self.inner.lock() {
            Ok(inner) => inner,
            Err(poisoned) => {
                let mut inner = poisoned.into_inner();
                if inner.state.take().is_some() {
                    inner.epoch = inner.epoch.saturating_add(1);
                }
                self.inner.clear_poison();
                inner
            }
        }
    }

    fn snapshot_from(inner: &Inner<T>) -> LockSnapshot {
        LockSnapshot {
            state: if inner.state.is_some() {
                LockState::Unlocked
            } else {
                LockState::Locked
            },
            epoch: inner.epoch,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[derive(Debug)]
    struct DropSpy {
        value: u8,
        dropped: Arc<AtomicBool>,
    }

    impl Drop for DropSpy {
        fn drop(&mut self) {
            self.dropped.store(true, Ordering::SeqCst);
        }
    }

    fn state(value: u8) -> (DropSpy, Arc<AtomicBool>) {
        let dropped = Arc::new(AtomicBool::new(false));
        (
            DropSpy {
                value,
                dropped: Arc::clone(&dropped),
            },
            dropped,
        )
    }

    #[test]
    fn starts_locked_at_epoch_zero() {
        let coordinator = LockCoordinator::<DropSpy>::new();

        assert_eq!(
            coordinator.snapshot(),
            LockSnapshot {
                state: LockState::Locked,
                epoch: 0,
            }
        );
    }

    #[test]
    fn installing_state_unlocks_and_advances_epoch() {
        let coordinator = LockCoordinator::new();
        let (secret, _) = state(7);

        let snapshot = coordinator.install_unlocked(secret);

        assert_eq!(snapshot.state, LockState::Unlocked);
        assert_eq!(snapshot.epoch, 1);
    }

    #[test]
    fn begin_authorized_denies_locked_state() {
        let coordinator = LockCoordinator::<DropSpy>::new();

        assert_eq!(
            coordinator.begin_authorized(),
            Err(AuthorizationError::Locked)
        );
    }

    #[test]
    fn current_guard_can_commit_an_action() {
        let coordinator = LockCoordinator::new();
        let (secret, _) = state(7);
        coordinator.install_unlocked(secret);
        let guard = coordinator.begin_authorized().unwrap();

        let result = coordinator
            .commit_if_current(guard, |state| {
                state.value += 1;
                Ok::<_, AuthorizationError>(state.value)
            })
            .unwrap();

        assert_eq!(result, 8);
        assert_eq!(coordinator.snapshot().state, LockState::Unlocked);
    }

    #[test]
    fn lock_discards_state_before_returning() {
        let coordinator = LockCoordinator::new();
        let (secret, dropped) = state(7);
        coordinator.install_unlocked(secret);

        let outcome = coordinator.lock(LockReason::Manual);

        assert!(outcome.changed);
        assert_eq!(outcome.state, LockState::Locked);
        assert_eq!(outcome.epoch, 2);
        assert!(dropped.load(Ordering::SeqCst));
    }

    #[test]
    fn duplicate_lock_is_idempotent() {
        let coordinator = LockCoordinator::<DropSpy>::new();

        let first = coordinator.lock(LockReason::Startup);
        let second = coordinator.lock(LockReason::Resumed);

        assert!(!first.changed);
        assert!(!second.changed);
        assert_eq!(first.epoch, 0);
        assert_eq!(second.epoch, 0);
    }

    #[test]
    fn stale_guard_is_rejected_after_lock() {
        let coordinator = LockCoordinator::new();
        let (secret, _) = state(7);
        coordinator.install_unlocked(secret);
        let guard = coordinator.begin_authorized().unwrap();
        coordinator.lock(LockReason::SessionLocked);
        let mut called = false;

        let result = coordinator.commit_if_current(guard, |_| {
            called = true;
            Ok::<_, AuthorizationError>(())
        });

        assert_eq!(result, Err(AuthorizationError::StaleAuthorization));
        assert!(!called);
    }

    #[test]
    fn reinstalling_state_invalidates_an_older_guard() {
        let coordinator = LockCoordinator::new();
        let (first, first_dropped) = state(1);
        coordinator.install_unlocked(first);
        let guard = coordinator.begin_authorized().unwrap();
        let (second, _) = state(2);

        coordinator.install_unlocked(second);

        assert!(first_dropped.load(Ordering::SeqCst));
        assert_eq!(
            coordinator.commit_if_current(guard, |_| Ok::<_, AuthorizationError>(())),
            Err(AuthorizationError::StaleAuthorization)
        );
    }

    #[test]
    fn completed_result_is_not_retroactively_revoked() {
        let coordinator = LockCoordinator::new();
        let (secret, _) = state(9);
        coordinator.install_unlocked(secret);
        let guard = coordinator.begin_authorized().unwrap();
        let result = coordinator
            .commit_if_current(guard, |state| Ok::<_, AuthorizationError>(state.value))
            .unwrap();

        coordinator.lock(LockReason::Suspending);

        assert_eq!(result, 9);
        assert_eq!(coordinator.snapshot().state, LockState::Locked);
    }

    #[test]
    fn panic_during_action_drops_state_and_fails_closed() {
        let coordinator = LockCoordinator::new();
        let (secret, dropped) = state(7);
        coordinator.install_unlocked(secret);
        let guard = coordinator.begin_authorized().unwrap();

        let panic = catch_unwind(AssertUnwindSafe(|| {
            let _ = coordinator
                .commit_if_current::<(), AuthorizationError>(guard, |_| panic!("controlled panic"));
        }));

        assert!(panic.is_err());
        assert!(dropped.load(Ordering::SeqCst));
        assert_eq!(coordinator.snapshot().state, LockState::Locked);
    }
}
