//! Núcleo criptográfico do cofre (feature `crypto-format`, fatia Sessões + desbloqueio).
//!
//! Primitivos **candidatos** do design: Argon2id (KDF), XChaCha20-Poly1305 (AEAD),
//! HKDF-SHA256 (expansão) e CBOR (envelope versionado). Parâmetros numéricos de
//! KDF/AEAD são provisórios (`⚠️ PT-01/PT-02`); nada aqui certifica controles
//! enquanto o modelo de ameaças estiver em revisão (D-05).
//!
//! Fronteira de segurança: todo material de chave vive aqui, em tipos
//! zeroizáveis ([`secret::Key32`]); nada de chave/senha cruza o IPC.

pub mod error;
pub mod secret;

pub use error::{CryptoError, Result};
pub use secret::Key32;
