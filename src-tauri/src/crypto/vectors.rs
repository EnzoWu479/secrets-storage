//! Vetores de teste determinísticos do formato criptográfico (TEST-01).
//!
//! Entradas **fixas** (params Argon2id reduzidos, salts/nonces/rands constantes) →
//! saídas **reproduzíveis** em hex. Servem de referência para uma releitura
//! independente (TEST-02): qualquer implementação do design candidato, alimentada com
//! estas entradas, deve produzir exatamente estas saídas. Também exercitam a
//! **rejeição autenticada** sob adulteração.
//!
//! ⚠️ Os valores golden dependem dos parâmetros **candidatos** (PT-01/PT-02) e do
//! layout CBOR (`⚠️ §12 #4`); mudarão se esses forem revisados — é o comportamento
//! esperado de um vetor de regressão.

use crate::crypto::codec::{from_cbor, to_cbor};
use crate::crypto::envelope::{
    create_vault, unlock, SessionContent, UnlockAuth, VaultEnvelope, VaultNonces, WrapAuth,
};
use crate::crypto::kdf::{self, KdfParams};
use crate::crypto::keyring::{create_keyring, unwrap_gmk, KeyringEnvelope};
use crate::crypto::{aead, keys, CryptoError, Key32};

// ---- Entradas fixas -------------------------------------------------------

const PARAMS: KdfParams = KdfParams {
    mem_kib: kdf::MIN_MEM_KIB,
    iters: 1,
    parallelism: 1,
};
const GMP: &[u8] = b"gmp-mestra-global";
const SALT_GLOBAL: [u8; 16] = [0x10; 16];
const GMK_RAND: [u8; 32] = [0xC0; 32];
const KEYRING_NONCE: [u8; 24] = [0x70; 24];
const PASSWORD: &[u8] = b"senha-da-sessao";
const SESSION_SALT: [u8; 16] = [0x20; 16];
const ROOT_RAND: [u8; 32] = [0x50; 32];
const UUID: [u8; 16] = [0xAB; 16];
const AEAD_NONCE: [u8; 24] = [0x33; 24];
const AEAD_PLAINTEXT: &[u8] = b"vetor-de-referencia";
const AEAD_AAD: &[u8] = b"aad-fixa";
const NONCES: VaultNonces = VaultNonces {
    key_wrap: [0x01; 24],
    payload: [0x02; 24],
};

// ---- Saídas esperadas (golden) --------------------------------------------

