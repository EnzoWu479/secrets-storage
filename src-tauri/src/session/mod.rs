//! Camada de sessões (feature `local-sessions`): liga o núcleo `crypto::*` ao app.
//!
//! Orquestra o gate de senha mestra global (GMP), o ciclo de vida das sessões e o
//! armazenamento local, expondo uma superfície estreita de comandos Tauri. **Continua
//! candidato** enquanto o gate D-05 (modelo de ameaças) e PT-01/PT-02 estiverem abertos.

pub mod commands;
pub mod error;
pub mod manager;
pub mod model;
mod storage;

#[cfg(test)]
mod tests;

pub use error::SessionError;
pub use manager::SessionManager;
