//! Keyring global (`keyring.vault`): GMP → gKEK → GMK (GKEY-01).
//!
//! Guarda a raiz global (GMK) envolvida sob uma chave (gKEK) derivada da senha
//! mestra global (GMP) via Argon2id. A GMK é aleatória e independente da GMP; nem a
//! GMK nem a gKEK são persistidas em claro. Não há verificador barato da GMP: provar
//! a GMP = derivar a gKEK e tentar o *unwrap* da GMK (falha de autenticação ⇒ GMP errada).
//!
//! A AAD do wrap são os **bytes CBOR do header** armazenados no envelope: alterar
//! versão, params de KDF ou `salt_global` quebra a autenticação.

use serde::{Deserialize, Serialize};

use crate::crypto::codec::{from_cbor, to_cbor, KdfDescriptor, WrapField, AEAD_XCHACHA20POLY1305};
use crate::crypto::kdf::{self, KdfParams};
use crate::crypto::{aead, CryptoError, Key32, Result};

/// Magic do keyring (rejeita arquivos que não são keyring).
const KEYRING_MAGIC: [u8; 4] = *b"SSGK";
/// Versão corrente do formato do keyring (FMT-01).
pub const KEYRING_FORMAT_VERSION: u16 = 1;

/// Cabeçalho autenticado do keyring global.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct KeyringHeader {
    magic: [u8; 4],
    format_version: u16,
    kdf: KdfDescriptor,
    salt_global: [u8; 16],
    aead_id: u8,
}

/// Envelope do keyring: header (opaco, usado como AAD) + GMK envolvida.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct KeyringEnvelope {
    /// Bytes CBOR canônicos do [`KeyringHeader`]; AAD do `gmk_wrap`.
    header_bytes: Vec<u8>,
    /// GMK envolvida: `AEAD(gKEK, GMK, aad = header_bytes)`.
    gmk_wrap: WrapField,
}

/// Cria um keyring novo envolvendo a GMK aleatória sob a gKEK derivada da GMP.
///
/// `gmk_rand` e `nonce` chegam por parâmetro (injetáveis); em produção vêm do CSPRNG.
pub fn create_keyring(
    gmp: &[u8],
    salt_global: [u8; 16],
    params: KdfParams,
    gmk_rand: &[u8; 32],
    nonce: &[u8; 24],
) -> Result<KeyringEnvelope> {
    params.validate()?;

    let header = KeyringHeader {
        magic: KEYRING_MAGIC,
        format_version: KEYRING_FORMAT_VERSION,
        kdf: KdfDescriptor::from_params(params),
        salt_global,
        aead_id: AEAD_XCHACHA20POLY1305,
    };
    let header_bytes = to_cbor(&header)?;

    let gkek = kdf::derive_kek(gmp, &salt_global, params)?;
    let gmk = Key32::from_bytes(*gmk_rand);
    let ciphertext = aead::wrap_key(&gkek, nonce, &gmk, &header_bytes);

    Ok(KeyringEnvelope {
        header_bytes,
        gmk_wrap: WrapField {
            nonce: *nonce,
            ciphertext,
        },
    })
}

/// Recupera a GMK provando a GMP. GMP errada ⇒ [`CryptoError::Authentication`].
pub fn unwrap_gmk(gmp: &[u8], env: &KeyringEnvelope) -> Result<Key32> {
    let header = parse_header(env)?;
    let params = header.kdf.to_params()?;
    let gkek = kdf::derive_kek(gmp, &header.salt_global, params)?;
    aead::unwrap_key(
        &gkek,
        &env.gmk_wrap.nonce,
        &env.gmk_wrap.ciphertext,
        &env.header_bytes,
    )
}

/// Troca a GMP reenvolvendo a **mesma** GMK sob uma nova gKEK/salt.
///
/// Prova a GMP atual (unwrap da GMK); GMP antiga errada ⇒ erro. As sessões e seus
/// conteúdos não mudam — só o keyring é regravado.
pub fn change_gmp(
    old_gmp: &[u8],
    new_gmp: &[u8],
    new_salt_global: [u8; 16],
    params: KdfParams,
    nonce: &[u8; 24],
    env: &KeyringEnvelope,
) -> Result<KeyringEnvelope> {
    let gmk = unwrap_gmk(old_gmp, env)?;
    params.validate()?;

    let header = KeyringHeader {
        magic: KEYRING_MAGIC,
        format_version: KEYRING_FORMAT_VERSION,
        kdf: KdfDescriptor::from_params(params),
        salt_global: new_salt_global,
        aead_id: AEAD_XCHACHA20POLY1305,
    };
    let header_bytes = to_cbor(&header)?;

    let new_gkek = kdf::derive_kek(new_gmp, &new_salt_global, params)?;
    let ciphertext = aead::wrap_key(&new_gkek, nonce, &gmk, &header_bytes);

    Ok(KeyringEnvelope {
        header_bytes,
        gmk_wrap: WrapField {
            nonce: *nonce,
            ciphertext,
        },
    })
}

