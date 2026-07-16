//! Envelope de sessão (`<uuid>.vault`): cofre CBOR versionado e autenticado.
//!
//! A `root_key` (aleatória, raiz da sessão) é envolvida por uma chave que depende do
//! `auth_mode` (GKEY-02): em `own`, `KEK = Argon2id(senha_propria, salt)`; em `global`,
//! `K_sessao = HKDF(GMK, uuid)`. O conteúdo é cifrado com `content_key = HKDF(root_key, epoch)`.
//!
//! Toda operação AEAD usa os **bytes CBOR do header** como AAD: qualquer alteração de
//! versão, uuid, `auth_mode`, params, salt, época ou nome quebra a autenticação (AEAD-01).
//! Como `auth_mode` entra na AAD, rebaixar `own`↔`global` sem a chave de destino falha.
//! Versão de formato superior à suportada é **fail-closed** (FMT-02); campos desconhecidos
//! no header são preservados por serem carregados como bytes opacos (FMT-03).

use serde::{Deserialize, Serialize};

use crate::crypto::codec::{from_cbor, to_cbor, KdfDescriptor, WrapField, AEAD_XCHACHA20POLY1305};
use crate::crypto::kdf::{self, KdfParams};
use crate::crypto::{aead, keys, CryptoError, Key32, Result};

/// Magic do cofre de sessão (rejeita arquivos que não são cofre).
const VAULT_MAGIC: [u8; 4] = *b"SSV1";
/// Versão corrente do formato do cofre (FMT-01).
pub const VAULT_FORMAT_VERSION: u16 = 1;

/// `auth_mode`: sessão global — `root_key` envolvida por `K_sessao` derivada da GMK.
pub const AUTH_MODE_GLOBAL: u8 = 0;
/// `auth_mode`: sessão com senha própria — `root_key` envolvida por `KEK` da senha.
pub const AUTH_MODE_OWN: u8 = 1;

/// Cabeçalho autenticado do cofre de sessão.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct Header {
    magic: [u8; 4],
    format_version: u16,
    session_uuid: [u8; 16],
    auth_mode: u8,
    kdf: KdfDescriptor,
    salt: [u8; 16],
    aead_id: u8,
    epoch: u32,
    session_name: String,
}

/// Envelope persistido: header opaco (AAD) + `root_key` envolvida + payload cifrado.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct VaultEnvelope {
    /// Bytes CBOR do [`Header`]; AAD de `key_wrap` e `payload`.
    header_bytes: Vec<u8>,
    /// `root_key` envolvida conforme o `auth_mode`.
    key_wrap: WrapField,
    /// Conteúdo da sessão cifrado com a `content_key`.
    payload: WrapField,
}

/// Conteúdo da sessão. Mínimo nesta fatia: os segredos ficam abertos (`Value`) para
/// não fixar o modelo — a fatia de segredos preenche `secrets` sem mudar o envelope.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct SessionContent {
    /// Versão do formato do conteúdo (independe da versão do envelope).
    pub content_format: u16,
    /// Lista de segredos; vazia nesta fatia.
    pub secrets: Vec<ciborium::value::Value>,
}

/// Nonces injetáveis de uma operação de cria/reenvelope (produção usa CSPRNG).
#[derive(Clone, Copy)]
pub struct VaultNonces {
    /// Nonce do `key_wrap` (envolvimento da `root_key`).
    pub key_wrap: [u8; 24],
    /// Nonce do `payload` (cifra do conteúdo).
    pub payload: [u8; 24],
}

/// Material para **envolver** a `root_key` (criação/reenvelope), conforme o `auth_mode`.
pub enum WrapAuth<'a> {
    /// Sessão global: envolve com `K_sessao` derivada da `gmk`.
    Global { gmk: &'a Key32 },
    /// Sessão com senha própria: envolve com `KEK = Argon2id(password, salt)`.
    Own {
        /// Senha própria da sessão.
        password: &'a [u8],
        /// Salt do Argon2id da sessão.
        salt: [u8; 16],
        /// Parâmetros do Argon2id.
        params: KdfParams,
    },
}

