//! Tradução pura de mensagens Win32 para motivos de bloqueio.

use crate::security::lock::LockReason;
use windows::Win32::UI::WindowsAndMessaging::{
    PBT_APMRESUMEAUTOMATIC, PBT_APMRESUMECRITICAL, PBT_APMRESUMESUSPEND, PBT_APMSUSPEND,
    WM_ENDSESSION, WM_POWERBROADCAST, WM_QUERYENDSESSION, WM_WTSSESSION_CHANGE, WTS_SESSION_LOCK,
};

/// Returns the lock reason for a security-relevant Win32 message and `wParam` pair.
///
/// Unknown messages and unsupported `wParam` values return `None`; this translator never
/// produces an unlock action.
pub fn lock_reason_from_windows_message(message: u32, wparam: usize) -> Option<LockReason> {
    match message {
        WM_WTSSESSION_CHANGE if wparam == WTS_SESSION_LOCK as usize => {
            Some(LockReason::SessionLocked)
        }
        WM_POWERBROADCAST if wparam == PBT_APMSUSPEND as usize => Some(LockReason::Suspending),
        WM_POWERBROADCAST
            if matches!(
                wparam,
                value if value == PBT_APMRESUMEAUTOMATIC as usize
                    || value == PBT_APMRESUMESUSPEND as usize
                    || value == PBT_APMRESUMECRITICAL as usize
            ) =>
        {
            Some(LockReason::Resumed)
        }
        WM_QUERYENDSESSION | WM_ENDSESSION => Some(LockReason::ShuttingDown),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::lock::LockReason;
    use windows::Win32::UI::WindowsAndMessaging::{
        PBT_APMRESUMEAUTOMATIC, PBT_APMRESUMECRITICAL, PBT_APMRESUMESUSPEND, PBT_APMSUSPEND,
        WM_ENDSESSION, WM_POWERBROADCAST, WM_QUERYENDSESSION, WM_WTSSESSION_CHANGE,
        WTS_SESSION_LOCK,
    };

    #[test]
    fn translates_security_relevant_windows_messages() {
        let cases = [
            (
                "session lock",
                WM_WTSSESSION_CHANGE,
                WTS_SESSION_LOCK as usize,
                Some(LockReason::SessionLocked),
            ),
            (
                "suspending",
                WM_POWERBROADCAST,
                PBT_APMSUSPEND as usize,
                Some(LockReason::Suspending),
            ),
            (
                "automatic resume",
                WM_POWERBROADCAST,
                PBT_APMRESUMEAUTOMATIC as usize,
                Some(LockReason::Resumed),
            ),
            (
                "suspend resume",
                WM_POWERBROADCAST,
                PBT_APMRESUMESUSPEND as usize,
                Some(LockReason::Resumed),
            ),
            (
                "critical resume",
                WM_POWERBROADCAST,
                PBT_APMRESUMECRITICAL as usize,
                Some(LockReason::Resumed),
            ),
            (
                "query end session",
                WM_QUERYENDSESSION,
                0,
                Some(LockReason::ShuttingDown),
            ),
            (
                "end session",
                WM_ENDSESSION,
                0,
                Some(LockReason::ShuttingDown),
            ),
            ("other WTS event", WM_WTSSESSION_CHANGE, 0, None),
            ("other power event", WM_POWERBROADCAST, 0, None),
            ("unknown message", 0, 0, None),
        ];

        for (name, message, wparam, expected) in cases {
            assert_eq!(
                lock_reason_from_windows_message(message, wparam),
                expected,
                "{name}"
            );
        }
    }
}