const GKEK: &str = "5591e9fc1ec51020b0a73e1c8efc9a2ce30b1c119be338b0bc065f63c6e508e3";
const KEK: &str = "5f8f7634e67172248d95c3be7b69c179613f2be48ab2f51e5d5145deaff14d20";
const KSESSAO: &str = "7e5874776a6067517435dd4febf857e7e7d93ddaa870320fc8a6650f010b58e7";
const CONTENT_KEY: &str = "fa8d5e13a22393cbfbf9ebbdb43e1e3c40a36c9fbdb338cc999e89dc44096b50";
const AEAD_CT: &str = "ab88616b0f3a47942688f83fcf2230f3827576c03c78451d91b618749f5e663e343b2b";
const KEYRING: &str = "a26c6865616465725f6279746573986e18a51865186d186118671869186318841818185318181853181818471818184b186e1866186f1872186d18611874185f18761865187218731869186f186e011863186b1864186618a4186218691864011867186d1865186d185f186b18691862181918200018651869187418651872187301186b1870186118721861186c186c1865186c18691873186d01186b18731861186c1874185f1867186c186f18621861186c18901010101010101010101010101010101018671861186518611864185f186918640168676d6b5f77726170a2656e6f6e636598181870187018701870187018701870187018701870187018701870187018701870187018701870187018701870187018706a636970686572746578749830189e18521518a81894182f182f18fe18b318d40f16189a18a918b51833189d18941870182c18e7186f1881184818661618fb111318d20f18c718ee186d182e1881183518c7182e185c18e5181b188218ea1869184e186018f4";
const VAULT_OWN: &str = "a36c6865616465725f627974657398cb18a91865186d1861186718691863188418181853181818531818185618181831186e1866186f1872186d18611874185f18761865187218731869186f186e01186c18731865187318731869186f186e185f18751875186918641890181818ab181818ab181818ab181818ab181818ab181818ab181818ab181818ab181818ab181818ab181818ab181818ab181818ab181818ab181818ab181818ab18691861187518741868185f186d186f18641865011863186b1864186618a4186218691864011867186d1865186d185f186b18691862181918200018651869187418651872187301186b1870186118721861186c186c1865186c18691873186d01186418731861186c187418901818182018181820181818201818182018181820181818201818182018181820181818201818182018181820181818201818182018181820181818201818182018671861186518611864185f1869186401186518651870186f1863186800186c18731865187318731869186f186e185f186e1861186d1865186618731865187318731861186f686b65795f77726170a2656e6f6e636598180101010101010101010101010101010101010101010101016a6369706865727465787498301882189c188518ff187918c118ec189018cc18a3182c185618561828182b18ed1844187018c418ef18b71899181f183c1890181a185f182b1518b8182718381890181b0f1899189a188c18f40818b518f5182d1618ad182a18ec1818677061796c6f6164a2656e6f6e636598180202020202020202020202020202020202020202020202026a63697068657274657874982a181b187618670b1824188318e9187418e9183e1895184d184a18df183918a418bf185b183418b51835181d1218dd1899189018581848188d18641860189c18551218a518e01885187c18a91846185e181a";
const VAULT_GLOBAL: &str = "a36c6865616465725f627974657398bd18a91865186d1861186718691863188418181853181818531818185618181831186e1866186f1872186d18611874185f18761865187218731869186f186e01186c18731865187318731869186f186e185f18751875186918641890181818ab181818ab181818ab181818ab181818ab181818ab181818ab181818ab181818ab181818ab181818ab181818ab181818ab181818ab181818ab181818ab18691861187518741868185f186d186f18641865001863186b1864186618a4186218691864011867186d1865186d185f186b18691862181a0001000018651869187418651872187303186b1870186118721861186c186c1865186c18691873186d01186418731861186c187418900000000000000000000000000000000018671861186518611864185f1869186401186518651870186f1863186800186c18731865187318731869186f186e185f186e1861186d1865186618731865187318731861186f686b65795f77726170a2656e6f6e636598180101010101010101010101010101010101010101010101016a63697068657274657874983018c418ef181d18a618af090818a718ca02182c18c918cd183818b8183d18eb181c184e1869189818b918b518ed18ed1879185e183c18f118d418ac18b6189e1847184b0c18a418ea1891182d1885184c189c188b011887184811677061796c6f6164a2656e6f6e636598180202020202020202020202020202020202020202020202026a63697068657274657874982a181b187618670b1824188318e9187418e9183e1895184d184a18df183918a418bf185b183418b51835181d1218dd1899189018d01893184b18951872186118b718fc183c188c187b185518dc18f2184918f0";

// ---- Helpers --------------------------------------------------------------

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn conteudo() -> SessionContent {
    SessionContent {
        content_format: 1,
        secrets: vec![],
    }
}

fn keyring() -> KeyringEnvelope {
    create_keyring(GMP, SALT_GLOBAL, PARAMS, &GMK_RAND, &KEYRING_NONCE).unwrap()
}

fn vault_own() -> VaultEnvelope {
    create_vault(
        WrapAuth::Own {
            password: PASSWORD,
            salt: SESSION_SALT,
            params: PARAMS,
        },
        UUID,
        "sessao",
        &ROOT_RAND,
        0,
        NONCES,
        &conteudo(),
    )
    .unwrap()
}

fn vault_global(gmk: &Key32) -> VaultEnvelope {
    create_vault(
        WrapAuth::Global { gmk },
        UUID,
        "sessao",
        &ROOT_RAND,
        0,
        NONCES,
        &conteudo(),
    )
    .unwrap()
}

/// Copia os bytes serializados e inverte 1 bit do **último** byte (dentro do
/// ciphertext/tag do último campo). Nos vetores golden o último elemento CBOR é
/// `18 XX` com `XX ≥ 24`, então a estrutura permanece válida e a falha é de
/// **autenticação** (não de parsing).
fn com_ultimo_byte_adulterado(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    let last = v.len() - 1;
    v[last] ^= 0x01;
    v
}

// ---- Vetores: primitivos --------------------------------------------------

#[test]
fn vetor_gkek_argon2id() {
    let gkek = kdf::derive_kek(GMP, &SALT_GLOBAL, PARAMS).unwrap();
    assert_eq!(hex(gkek.as_bytes()), GKEK);
}