/// Material para **desbloquear** a `root_key`; salt/params vêm do header autenticado.
pub enum UnlockAuth<'a> {
    /// Sessão global: deriva `K_sessao` da `gmk` e do `uuid` do header.
    Global { gmk: &'a Key32 },
    /// Sessão com senha própria: deriva `KEK` da senha + salt/params do header.
    Own {
        /// Senha própria informada para o desbloqueio.
        password: &'a [u8],
    },
}

/// Cofre aberto: material de chave (não cruza o IPC) + conteúdo + metadados do header.
pub struct UnlockedVault {
    /// Raiz da sessão recuperada.
    pub root_key: Key32,
    /// Conteúdo decifrado.
    pub content: SessionContent,
    /// UUID da sessão (autenticado).
    pub session_uuid: [u8; 16],
    /// Nome da sessão (autenticado).
    pub session_name: String,
    /// Época corrente da `content_key`.
    pub epoch: u32,
    /// `auth_mode` autenticado do envelope.
    pub auth_mode: u8,
}

/// Cria um cofre novo com `root_key` derivada de `root_rand` (injetável).
pub fn create_vault(
    auth: WrapAuth<'_>,
    session_uuid: [u8; 16],
    session_name: &str,
    root_rand: &[u8; 32],
    epoch: u32,
    nonces: VaultNonces,
    content: &SessionContent,
) -> Result<VaultEnvelope> {
    let root_key = keys::generate_root_key(root_rand);
    assemble_vault(
        auth,
        session_uuid,
        session_name,
        &root_key,
        epoch,
        nonces,
        content,
    )
}

/// Desbloqueia o cofre, provando a chave de envelopamento conforme o `auth_mode`.
///
/// Senha/GMK errada — ou header adulterado, incl. rebaixamento de `auth_mode` —
/// resultam em [`CryptoError::Authentication`]. Versão superior → fail-closed.
pub fn unlock(auth: UnlockAuth<'_>, env: &VaultEnvelope) -> Result<UnlockedVault> {
    let header = parse_header(env)?;

    let wrap_key = match auth {
        UnlockAuth::Own { password } => {
            let params = header.kdf.to_params()?;
            kdf::derive_kek(password, &header.salt, params)?
        }
        UnlockAuth::Global { gmk } => keys::derive_session_wrap_key(gmk, &header.session_uuid),
    };

    let root_key = aead::unwrap_key(
        &wrap_key,
        &env.key_wrap.nonce,
        &env.key_wrap.ciphertext,
        &env.header_bytes,
    )?;

    let content_key = keys::derive_content_key(&root_key, header.epoch);
    let content_bytes = aead::open(
        &content_key,
        &env.payload.nonce,
        &env.payload.ciphertext,
        &env.header_bytes,
    )?;
    let content: SessionContent = from_cbor(&content_bytes)?;

    Ok(UnlockedVault {
        root_key,
        content,
        session_uuid: header.session_uuid,
        session_name: header.session_name,
        epoch: header.epoch,
        auth_mode: header.auth_mode,
    })
}

/// Reenvolve a **mesma** `root_key` sob nova chave (troca de senha / alternância de modo).
///
/// Prova a chave atual (`unlock`) antes de gravar (VAULT-04); preserva uuid/nome/época
/// e reautentica o payload sob o novo header. O conteúdo (plaintext) não muda.
pub fn rewrap(
    current: UnlockAuth<'_>,
    new: WrapAuth<'_>,
    nonces: VaultNonces,
    env: &VaultEnvelope,
) -> Result<VaultEnvelope> {
    let unlocked = unlock(current, env)?;
    assemble_vault(
        new,
        unlocked.session_uuid,
        &unlocked.session_name,
        &unlocked.root_key,
        unlocked.epoch,
        nonces,
        &unlocked.content,
    )
}

