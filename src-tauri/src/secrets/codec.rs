//! Conversão fechada entre o modelo tipado de segredos e records CBOR v1.

use std::io::{self, Write};

use ciborium::value::Value;
use uuid::Uuid;

use crate::crypto::envelope::SessionContent;
use crate::secrets::model::{
    validate_new, CreateSecretInput, SecretDataInput, SecretDataV1, SecretError, SecretRecordV1,
    MAX_RECORDS_PER_SESSION, MAX_SERIALIZED_RECORD_BYTES, SECRET_RECORD_VERSION,
};
use crate::secrets::move_state::MoveState;

const CONTENT_FORMAT_V1: u16 = 1;

/// Converte os records CBOR da sessão em records v1 validados.
pub fn decode_records(content: &SessionContent) -> Result<Vec<SecretRecordV1>, SecretError> {
    if content.content_format != CONTENT_FORMAT_V1
        || content.secrets.len() > MAX_RECORDS_PER_SESSION
    {
        return Err(SecretError::InvalidInput);
    }

    content
        .secrets
        .iter()
        .map(|value| {
            ensure_serialized_size(value)?;
            decode_record(value)
        })
        .collect()
}

/// Converte records v1 em mapas CBOR fechados, validando-os antes da saída.
pub fn encode_records(records: &[SecretRecordV1]) -> Result<Vec<Value>, SecretError> {
    if records.len() > MAX_RECORDS_PER_SESSION {
        return Err(SecretError::InvalidInput);
    }

    records
        .iter()
        .map(|record| {
            let value = encode_record(record)?;
            ensure_serialized_size(&value)?;
            decode_record(&value)?;
            Ok(value)
        })
        .collect()
}

fn decode_record(value: &Value) -> Result<SecretRecordV1, SecretError> {
    let fields = closed_map(
        value,
        &[
            "version",
            "id",
            "revision",
            "name",
            "created_at_ms",
            "updated_at_ms",
            "move_state",
            "data",
        ],
    )?;
    let version = u16_value(field(fields, "version")?)?;
    if version != SECRET_RECORD_VERSION {
        return Err(SecretError::InvalidInput);
    }
    let id = uuid_value(field(fields, "id")?)?;
    let revision = u64_value(field(fields, "revision")?)?;
    let name = text_value(field(fields, "name")?)?.to_owned();
    let created_at_ms = i64_value(field(fields, "created_at_ms")?)?;
    let updated_at_ms = i64_value(field(fields, "updated_at_ms")?)?;
    let move_state = decode_move_state(field(fields, "move_state")?)?;
    validate_move_state(revision, move_state.as_ref())?;
    let data = decode_data(field(fields, "data")?)?;
    let validated = validate_new(CreateSecretInput { name, data })?;

    Ok(SecretRecordV1 {
        version,
        id,
        revision,
        name: validated.name,
        created_at_ms,
        updated_at_ms,
        move_state,
        data: validated.data,
    })
}

fn decode_move_state(value: &Value) -> Result<Option<MoveState>, SecretError> {
    if matches!(value, Value::Null) {
        return Ok(None);
    }
    let entries = map_value(value)?;
    match text_value(field(entries, "state")?)? {
        "pending-move" => {
            let fields = closed_map(
                value,
                &["state", "move_id", "target_session_id", "original_revision"],
            )?;
            Ok(Some(MoveState::PendingMove {
                move_id: uuid_value(field(fields, "move_id")?)?,
                target_session_id: uuid_value(field(fields, "target_session_id")?)?,
                original_revision: u64_value(field(fields, "original_revision")?)?,
            }))
        }
        "staged" => {
            let fields = closed_map(
                value,
                &["state", "move_id", "source_session_id", "original_revision"],
            )?;
            Ok(Some(MoveState::Staged {
                move_id: uuid_value(field(fields, "move_id")?)?,
                source_session_id: uuid_value(field(fields, "source_session_id")?)?,
                original_revision: u64_value(field(fields, "original_revision")?)?,
            }))
        }
        _ => Err(SecretError::InvalidInput),
    }
}

