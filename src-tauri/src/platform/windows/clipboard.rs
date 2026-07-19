//! Adaptador Win32 do clipboard usado pelo core.

use std::mem::size_of;
use std::thread;
use std::time::Duration;

use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardSequenceNumber, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use zeroize::Zeroizing;

use crate::secrets::clipboard::{ClipboardPort, ClipboardPortError};
use crate::secrets::model::SecretText;

const CF_UNICODETEXT_FORMAT: u32 = 13;
const OPEN_ATTEMPTS: usize = 8;
const RETRY_DELAY: Duration = Duration::from_millis(10);

#[derive(Clone, Copy, Debug, Default)]
pub struct WindowsClipboard;

impl ClipboardPort for WindowsClipboard {
    fn copy_text(&self, value: &SecretText) -> Result<u64, ClipboardPortError> {
        let utf16 = Zeroizing::new(
            value
                .as_str()
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect::<Vec<_>>(),
        );
        let byte_length = utf16
            .len()
            .checked_mul(size_of::<u16>())
            .ok_or(ClipboardPortError::Unavailable)?;
        let mut memory = GlobalMemory::allocate(byte_length)?;
        memory.write_utf16(&utf16)?;

        let clipboard = ClipboardGuard::open_with_retries()?;
        unsafe { EmptyClipboard() }.map_err(|_| ClipboardPortError::Unavailable)?;
        unsafe { SetClipboardData(CF_UNICODETEXT_FORMAT, Some(HANDLE(memory.handle().0))) }
            .map_err(|_| ClipboardPortError::Unavailable)?;
        memory.transfer_to_clipboard();
        clipboard.close()?;

        // Win32 publishes the final sequence only after CloseClipboard. Another process can
        // replace the value in the unavoidable interval between close and this query.
        self.sequence_number()
    }

    fn sequence_number(&self) -> Result<u64, ClipboardPortError> {
        current_sequence()
    }

    fn clear(&self) -> Result<(), ClipboardPortError> {
        let clipboard = ClipboardGuard::open_with_retries()?;
        unsafe { EmptyClipboard() }.map_err(|_| ClipboardPortError::Unavailable)?;
        clipboard.close()
    }
}

fn current_sequence() -> Result<u64, ClipboardPortError> {
    let sequence = unsafe { GetClipboardSequenceNumber() };
    if sequence == 0 {
        Err(ClipboardPortError::Unavailable)
    } else {
        Ok(u64::from(sequence))
    }
}

struct ClipboardGuard {
    is_open: bool,
}

impl ClipboardGuard {
    fn open_with_retries() -> Result<Self, ClipboardPortError> {
        for attempt in 0..OPEN_ATTEMPTS {
            if unsafe { OpenClipboard(None) }.is_ok() {
                return Ok(Self { is_open: true });
            }
            if attempt + 1 < OPEN_ATTEMPTS {
                thread::sleep(RETRY_DELAY);
            }
        }
        Err(ClipboardPortError::Unavailable)
    }

    fn close(mut self) -> Result<(), ClipboardPortError> {
        unsafe { CloseClipboard() }.map_err(|_| ClipboardPortError::Unavailable)?;
        self.is_open = false;
        Ok(())
    }
}

impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        if self.is_open {
            let _ = unsafe { CloseClipboard() };
        }
    }
}

struct GlobalMemory {
    handle: Option<HGLOBAL>,
}

impl GlobalMemory {
    fn allocate(byte_length: usize) -> Result<Self, ClipboardPortError> {
        let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, byte_length) }
            .map_err(|_| ClipboardPortError::Unavailable)?;
        Ok(Self {
            handle: Some(handle),
        })
    }

    fn handle(&self) -> HGLOBAL {
        self.handle.expect("global memory handle")
    }

    fn write_utf16(&mut self, value: &[u16]) -> Result<(), ClipboardPortError> {
        let handle = self.handle();
        let destination = unsafe { GlobalLock(handle) }.cast::<u16>();
        if destination.is_null() {
            return Err(ClipboardPortError::Unavailable);
        }
        unsafe { std::ptr::copy_nonoverlapping(value.as_ptr(), destination, value.len()) };
        let _ = unsafe { GlobalUnlock(handle) };
        Ok(())
    }

    fn transfer_to_clipboard(&mut self) {
        self.handle = None;
    }
}

impl Drop for GlobalMemory {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = unsafe { GlobalFree(Some(handle)) };
        }
    }
}
