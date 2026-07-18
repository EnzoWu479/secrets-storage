#![cfg(all(windows, feature = "security-proof"))]

use secrets_storage_lib::security::diagnostics::PageLockStatus;
use secrets_storage_lib::security::memory::{
    PageLockAttempt, SensitiveMemoryError, SensitiveRegion,
};

const CANARY: &[u8] = b"WINT05-CONTROLLED-CANARY";

#[test]
fn rejects_an_empty_region_before_allocating() {
    let error = match SensitiveRegion::new(&[]) {
        Err(error) => error,
        Ok(_) => panic!("empty input must be rejected"),
    };

    assert_eq!(error, SensitiveMemoryError::EmptyInput);
    assert_eq!(error.code(), "empty_sensitive_input");
}

#[test]
fn allocates_a_dedicated_region_for_the_canary() {
    let region = SensitiveRegion::new(CANARY).unwrap();

    assert_eq!(region.len(), CANARY.len());
    assert!(region.capacity() >= region.len());
    assert!(region.matches_for_proof(CANARY));
}

#[test]
fn reports_a_consistent_real_page_lock_result() {
    let region = SensitiveRegion::new(CANARY).unwrap();

    match region.page_lock_status() {
        PageLockStatus::Active => assert_eq!(region.page_lock_platform_code(), None),
        PageLockStatus::Degraded => assert!(region.page_lock_platform_code().is_some()),
    }
}

#[test]
fn degrades_with_a_non_sensitive_code_when_page_lock_fails() {
    let region =
        SensitiveRegion::new_with_page_lock_for_proof(CANARY, PageLockAttempt::Fail(1450)).unwrap();

    assert_eq!(region.page_lock_status(), PageLockStatus::Degraded);
    assert_eq!(region.page_lock_platform_code(), Some(1450));
}

#[test]
fn proof_zeroization_overwrites_the_entire_capacity() {
    let mut region =
        SensitiveRegion::new_with_page_lock_for_proof(CANARY, PageLockAttempt::Fail(1450)).unwrap();

    let observation = region.zeroize_for_proof();

    assert_eq!(observation.bytes_overwritten, region.capacity());
    assert!(region.capacity_is_zeroed_for_proof());
}

#[test]
fn explicit_proof_release_uses_the_drop_cleanup_path() {
    let region = SensitiveRegion::new(CANARY).unwrap();
    let was_locked = region.page_lock_status() == PageLockStatus::Active;

    let observation = region.release_for_proof();

    assert!(observation.capacity_zeroed);
    assert_eq!(observation.unlock_attempted, was_locked);
    assert!(observation.free_succeeded);
}

#[test]
fn degraded_release_skips_unlock_but_still_zeroes_and_frees() {
    let region =
        SensitiveRegion::new_with_page_lock_for_proof(CANARY, PageLockAttempt::Fail(1450)).unwrap();

    let observation = region.release_for_proof();

    assert!(observation.capacity_zeroed);
    assert!(!observation.unlock_attempted);
    assert!(observation.free_succeeded);
}
