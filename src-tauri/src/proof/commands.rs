use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{Runtime, State, WebviewWindow};
use zeroize::Zeroize;

use crate::security::diagnostics::{ListenerStatus, PageLockStatus, SecurityState};
use crate::security::lock::{AuthorizationError, LockReason, LockState};
use crate::security::memory::{SensitiveMemoryError, SensitiveRegion};
use crate::{ApplicationLockCoordinator, ApplicationSensitiveState};

const MAX_CANARY_BYTES: usize = 4096;
const MAX_IDENTIFIER_BYTES: usize = 64;
const PROOF_WINDOW_LABEL: &str = "security-proof";

struct ProofSensitiveState {
    region: SensitiveRegion,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallCanaryInput {
    pub bytes: Vec<u8>,
    pub scenario_nonce: String,
}

impl Zeroize for InstallCanaryInput {
    fn zeroize(&mut self) {
        self.bytes.zeroize();
        self.scenario_nonce.zeroize();
    }
}

impl Drop for InstallCanaryInput {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl InstallCanaryInput {
    pub fn zeroize_for_proof(&mut self) {
        self.zeroize();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct InstallCanaryResponse {
    pub state: SecurityState,
    pub epoch: u64,
    pub page_lock_status: PageLockStatus,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorizedProbeInput {
    pub identifier: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct AuthorizedProbeResponse {
    pub authorized: bool,
    pub epoch: u64,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProofLockReason {
    Manual,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockInput {
    pub reason: ProofLockReason,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct LockResponse {
    pub state: SecurityState,
    pub epoch: u64,
    pub changed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct StatusResponse {
    pub state: SecurityState,
    pub epoch: u64,
    pub listener_status: ListenerStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProofCommandErrorCode {
    WindowNotAuthorized,
    EmptyCanary,
    CanaryTooLarge,
    InvalidScenarioNonce,
    InvalidInstallInput,
    InvalidProbeIdentifier,
    InvalidAuthorizedProbeInput,
    InvalidLockInput,
    Locked,
    StaleAuthorization,
    SensitiveMemoryUnavailable,
    InvalidSensitiveState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ProofCommandError {
    pub code: ProofCommandErrorCode,
}

impl ProofCommandError {
    const fn new(code: ProofCommandErrorCode) -> Self {
        Self { code }
    }
}

impl From<AuthorizationError> for ProofCommandError {
    fn from(error: AuthorizationError) -> Self {
        let code = match error {
            AuthorizationError::Locked => ProofCommandErrorCode::Locked,
            AuthorizationError::StaleAuthorization => ProofCommandErrorCode::StaleAuthorization,
        };
        Self::new(code)
    }
}

impl From<SensitiveMemoryError> for ProofCommandError {
    fn from(_error: SensitiveMemoryError) -> Self {
        Self::new(ProofCommandErrorCode::SensitiveMemoryUnavailable)
    }
}

fn require_proof_window(label: &str) -> Result<(), ProofCommandError> {
    (label == PROOF_WINDOW_LABEL)
        .then_some(())
        .ok_or_else(|| ProofCommandError::new(ProofCommandErrorCode::WindowNotAuthorized))
}

fn parse_input<T: DeserializeOwned>(
    input: Option<Value>,
    code: ProofCommandErrorCode,
) -> Result<T, ProofCommandError> {
    input
        .and_then(|value| serde_json::from_value(value).ok())
        .ok_or_else(|| ProofCommandError::new(code))
}

fn valid_bounded_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn validate_install_input(input: &InstallCanaryInput) -> Result<(), ProofCommandError> {
    if input.bytes.is_empty() {
        return Err(ProofCommandError::new(ProofCommandErrorCode::EmptyCanary));
    }
    if input.bytes.len() > MAX_CANARY_BYTES {
        return Err(ProofCommandError::new(
            ProofCommandErrorCode::CanaryTooLarge,
        ));
    }
    if !valid_bounded_identifier(&input.scenario_nonce) {
        return Err(ProofCommandError::new(
            ProofCommandErrorCode::InvalidScenarioNonce,
        ));
    }
    Ok(())
}

fn validate_probe_input(input: &AuthorizedProbeInput) -> Result<(), ProofCommandError> {
    valid_bounded_identifier(&input.identifier)
        .then_some(())
        .ok_or_else(|| ProofCommandError::new(ProofCommandErrorCode::InvalidProbeIdentifier))
}

fn state_code(state: LockState) -> SecurityState {
    match state {
        LockState::Locked => SecurityState::Locked,
        LockState::Unlocked => SecurityState::Unlocked,
    }
}

fn install_canary(
    coordinator: &ApplicationLockCoordinator,
    input: InstallCanaryInput,
) -> Result<InstallCanaryResponse, ProofCommandError> {
    validate_install_input(&input)?;
    let region = SensitiveRegion::new(&input.bytes)?;
    let page_lock_status = region.page_lock_status();
    let snapshot =
        coordinator.install_unlocked(ApplicationSensitiveState::new(ProofSensitiveState {
            region,
        }));

    Ok(InstallCanaryResponse {
        state: state_code(snapshot.state),
        epoch: snapshot.epoch,
        page_lock_status,
    })
}

fn authorized_probe(
    coordinator: &ApplicationLockCoordinator,
    input: AuthorizedProbeInput,
    before_commit: impl FnOnce(),
) -> Result<AuthorizedProbeResponse, ProofCommandError> {
    validate_probe_input(&input)?;
    let guard = coordinator.begin_authorized()?;
    let epoch = coordinator.snapshot().epoch;
    before_commit();

    coordinator
        .commit_if_current(guard, |state| {
            let proof_state = state
                .value_mut::<ProofSensitiveState>()
                .ok_or(AuthorizationError::Locked)?;
            (!proof_state.region.is_empty())
                .then_some(AuthorizedProbeResponse {
                    authorized: true,
                    epoch,
                })
                .ok_or(AuthorizationError::Locked)
        })
        .map_err(ProofCommandError::from)
}

pub fn authorized_probe_with_hook_for_proof(
    coordinator: &ApplicationLockCoordinator,
    input: AuthorizedProbeInput,
    before_commit: impl FnOnce(),
) -> Result<AuthorizedProbeResponse, ProofCommandError> {
    authorized_probe(coordinator, input, before_commit)
}

#[tauri::command]
pub fn proof_install_canary<R: Runtime>(
    window: WebviewWindow<R>,
    coordinator: State<'_, Arc<ApplicationLockCoordinator>>,
    input: Option<Value>,
) -> Result<InstallCanaryResponse, ProofCommandError> {
    require_proof_window(window.label())?;
    let input = parse_input(input, ProofCommandErrorCode::InvalidInstallInput)?;
    install_canary(coordinator.inner().as_ref(), input)
}

#[tauri::command]
pub fn proof_authorized_probe<R: Runtime>(
    window: WebviewWindow<R>,
    coordinator: State<'_, Arc<ApplicationLockCoordinator>>,
    input: Option<Value>,
) -> Result<AuthorizedProbeResponse, ProofCommandError> {
    require_proof_window(window.label())?;
    let input = parse_input(input, ProofCommandErrorCode::InvalidAuthorizedProbeInput)?;
    authorized_probe(coordinator.inner().as_ref(), input, || {})
}

#[tauri::command]
pub fn proof_lock<R: Runtime>(
    window: WebviewWindow<R>,
    coordinator: State<'_, Arc<ApplicationLockCoordinator>>,
    input: Option<Value>,
) -> Result<LockResponse, ProofCommandError> {
    require_proof_window(window.label())?;
    let input: LockInput = parse_input(input, ProofCommandErrorCode::InvalidLockInput)?;
    let reason = match input.reason {
        ProofLockReason::Manual => LockReason::Manual,
    };
    let outcome = coordinator.lock(reason);
    Ok(LockResponse {
        state: state_code(outcome.state),
        epoch: outcome.epoch,
        changed: outcome.changed,
    })
}

#[tauri::command]
pub fn proof_status<R: Runtime>(
    window: WebviewWindow<R>,
    coordinator: State<'_, Arc<ApplicationLockCoordinator>>,
) -> Result<StatusResponse, ProofCommandError> {
    require_proof_window(window.label())?;
    let snapshot = coordinator.snapshot();
    Ok(StatusResponse {
        state: state_code(snapshot.state),
        epoch: snapshot.epoch,
        listener_status: ListenerStatus::Unavailable,
    })
}
