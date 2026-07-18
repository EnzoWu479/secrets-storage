//! Região de memória dedicada para material sensível em Windows.
//!
//! A região é exclusivamente proprietária de uma reserva `VirtualAlloc`. Isso
//! permite sobrescrever toda a capacidade antes de liberar as páginas, sem
//! tocar em objetos vizinhos de um allocator compartilhado.

use std::ffi::c_void;
use std::mem::{size_of, MaybeUninit};
use std::ptr::{self, NonNull};
use std::slice;

use thiserror::Error;
use windows::Win32::Foundation::GetLastError;
use windows::Win32::System::Memory::{
    VirtualAlloc, VirtualFree, VirtualLock, VirtualQuery, VirtualUnlock, MEMORY_BASIC_INFORMATION,
    MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE,
};
use zeroize::Zeroize;

use super::diagnostics::PageLockStatus;

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum SensitiveMemoryError {
    #[error("sensitive input is empty")]
    EmptyInput,
    #[error("sensitive region allocation failed with platform code {platform_code}")]
    AllocationFailed { platform_code: u32 },
    #[error("sensitive region query failed with platform code {platform_code}")]
    RegionQueryFailed { platform_code: u32 },
    #[error("sensitive region reported an invalid capacity")]
    InvalidRegionCapacity,
}

impl SensitiveMemoryError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::EmptyInput => "empty_sensitive_input",
            Self::AllocationFailed { .. } => "sensitive_allocation_failed",
            Self::RegionQueryFailed { .. } => "sensitive_region_query_failed",
            Self::InvalidRegionCapacity => "invalid_sensitive_region_capacity",
        }
    }
}

#[cfg(feature = "security-proof")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageLockAttempt {
    System,
    Fail(u32),
}

#[cfg(feature = "security-proof")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZeroizeObservation {
    pub bytes_overwritten: usize,
}

#[cfg(feature = "security-proof")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReleaseObservation {
    pub capacity_zeroed: bool,
    pub unlock_attempted: bool,
    pub free_succeeded: bool,
}

pub struct SensitiveRegion {
    pointer: Option<NonNull<c_void>>,
    len: usize,
    capacity: usize,
    page_lock_status: PageLockStatus,
    page_lock_platform_code: Option<u32>,
}

impl SensitiveRegion {
    pub fn new(bytes: &[u8]) -> Result<Self, SensitiveMemoryError> {
        Self::allocate(bytes, PageLockStrategy::System)
    }