fn validate_move_state(revision: u64, move_state: Option<&MoveState>) -> Result<(), SecretError> {
    match move_state {
        None => Ok(()),
        Some(MoveState::PendingMove {
            original_revision, ..
        }) if revision == *original_revision => Ok(()),
        Some(MoveState::Staged {
            original_revision, ..
        }) if original_revision.checked_add(1) == Some(revision) => Ok(()),
        _ => Err(SecretError::InvalidInput),
    }
}

fn decode_data(value: &Value) -> Result<SecretDataInput, SecretError> {
    let entries = map_value(value)?;
    let kind = text_value(field(entries, "type")?)?;
    match kind {
        "password" => {
            let fields = closed_map(value, &["type", "username", "password", "url", "notes"])?;
            Ok(SecretDataInput::Password {
                username: owned_text(fields, "username")?,
                password: owned_text(fields, "password")?,
                url: optional_owned_text(fields, "url")?,
                notes: optional_owned_text(fields, "notes")?,
            })
        }
        "api-key" => {
            let fields = closed_map(value, &["type", "key", "environment", "scopes"])?;
            Ok(SecretDataInput::ApiKey {
                key: owned_text(fields, "key")?,
                environment: optional_owned_text(fields, "environment")?,
                scopes: text_array(field(fields, "scopes")?)?,
            })
        }
        "token" => {
            let fields = closed_map(value, &["type", "value", "expires_at", "notes"])?;
            Ok(SecretDataInput::Token {
                value: owned_text(fields, "value")?,
                expires_at: optional_owned_text(fields, "expires_at")?,
                notes: optional_owned_text(fields, "notes")?,
            })
        }
        "secure-note" => {
            let fields = closed_map(value, &["type", "text"])?;
            Ok(SecretDataInput::SecureNote {
                text: owned_text(fields, "text")?,
            })
        }
        "ssh-key" => {
            let fields = closed_map(value, &["type", "public_key", "private_key", "passphrase"])?;
            Ok(SecretDataInput::SshKey {
                public_key: optional_owned_text(fields, "public_key")?,
                private_key: owned_text(fields, "private_key")?,
                passphrase: optional_owned_text(fields, "passphrase")?,
            })
        }
        _ => Err(SecretError::InvalidInput),
    }
}

fn encode_record(record: &SecretRecordV1) -> Result<Value, SecretError> {
    if record.version != SECRET_RECORD_VERSION {
        return Err(SecretError::InvalidInput);
    }
    Ok(map([
        ("version", Value::Integer(record.version.into())),
        ("id", Value::Bytes(record.id.as_bytes().to_vec())),
        ("revision", Value::Integer(record.revision.into())),
        ("name", Value::Text(record.name.clone())),
        ("created_at_ms", Value::Integer(record.created_at_ms.into())),
        ("updated_at_ms", Value::Integer(record.updated_at_ms.into())),
        ("move_state", encode_move_state(record.move_state.as_ref())),
        ("data", encode_data(&record.data)),
    ]))
}

fn encode_move_state(move_state: Option<&MoveState>) -> Value {
    match move_state {
        None => Value::Null,
        Some(MoveState::PendingMove {
            move_id,
            target_session_id,
            original_revision,
        }) => map([
            ("state", Value::Text("pending-move".into())),
            ("move_id", Value::Bytes(move_id.as_bytes().to_vec())),
            (
                "target_session_id",
                Value::Bytes(target_session_id.as_bytes().to_vec()),
            ),
            (
                "original_revision",
                Value::Integer((*original_revision).into()),
            ),
        ]),
        Some(MoveState::Staged {
            move_id,
            source_session_id,
            original_revision,
        }) => map([
            ("state", Value::Text("staged".into())),
            ("move_id", Value::Bytes(move_id.as_bytes().to_vec())),
            (
                "source_session_id",
                Value::Bytes(source_session_id.as_bytes().to_vec()),
            ),
            (
                "original_revision",
                Value::Integer((*original_revision).into()),
            ),
        ]),
    }
}

