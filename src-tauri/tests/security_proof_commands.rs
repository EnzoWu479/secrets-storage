#![cfg(all(windows, feature = "security-proof"))]

use std::sync::Arc;

use secrets_storage_lib::proof::commands::{
    authorized_probe_with_hook_for_proof, AuthorizedProbeInput, InstallCanaryInput,
    ProofCommandErrorCode,
};
use secrets_storage_lib::security::lock::{LockReason, LockState};
use secrets_storage_lib::security::memory::SensitiveRegion;
use secrets_storage_lib::{ApplicationLockCoordinator, ApplicationSensitiveState};
use serde_json::{json, Value};
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{
    get_ipc_response, mock_builder, mock_context, noop_assets, MockRuntime, INVOKE_KEY,
};
use tauri::webview::InvokeRequest;
use tauri::{App, Manager, WebviewWindow, WebviewWindowBuilder};

const CANARY: &str = "IPC-CANARY-DO-NOT-RETURN";

fn create_app() -> App<MockRuntime> {
    mock_builder()
        .manage(Arc::new(ApplicationLockCoordinator::default()))
        .invoke_handler(tauri::generate_handler![
            secrets_storage_lib::proof::commands::proof_install_canary,
            secrets_storage_lib::proof::commands::proof_authorized_probe,
            secrets_storage_lib::proof::commands::proof_lock,
            secrets_storage_lib::proof::commands::proof_status
        ])
        .build(mock_context(noop_assets()))
        .expect("mock Tauri app must build")
}

fn create_window(app: &App<MockRuntime>, label: &str) -> WebviewWindow<MockRuntime> {
    WebviewWindowBuilder::new(app, label, Default::default())
        .build()
        .expect("mock webview must build")
}

fn invoke(window: &WebviewWindow<MockRuntime>, command: &str, body: Value) -> Result<Value, Value> {
    get_ipc_response(
        window,
        InvokeRequest {
            cmd: command.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: "http://tauri.localhost".parse().unwrap(),
            body: InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_owned(),
        },
    )
    .map(|response| response.deserialize::<Value>().unwrap())
}

fn install_body(bytes: &[u8]) -> Value {
    json!({
        "input": {
            "bytes": bytes,
            "scenarioNonce": "scenario_01"
        }
    })
}

fn error_code(error: &Value) -> Option<&str> {
    error.get("code").and_then(Value::as_str)
}

#[test]
fn install_canary_returns_only_allowlisted_metadata() {
    let app = create_app();
    let proof = create_window(&app, "security-proof");

    let response = invoke(
        &proof,
        "proof_install_canary",
        install_body(CANARY.as_bytes()),
    )
    .unwrap();
    let fields: Vec<_> = response.as_object().unwrap().keys().cloned().collect();

    assert_eq!(fields.len(), 3);
    assert_eq!(response["state"], "unlocked");
    assert_eq!(response["epoch"], 1);
    assert!(matches!(
        response["page_lock_status"].as_str(),
        Some("active" | "degraded")
    ));
    assert!(!response.to_string().contains(CANARY));
    assert!(!response.to_string().contains("platform"));
}

#[test]
fn install_rejects_empty_canary_before_unlocking() {
    let app = create_app();
    let proof = create_window(&app, "security-proof");

    let error = invoke(&proof, "proof_install_canary", install_body(&[])).unwrap_err();

    assert_eq!(error_code(&error), Some("empty_canary"));
    assert_eq!(
        app.state::<Arc<ApplicationLockCoordinator>>()
            .snapshot()
            .state,
        LockState::Locked
    );
}

#[test]
fn install_rejects_oversize_canary_before_unlocking() {
    let app = create_app();
    let proof = create_window(&app, "security-proof");

    let error = invoke(&proof, "proof_install_canary", install_body(&vec![7; 4097])).unwrap_err();

    assert_eq!(error_code(&error), Some("canary_too_large"));
    assert_eq!(
        app.state::<Arc<ApplicationLockCoordinator>>()
            .snapshot()
            .state,
        LockState::Locked
    );
}

#[test]
fn install_rejects_invalid_nonce_before_unlocking() {
    let app = create_app();
    let proof = create_window(&app, "security-proof");
    let body = json!({
        "input": {
            "bytes": [1, 2, 3],
            "scenarioNonce": "../not-allowed"
        }
    });

    let error = invoke(&proof, "proof_install_canary", body).unwrap_err();

    assert_eq!(error_code(&error), Some("invalid_scenario_nonce"));
    assert_eq!(
        app.state::<Arc<ApplicationLockCoordinator>>()
            .snapshot()
            .state,
        LockState::Locked
    );
}

#[test]
fn install_input_zeroization_overwrites_owned_canary_bytes() {
    let mut input = InstallCanaryInput {
        bytes: CANARY.as_bytes().to_vec(),
        scenario_nonce: "scenario_01".into(),
    };

    input.zeroize_for_proof();

    assert!(input.bytes.iter().all(|byte| *byte == 0));
    assert!(input.scenario_nonce.is_empty());
}

#[test]
fn authorized_probe_returns_authorization_without_echoing_identifier() {
    let app = create_app();
    let proof = create_window(&app, "security-proof");
    invoke(
        &proof,
        "proof_install_canary",
        install_body(CANARY.as_bytes()),
    )
    .unwrap();

    let response = invoke(
        &proof,
        "proof_authorized_probe",
        json!({ "input": { "identifier": "probe_01" } }),
    )
    .unwrap();

    assert_eq!(response, json!({ "authorized": true, "epoch": 1 }));
    assert!(!response.to_string().contains("probe_01"));
}

