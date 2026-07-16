//! AEAD com XChaCha20-Poly1305: `seal`/`open` com AAD, e `wrap`/`unwrap` de chave.
//!
//! O nonce de 192 bits (24 bytes) chega **por parâmetro** (injetável). A decisão
//! final entre nonce aleatório e contador está presa a `⚠️ PT-02` — aqui o núcleo
//! apenas consome o nonce fornecido.
//!
//! `open` autentica **antes** de devolver qualquer byte: adulteração de ciphertext,
//! tag, nonce ou AAD, e chave incorreta, resultam todas em [`CryptoError::Authentication`]
//! (AEAD-02) — nunca em plaintext silencioso.

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use zeroize::Zeroize;

use crate::crypto::{CryptoError, Key32, Result};

/// Cifra `plaintext` autenticando `aad`, devolvendo `ciphertext ‖ tag`.
///
/// Infalível na prática: o XChaCha20-Poly1305 só rejeita plaintext além do limite
/// de tamanho da cifra (dezenas de GiB), fora do uso deste cofre.
pub fn seal(key: &Key32, nonce: &[u8; 24], plaintext: &[u8], aad: &[u8]) -> Vec<u8> {
    let cipher =
        XChaCha20Poly1305::new_from_slice(key.as_bytes()).expect("Key32 tem exatamente 32 bytes");
    cipher
        .encrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .expect("encrypt só falha com plaintext além do limite de tamanho do XChaCha20-Poly1305")
}

/// Decifra e autentica `ciphertext` (que inclui a tag) contra `aad`.
///
/// Retorna [`CryptoError::Authentication`] se algo (ct, tag, nonce, aad ou chave)
/// não conferir. Não distingue os casos, para não vazar informação.
pub fn open(key: &Key32, nonce: &[u8; 24], ciphertext: &[u8], aad: &[u8]) -> Result<Vec<u8>> {
    let cipher =
        XChaCha20Poly1305::new_from_slice(key.as_bytes()).expect("Key32 tem exatamente 32 bytes");
    cipher
        .decrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|_| CryptoError::Authentication)
}

/// Envolve (`wrap`) uma chave de 32 bytes sob `kek`, autenticando `aad`.
pub fn wrap_key(kek: &Key32, nonce: &[u8; 24], key: &Key32, aad: &[u8]) -> Vec<u8> {
    seal(kek, nonce, key.as_bytes(), aad)
}

/// Desenvolve (`unwrap`) uma chave de 32 bytes envolvida por [`wrap_key`].
///
/// Falha de autenticação (kek/aad errados ou adulteração) → [`CryptoError::Authentication`];
/// tamanho diferente de 32 → [`CryptoError::MalformedEnvelope`]. O buffer intermediário
/// com o material desenvolvido é zeroizado antes de retornar.
pub fn unwrap_key(kek: &Key32, nonce: &[u8; 24], wrapped: &[u8], aad: &[u8]) -> Result<Key32> {
    let mut plain = open(kek, nonce, wrapped, aad)?;
    let result = <[u8; 32]>::try_from(plain.as_slice())
        .map(Key32::from_bytes)
        .map_err(|_| CryptoError::MalformedEnvelope);
    plain.zeroize();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> Key32 {
        Key32::from_bytes([byte; 32])
    }

    const NONCE: [u8; 24] = [0x24; 24];

    #[test]
    fn roundtrip_recupera_o_plaintext() {
        let k = key(1);
        let ct = seal(&k, &NONCE, b"segredo importante", b"aad-do-header");
        let pt = open(&k, &NONCE, &ct, b"aad-do-header").unwrap();
        assert_eq!(pt, b"segredo importante");
    }

    #[test]
    fn adulterar_ciphertext_falha() {
        let k = key(1);
        let mut ct = seal(&k, &NONCE, b"segredo", b"aad");
        ct[0] ^= 0x01;
        assert!(matches!(
            open(&k, &NONCE, &ct, b"aad"),
            Err(CryptoError::Authentication)
        ));
    }

    #[test]
    fn adulterar_tag_falha() {
        let k = key(1);
        let mut ct = seal(&k, &NONCE, b"segredo", b"aad");
        let last = ct.len() - 1; // a tag são os 16 bytes finais
        ct[last] ^= 0x01;
        assert!(open(&k, &NONCE, &ct, b"aad").is_err());
    }

    #[test]
    fn nonce_errado_falha() {
        let k = key(1);
        let ct = seal(&k, &NONCE, b"segredo", b"aad");
        let outro_nonce = [0x99; 24];
        assert!(open(&k, &outro_nonce, &ct, b"aad").is_err());
    }

    #[test]
    fn aad_divergente_falha() {
        let k = key(1);
        let ct = seal(&k, &NONCE, b"segredo", b"aad-original");
        assert!(open(&k, &NONCE, &ct, b"aad-alterada").is_err());
    }

    #[test]
    fn chave_errada_falha() {
        let ct = seal(&key(1), &NONCE, b"segredo", b"aad");
        assert!(open(&key(2), &NONCE, &ct, b"aad").is_err());
    }

    #[test]
    fn wrap_unwrap_recupera_a_chave() {
        let kek = key(1);
        let alvo = key(0xAB);
        let wrapped = wrap_key(&kek, &NONCE, &alvo, b"header");
        let recuperada = unwrap_key(&kek, &NONCE, &wrapped, b"header").unwrap();
        assert_eq!(recuperada.as_bytes(), alvo.as_bytes());
    }

    #[test]
    fn unwrap_com_kek_errada_falha() {
        let wrapped = wrap_key(&key(1), &NONCE, &key(0xAB), b"header");
        assert!(matches!(
            unwrap_key(&key(2), &NONCE, &wrapped, b"header"),
            Err(CryptoError::Authentication)
        ));
    }
}