fn encode_data(data: &SecretDataV1) -> Value {
    match data {
        SecretDataV1::Password {
            username,
            password,
            url,
            notes,
        } => map([
            ("type", Value::Text("password".into())),
            ("username", Value::Text(username.clone())),
            ("password", Value::Text(password.as_str().to_owned())),
            ("url", optional_text_value(url.as_deref())),
            (
                "notes",
                optional_text_value(notes.as_ref().map(|value| value.as_str())),
            ),
        ]),
        SecretDataV1::ApiKey {
            key,
            environment,
            scopes,
        } => map([
            ("type", Value::Text("api-key".into())),
            ("key", Value::Text(key.as_str().to_owned())),
            ("environment", optional_text_value(environment.as_deref())),
            (
                "scopes",
                Value::Array(scopes.iter().cloned().map(Value::Text).collect()),
            ),
        ]),
        SecretDataV1::Token {
            value,
            expires_at,
            notes,
        } => map([
            ("type", Value::Text("token".into())),
            ("value", Value::Text(value.as_str().to_owned())),
            ("expires_at", optional_text_value(expires_at.as_deref())),
            (
                "notes",
                optional_text_value(notes.as_ref().map(|value| value.as_str())),
            ),
        ]),
        SecretDataV1::SecureNote { text } => map([
            ("type", Value::Text("secure-note".into())),
            ("text", Value::Text(text.as_str().to_owned())),
        ]),
        SecretDataV1::SshKey {
            public_key,
            private_key,
            passphrase,
        } => map([
            ("type", Value::Text("ssh-key".into())),
            ("public_key", optional_text_value(public_key.as_deref())),
            ("private_key", Value::Text(private_key.as_str().to_owned())),
            (
                "passphrase",
                optional_text_value(passphrase.as_ref().map(|value| value.as_str())),
            ),
        ]),
    }
}

fn map<const N: usize>(fields: [(&str, Value); N]) -> Value {
    Value::Map(
        fields
            .into_iter()
            .map(|(name, value)| (Value::Text(name.into()), value))
            .collect(),
    )
}

fn optional_text_value(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |value| Value::Text(value.to_owned()))
}

fn closed_map<'a>(
    value: &'a Value,
    allowed_fields: &[&str],
) -> Result<&'a [(Value, Value)], SecretError> {
    let entries = map_value(value)?;
    if entries.len() != allowed_fields.len() {
        return Err(SecretError::InvalidInput);
    }
    for (index, (key, _)) in entries.iter().enumerate() {
        let Value::Text(name) = key else {
            return Err(SecretError::InvalidInput);
        };
        if !allowed_fields.contains(&name.as_str())
            || entries[..index]
                .iter()
                .any(|(previous, _)| matches!(previous, Value::Text(value) if value == name))
        {
            return Err(SecretError::InvalidInput);
        }
    }
    Ok(entries)
}

fn map_value(value: &Value) -> Result<&[(Value, Value)], SecretError> {
    match value {
        Value::Map(entries) => Ok(entries),
        _ => Err(SecretError::InvalidInput),
    }
}

fn field<'a>(entries: &'a [(Value, Value)], name: &str) -> Result<&'a Value, SecretError> {
    entries
        .iter()
        .find_map(|(key, value)| {
            matches!(key, Value::Text(field_name) if field_name == name).then_some(value)
        })
        .ok_or(SecretError::InvalidInput)
}

fn text_value(value: &Value) -> Result<&str, SecretError> {
    match value {
        Value::Text(value) => Ok(value),
        _ => Err(SecretError::InvalidInput),
    }
}

fn owned_text(entries: &[(Value, Value)], name: &str) -> Result<String, SecretError> {
    Ok(text_value(field(entries, name)?)?.to_owned())
}

fn optional_owned_text(
    entries: &[(Value, Value)],
    name: &str,
) -> Result<Option<String>, SecretError> {
    match field(entries, name)? {
        Value::Null => Ok(None),
        Value::Text(value) => Ok(Some(value.clone())),
        _ => Err(SecretError::InvalidInput),
    }
}

fn text_array(value: &Value) -> Result<Vec<String>, SecretError> {
    match value {
        Value::Array(values) => values
            .iter()
            .map(|value| Ok(text_value(value)?.to_owned()))
            .collect(),
        _ => Err(SecretError::InvalidInput),
    }
}

