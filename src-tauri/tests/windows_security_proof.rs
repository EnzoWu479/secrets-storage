#![cfg(all(windows, feature = "security-proof"))]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::Arc;
use std::time::Duration;

use secrets_storage_lib::platform::windows::events::{
    wts_retry_action_for_proof, ListenerRegistrationStatus, RetryAction, WindowsEventPump,
};
use secrets_storage_lib::security::lock::LockReason;
use secrets_storage_lib::security::lock::{LockCoordinator, LockState};
use secrets_storage_lib::security::memory::SensitiveRegion;
use secrets_storage_lib::{
    apply_exit_lifecycle, ApplicationLifecycle, ApplicationSensitiveState, ExitLifecycleEvent,
};
use windows::Win32::UI::WindowsAndMessaging::{
    PBT_APMRESUMEAUTOMATIC, PBT_APMRESUMECRITICAL, PBT_APMRESUMESUSPEND, PBT_APMSUSPEND, WM_CLOSE,
    WM_POWERBROADCAST, WM_WTSSESSION_CHANGE, WTS_SESSION_LOCK,
};

const SIGNAL_TIMEOUT: Duration = Duration::from_secs(2);
const RPC_S_INVALID_BINDING: u32 = 1702;

struct DropMarker(Arc<AtomicBool>);

