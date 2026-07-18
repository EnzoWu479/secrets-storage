//! Tradução pura de mensagens Win32 para motivos de bloqueio.

use crate::security::lock::LockReason;
use std::collections::{HashMap, VecDeque};
use std::ffi::c_void;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use thiserror::Error;
use windows::core::w;
use windows::Win32::Foundation::{GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::RemoteDesktop::{
    WTSRegisterSessionNotification, WTSUnRegisterSessionNotification, NOTIFY_FOR_THIS_SESSION,
};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, PostMessageW,
    PostThreadMessageW, RegisterClassW, TranslateMessage, MSG, PBT_APMRESUMEAUTOMATIC,
    PBT_APMRESUMECRITICAL, PBT_APMRESUMESUSPEND, PBT_APMSUSPEND, WINDOW_EX_STYLE, WM_APP, WM_CLOSE,
    WM_DESTROY, WM_ENDSESSION, WM_POWERBROADCAST, WM_QUERYENDSESSION, WM_WTSSESSION_CHANGE,
    WNDCLASSW, WS_OVERLAPPED, WTS_SESSION_LOCK,
};
#[cfg(feature = "security-proof")]
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, IsWindowVisible, GWL_STYLE, WS_CHILD,
};

const SHUTDOWN_MESSAGE: u32 = WM_APP + 0x51;
const PROOF_MESSAGE: u32 = WM_APP + 0x52;
const RPC_S_INVALID_BINDING_CODE: u32 = 1702;
const WTS_MAX_ATTEMPTS: u32 = 3;

static WINDOW_CHANNELS: OnceLock<Mutex<HashMap<usize, WindowChannel>>> = OnceLock::new();
static CLASS_REGISTRATION: OnceLock<Result<(), u32>> = OnceLock::new();

