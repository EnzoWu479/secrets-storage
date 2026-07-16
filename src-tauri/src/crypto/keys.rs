//! Chave-raiz e derivação de subchaves via HKDF-SHA256.
//!
//! A `root_key` é o material de mais alto nível de uma sessão; dela derivam-se
//! subchaves **por propósito e época** com rótulos `info` distintos, de modo que
//! nenhuma chave seja reutilizada entre contextos (KEY-01/KEY-02). `K_sessao`
//! (GKEY-02) é derivada da GMK global por UUID de sessão.
//!
//! Toda entrada aleatória (`generate_root_key`) chega **por parâmetro**: a geração
//! via CSPRNG fica na borda de produção, mantendo o núcleo determinístico e testável.

use hkdf::Hkdf;
use sha2::Sha256;

use crate::crypto::Key32;

/// Rótulo de domínio das chaves de conteúdo (versionado).
const CONTENT_LABEL: &[u8] = b"ssv:content:v1:";
/// Rótulo de domínio das chaves de envelopamento de sessão global (versionado).
const SESSION_WRAP_LABEL: &[u8] = b"ssv:session-wrap:v1:";

/// Materializa a `root_key` a partir de 32 bytes já aleatórios (injetáveis).
///
/// A responsabilidade de gerar `rand` via CSPRNG é do chamador de produção; aqui
/// a função é uma passagem determinística para permitir vetores reproduzíveis.
pub fn generate_root_key(rand: &[u8; 32]) -> Key32 {
    Key32::from_bytes(*rand)
}

/// Deriva a chave de conteúdo da época `epoch` a partir da `root`.
///
/// `info = "ssv:content:v1:" ‖ epoch_be`. Épocas distintas → chaves distintas (ROT-01).
pub fn derive_content_key(root: &Key32, epoch: u32) -> Key32 {
    let mut info = Vec::with_capacity(CONTENT_LABEL.len() + 4);
    info.extend_from_slice(CONTENT_LABEL);
    info.extend_from_slice(&epoch.to_be_bytes());
    expand32(root.as_bytes(), &info)
}

/// Deriva a chave de envelopamento (`K_sessao`) de uma sessão global a partir da GMK.
///
/// `info = "ssv:session-wrap:v1:" ‖ uuid`. UUIDs distintos → chaves distintas (GKEY-02).
pub fn derive_session_wrap_key(gmk: &Key32, uuid: &[u8; 16]) -> Key32 {
    let mut info = Vec::with_capacity(SESSION_WRAP_LABEL.len() + 16);
    info.extend_from_slice(SESSION_WRAP_LABEL);
    info.extend_from_slice(uuid);
    expand32(gmk.as_bytes(), &info)
}

/// HKDF-SHA256 (extract sem salt + expand) para 32 bytes de saída, com `info` de domínio.
fn expand32(ikm: &[u8; 32], info: &[u8]) -> Key32 {
    let hk = Hkdf::<Sha256>::new(None, ikm);
    let mut okm = [0u8; 32];
    hk.expand(info, &mut okm)
        .expect("32 bytes é um comprimento de saída válido para HKDF-SHA256");
    Key32::from_bytes(okm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_key_reflete_os_bytes_de_entrada() {
        let seed = [9u8; 32];
        let root = generate_root_key(&seed);
        assert_eq!(root.as_bytes(), &seed);
    }

    #[test]
    fn content_key_e_deterministica() {
        let root = generate_root_key(&[1u8; 32]);
        let a = derive_content_key(&root, 0);
        let b = derive_content_key(&root, 0);
        assert_eq!(a.as_bytes(), b.as_bytes());
    }

    #[test]
    fn content_key_muda_com_a_epoca() {
        let root = generate_root_key(&[1u8; 32]);
        let e0 = derive_content_key(&root, 0);
        let e1 = derive_content_key(&root, 1);
        assert_ne!(e0.as_bytes(), e1.as_bytes());
    }

    #[test]
    fn session_wrap_key_e_deterministica_e_muda_com_o_uuid() {
        let gmk = generate_root_key(&[2u8; 32]);
        let uuid_a = [0xAAu8; 16];
        let uuid_b = [0xBBu8; 16];
        let a1 = derive_session_wrap_key(&gmk, &uuid_a);
        let a2 = derive_session_wrap_key(&gmk, &uuid_a);
        let b = derive_session_wrap_key(&gmk, &uuid_b);
        assert_eq!(a1.as_bytes(), a2.as_bytes());
        assert_ne!(a1.as_bytes(), b.as_bytes());
    }

    #[test]
    fn propositos_distintos_geram_subchaves_distintas() {
        // Mesmo material de entrada, rótulos de propósito diferentes → chaves diferentes.
        let material = generate_root_key(&[7u8; 32]);
        let uuid = [0u8; 16];
        // content_key(epoch=0) vs session_wrap_key(uuid=0): rótulos distintos.
        let content = derive_content_key(&material, 0);
        let session = derive_session_wrap_key(&material, &uuid);
        assert_ne!(content.as_bytes(), session.as_bytes());
    }
}