    #[cfg(feature = "security-proof")]
    pub fn new_with_page_lock_for_proof(
        bytes: &[u8],
        attempt: PageLockAttempt,
    ) -> Result<Self, SensitiveMemoryError> {
        let strategy = match attempt {
            PageLockAttempt::System => PageLockStrategy::System,
            PageLockAttempt::Fail(platform_code) => PageLockStrategy::ForcedFailure(platform_code),
        };

        Self::allocate(bytes, strategy)
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    pub const fn page_lock_status(&self) -> PageLockStatus {
        self.page_lock_status
    }

    pub const fn page_lock_platform_code(&self) -> Option<u32> {
        self.page_lock_platform_code
    }

    #[cfg(feature = "security-proof")]
    pub fn matches_for_proof(&self, expected: &[u8]) -> bool {
        self.bytes().get(..self.len) == Some(expected)
    }

    #[cfg(feature = "security-proof")]
    pub fn capacity_is_zeroed_for_proof(&self) -> bool {
        self.bytes().iter().all(|byte| *byte == 0)
    }

    #[cfg(feature = "security-proof")]
    pub fn zeroize_for_proof(&mut self) -> ZeroizeObservation {
        self.zeroize_capacity();
        ZeroizeObservation {
            bytes_overwritten: self.capacity,
        }
    }

    #[cfg(feature = "security-proof")]
    pub fn release_for_proof(mut self) -> ReleaseObservation {
        let (capacity_zeroed, unlock_attempted, free_succeeded) = self.release();
        ReleaseObservation {
            capacity_zeroed,
            unlock_attempted,
            free_succeeded,
        }
    }

    fn allocate(
        bytes: &[u8],
        lock_strategy: PageLockStrategy,
    ) -> Result<Self, SensitiveMemoryError> {
        if bytes.is_empty() {
            return Err(SensitiveMemoryError::EmptyInput);
        }

        // SAFETY:
        // - Windows owns the returned allocation until `VirtualFree`.
        // - a null pointer is rejected before any dereference;
        // - `VirtualQuery` initializes the metadata used to bound every slice;
        // - the input copy is within the requested allocation;
        // - the pointer never escapes this non-Clone owner.
        unsafe {
            let raw = VirtualAlloc(None, bytes.len(), MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
            let pointer = match NonNull::new(raw) {
                Some(pointer) => pointer,
                None => {
                    let platform_code = GetLastError().0;
                    return Err(SensitiveMemoryError::AllocationFailed { platform_code });
                }
            };

            let mut information = MaybeUninit::<MEMORY_BASIC_INFORMATION>::uninit();
            let queried = VirtualQuery(
                Some(pointer.as_ptr()),
                information.as_mut_ptr(),
                size_of::<MEMORY_BASIC_INFORMATION>(),
            );
            if queried != size_of::<MEMORY_BASIC_INFORMATION>() {
                let platform_code = GetLastError().0;
                let _ = VirtualFree(pointer.as_ptr(), 0, MEM_RELEASE);
                return Err(SensitiveMemoryError::RegionQueryFailed { platform_code });
            }

            let capacity = information.assume_init().RegionSize;
            if capacity < bytes.len() {
                let _ = VirtualFree(pointer.as_ptr(), 0, MEM_RELEASE);
                return Err(SensitiveMemoryError::InvalidRegionCapacity);
            }
            ptr::copy_nonoverlapping(bytes.as_ptr(), pointer.as_ptr().cast::<u8>(), bytes.len());

            let (page_lock_status, page_lock_platform_code) = match lock_strategy {
                PageLockStrategy::System => match VirtualLock(pointer.as_ptr(), capacity) {
                    Ok(()) => (PageLockStatus::Active, None),
                    Err(_) => (PageLockStatus::Degraded, Some(GetLastError().0)),
                },
                #[cfg(feature = "security-proof")]
                PageLockStrategy::ForcedFailure(platform_code) => {
                    (PageLockStatus::Degraded, Some(platform_code))
                }
            };

            Ok(Self {
                pointer: Some(pointer),
                len: bytes.len(),
                capacity,
                page_lock_status,
                page_lock_platform_code,
            })
        }
    }

    #[cfg(feature = "security-proof")]
    fn bytes(&self) -> &[u8] {
        let pointer = self
            .pointer
            .expect("a live sensitive region always owns its allocation");

        // SAFETY: `pointer` owns a live allocation of exactly `capacity` bytes
        // until `release` takes it. Shared access cannot mutate the allocation.
        unsafe { slice::from_raw_parts(pointer.as_ptr().cast::<u8>(), self.capacity) }
    }

    fn zeroize_capacity(&mut self) -> bool {
        let Some(pointer) = self.pointer else {
            return true;
        };

        // SAFETY: this owner has exclusive access and the live allocation was
        // measured by `VirtualQuery` before `capacity` was stored.
        unsafe {
            slice::from_raw_parts_mut(pointer.as_ptr().cast::<u8>(), self.capacity).zeroize();
        }
        self.len = 0;
        // This read occurs while the allocation is still live and is used by
        // the proof observation returned from the shared cleanup path.
        unsafe {
            slice::from_raw_parts(pointer.as_ptr().cast::<u8>(), self.capacity)
                .iter()
                .all(|byte| *byte == 0)
        }
    }

    fn release(&mut self) -> (bool, bool, bool) {
        let Some(pointer) = self.pointer else {
            return (true, false, true);
        };

        let capacity_zeroed = self.zeroize_capacity();
        let unlock_attempted = self.page_lock_status == PageLockStatus::Active;

        // SAFETY: `pointer` is the base returned by `VirtualAlloc`; `capacity`
        // is its queried region size. `VirtualFree(..., 0, MEM_RELEASE)` is the
        // required release form for a reserved region.
        let free_succeeded = unsafe {
            if unlock_attempted {
                let _ = VirtualUnlock(pointer.as_ptr(), self.capacity);
            }
            VirtualFree(pointer.as_ptr(), 0, MEM_RELEASE).is_ok()
        };
        if free_succeeded {
            self.pointer = None;
        }

        (capacity_zeroed, unlock_attempted, free_succeeded)
    }
}

impl Drop for SensitiveRegion {
    fn drop(&mut self) {
        let _ = self.release();
    }
}

enum PageLockStrategy {
    System,
    #[cfg(feature = "security-proof")]
    ForcedFailure(u32),
}
