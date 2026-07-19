use serde::Deserialize;
use thiserror::Error;
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::secrets::move_state::MoveState;

pub const SECRET_RECORD_VERSION: u16 = 1;
pub const MAX_NAME_BYTES: usize = 256;
pub const MAX_METADATA_BYTES: usize = 4 * 1024;
pub const MAX_SENSITIVE_VALUE_BYTES: usize = 1024 * 1024;
pub const MAX_SERIALIZED_RECORD_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_SCOPES: usize = 128;
pub const MAX_SCOPE_BYTES: usize = 256;
pub const MAX_PAGE_SIZE: usize = 100;
pub const MAX_SEARCH_QUERY_BYTES: usize = 512;
pub const MAX_RECORDS_PER_SESSION: usize = 10_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretKind {
    Password,
    ApiKey,
    Token,
    SecureNote,
    SshKey,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SecretError {
    #[error("entrada de segredo inválida")]
    InvalidInput,
    #[error("revisão do segredo está obsoleta")]
    RevisionConflict,
    #[error("segredo não encontrado")]
    NotFound,
    #[error("limite de segredos da sessão atingido")]
    CapacityExceeded,
    #[error("identificador de segredo já existe")]
    IdCollision,
}

pub struct SecretText(Zeroizing<String>);

impl SecretText {
    fn validated(value: String, allow_empty: bool) -> Result<Self, SecretError> {
        validate_text(&value, MAX_SENSITIVE_VALUE_BYTES, allow_empty)?;
        Ok(Self(Zeroizing::new(value)))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Clone for SecretText {
    fn clone(&self) -> Self {
        Self(Zeroizing::new(self.as_str().to_owned()))
    }
}

#[derive(Clone)]
pub enum SecretDataV1 {
    Password {
        username: String,
        password: SecretText,
        url: Option<String>,
        notes: Option<SecretText>,
    },
    ApiKey {
        key: SecretText,
        environment: Option<String>,
        scopes: Vec<String>,
    },
    Token {
        value: SecretText,
        expires_at: Option<String>,
        notes: Option<SecretText>,
    },
    SecureNote {
        text: SecretText,
    },
    SshKey {
        public_key: Option<String>,
        private_key: SecretText,
        passphrase: Option<SecretText>,
    },
}

impl SecretDataV1 {
    pub fn kind(&self) -> SecretKind {
        match self {
            Self::Password { .. } => SecretKind::Password,
            Self::ApiKey { .. } => SecretKind::ApiKey,
            Self::Token { .. } => SecretKind::Token,
            Self::SecureNote { .. } => SecretKind::SecureNote,
            Self::SshKey { .. } => SecretKind::SshKey,
        }
    }
}

#[derive(Clone)]
pub struct SecretRecordV1 {
    pub version: u16,
    pub id: Uuid,
    pub revision: u64,
    pub name: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub move_state: Option<MoveState>,
    pub data: SecretDataV1,
}

impl SecretRecordV1 {
    pub fn kind(&self) -> SecretKind {
        self.data.kind()
    }

    #[cfg(test)]
    fn new_for_test(id: Uuid, revision: u64, name: String, data: SecretDataInput) -> Self {
        Self {
            version: SECRET_RECORD_VERSION,
            id,
            revision,
            name,
            created_at_ms: 0,
            updated_at_ms: 0,
            move_state: None,
            data: validate_data(data).expect("fixture de teste válida"),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateSecretInput {
    pub name: String,
    pub data: SecretDataInput,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecretPatchInput {
    pub name: Option<String>,
    pub data: Option<SecretDataInput>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum SecretDataInput {
    Password {
        username: String,
        password: String,
        url: Option<String>,
        notes: Option<String>,
    },
    ApiKey {
        key: String,
        environment: Option<String>,
        scopes: Vec<String>,
    },
    Token {
        value: String,
        expires_at: Option<String>,
        notes: Option<String>,
    },
    SecureNote {
        text: String,
    },
    SshKey {
        public_key: Option<String>,
        private_key: String,
        passphrase: Option<String>,
    },
}

pub struct NewSecret {
    pub name: String,
    pub data: SecretDataV1,
}

impl NewSecret {
    pub fn kind(&self) -> SecretKind {
        self.data.kind()
    }
}

pub fn validate_new(input: CreateSecretInput) -> Result<NewSecret, SecretError> {
    validate_name(&input.name)?;
    let data = validate_data(input.data)?;
    Ok(NewSecret {
        name: input.name,
        data,
    })
}

pub fn apply_patch(
    record: &mut SecretRecordV1,
    expected_revision: u64,
    patch: SecretPatchInput,
) -> Result<(), SecretError> {
    if record.revision != expected_revision {
        return Err(SecretError::RevisionConflict);
    }
    if patch.name.is_none() && patch.data.is_none() {
        return Err(SecretError::InvalidInput);
    }

    if let Some(name) = patch.name.as_deref() {
        validate_name(name)?;
    }
    let data = patch.data.map(validate_data).transpose()?;
    let next_revision = record
        .revision
        .checked_add(1)
        .ok_or(SecretError::InvalidInput)?;

    if let Some(name) = patch.name {
        record.name = name;
    }
    if let Some(data) = data {
        record.data = data;
    }
    record.revision = next_revision;
    Ok(())
}

fn validate_data(input: SecretDataInput) -> Result<SecretDataV1, SecretError> {
    match input {
        SecretDataInput::Password {
            username,
            password,
            url,
            notes,
        } => {
            validate_metadata(&username, true)?;
            validate_optional_metadata(url.as_deref(), true)?;
            let password = SecretText::validated(password, false)?;
            let notes = validate_optional_secret(notes)?;
            Ok(SecretDataV1::Password {
                username,
                password,
                url,
                notes,
            })
        }
        SecretDataInput::ApiKey {
            key,
            environment,
            scopes,
        } => {
            validate_optional_metadata(environment.as_deref(), true)?;
            if scopes.len() > MAX_SCOPES {
                return Err(SecretError::InvalidInput);
            }
            for scope in &scopes {
                validate_text(scope, MAX_SCOPE_BYTES, false)?;
            }
            Ok(SecretDataV1::ApiKey {
                key: SecretText::validated(key, false)?,
                environment,
                scopes,
            })
        }
        SecretDataInput::Token {
            value,
            expires_at,
            notes,
        } => {
            if let Some(expires_at) = expires_at.as_deref() {
                validate_metadata(expires_at, false)?;
                if !is_valid_rfc3339(expires_at) {
                    return Err(SecretError::InvalidInput);
                }
            }
            Ok(SecretDataV1::Token {
                value: SecretText::validated(value, false)?,
                expires_at,
                notes: validate_optional_secret(notes)?,
            })
        }
        SecretDataInput::SecureNote { text } => Ok(SecretDataV1::SecureNote {
            text: SecretText::validated(text, false)?,
        }),
        SecretDataInput::SshKey {
            public_key,
            private_key,
            passphrase,
        } => {
            validate_optional_metadata(public_key.as_deref(), true)?;
            if !is_structured_private_key(&private_key) {
                return Err(SecretError::InvalidInput);
            }
            Ok(SecretDataV1::SshKey {
                public_key,
                private_key: SecretText::validated(private_key, false)?,
                passphrase: validate_optional_secret(passphrase)?,
            })
        }
    }
}

fn validate_name(value: &str) -> Result<(), SecretError> {
    validate_text(value, MAX_NAME_BYTES, false)
}

fn validate_metadata(value: &str, allow_empty: bool) -> Result<(), SecretError> {
    validate_text(value, MAX_METADATA_BYTES, allow_empty)
}

fn validate_optional_metadata(value: Option<&str>, allow_empty: bool) -> Result<(), SecretError> {
    if let Some(value) = value {
        validate_metadata(value, allow_empty)?;
    }
    Ok(())
}

fn validate_optional_secret(value: Option<String>) -> Result<Option<SecretText>, SecretError> {
    value
        .map(|value| SecretText::validated(value, true))
        .transpose()
}

fn validate_text(value: &str, max_bytes: usize, allow_empty: bool) -> Result<(), SecretError> {
    if (!allow_empty && value.is_empty()) || value.len() > max_bytes || value.contains('\0') {
        return Err(SecretError::InvalidInput);
    }
    Ok(())
}

fn is_structured_private_key(value: &str) -> bool {
    let value = value.trim();
    const MARKERS: [(&str, &str); 5] = [
        (
            "-----BEGIN OPENSSH PRIVATE KEY-----",
            "-----END OPENSSH PRIVATE KEY-----",
        ),
        ("-----BEGIN PRIVATE KEY-----", "-----END PRIVATE KEY-----"),
        (
            "-----BEGIN RSA PRIVATE KEY-----",
            "-----END RSA PRIVATE KEY-----",
        ),
        (
            "-----BEGIN EC PRIVATE KEY-----",
            "-----END EC PRIVATE KEY-----",
        ),
        (
            "-----BEGIN DSA PRIVATE KEY-----",
            "-----END DSA PRIVATE KEY-----",
        ),
    ];
    MARKERS
        .iter()
        .any(|(begin, end)| value.starts_with(begin) && value.ends_with(end))
}

fn is_valid_rfc3339(value: &str) -> bool {
    if !value.is_ascii() {
        return false;
    }
    let Some((date, time_with_zone)) = value.split_once('T') else {
        return false;
    };
    if date.len() != 10 || &date[4..5] != "-" || &date[7..8] != "-" {
        return false;
    }
    let Some(year) = parse_number(&date[0..4]) else {
        return false;
    };
    let Some(month) = parse_number(&date[5..7]) else {
        return false;
    };
    let Some(day) = parse_number(&date[8..10]) else {
        return false;
    };
    if year == 0 || !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
        return false;
    }

    let time = if let Some(time) = time_with_zone.strip_suffix('Z') {
        time
    } else {
        if time_with_zone.len() < 6 {
            return false;
        }
        let zone_start = time_with_zone.len() - 6;
        let zone = &time_with_zone[zone_start..];
        if !matches!(&zone[0..1], "+" | "-") || &zone[3..4] != ":" {
            return false;
        }
        let (Some(hours), Some(minutes)) = (parse_number(&zone[1..3]), parse_number(&zone[4..6]))
        else {
            return false;
        };
        if hours > 23 || minutes > 59 {
            return false;
        }
        &time_with_zone[..zone_start]
    };

    let (clock, fraction) = time
        .split_once('.')
        .map_or((time, None), |(clock, fraction)| (clock, Some(fraction)));
    if clock.len() != 8 || &clock[2..3] != ":" || &clock[5..6] != ":" {
        return false;
    }
    if fraction
        .is_some_and(|value| value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return false;
    }
    let (Some(hours), Some(minutes), Some(seconds)) = (
        parse_number(&clock[0..2]),
        parse_number(&clock[3..5]),
        parse_number(&clock[6..8]),
    ) else {
        return false;
    };
    hours <= 23 && minutes <= 59 && seconds <= 59
}

fn parse_number(value: &str) -> Option<u32> {
    if value.bytes().all(|byte| byte.is_ascii_digit()) {
        value.parse().ok()
    } else {
        None
    }
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(400) || (year.is_multiple_of(4) && !year.is_multiple_of(100)) => {
            29
        }
        2 => 28,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(data: SecretDataInput) -> CreateSecretInput {
        CreateSecretInput {
            name: "Conta principal".into(),
            data,
        }
    }

    fn password() -> SecretDataInput {
        SecretDataInput::Password {
            username: "enzo".into(),
            password: "correct horse battery staple".into(),
            url: Some("https://example.com".into()),
            notes: Some("somente para testes".into()),
        }
    }

    #[test]
    fn valida_password() {
        let secret = validate_new(input(password())).expect("password válido");
        assert_eq!(secret.kind(), SecretKind::Password);
    }

    #[test]
    fn valida_api_key() {
        let secret = validate_new(input(SecretDataInput::ApiKey {
            key: "api-canary".into(),
            environment: Some("produção".into()),
            scopes: vec!["read".into(), "write".into()],
        }))
        .expect("api key válida");
        assert_eq!(secret.kind(), SecretKind::ApiKey);
    }

    #[test]
    fn valida_token_com_expiracao_rfc3339() {
        let secret = validate_new(input(SecretDataInput::Token {
            value: "token-canary".into(),
            expires_at: Some("2027-02-28T23:59:59Z".into()),
            notes: None,
        }))
        .expect("token válido");
        assert_eq!(secret.kind(), SecretKind::Token);
    }

    #[test]
    fn valida_nota_secreta() {
        let secret = validate_new(input(SecretDataInput::SecureNote {
            text: "conteúdo".into(),
        }))
        .expect("nota válida");
        assert_eq!(secret.kind(), SecretKind::SecureNote);
    }

    #[test]
    fn valida_chave_ssh_privada() {
        let secret = validate_new(input(SecretDataInput::SshKey {
            public_key: Some("ssh-ed25519 AAAA test@example".into()),
            private_key:
                "-----BEGIN OPENSSH PRIVATE KEY-----\nAAAA\n-----END OPENSSH PRIVATE KEY-----"
                    .into(),
            passphrase: Some("passphrase".into()),
        }))
        .expect("chave válida");
        assert_eq!(secret.kind(), SecretKind::SshKey);
    }

    #[test]
    fn rejeita_nome_vazio() {
        let mut candidate = input(password());
        candidate.name.clear();
        assert!(matches!(
            validate_new(candidate),
            Err(SecretError::InvalidInput)
        ));
    }

    #[test]
    fn rejeita_nul_em_campo() {
        let mut candidate = input(password());
        candidate.name = "nome\0oculto".into();
        assert!(matches!(
            validate_new(candidate),
            Err(SecretError::InvalidInput)
        ));
    }

    #[test]
    fn mede_limite_do_nome_em_bytes_utf8() {
        let mut candidate = input(password());
        candidate.name = "á".repeat(129);
        assert!(matches!(
            validate_new(candidate),
            Err(SecretError::InvalidInput)
        ));
    }

    #[test]
    fn rejeita_valor_sensivel_acima_de_um_mib() {
        let candidate = input(SecretDataInput::SecureNote {
            text: "x".repeat(MAX_SENSITIVE_VALUE_BYTES + 1),
        });
        assert!(matches!(
            validate_new(candidate),
            Err(SecretError::InvalidInput)
        ));
    }

    #[test]
    fn rejeita_metadado_acima_de_quatro_kib() {
        let candidate = input(SecretDataInput::Password {
            username: "x".repeat(MAX_METADATA_BYTES + 1),
            password: "secret".into(),
            url: None,
            notes: None,
        });
        assert!(matches!(
            validate_new(candidate),
            Err(SecretError::InvalidInput)
        ));
    }

    #[test]
    fn rejeita_mais_de_128_escopos() {
        let candidate = input(SecretDataInput::ApiKey {
            key: "key".into(),
            environment: None,
            scopes: vec!["scope".into(); MAX_SCOPES + 1],
        });
        assert!(matches!(
            validate_new(candidate),
            Err(SecretError::InvalidInput)
        ));
    }

    #[test]
    fn rejeita_escopo_acima_de_256_bytes() {
        let candidate = input(SecretDataInput::ApiKey {
            key: "key".into(),
            environment: None,
            scopes: vec!["x".repeat(MAX_SCOPE_BYTES + 1)],
        });
        assert!(matches!(
            validate_new(candidate),
            Err(SecretError::InvalidInput)
        ));
    }

    #[test]
    fn rejeita_data_de_expiracao_invalida() {
        let candidate = input(SecretDataInput::Token {
            value: "token".into(),
            expires_at: Some("2027-02-29T12:00:00Z".into()),
            notes: None,
        });
        assert!(matches!(
            validate_new(candidate),
            Err(SecretError::InvalidInput)
        ));
    }

    #[test]
    fn rejeita_campo_desconhecido_no_input() {
        let json = r#"{
            "name": "Conta",
            "unexpected": true,
            "data": {"type": "secure-note", "text": "secret"}
        }"#;
        assert!(serde_json::from_str::<CreateSecretInput>(json).is_err());
    }

    #[test]
    fn rejeita_tipo_desconhecido_no_input() {
        let json = r#"{
            "name": "Conta",
            "data": {"type": "totp", "value": "secret"}
        }"#;
        assert!(serde_json::from_str::<CreateSecretInput>(json).is_err());
    }

    #[test]
    fn rejeita_revisao_obsoleta_sem_alterar_record() {
        let mut record =
            SecretRecordV1::new_for_test(uuid::Uuid::nil(), 7, "Original".into(), password());
        let result = apply_patch(
            &mut record,
            6,
            SecretPatchInput {
                name: Some("Alterado".into()),
                data: None,
            },
        );
        assert!(matches!(result, Err(SecretError::RevisionConflict)));
        assert_eq!(record.name, "Original");
        assert_eq!(record.revision, 7);
    }

    #[test]
    fn patch_valido_incrementa_revisao() {
        let mut record =
            SecretRecordV1::new_for_test(uuid::Uuid::nil(), 7, "Original".into(), password());
        apply_patch(
            &mut record,
            7,
            SecretPatchInput {
                name: Some("Alterado".into()),
                data: None,
            },
        )
        .expect("patch válido");
        assert_eq!(record.name, "Alterado");
        assert_eq!(record.revision, 8);
    }

    #[test]
    fn patch_invalido_nao_aplica_alteracao_parcial() {
        let mut record =
            SecretRecordV1::new_for_test(uuid::Uuid::nil(), 7, "Original".into(), password());
        let result = apply_patch(
            &mut record,
            7,
            SecretPatchInput {
                name: Some("Novo nome".into()),
                data: Some(SecretDataInput::SecureNote {
                    text: String::new(),
                }),
            },
        );
        assert!(matches!(result, Err(SecretError::InvalidInput)));
        assert_eq!(record.name, "Original");
        assert_eq!(record.revision, 7);
    }
}