fn uuid_value(value: &Value) -> Result<Uuid, SecretError> {
    let Value::Bytes(bytes) = value else {
        return Err(SecretError::InvalidInput);
    };
    let bytes: [u8; 16] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| SecretError::InvalidInput)?;
    Ok(Uuid::from_bytes(bytes))
}

fn u16_value(value: &Value) -> Result<u16, SecretError> {
    integer(value)?
        .try_into()
        .map_err(|_| SecretError::InvalidInput)
}

fn u64_value(value: &Value) -> Result<u64, SecretError> {
    integer(value)?
        .try_into()
        .map_err(|_| SecretError::InvalidInput)
}

fn i64_value(value: &Value) -> Result<i64, SecretError> {
    integer(value)?
        .try_into()
        .map_err(|_| SecretError::InvalidInput)
}

fn integer(value: &Value) -> Result<ciborium::value::Integer, SecretError> {
    match value {
        Value::Integer(value) => Ok(*value),
        _ => Err(SecretError::InvalidInput),
    }
}

fn ensure_serialized_size(value: &Value) -> Result<(), SecretError> {
    let mut writer = LimitedWriter::new(MAX_SERIALIZED_RECORD_BYTES);
    ciborium::into_writer(value, &mut writer).map_err(|_| SecretError::InvalidInput)
}

struct LimitedWriter {
    written: usize,
    limit: usize,
}

impl LimitedWriter {
    fn new(limit: usize) -> Self {
        Self { written: 0, limit }
    }
}