struct WindowChannel {
    signals: Sender<LockReason>,
    proof_messages: VecDeque<(u32, usize)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetryAction {
    Retry,
    Stop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowFacts {
    pub is_top_level: bool,
    pub is_visible: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListenerRegistrationStatus {
    Active,
    Degraded { platform_code: u32 },
}

#[derive(Debug, Error)]
pub enum WindowsEventPumpError {
    #[error("Windows event pump startup failed with platform code {platform_code}")]
    Startup { platform_code: u32 },
    #[error("Windows event pump thread stopped before startup completed")]
    StartupChannelClosed,
    #[error("Windows event pump message post failed with platform code {platform_code}")]
    MessagePost { platform_code: u32 },
    #[error("Windows event pump message loop failed with platform code {platform_code}")]
    MessageLoop { platform_code: u32 },
    #[error("Windows event pump cleanup failed with platform code {platform_code}")]
    Cleanup { platform_code: u32 },
    #[error("Windows event pump thread panicked")]
    ThreadPanicked,
}

pub struct WindowsEventPump {
    window: usize,
    thread_id: u32,
    listener_registration_status: ListenerRegistrationStatus,
    thread: Option<JoinHandle<Result<(), WindowsEventPumpError>>>,
}

impl WindowsEventPump {
    pub fn start() -> Result<(Self, Receiver<LockReason>), WindowsEventPumpError> {
        let (signal_sender, signal_receiver) = mpsc::channel();
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("windows-security-events".into())
            .spawn(move || event_thread(signal_sender, ready_sender))
            .map_err(|_| WindowsEventPumpError::Startup { platform_code: 0 })?;

        match ready_receiver.recv() {
            Ok(Ok((window, thread_id, listener_registration_status))) => Ok((
                Self {
                    window,
                    thread_id,
                    listener_registration_status,
                    thread: Some(thread),
                },
                signal_receiver,
            )),
            Ok(Err(error)) => {
                let _ = thread.join();
                Err(error)
            }
            Err(_) => {
                let _ = thread.join();
                Err(WindowsEventPumpError::StartupChannelClosed)
            }
        }
    }

    pub fn shutdown(&mut self) -> Result<(), WindowsEventPumpError> {
        if self.thread.is_none() {
            return Ok(());
        }

        if let Err(error) = self.post_shutdown() {
            if self.thread.as_ref().is_some_and(JoinHandle::is_finished) {
                let thread = self
                    .thread
                    .take()
                    .expect("finished thread was present before joining");
                return thread
                    .join()
                    .map_err(|_| WindowsEventPumpError::ThreadPanicked)?;
            }
            return Err(error);
        }
        let thread = self
            .thread
            .take()
            .expect("thread presence was checked before posting shutdown");
        thread
            .join()
            .map_err(|_| WindowsEventPumpError::ThreadPanicked)?
    }

    fn post_shutdown(&self) -> Result<(), WindowsEventPumpError> {
        if post_message(self.window, SHUTDOWN_MESSAGE, 0).is_ok() {
            return Ok(());
        }
        // SAFETY: `thread_id` is captured by the event thread after its queue
        // and window are created, before the pump is returned to the caller.
        unsafe { PostThreadMessageW(self.thread_id, SHUTDOWN_MESSAGE, WPARAM(0), LPARAM(0)) }
            .map_err(|error| WindowsEventPumpError::MessagePost {
                platform_code: win32_code(&error),
            })
    }

    pub const fn listener_registration_status(&self) -> ListenerRegistrationStatus {
        self.listener_registration_status
    }

    #[cfg(feature = "security-proof")]
    pub fn post_for_proof(&self, message: u32, wparam: usize) -> Result<(), WindowsEventPumpError> {
        let mut channels = channel_registry()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let channel = channels
            .get_mut(&self.window)
            .ok_or(WindowsEventPumpError::MessagePost { platform_code: 0 })?;
        channel.proof_messages.push_back((message, wparam));
        if let Err(error) = post_message(self.window, PROOF_MESSAGE, 0) {
            let _ = channel.proof_messages.pop_back();
            return Err(error);
        }
        Ok(())
    }

    #[cfg(feature = "security-proof")]
    pub fn post_raw_for_proof(
        &self,
        message: u32,
        wparam: usize,
    ) -> Result<(), WindowsEventPumpError> {
        post_message(self.window, message, wparam)
    }

    #[cfg(feature = "security-proof")]
    pub fn window_facts_for_proof(&self) -> Result<WindowFacts, WindowsEventPumpError> {
        let window = hwnd_from_usize(self.window);
        // SAFETY: the pump owns a live HWND until shutdown joins its thread.
        let (is_top_level, is_visible) = unsafe {
            (
                GetWindowLongPtrW(window, GWL_STYLE) & WS_CHILD.0 as isize == 0,
                IsWindowVisible(window).as_bool(),
            )
        };
        Ok(WindowFacts {
            is_top_level,
            is_visible,
        })
    }
}

impl Drop for WindowsEventPump {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

#[cfg(feature = "security-proof")]
pub const fn wts_retry_action_for_proof(platform_code: u32, attempt: u32) -> RetryAction {
    wts_retry_action(platform_code, attempt)
}

const fn wts_retry_action(platform_code: u32, attempt: u32) -> RetryAction {
    if platform_code == RPC_S_INVALID_BINDING_CODE && attempt + 1 < WTS_MAX_ATTEMPTS {
        RetryAction::Retry
    } else {
        RetryAction::Stop
    }
}

fn post_message(window: usize, message: u32, wparam: usize) -> Result<(), WindowsEventPumpError> {
    // SAFETY: `window` is published only after CreateWindowExW succeeds.
    unsafe {
        PostMessageW(
            Some(hwnd_from_usize(window)),
            message,
            WPARAM(wparam),
            LPARAM(0),
        )
    }
    .map_err(|error| WindowsEventPumpError::MessagePost {
        platform_code: win32_code(&error),
    })
}

fn event_thread(
    signal_sender: Sender<LockReason>,
    ready_sender: mpsc::SyncSender<
        Result<(usize, u32, ListenerRegistrationStatus), WindowsEventPumpError>,
    >,
) -> Result<(), WindowsEventPumpError> {
    let result = create_event_window();
    let window = match result {
        Ok(window) => window,
        Err(error) => {
            let _ = ready_sender.send(Err(error));
            return Ok(());
        }
    };
    let window_key = hwnd_to_usize(window);
    channel_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(
            window_key,
            WindowChannel {
                signals: signal_sender,
                proof_messages: VecDeque::new(),
            },
        );

    let registration_status = register_wts_with_retry(window);
    if ready_sender
        .send(Ok((
            window_key,
            // SAFETY: called by the event thread itself.
            unsafe { GetCurrentThreadId() },
            registration_status,
        )))
        .is_err()
    {
        let _ = cleanup_window(window, registration_status);
        return Ok(());
    }

    // SAFETY: the message queue and window belong to this thread. Every
    // successful GetMessageW result is initialized before dispatch.
    unsafe {
        let mut message = MSG::default();
        loop {
            let result = GetMessageW(&mut message, None, 0, 0).0;
            if result == -1 {
                let platform_code = GetLastError().0;
                let _ = cleanup_window(window, registration_status);
                return Err(WindowsEventPumpError::MessageLoop { platform_code });
            }
            if result == 0 {
                break;
            }
            if message.message == SHUTDOWN_MESSAGE {
                break;
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }

    cleanup_window(window, registration_status)
}

fn create_event_window() -> Result<HWND, WindowsEventPumpError> {
    // SAFETY: the class definition and UTF-16 strings remain static for the
    // process lifetime; the callback follows the Win32 ABI.
    unsafe {
        let module = GetModuleHandleW(None).map_err(|error| WindowsEventPumpError::Startup {
            platform_code: win32_code(&error),
        })?;
        let instance = HINSTANCE(module.0);
        if let Err(platform_code) =
            *CLASS_REGISTRATION.get_or_init(|| register_window_class(instance))
        {
            return Err(WindowsEventPumpError::Startup { platform_code });
        }

        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("SecretsStorageSecurityEventPump_7F3A"),
            w!(""),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance),
            None,
        )
        .map_err(|error| WindowsEventPumpError::Startup {
            platform_code: win32_code(&error),
        })
    }
}

fn register_window_class(instance: HINSTANCE) -> Result<(), u32> {
    let class = WNDCLASSW {
        lpfnWndProc: Some(event_window_proc),
        hInstance: instance,
        lpszClassName: w!("SecretsStorageSecurityEventPump_7F3A"),
        ..Default::default()
    };
    // SAFETY: registration is executed once and the callback/class name have
    // process-static lifetimes.
    if unsafe { RegisterClassW(&class) } == 0 {
        // SAFETY: read immediately after the failed Win32 call.
        Err(unsafe { GetLastError().0 })
    } else {
        Ok(())
    }
}

fn register_wts_with_retry(window: HWND) -> ListenerRegistrationStatus {
    for attempt in 0..WTS_MAX_ATTEMPTS {
        // SAFETY: `window` is a live top-level window owned by this thread.
        match unsafe { WTSRegisterSessionNotification(window, NOTIFY_FOR_THIS_SESSION) } {
            Ok(()) => return ListenerRegistrationStatus::Active,
            Err(error) => {
                let code = win32_code(&error);
                if wts_retry_action(code, attempt) == RetryAction::Stop {
                    return ListenerRegistrationStatus::Degraded {
                        platform_code: code,
                    };
                }
                thread::sleep(Duration::from_millis(25 * (attempt as u64 + 1)));
            }
        }
    }
    unreachable!("the final WTS attempt always returns a status")
}

fn cleanup_window(
    window: HWND,
    registration_status: ListenerRegistrationStatus,
) -> Result<(), WindowsEventPumpError> {
    // SAFETY: both calls execute on the owning thread while HWND is live.
    let unregister_result = unsafe {
        if registration_status == ListenerRegistrationStatus::Active {
            WTSUnRegisterSessionNotification(window)
        } else {
            Ok(())
        }
    };
    channel_registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(&hwnd_to_usize(window));
    // SAFETY: the HWND remains owned by this thread until this final call.
    let destroy_result = unsafe { DestroyWindow(window) };

    if let Err(error) = unregister_result {
        return Err(WindowsEventPumpError::Cleanup {
            platform_code: win32_code(&error),
        });
    }
    destroy_result.map_err(|error| WindowsEventPumpError::Cleanup {
        platform_code: win32_code(&error),
    })
}

unsafe extern "system" fn event_window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_CLOSE {
        return LRESULT(0);
    }
    let window_key = hwnd_to_usize(window);
    let delivery = {
        let mut channels = channel_registry()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        channels.get_mut(&window_key).and_then(|channel| {
            let translated = if message == PROOF_MESSAGE {
                channel
                    .proof_messages
                    .pop_front()
                    .and_then(|(message, wparam)| lock_reason_from_windows_message(message, wparam))
            } else {
                lock_reason_from_windows_message(message, wparam.0)
            };
            translated.map(|reason| (channel.signals.clone(), reason))
        })
    };
    if let Some((sender, reason)) = delivery {
        let _ = sender.send(reason);
    }
    if message == WM_DESTROY {
        return LRESULT(0);
    }
    // SAFETY: unhandled messages are delegated to the system default proc.
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

fn channel_registry() -> &'static Mutex<HashMap<usize, WindowChannel>> {
    WINDOW_CHANNELS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn hwnd_to_usize(window: HWND) -> usize {
    window.0 as usize
}

fn hwnd_from_usize(window: usize) -> HWND {
    HWND(window as *mut c_void)
}

fn win32_code(error: &windows::core::Error) -> u32 {
    let hresult = error.code().0 as u32;
    if hresult & 0xffff_0000 == 0x8007_0000 {
        hresult & 0xffff
    } else {
        hresult
    }
}

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
