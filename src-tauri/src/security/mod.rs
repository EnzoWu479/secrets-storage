//! Controles de segurança independentes da interface Tauri.

pub mod diagnostics;
pub mod lock;
#[cfg(all(windows, feature = "security-proof"))]
pub mod memory;