#[test]
fn vetor_kek_propria_argon2id() {
    let kek = kdf::derive_kek(PASSWORD, &SESSION_SALT, PARAMS).unwrap();
    assert_eq!(hex(kek.as_bytes()), KEK);
}

#[test]
fn vetor_k_sessao_hkdf() {
    let gmk = Key32::from_bytes(GMK_RAND);
    let k = keys::derive_session_wrap_key(&gmk, &UUID);
    assert_eq!(hex(k.as_bytes()), KSESSAO);
}

#[test]
fn vetor_content_key_hkdf() {
    let root = keys::generate_root_key(&ROOT_RAND);
    let ck = keys::derive_content_key(&root, 0);
    assert_eq!(hex(ck.as_bytes()), CONTENT_KEY);
}

#[test]
fn vetor_aead_ciphertext() {
    let gkek = kdf::derive_kek(GMP, &SALT_GLOBAL, PARAMS).unwrap();
    let ct = aead::seal(&gkek, &AEAD_NONCE, AEAD_PLAINTEXT, AEAD_AAD);
    assert_eq!(hex(&ct), AEAD_CT);
    // roundtrip confere
    let pt = aead::open(&gkek, &AEAD_NONCE, &ct, AEAD_AAD).unwrap();
    assert_eq!(pt, AEAD_PLAINTEXT);
}

// ---- Vetores: envelopes completos (serialização CBOR) ---------------------

#[test]
fn vetor_keyring_serializado() {
    assert_eq!(hex(&to_cbor(&keyring()).unwrap()), KEYRING);
}

#[test]
fn vetor_vault_own_serializado() {
    assert_eq!(hex(&to_cbor(&vault_own()).unwrap()), VAULT_OWN);
}

#[test]
fn vetor_vault_global_serializado() {
    let gmk = Key32::from_bytes(GMK_RAND);
    assert_eq!(hex(&to_cbor(&vault_global(&gmk)).unwrap()), VAULT_GLOBAL);
}

// ---- Vetores: recuperação -------------------------------------------------

#[test]
fn recupera_gmk_do_keyring() {
    let gmk = unwrap_gmk(GMP, &keyring()).unwrap();
    assert_eq!(gmk.as_bytes(), &GMK_RAND);
}

#[test]
fn recupera_root_e_conteudo_own() {
    let aberto = unlock(UnlockAuth::Own { password: PASSWORD }, &vault_own()).unwrap();
    assert_eq!(aberto.root_key.as_bytes(), &ROOT_RAND);
    assert_eq!(aberto.content, conteudo());
}

#[test]
fn recupera_root_e_conteudo_global() {
    let gmk = Key32::from_bytes(GMK_RAND);
    let aberto = unlock(UnlockAuth::Global { gmk: &gmk }, &vault_global(&gmk)).unwrap();
    assert_eq!(aberto.root_key.as_bytes(), &ROOT_RAND);
    assert_eq!(aberto.content, conteudo());
}

// ---- Vetores: adulteração → rejeição autenticada --------------------------

#[test]
fn adulteracao_keyring_e_rejeitada() {
    let bytes = com_ultimo_byte_adulterado(&to_cbor(&keyring()).unwrap());
    let env: KeyringEnvelope = from_cbor(&bytes).unwrap();
    assert!(matches!(
        unwrap_gmk(GMP, &env),
        Err(CryptoError::Authentication)
    ));
}

#[test]
fn adulteracao_vault_own_e_rejeitada() {
    let bytes = com_ultimo_byte_adulterado(&to_cbor(&vault_own()).unwrap());
    let env: VaultEnvelope = from_cbor(&bytes).unwrap();
    assert!(matches!(
        unlock(UnlockAuth::Own { password: PASSWORD }, &env),
        Err(CryptoError::Authentication)
    ));
}

#[test]
fn adulteracao_vault_global_e_rejeitada() {
    let gmk = Key32::from_bytes(GMK_RAND);
    let bytes = com_ultimo_byte_adulterado(&to_cbor(&vault_global(&gmk)).unwrap());
    let env: VaultEnvelope = from_cbor(&bytes).unwrap();
    assert!(matches!(
        unlock(UnlockAuth::Global { gmk: &gmk }, &env),
        Err(CryptoError::Authentication)
    ));
}