/// Desserializa e valida o header (magic + versão suportada) **antes** de autenticar.
fn parse_header(env: &KeyringEnvelope) -> Result<KeyringHeader> {
    let header: KeyringHeader = from_cbor(&env.header_bytes)?;
    if header.magic != KEYRING_MAGIC {
        return Err(CryptoError::InvalidMagic);
    }
    if header.format_version > KEYRING_FORMAT_VERSION {
        return Err(CryptoError::UnsupportedVersion);
    }
    if header.aead_id != AEAD_XCHACHA20POLY1305 {
        return Err(CryptoError::MalformedEnvelope);
    }
    Ok(header)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PARAMS: KdfParams = KdfParams {
        mem_kib: kdf::MIN_MEM_KIB,
        iters: 1,
        parallelism: 1,
    };
    const SALT: [u8; 16] = [0x11; 16];
    const GMK_RAND: [u8; 32] = [0xC0; 32];
    const NONCE: [u8; 24] = [0x77; 24];

    fn keyring() -> KeyringEnvelope {
        create_keyring(b"gmp-correta", SALT, TEST_PARAMS, &GMK_RAND, &NONCE).unwrap()
    }

    #[test]
    fn create_e_unwrap_recuperam_a_gmk() {
        let env = keyring();
        let gmk = unwrap_gmk(b"gmp-correta", &env).unwrap();
        assert_eq!(gmk.as_bytes(), &GMK_RAND);
    }

    #[test]
    fn gmp_errada_falha_na_autenticacao() {
        let env = keyring();
        assert!(matches!(
            unwrap_gmk(b"gmp-errada", &env),
            Err(CryptoError::Authentication)
        ));
    }

    #[test]
    fn change_gmp_preserva_a_mesma_gmk() {
        let env = keyring();
        let novo_salt = [0x22; 16];
        let novo_nonce = [0x88; 24];
        let env2 = change_gmp(
            b"gmp-correta",
            b"gmp-nova",
            novo_salt,
            TEST_PARAMS,
            &novo_nonce,
            &env,
        )
        .unwrap();

        // A nova GMP recupera a mesma GMK...
        let gmk = unwrap_gmk(b"gmp-nova", &env2).unwrap();
        assert_eq!(gmk.as_bytes(), &GMK_RAND);
        // ...e a GMP antiga não abre mais o keyring novo.
        assert!(unwrap_gmk(b"gmp-correta", &env2).is_err());
    }

    #[test]
    fn change_gmp_com_gmp_antiga_errada_falha() {
        let env = keyring();
        assert!(change_gmp(
            b"gmp-errada",
            b"gmp-nova",
            [0x22; 16],
            TEST_PARAMS,
            &[0x88; 24],
            &env
        )
        .is_err());
    }

    #[test]
    fn adulterar_o_header_quebra_a_autenticacao() {
        let mut env = keyring();
        // Reescreve o header com outro salt_global, mantendo o wrap original.
        let mut header: KeyringHeader = from_cbor(&env.header_bytes).unwrap();
        header.salt_global = [0xFF; 16];
        env.header_bytes = to_cbor(&header).unwrap();
        assert!(matches!(
            unwrap_gmk(b"gmp-correta", &env),
            Err(CryptoError::Authentication)
        ));
    }

    #[test]
    fn magic_invalido_e_rejeitado() {
        let mut env = keyring();
        let mut header: KeyringHeader = from_cbor(&env.header_bytes).unwrap();
        header.magic = *b"XXXX";
        env.header_bytes = to_cbor(&header).unwrap();
        assert!(matches!(
            unwrap_gmk(b"gmp-correta", &env),
            Err(CryptoError::InvalidMagic)
        ));
    }

    #[test]
    fn versao_superior_e_fail_closed() {
        let mut env = keyring();
        let mut header: KeyringHeader = from_cbor(&env.header_bytes).unwrap();
        header.format_version = KEYRING_FORMAT_VERSION + 1;
        env.header_bytes = to_cbor(&header).unwrap();
        assert!(matches!(
            unwrap_gmk(b"gmp-correta", &env),
            Err(CryptoError::UnsupportedVersion)
        ));
    }
}