/// Monta o envelope a partir de uma `root_key` já materializada.
fn assemble_vault(
    auth: WrapAuth<'_>,
    session_uuid: [u8; 16],
    session_name: &str,
    root_key: &Key32,
    epoch: u32,
    nonces: VaultNonces,
    content: &SessionContent,
) -> Result<VaultEnvelope> {
    let (auth_mode, salt, kdf_desc) = match &auth {
        WrapAuth::Own { salt, params, .. } => {
            params.validate()?;
            (AUTH_MODE_OWN, *salt, KdfDescriptor::from_params(*params))
        }
        // Sessões global não usam salt/kdf; gravamos zeros + candidato como placeholder autenticado.
        WrapAuth::Global { .. } => (
            AUTH_MODE_GLOBAL,
            [0u8; 16],
            KdfDescriptor::from_params(KdfParams::CANDIDATE),
        ),
    };

    let header = Header {
        magic: VAULT_MAGIC,
        format_version: VAULT_FORMAT_VERSION,
        session_uuid,
        auth_mode,
        kdf: kdf_desc,
        salt,
        aead_id: AEAD_XCHACHA20POLY1305,
        epoch,
        session_name: session_name.to_owned(),
    };
    let header_bytes = to_cbor(&header)?;

    let wrapped_root = match &auth {
        WrapAuth::Own {
            password,
            salt,
            params,
        } => {
            let kek = kdf::derive_kek(password, salt, *params)?;
            aead::wrap_key(&kek, &nonces.key_wrap, root_key, &header_bytes)
        }
        WrapAuth::Global { gmk } => {
            let k_sessao = keys::derive_session_wrap_key(gmk, &session_uuid);
            aead::wrap_key(&k_sessao, &nonces.key_wrap, root_key, &header_bytes)
        }
    };

    let content_key = keys::derive_content_key(root_key, epoch);
    let content_bytes = to_cbor(content)?;
    let payload_ct = aead::seal(&content_key, &nonces.payload, &content_bytes, &header_bytes);

    Ok(VaultEnvelope {
        header_bytes,
        key_wrap: WrapField {
            nonce: nonces.key_wrap,
            ciphertext: wrapped_root,
        },
        payload: WrapField {
            nonce: nonces.payload,
            ciphertext: payload_ct,
        },
    })
}