impl Write for LimitedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let next = self
            .written
            .checked_add(bytes.len())
            .ok_or_else(|| io::Error::other("record CBOR size overflow"))?;
        if next > self.limit {
            return Err(io::Error::other("record CBOR size limit exceeded"));
        }
        self.written = next;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use ciborium::value::Value;
    use uuid::Uuid;

    use super::*;
    use crate::crypto::envelope::SessionContent;
    use crate::secrets::model::{
        validate_new, CreateSecretInput, SecretDataInput, SecretDataV1, SecretKind, SecretRecordV1,
        MAX_SENSITIVE_VALUE_BYTES, SECRET_RECORD_VERSION,
    };
    use crate::secrets::move_state::MoveState;

    fn cbor_bytes(content: &SessionContent) -> Vec<u8> {
        let mut bytes = Vec::new();
        ciborium::into_writer(content, &mut bytes).expect("snapshot serializável");
        bytes
    }

    fn record(data: SecretDataInput) -> SecretRecordV1 {
        let validated = validate_new(CreateSecretInput {
            name: "Registro".into(),
            data,
        })
        .expect("fixture válida");
        SecretRecordV1 {
            version: SECRET_RECORD_VERSION,
            id: Uuid::from_bytes([0x42; 16]),
            revision: 7,
            name: validated.name,
            created_at_ms: 1_700_000_000_000,
            updated_at_ms: 1_700_000_001_000,
            move_state: None,
            data: validated.data,
        }
    }

    fn content_for(record: &SecretRecordV1) -> SessionContent {
        SessionContent {
            content_format: 1,
            secrets: encode_records(std::slice::from_ref(record)).expect("encode válido"),
        }
    }

    fn roundtrip(record: SecretRecordV1, expected_kind: SecretKind) -> SecretRecordV1 {
        let decoded = decode_records(&content_for(&record)).expect("decode válido");
        assert!(decoded.len() == 1);
        let decoded = decoded.into_iter().next().expect("um record");
        assert!(decoded.version == SECRET_RECORD_VERSION);
        assert!(decoded.id == record.id);
        assert!(decoded.revision == record.revision);
        assert!(decoded.name == record.name);
        assert!(decoded.created_at_ms == record.created_at_ms);
        assert!(decoded.updated_at_ms == record.updated_at_ms);
        assert!(decoded.kind() == expected_kind);
        decoded
    }

    fn record_map(value: &mut Value) -> &mut Vec<(Value, Value)> {
        match value {
            Value::Map(entries) => entries,
            _ => panic!("record deveria ser mapa"),
        }
    }

    fn field_mut<'a>(entries: &'a mut [(Value, Value)], field: &str) -> &'a mut Value {
        entries
            .iter_mut()
            .find_map(|(key, value)| {
                matches!(key, Value::Text(name) if name == field).then_some(value)
            })
            .expect("campo existente")
    }

    #[test]
    fn roundtrip_password() {
        let decoded = roundtrip(
            record(SecretDataInput::Password {
                username: "user".into(),
                password: "password-canary".into(),
                url: Some("https://example.com".into()),
                notes: Some("notes-canary".into()),
            }),
            SecretKind::Password,
        );
        match decoded.data {
            SecretDataV1::Password {
                username,
                password,
                url,
                notes,
            } => {
                assert!(username == "user");
                assert!(password.as_str() == "password-canary");
                assert!(url.as_deref() == Some("https://example.com"));
                assert!(notes
                    .as_ref()
                    .is_some_and(|value| value.as_str() == "notes-canary"));
            }
            _ => panic!("tipo incorreto"),
        }
    }

    #[test]
    fn roundtrip_api_key() {
        let decoded = roundtrip(
            record(SecretDataInput::ApiKey {
                key: "api-key-canary".into(),
                environment: Some("produção".into()),
                scopes: vec!["read".into(), "write".into()],
            }),
            SecretKind::ApiKey,
        );
        match decoded.data {
            SecretDataV1::ApiKey {
                key,
                environment,
                scopes,
            } => {
                assert!(key.as_str() == "api-key-canary");
                assert!(environment.as_deref() == Some("produção"));
                assert!(scopes == ["read", "write"]);
            }
            _ => panic!("tipo incorreto"),
        }
    }

    #[test]
    fn roundtrip_token() {
        let decoded = roundtrip(
            record(SecretDataInput::Token {
                value: "token-canary".into(),
                expires_at: Some("2027-02-28T23:59:59Z".into()),
                notes: Some("token-notes-canary".into()),
            }),
            SecretKind::Token,
        );
        match decoded.data {
            SecretDataV1::Token {
                value,
                expires_at,
                notes,
            } => {
                assert!(value.as_str() == "token-canary");
                assert!(expires_at.as_deref() == Some("2027-02-28T23:59:59Z"));
                assert!(notes
                    .as_ref()
                    .is_some_and(|value| value.as_str() == "token-notes-canary"));
            }
            _ => panic!("tipo incorreto"),
        }
    }

    #[test]
    fn roundtrip_secure_note() {
        let decoded = roundtrip(
            record(SecretDataInput::SecureNote {
                text: "secure-note-canary".into(),
            }),
            SecretKind::SecureNote,
        );
        match decoded.data {
            SecretDataV1::SecureNote { text } => {
                assert!(text.as_str() == "secure-note-canary");
            }
            _ => panic!("tipo incorreto"),
        }
    }

    #[test]
    fn roundtrip_ssh_key() {
        let decoded = roundtrip(
            record(SecretDataInput::SshKey {
                public_key: Some("ssh-ed25519 AAAA test@example".into()),
                private_key:
                    "-----BEGIN OPENSSH PRIVATE KEY-----\nAAAA\n-----END OPENSSH PRIVATE KEY-----"
                        .into(),
                passphrase: Some("ssh-passphrase-canary".into()),
            }),
            SecretKind::SshKey,
        );
        match decoded.data {
            SecretDataV1::SshKey {
                public_key,
                private_key,
                passphrase,
            } => {
                assert!(public_key.as_deref() == Some("ssh-ed25519 AAAA test@example"));
                assert!(private_key
                    .as_str()
                    .starts_with("-----BEGIN OPENSSH PRIVATE KEY-----"));
                assert!(passphrase
                    .as_ref()
                    .is_some_and(|value| value.as_str() == "ssh-passphrase-canary"));
            }
            _ => panic!("tipo incorreto"),
        }
    }

    #[test]
    fn rejeita_content_format_futuro() {
        let content = SessionContent {
            content_format: 2,
            secrets: Vec::new(),
        };
        assert!(decode_records(&content).is_err());
    }

    #[test]
    fn rejeita_versao_de_record_futura() {
        let mut content = content_for(&record(SecretDataInput::SecureNote {
            text: "canary".into(),
        }));
        let entries = record_map(&mut content.secrets[0]);
        *field_mut(entries, "version") = Value::Integer(2.into());
        assert!(decode_records(&content).is_err());
    }

    #[test]
    fn rejeita_tipo_desconhecido() {
        let mut content = content_for(&record(SecretDataInput::SecureNote {
            text: "canary".into(),
        }));
        let entries = record_map(&mut content.secrets[0]);
        let data = field_mut(entries, "data");
        let data_entries = record_map(data);
        *field_mut(data_entries, "type") = Value::Text("totp".into());
        assert!(decode_records(&content).is_err());
    }

    #[test]
    fn rejeita_campo_desconhecido_no_record() {
        let mut content = content_for(&record(SecretDataInput::SecureNote {
            text: "canary".into(),
        }));
        record_map(&mut content.secrets[0])
            .push((Value::Text("future_field".into()), Value::Bool(true)));
        assert!(decode_records(&content).is_err());
    }

    #[test]
    fn rejeita_campo_desconhecido_nos_dados() {
        let mut content = content_for(&record(SecretDataInput::SecureNote {
            text: "canary".into(),
        }));
        let entries = record_map(&mut content.secrets[0]);
        let data_entries = record_map(field_mut(entries, "data"));
        data_entries.push((Value::Text("future_field".into()), Value::Bool(true)));
        assert!(decode_records(&content).is_err());
    }

    #[test]
    fn rejeita_record_cbor_malformado() {
        let content = SessionContent {
            content_format: 1,
            secrets: vec![Value::Array(vec![Value::Integer(1.into())])],
        };
        assert!(decode_records(&content).is_err());
    }

    #[test]
    fn decode_invalido_nao_muta_session_content() {
        let content = SessionContent {
            content_format: 1,
            secrets: vec![Value::Bytes(vec![0xff, 0x00])],
        };
        let before = cbor_bytes(&content);
        assert!(decode_records(&content).is_err());
        let after = cbor_bytes(&content);
        assert!(before == after);
    }

    #[test]
    fn rejeita_record_serializado_acima_de_dois_mib() {
        const BEGIN: &str = "-----BEGIN OPENSSH PRIVATE KEY-----\n";
        const END: &str = "\n-----END OPENSSH PRIVATE KEY-----";
        let private_key = format!(
            "{BEGIN}{}{END}",
            "x".repeat(MAX_SENSITIVE_VALUE_BYTES - BEGIN.len() - END.len())
        );
        let oversized = record(SecretDataInput::SshKey {
            public_key: None,
            private_key,
            passphrase: Some("y".repeat(MAX_SENSITIVE_VALUE_BYTES)),
        });
        assert!(encode_records(&[oversized]).is_err());
    }

    #[test]
    fn roundtrip_preserva_pending_move() {
        let mut original = record(SecretDataInput::SecureNote {
            text: "canary".into(),
        });
        original.move_state = Some(MoveState::PendingMove {
            move_id: Uuid::from_u128(1),
            target_session_id: Uuid::from_u128(2),
            original_revision: 7,
        });

        let decoded = decode_records(&content_for(&original)).expect("decode");

        assert!(matches!(
            decoded[0].move_state,
            Some(MoveState::PendingMove {
                move_id,
                target_session_id,
                original_revision: 7
            }) if move_id == Uuid::from_u128(1) && target_session_id == Uuid::from_u128(2)
        ));
    }

    #[test]
    fn roundtrip_preserva_staged_oculto() {
        let mut original = record(SecretDataInput::SecureNote {
            text: "canary".into(),
        });
        original.revision = 8;
        original.move_state = Some(MoveState::Staged {
            move_id: Uuid::from_u128(1),
            source_session_id: Uuid::from_u128(2),
            original_revision: 7,
        });

        let decoded = decode_records(&content_for(&original)).expect("decode");

        assert!(!decoded[0].is_visible());
        assert!(matches!(
            decoded[0].move_state,
            Some(MoveState::Staged {
                move_id,
                source_session_id,
                original_revision: 7
            }) if move_id == Uuid::from_u128(1) && source_session_id == Uuid::from_u128(2)
        ));
    }
}
