use crate::secrets::model::SecretText;
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ClipboardTimeout {
    ThirtySeconds,
    OneMinute,
    #[default]
    FiveMinutes,
    TenMinutes,
    FifteenMinutes,
}

impl ClipboardTimeout {
    pub const ALL: [Self; 5] = [
        Self::ThirtySeconds,
        Self::OneMinute,
        Self::FiveMinutes,
        Self::TenMinutes,
        Self::FifteenMinutes,
    ];

    pub const fn seconds(self) -> u64 {
        match self {
            Self::ThirtySeconds => 30,
            Self::OneMinute => 60,
            Self::FiveMinutes => 300,
            Self::TenMinutes => 600,
            Self::FifteenMinutes => 900,
        }
    }
}

impl TryFrom<u64> for ClipboardTimeout {
    type Error = UnsupportedClipboardTimeout;

    fn try_from(seconds: u64) -> Result<Self, Self::Error> {
        Self::ALL
            .into_iter()
            .find(|preset| preset.seconds() == seconds)
            .ok_or(UnsupportedClipboardTimeout)
    }
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
#[error("unsupported clipboard timeout")]
pub struct UnsupportedClipboardTimeout;

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum ClipboardPortError {
    #[error("clipboard unavailable")]
    Unavailable,
}

pub trait ClipboardPort {
    fn copy_text(&self, value: &SecretText) -> Result<u64, ClipboardPortError>;
    fn sequence_number(&self) -> Result<u64, ClipboardPortError>;
    fn clear(&self) -> Result<(), ClipboardPortError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClipboardOwnership {
    pub session_id: Uuid,
    pub sequence: u64,
    pub deadline_ms: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClipboardCopyReceipt {
    pub deadline_ms: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClipboardClearResult {
    Cleared,
    NotOwned,
    Inconclusive,
    NoOwnedValue,
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum ClipboardCoordinatorError {
    #[error("clipboard unavailable")]
    ClipboardUnavailable,
    #[error("clipboard deadline overflow")]
    DeadlineOverflow,
}

pub struct ClipboardCoordinator<P> {
    port: P,
    ownership: Option<ClipboardOwnership>,
}

impl<P: ClipboardPort> ClipboardCoordinator<P> {
    pub fn new(port: P) -> Self {
        Self {
            port,
            ownership: None,
        }
    }

    pub fn copy(
        &mut self,
        session_id: Uuid,
        value: &SecretText,
        timeout: ClipboardTimeout,
        now_ms: u64,
    ) -> Result<ClipboardCopyReceipt, ClipboardCoordinatorError> {
        let timeout_ms = timeout
            .seconds()
            .checked_mul(1_000)
            .ok_or(ClipboardCoordinatorError::DeadlineOverflow)?;
        let deadline_ms = now_ms
            .checked_add(timeout_ms)
            .ok_or(ClipboardCoordinatorError::DeadlineOverflow)?;
        let sequence = self
            .port
            .copy_text(value)
            .map_err(|_| ClipboardCoordinatorError::ClipboardUnavailable)?;

        self.ownership = Some(ClipboardOwnership {
            session_id,
            sequence,
            deadline_ms,
        });
        Ok(ClipboardCopyReceipt { deadline_ms })
    }

    pub fn clear_expired(&mut self, now_ms: u64) -> ClipboardClearResult {
        match self.ownership {
            Some(ownership) if now_ms >= ownership.deadline_ms => self.clear_owned(),
            _ => ClipboardClearResult::NoOwnedValue,
        }
    }

    pub fn clear_now(&mut self) -> ClipboardClearResult {
        if self.ownership.is_none() {
            return ClipboardClearResult::NoOwnedValue;
        }
        self.clear_owned()
    }

    pub fn clear_for_session_lock(&mut self, session_id: Uuid) -> ClipboardClearResult {
        match self.ownership {
            Some(ownership) if ownership.session_id == session_id => self.clear_owned(),
            _ => ClipboardClearResult::NoOwnedValue,
        }
    }

    fn clear_owned(&mut self) -> ClipboardClearResult {
        let Some(ownership) = self.ownership else {
            return ClipboardClearResult::NoOwnedValue;
        };

        let current_sequence = match self.port.sequence_number() {
            Ok(sequence) => sequence,
            Err(_) => return ClipboardClearResult::Inconclusive,
        };
        if current_sequence != ownership.sequence {
            self.ownership = None;
            return ClipboardClearResult::NotOwned;
        }
        if self.port.clear().is_err() {
            return ClipboardClearResult::Inconclusive;
        }

        self.ownership = None;
        ClipboardClearResult::Cleared
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::model::{validate_new, CreateSecretInput, SecretDataInput, SecretDataV1};
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeClipboard {
        state: Mutex<FakeState>,
    }

    #[derive(Default)]
    struct FakeState {
        sequence: u64,
        copied: Option<String>,
        fail_sequence: bool,
        fail_clear: bool,
        clear_count: usize,
    }

    impl ClipboardPort for FakeClipboard {
        fn copy_text(&self, value: &SecretText) -> Result<u64, ClipboardPortError> {
            let mut state = self.state.lock().expect("fake");
            state.sequence += 1;
            state.copied = Some(value.as_str().to_owned());
            Ok(state.sequence)
        }

        fn sequence_number(&self) -> Result<u64, ClipboardPortError> {
            let state = self.state.lock().expect("fake");
            if state.fail_sequence {
                Err(ClipboardPortError::Unavailable)
            } else {
                Ok(state.sequence)
            }
        }

        fn clear(&self) -> Result<(), ClipboardPortError> {
            let mut state = self.state.lock().expect("fake");
            if state.fail_clear {
                return Err(ClipboardPortError::Unavailable);
            }
            state.sequence += 1;
            state.copied = None;
            state.clear_count += 1;
            Ok(())
        }
    }

    impl FakeClipboard {
        fn replace_by_user(&self) {
            let mut state = self.state.lock().expect("fake");
            state.sequence += 1;
            state.copied = Some("conteúdo posterior".into());
        }

        fn fail_sequence(&self) {
            self.state.lock().expect("fake").fail_sequence = true;
        }

        fn fail_clear(&self) {
            self.state.lock().expect("fake").fail_clear = true;
        }

        fn clear_count(&self) -> usize {
            self.state.lock().expect("fake").clear_count
        }
    }

    fn sensitive_value() -> SecretText {
        let validated = validate_new(CreateSecretInput {
            name: "Nota".into(),
            data: SecretDataInput::SecureNote {
                text: "clipboard-canary".into(),
            },
        })
        .expect("fixture");
        match validated.data {
            SecretDataV1::SecureNote { text } => text,
            _ => unreachable!(),
        }
    }

    #[test]
    fn default_e_cinco_minutos() {
        assert_eq!(ClipboardTimeout::default().seconds(), 300);
    }

    #[test]
    fn presets_sao_limitados_aos_cinco_valores() {
        assert_eq!(
            ClipboardTimeout::ALL.map(ClipboardTimeout::seconds),
            [30, 60, 300, 600, 900]
        );
        assert!(ClipboardTimeout::try_from(31).is_err());
    }

    #[test]
    fn copy_armazena_apenas_ownership_e_deadline() {
        let port = FakeClipboard::default();
        let mut coordinator = ClipboardCoordinator::new(port);
        let session = Uuid::from_u128(1);

        let receipt = coordinator
            .copy(
                session,
                &sensitive_value(),
                ClipboardTimeout::FiveMinutes,
                1_000,
            )
            .expect("copy");

        assert_eq!(receipt.deadline_ms, 301_000);
        assert_eq!(
            coordinator.ownership,
            Some(ClipboardOwnership {
                session_id: session,
                sequence: 1,
                deadline_ms: 301_000
            })
        );
    }

    #[test]
    fn timeout_limpa_quando_sequence_ainda_pertence_ao_app() {
        let port = FakeClipboard::default();
        let mut coordinator = ClipboardCoordinator::new(port);
        coordinator
            .copy(
                Uuid::from_u128(1),
                &sensitive_value(),
                ClipboardTimeout::ThirtySeconds,
                1_000,
            )
            .expect("copy");

        let result = coordinator.clear_expired(31_000);

        assert_eq!(result, ClipboardClearResult::Cleared);
        assert!(coordinator.ownership.is_none());
        assert_eq!(coordinator.port.clear_count(), 1);
    }

    #[test]
    fn timeout_antes_do_deadline_nao_limpa() {
        let port = FakeClipboard::default();
        let mut coordinator = ClipboardCoordinator::new(port);
        coordinator
            .copy(
                Uuid::from_u128(1),
                &sensitive_value(),
                ClipboardTimeout::ThirtySeconds,
                1_000,
            )
            .expect("copy");

        assert_eq!(
            coordinator.clear_expired(30_999),
            ClipboardClearResult::NoOwnedValue
        );
        assert_eq!(coordinator.port.clear_count(), 0);
    }

    #[test]
    fn conteudo_posterior_do_usuario_nao_e_apagado() {
        let port = FakeClipboard::default();
        let mut coordinator = ClipboardCoordinator::new(port);
        coordinator
            .copy(
                Uuid::from_u128(1),
                &sensitive_value(),
                ClipboardTimeout::ThirtySeconds,
                0,
            )
            .expect("copy");
        coordinator.port.replace_by_user();

        let result = coordinator.clear_expired(30_000);

        assert_eq!(result, ClipboardClearResult::NotOwned);
        assert_eq!(coordinator.port.clear_count(), 0);
        assert!(coordinator.ownership.is_none());
    }

    #[test]
    fn limpar_agora_reusa_verificacao_de_ownership() {
        let port = FakeClipboard::default();
        let mut coordinator = ClipboardCoordinator::new(port);
        coordinator
            .copy(
                Uuid::from_u128(1),
                &sensitive_value(),
                ClipboardTimeout::FiveMinutes,
                0,
            )
            .expect("copy");

        assert_eq!(coordinator.clear_now(), ClipboardClearResult::Cleared);
        assert_eq!(coordinator.port.clear_count(), 1);
    }

    #[test]
    fn lock_da_sessao_solicita_limpeza_antecipada() {
        let port = FakeClipboard::default();
        let mut coordinator = ClipboardCoordinator::new(port);
        let session = Uuid::from_u128(1);
        coordinator
            .copy(
                session,
                &sensitive_value(),
                ClipboardTimeout::FiveMinutes,
                0,
            )
            .expect("copy");

        assert_eq!(
            coordinator.clear_for_session_lock(session),
            ClipboardClearResult::Cleared
        );
    }

    #[test]
    fn lock_de_outra_sessao_nao_limpa() {
        let port = FakeClipboard::default();
        let mut coordinator = ClipboardCoordinator::new(port);
        coordinator
            .copy(
                Uuid::from_u128(1),
                &sensitive_value(),
                ClipboardTimeout::FiveMinutes,
                0,
            )
            .expect("copy");

        assert_eq!(
            coordinator.clear_for_session_lock(Uuid::from_u128(2)),
            ClipboardClearResult::NoOwnedValue
        );
        assert_eq!(coordinator.port.clear_count(), 0);
    }

    #[test]
    fn falha_ao_ler_sequence_e_inconclusiva() {
        let port = FakeClipboard::default();
        let mut coordinator = ClipboardCoordinator::new(port);
        coordinator
            .copy(
                Uuid::from_u128(1),
                &sensitive_value(),
                ClipboardTimeout::FiveMinutes,
                0,
            )
            .expect("copy");
        coordinator.port.fail_sequence();

        assert_eq!(coordinator.clear_now(), ClipboardClearResult::Inconclusive);
        assert!(coordinator.ownership.is_some());
    }

    #[test]
    fn falha_ao_limpar_e_inconclusiva() {
        let port = FakeClipboard::default();
        let mut coordinator = ClipboardCoordinator::new(port);
        coordinator
            .copy(
                Uuid::from_u128(1),
                &sensitive_value(),
                ClipboardTimeout::FiveMinutes,
                0,
            )
            .expect("copy");
        coordinator.port.fail_clear();

        assert_eq!(coordinator.clear_now(), ClipboardClearResult::Inconclusive);
        assert!(coordinator.ownership.is_some());
    }
}