/// Desserializa e valida o header (magic + versão) **antes** de qualquer decifra.
fn parse_header(env: &VaultEnvelope) -> Result<Header> {
    let header: Header = from_cbor(&env.header_bytes)?;
    if header.magic != VAULT_MAGIC {
        return Err(CryptoError::InvalidMagic);
    }
    if header.format_version > VAULT_FORMAT_VERSION {
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
    const UUID: [u8; 16] = [0xAB; 16];
    const SALT: [u8; 16] = [0x11; 16];
    const ROOT: [u8; 32] = [0x5A; 32];
    const NONCES: VaultNonces = VaultNonces {
        key_wrap: [0x01; 24],
        payload: [0x02; 24],
    };

    fn conteudo() -> SessionContent {
        SessionContent {
            content_format: 1,
            secrets: vec![],
        }
    }

    fn own_auth<'a>(password: &'a [u8]) -> WrapAuth<'a> {
        WrapAuth::Own {
            password,
            salt: SALT,
            params: TEST_PARAMS,
        }
    }

    fn vault_own() -> VaultEnvelope {
        create_vault(
            own_auth(b"senha-propria"),
            UUID,
            "trabalho",
            &ROOT,
            0,
            NONCES,
            &conteudo(),
        )
        .unwrap()
    }

    #[test]
    fn roundtrip_own_recupera_root_e_conteudo() {
        let env = vault_own();
        let aberto = unlock(
            UnlockAuth::Own {
                password: b"senha-propria",
            },
            &env,
        )
        .unwrap();
        assert_eq!(aberto.root_key.as_bytes(), &ROOT);
        assert_eq!(aberto.content, conteudo());
        assert_eq!(aberto.session_name, "trabalho");
        assert_eq!(aberto.auth_mode, AUTH_MODE_OWN);
    }

    #[test]
    fn roundtrip_global_recupera_root_e_conteudo() {
        let gmk = Key32::from_bytes([0xC0; 32]);
        let env = create_vault(
            WrapAuth::Global { gmk: &gmk },
            UUID,
            "global",
            &ROOT,
            0,
            NONCES,
            &conteudo(),
        )
        .unwrap();
        let aberto = unlock(UnlockAuth::Global { gmk: &gmk }, &env).unwrap();
        assert_eq!(aberto.root_key.as_bytes(), &ROOT);
        assert_eq!(aberto.content, conteudo());
        assert_eq!(aberto.auth_mode, AUTH_MODE_GLOBAL);
    }

    #[test]
    fn senha_errada_falha() {
        let env = vault_own();
        assert!(matches!(
            unlock(
                UnlockAuth::Own {
                    password: b"errada"
                },
                &env
            ),
            Err(CryptoError::Authentication)
        ));
    }

    #[test]
    fn gmk_errada_falha() {
        let gmk = Key32::from_bytes([0xC0; 32]);
        let env = create_vault(
            WrapAuth::Global { gmk: &gmk },
            UUID,
            "global",
            &ROOT,
            0,
            NONCES,
            &conteudo(),
        )
        .unwrap();
        let outra = Key32::from_bytes([0xC1; 32]);
        assert!(matches!(
            unlock(UnlockAuth::Global { gmk: &outra }, &env),
            Err(CryptoError::Authentication)
        ));
    }

    /// Reescreve o header (tipado) e mantém os wraps originais → autenticação deve falhar.
    fn com_header_adulterado(
        env: &VaultEnvelope,
        mutar: impl FnOnce(&mut Header),
    ) -> VaultEnvelope {
        let mut header: Header = from_cbor(&env.header_bytes).unwrap();
        mutar(&mut header);
        VaultEnvelope {
            header_bytes: to_cbor(&header).unwrap(),
            key_wrap: env.key_wrap.clone(),
            payload: env.payload.clone(),
        }
    }

    #[test]
    fn adulterar_campos_do_header_e_rejeitado() {
        let env = vault_own();
        let pw = UnlockAuth::Own {
            password: b"senha-propria",
        };

        let uuid_alt = com_header_adulterado(&env, |h| h.session_uuid = [0x00; 16]);
        let epoch_alt = com_header_adulterado(&env, |h| h.epoch = 99);
        let salt_alt = com_header_adulterado(&env, |h| h.salt = [0xFF; 16]);
        let nome_alt = com_header_adulterado(&env, |h| h.session_name = "outro".to_owned());

        for adulterado in [uuid_alt, epoch_alt, salt_alt, nome_alt] {
            assert!(matches!(
                unlock(
                    UnlockAuth::Own {
                        password: b"senha-propria"
                    },
                    &adulterado
                ),
                Err(CryptoError::Authentication)
            ));
        }
        // sanidade: o env original ainda abre
        assert!(unlock(pw, &env).is_ok());
    }

    #[test]
    fn rebaixar_own_para_global_sem_a_gmk_falha() {
        let env = vault_own();
        // Reescreve auth_mode → global e tenta abrir como global.
        let rebaixado = com_header_adulterado(&env, |h| h.auth_mode = AUTH_MODE_GLOBAL);
        let gmk = Key32::from_bytes([0x00; 32]);
        assert!(matches!(
            unlock(UnlockAuth::Global { gmk: &gmk }, &rebaixado),
            Err(CryptoError::Authentication)
        ));
    }

    #[test]
    fn rewrap_troca_de_senha_mantem_root_e_conteudo() {
        let env = vault_own();
        let novos_nonces = VaultNonces {
            key_wrap: [0x0A; 24],
            payload: [0x0B; 24],
        };
        let novo_salt = [0x33; 16];
        let env2 = rewrap(
            UnlockAuth::Own {
                password: b"senha-propria",
            },
            WrapAuth::Own {
                password: b"senha-nova",
                salt: novo_salt,
                params: TEST_PARAMS,
            },
            novos_nonces,
            &env,
        )
        .unwrap();

        let aberto = unlock(
            UnlockAuth::Own {
                password: b"senha-nova",
            },
            &env2,
        )
        .unwrap();
        assert_eq!(aberto.root_key.as_bytes(), &ROOT);
        assert_eq!(aberto.content, conteudo());
        // Senha antiga não abre o envelope reenvolvido.
        assert!(unlock(
            UnlockAuth::Own {
                password: b"senha-propria"
            },
            &env2
        )
        .is_err());
    }

    #[test]
    fn rewrap_alterna_own_para_global() {
        let env = vault_own();
        let gmk = Key32::from_bytes([0xC0; 32]);
        let env2 = rewrap(
            UnlockAuth::Own {
                password: b"senha-propria",
            },
            WrapAuth::Global { gmk: &gmk },
            NONCES,
            &env,
        )
        .unwrap();

        let aberto = unlock(UnlockAuth::Global { gmk: &gmk }, &env2).unwrap();
        assert_eq!(aberto.auth_mode, AUTH_MODE_GLOBAL);
        assert_eq!(aberto.root_key.as_bytes(), &ROOT);
        assert_eq!(aberto.content, conteudo());
    }

    #[test]
    fn versao_superior_e_fail_closed() {
        let env = vault_own();
        let futuro = com_header_adulterado(&env, |h| h.format_version = VAULT_FORMAT_VERSION + 1);
        assert!(matches!(
            unlock(
                UnlockAuth::Own {
                    password: b"senha-propria"
                },
                &futuro
            ),
            Err(CryptoError::UnsupportedVersion)
        ));
    }

    #[test]
    fn magic_invalido_e_rejeitado() {
        let env = vault_own();
        let alterado = com_header_adulterado(&env, |h| h.magic = *b"XXXX");
        assert!(matches!(
            unlock(
                UnlockAuth::Own {
                    password: b"senha-propria"
                },
                &alterado
            ),
            Err(CryptoError::InvalidMagic)
        ));
    }

    #[test]
    fn campos_desconhecidos_no_header_sao_preservados() {
        use ciborium::value::Value;

        // Header base tipado → Value (mapa CBOR), com um campo extra desconhecido.
        let base = Header {
            magic: VAULT_MAGIC,
            format_version: VAULT_FORMAT_VERSION,
            session_uuid: UUID,
            auth_mode: AUTH_MODE_OWN,
            kdf: KdfDescriptor::from_params(TEST_PARAMS),
            salt: SALT,
            aead_id: AEAD_XCHACHA20POLY1305,
            epoch: 0,
            session_name: "trabalho".to_owned(),
        };
        let mut val: Value = from_cbor(&to_cbor(&base).unwrap()).unwrap();
        if let Value::Map(entries) = &mut val {
            entries.push((Value::Text("flag_futura".to_owned()), Value::Bool(true)));
        } else {
            panic!("header deveria serializar como mapa CBOR");
        }
        let header_bytes = to_cbor(&val).unwrap();

        // Sela os wraps manualmente contra esses header_bytes (com o campo extra).
        let root = keys::generate_root_key(&ROOT);
        let kek = kdf::derive_kek(b"senha-propria", &SALT, TEST_PARAMS).unwrap();
        let key_ct = aead::wrap_key(&kek, &NONCES.key_wrap, &root, &header_bytes);
        let ck = keys::derive_content_key(&root, 0);
        let content = conteudo();
        let pay_ct = aead::seal(
            &ck,
            &NONCES.payload,
            &to_cbor(&content).unwrap(),
            &header_bytes,
        );
        let env = VaultEnvelope {
            header_bytes,
            key_wrap: WrapField {
                nonce: NONCES.key_wrap,
                ciphertext: key_ct,
            },
            payload: WrapField {
                nonce: NONCES.payload,
                ciphertext: pay_ct,
            },
        };

        // Round-trip do envelope inteiro (grava/lê) e desbloqueio funcionam...
        let disco = to_cbor(&env).unwrap();
        let env2: VaultEnvelope = from_cbor(&disco).unwrap();
        let aberto = unlock(
            UnlockAuth::Own {
                password: b"senha-propria",
            },
            &env2,
        )
        .unwrap();
        assert_eq!(aberto.content, content);

        // ...e o campo desconhecido continua presente no header após o round-trip.
        let hv: Value = from_cbor(&env2.header_bytes).unwrap();
        let tem_flag = matches!(hv, Value::Map(ref m)
            if m.iter().any(|(k, _)| matches!(k, Value::Text(t) if t == "flag_futura")));
        assert!(tem_flag, "campo desconhecido do header foi descartado");
    }
}
