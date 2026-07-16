//! Serialização CBOR compartilhada pelos envelopes (`keyring`, `envelope`).
//!
//! Os envelopes carregam o **header como bytes CBOR opacos** (ver `keyring`/`envelope`):
//! esses mesmos bytes são a AAD de toda operação AEAD. Serializar uma única vez e
//! reusar os bytes armazenados como AAD (em vez de re-serializar o header tipado)
//! garante que a autenticação seja estável e que **campos desconhecidos** dentro do
//! header sejam preservados (FMT-03) — o parser tipado ignora o que não conhece, mas
//! os bytes originais permanecem intactos.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::crypto::kdf::KdfParams;
use crate::crypto::{CryptoError, Result};

/// Identificador do Argon2id no descritor de KDF persistido.
pub const KDF_ARGON2ID: u8 = 1;
/// Identificador do XChaCha20-Poly1305 no header persistido.
pub const AEAD_XCHACHA20POLY1305: u8 = 1;

/// Descritor serializável dos parâmetros de KDF (parte autenticada do header).
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct KdfDescriptor {
    /// Identificador do algoritmo (só [`KDF_ARGON2ID`] nesta fatia).
    pub id: u8,
    /// Custo de memória, em KiB.
    pub mem_kib: u32,
    /// Iterações (passes).
    pub iters: u32,
    /// Paralelismo (lanes).
    pub parallelism: u32,
}

impl KdfDescriptor {
    /// Constrói o descritor a partir de [`KdfParams`] (algoritmo fixo Argon2id).
    pub fn from_params(params: KdfParams) -> Self {
        Self {
            id: KDF_ARGON2ID,
            mem_kib: params.mem_kib,
            iters: params.iters,
            parallelism: params.parallelism,
        }
    }

    /// Reconstrói [`KdfParams`], rejeitando algoritmo desconhecido.
    pub fn to_params(self) -> Result<KdfParams> {
        if self.id != KDF_ARGON2ID {
            return Err(CryptoError::InvalidKdfParams);
        }
        Ok(KdfParams {
            mem_kib: self.mem_kib,
            iters: self.iters,
            parallelism: self.parallelism,
        })
    }
}

/// Campo de dado envolvido/cifrado: nonce de 24 bytes + `ciphertext ‖ tag`.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct WrapField {
    /// Nonce de 192 bits usado no AEAD.
    pub nonce: [u8; 24],
    /// Saída do AEAD (ciphertext concatenado com a tag).
    pub ciphertext: Vec<u8>,
}

/// Serializa `value` em CBOR.
pub fn to_cbor<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    ciborium::into_writer(value, &mut buf).map_err(|_| CryptoError::MalformedEnvelope)?;
    Ok(buf)
}

/// Desserializa CBOR em `T`, mapeando qualquer falha para [`CryptoError::MalformedEnvelope`].
pub fn from_cbor<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    ciborium::from_reader(bytes).map_err(|_| CryptoError::MalformedEnvelope)
}