#[test]
fn authorized_probe_rejects_locked_state_with_stable_code() {
    let app = create_app();
    let proof = create_window(&app, "security-proof");

    let error = invoke(
        &proof,
        "proof_authorized_probe",
        json!({ "input": { "identifier": "probe_01" } }),
    )
    .unwrap_err();

    assert_eq!(error_code(&error), Some("locked"));
}

#[test]
fn authorized_probe_rejects_oversize_identifier() {
    let app = create_app();
    let proof = create_window(&app, "security-proof");

    let error = invoke(
        &proof,
        "proof_authorized_probe",
        json!({ "input": { "identifier": "a".repeat(65) } }),
    )
    .unwrap_err();

    assert_eq!(error_code(&error), Some("invalid_probe_identifier"));
}

#[test]
fn lock_before_probe_commit_rejects_stale_authorization() {
    let coordinator = ApplicationLockCoordinator::default();
    let region = SensitiveRegion::new(CANARY.as_bytes()).unwrap();
    coordinator.install_unlocked(ApplicationSensitiveState::new(region));

    let result = authorized_probe_with_hook_for_proof(
        &coordinator,
        AuthorizedProbeInput {
            identifier: "probe_01".into(),
        },
        || {
            coordinator.lock(LockReason::Manual);
        },
    );

    assert_eq!(
        result.unwrap_err().code,
        ProofCommandErrorCode::StaleAuthorization
    );
    assert_eq!(coordinator.snapshot().state, LockState::Locked);
}

#[test]
fn manual_lock_returns_only_state_epoch_and_changed() {
    let app = create_app();
    let proof = create_window(&app, "security-proof");
    invoke(
        &proof,
        "proof_install_canary",
        install_body(CANARY.as_bytes()),
    )
    .unwrap();

    let response = invoke(
        &proof,
        "proof_lock",
        json!({ "input": { "reason": "manual" } }),
    )
    .unwrap();

    assert_eq!(
        response,
        json!({ "state": "locked", "epoch": 2, "changed": true })
    );
}

#[test]
fn duplicate_lock_is_idempotent() {
    let app = create_app();
    let proof = create_window(&app, "security-proof");
    invoke(
        &proof,
        "proof_install_canary",
        install_body(CANARY.as_bytes()),
    )
    .unwrap();
    invoke(
        &proof,
        "proof_lock",
        json!({ "input": { "reason": "manual" } }),
    )
    .unwrap();

    let response = invoke(
        &proof,
        "proof_lock",
        json!({ "input": { "reason": "manual" } }),
    )
    .unwrap();

    assert_eq!(
        response,
        json!({ "state": "locked", "epoch": 2, "changed": false })
    );
}

#[test]
fn status_returns_only_allowlisted_locked_metadata() {
    let app = create_app();
    let proof = create_window(&app, "security-proof");

    let response = invoke(&proof, "proof_status", json!({})).unwrap();

    assert_eq!(
        response,
        json!({
            "state": "locked",
            "epoch": 0,
            "listener_status": "unavailable"
        })
    );
}

#[test]
fn main_window_is_denied_even_with_valid_payload() {
    let app = create_app();
    let main = create_window(&app, "main");

    let error = invoke(
        &main,
        "proof_install_canary",
        install_body(CANARY.as_bytes()),
    )
    .unwrap_err();

    assert_eq!(error_code(&error), Some("window_not_authorized"));
    assert_eq!(
        app.state::<Arc<ApplicationLockCoordinator>>()
            .snapshot()
            .state,
        LockState::Locked
    );
}

#[test]
fn unknown_command_is_denied() {
    let app = create_app();
    let proof = create_window(&app, "security-proof");

    let error = invoke(&proof, "proof_unknown", json!({})).unwrap_err();

    assert!(error.as_str().is_some());
}

#[test]
fn invalid_lock_reason_is_denied_without_state_change() {
    let app = create_app();
    let proof = create_window(&app, "security-proof");
    invoke(
        &proof,
        "proof_install_canary",
        install_body(CANARY.as_bytes()),
    )
    .unwrap();

    let error = invoke(
        &proof,
        "proof_lock",
        json!({ "input": { "reason": "shutdown" } }),
    )
    .unwrap_err();

    assert_eq!(error_code(&error), Some("invalid_lock_input"));
    assert_eq!(
        app.state::<Arc<ApplicationLockCoordinator>>()
            .snapshot()
            .state,
        LockState::Unlocked
    );
}

#[test]
fn unknown_install_field_is_denied_without_state_change() {
    let app = create_app();
    let proof = create_window(&app, "security-proof");
    let body = json!({
        "input": {
            "bytes": [1, 2, 3],
            "scenarioNonce": "scenario_01",
            "path": "C:/sensitive"
        }
    });

    let error = invoke(&proof, "proof_install_canary", body).unwrap_err();

    assert_eq!(error_code(&error), Some("invalid_install_input"));
    assert_eq!(
        app.state::<Arc<ApplicationLockCoordinator>>()
            .snapshot()
            .state,
        LockState::Locked
    );
}