impl Drop for DropMarker {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

fn start_pump() -> (WindowsEventPump, Receiver<LockReason>) {
    WindowsEventPump::start().expect("the Windows event pump must start")
}

#[test]
fn creates_a_hidden_top_level_window() {
    let (mut pump, _signals) = start_pump();

    let facts = pump.window_facts_for_proof().unwrap();

    assert!(facts.is_top_level);
    assert!(!facts.is_visible);
    pump.shutdown().unwrap();
}

#[test]
fn reports_the_non_sensitive_wts_registration_status() {
    let (mut pump, _signals) = start_pump();

    match pump.listener_registration_status() {
        ListenerRegistrationStatus::Active => {}
        ListenerRegistrationStatus::Degraded { platform_code } => {
            assert_ne!(platform_code, 0);
        }
    }
    pump.shutdown().unwrap();
}

#[test]
fn forwards_a_session_lock_signal() {
    let (mut pump, signals) = start_pump();

    pump.post_for_proof(WM_WTSSESSION_CHANGE, WTS_SESSION_LOCK as usize)
        .unwrap();

    assert_eq!(
        signals.recv_timeout(SIGNAL_TIMEOUT).unwrap(),
        LockReason::SessionLocked
    );
    pump.shutdown().unwrap();
}

#[test]
fn forwards_a_suspend_signal() {
    let (mut pump, signals) = start_pump();

    pump.post_for_proof(WM_POWERBROADCAST, PBT_APMSUSPEND as usize)
        .unwrap();

    assert_eq!(
        signals.recv_timeout(SIGNAL_TIMEOUT).unwrap(),
        LockReason::Suspending
    );
    pump.shutdown().unwrap();
}

#[test]
fn forwards_each_resume_as_a_fail_closed_lock_signal() {
    for resume in [
        PBT_APMRESUMEAUTOMATIC,
        PBT_APMRESUMESUSPEND,
        PBT_APMRESUMECRITICAL,
    ] {
        let (mut pump, signals) = start_pump();
        pump.post_for_proof(WM_POWERBROADCAST, resume as usize)
            .unwrap();

        assert_eq!(
            signals.recv_timeout(SIGNAL_TIMEOUT).unwrap(),
            LockReason::Resumed
        );
        pump.shutdown().unwrap();
    }
}

#[test]
fn ignores_an_unknown_message_without_unlocking() {
    let (mut pump, signals) = start_pump();

    pump.post_for_proof(WM_POWERBROADCAST, u32::MAX as usize)
        .unwrap();

    assert_eq!(
        signals.recv_timeout(Duration::from_millis(100)),
        Err(RecvTimeoutError::Timeout)
    );
    pump.shutdown().unwrap();
}

#[test]
fn retries_only_invalid_binding_and_stops_at_the_limit() {
    assert_eq!(
        wts_retry_action_for_proof(RPC_S_INVALID_BINDING, 0),
        RetryAction::Retry
    );
    assert_eq!(
        wts_retry_action_for_proof(RPC_S_INVALID_BINDING, 2),
        RetryAction::Stop
    );
    assert_eq!(wts_retry_action_for_proof(5, 0), RetryAction::Stop);
}

#[test]
fn shutdown_is_idempotent() {
    let (mut pump, _signals) = start_pump();

    pump.shutdown().unwrap();
    pump.shutdown().unwrap();
}

#[test]
fn ignores_external_close_and_keeps_the_pump_alive() {
    let (mut pump, signals) = start_pump();

    pump.post_raw_for_proof(WM_CLOSE, 0).unwrap();
    pump.post_for_proof(WM_WTSSESSION_CHANGE, WTS_SESSION_LOCK as usize)
        .unwrap();

    assert_eq!(
        signals.recv_timeout(SIGNAL_TIMEOUT).unwrap(),
        LockReason::SessionLocked
    );
    pump.shutdown().unwrap();
}

#[test]
fn exit_requested_locks_and_drops_state_before_returning() {
    let dropped = Arc::new(AtomicBool::new(false));
    let coordinator = LockCoordinator::default();
    coordinator.install_unlocked(DropMarker(Arc::clone(&dropped)));

    let outcome = apply_exit_lifecycle(&coordinator, ExitLifecycleEvent::ExitRequested).unwrap();

    assert_eq!(outcome.state, LockState::Locked);
    assert_eq!(outcome.reason, LockReason::Exiting);
    assert!(outcome.changed);
    assert!(dropped.load(Ordering::SeqCst));
}

#[test]
fn exit_reapplies_the_fail_closed_lock_as_a_fallback() {
    let coordinator = LockCoordinator::default();
    coordinator.install_unlocked(());

    let outcome = apply_exit_lifecycle(&coordinator, ExitLifecycleEvent::Exit).unwrap();

    assert_eq!(outcome.state, LockState::Locked);
    assert_eq!(outcome.reason, LockReason::Exiting);
}

#[test]
fn duplicate_exit_is_idempotent() {
    let coordinator = LockCoordinator::default();
    coordinator.install_unlocked(());
    let first = apply_exit_lifecycle(&coordinator, ExitLifecycleEvent::ExitRequested).unwrap();

    let second = apply_exit_lifecycle(&coordinator, ExitLifecycleEvent::Exit).unwrap();

    assert!(!second.changed);
    assert_eq!(second.epoch, first.epoch);
}

#[test]
fn window_close_does_not_replace_a_global_exit_event() {
    let coordinator = LockCoordinator::default();
    coordinator.install_unlocked(());

    let outcome = apply_exit_lifecycle(&coordinator, ExitLifecycleEvent::WindowCloseRequested);

    assert!(outcome.is_none());
    assert_eq!(coordinator.snapshot().state, LockState::Unlocked);
}

#[test]
fn lifecycle_manages_and_locks_the_same_authoritative_coordinator() {
    let dropped = Arc::new(AtomicBool::new(false));
    let lifecycle = ApplicationLifecycle::new();
    let managed = lifecycle.coordinator();
    let captured = lifecycle.coordinator();
    assert!(Arc::ptr_eq(&managed, &captured));
    managed.install_unlocked(ApplicationSensitiveState::new(DropMarker(Arc::clone(
        &dropped,
    ))));

    let outcome = lifecycle.apply(ExitLifecycleEvent::ExitRequested).unwrap();

    assert!(outcome.changed);
    assert!(dropped.load(Ordering::SeqCst));
    assert_eq!(managed.snapshot().state, LockState::Locked);
}

#[test]
fn sensitive_region_can_be_owned_by_the_cross_thread_application_state() {
    fn assert_send<T: Send>() {}

    assert_send::<SensitiveRegion>();
}
