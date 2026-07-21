//! Modelo tipado de sessões: `AuthMode`, `LockPolicy`, `SessionEntry`,
//! `Registry`, normalização de nome e validação. Camada pura (sem relógio,
//! sem IO): o `SessionManager` injeta timestamps e persistência.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Versão do formato do `registry.json`.
pub const REGISTRY_VERSION: u32 = 1;
/// Limite defensivo do nome de uma sessão, em bytes UTF-8 (após `trim`).
pub const MAX_SESSION_NAME_BYTES: usize = 256;
/// Limite defensivo da dica, em bytes UTF-8.
pub const MAX_HINT_BYTES: usize = 512;
/// Inatividade mínima aceita pelo core (o frontend confirma "nunca").
pub const MIN_INACTIVITY_SECS: u64 = 60;
/// Inatividade padrão de uma sessão recém-criada (15 minutos).
pub const DEFAULT_INACTIVITY_SECS: u64 = 900;
/// Teto defensivo de sessões por instalação.
pub const MAX_SESSIONS: usize = 1024;

/// Modo de autenticação de uma sessão. `Global` (padrão) compartilha o domínio
/// de confiança da senha mestra global; `Own` mantém isolamento total.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuthMode {
    #[default]
    Global,
    Own,
}

/// Erros do domínio de sessões. Mensagens não ecoam senha, dica, path nem
/// material criptográfico (C-15).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SessionError {
    #[error("entrada de sessão inválida")]
    InvalidInput,
    #[error("já existe uma sessão com esse nome")]
    DuplicateName,
    #[error("sessão não encontrada")]
    NotFound,
    #[error("limite de sessões atingido")]
    CapacityExceeded,
    #[error("senha incorreta")]
    WrongPassword,
    #[error("senha não atende ao mínimo de força")]
    WeakPassword,
    #[error("aplicativo bloqueado")]
    AppLocked,
    #[error("sessão bloqueada")]
    Locked,
    #[error("modo de autenticação incompatível com a operação")]
    AuthModeMismatch,
    #[error("registro de sessões incompatível ou corrompido")]
    IncompatibleRegistry,
    #[error("cofre de sessão incompatível ou corrompido")]
    IncompatibleVault,
    #[error("registro de sessões foi adulterado")]
    TamperedRegistry,
    #[error("tentativas em excesso; aguarde antes de tentar novamente")]
    TooManyAttempts,
}

/// Política de bloqueio de uma sessão. `inactivity_secs = None` representa
/// "nunca" (exige confirmação explícita no frontend — VAULT-01 AC4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockPolicy {
    pub inactivity_secs: Option<u64>,
    pub on_windows_lock: bool,
    pub on_windows_suspend: bool,
}

impl Default for LockPolicy {
    fn default() -> Self {
        Self {
            inactivity_secs: Some(DEFAULT_INACTIVITY_SECS),
            on_windows_lock: true,
            on_windows_suspend: true,
        }
    }
}

impl LockPolicy {
    /// Aceita `None` ("nunca") ou qualquer valor `>= MIN_INACTIVITY_SECS`.
    pub fn validate(&self) -> Result<(), SessionError> {
        match self.inactivity_secs {
            Some(secs) if secs < MIN_INACTIVITY_SECS => Err(SessionError::InvalidInput),
            _ => Ok(()),
        }
    }
}

/// Entrada não secreta do registro (legível com o app bloqueado — AD-013).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEntry {
    pub id: Uuid,
    pub name: String,
    pub name_normalized: String,
    pub auth_mode: AuthMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    pub lock_policy: LockPolicy,
    pub created_at: String,
}

impl SessionEntry {
    /// Constrói uma entrada validada, computando `name_normalized`.
    pub fn new(
        id: Uuid,
        name: &str,
        auth_mode: AuthMode,
        hint: Option<String>,
        lock_policy: LockPolicy,
        created_at: String,
    ) -> Result<Self, SessionError> {
        let name = validate_name(name)?;
        let hint = validate_hint(hint)?;
        lock_policy.validate()?;
        validate_text(&created_at, MAX_HINT_BYTES, false)?;
        let name_normalized = normalize_name(&name);
        Ok(Self {
            id,
            name,
            name_normalized,
            auth_mode,
            hint,
            lock_policy,
            created_at,
        })
    }
}

/// Normaliza um nome para comparação de unicidade case-insensitive.
pub fn normalize_name(name: &str) -> String {
    name.trim().to_lowercase()
}

/// Valida e retorna o nome canônico (aparado). Rejeita vazio, NUL e excesso.
pub fn validate_name(name: &str) -> Result<String, SessionError> {
    let trimmed = name.trim();
    if trimmed.is_empty()
        || trimmed.len() > MAX_SESSION_NAME_BYTES
        || trimmed.contains('\0')
    {
        return Err(SessionError::InvalidInput);
    }
    Ok(trimmed.to_owned())
}

fn validate_hint(hint: Option<String>) -> Result<Option<String>, SessionError> {
    match hint {
        None => Ok(None),
        Some(value) => {
            validate_text(&value, MAX_HINT_BYTES, false)?;
            Ok(Some(value))
        }
    }
}

fn validate_text(value: &str, max_bytes: usize, allow_empty: bool) -> Result<(), SessionError> {
    if (!allow_empty && value.trim().is_empty()) || value.len() > max_bytes || value.contains('\0')
    {
        return Err(SessionError::InvalidInput);
    }
    Ok(())
}

/// Registro persistido de sessões (metadado não secreto).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Registry {
    pub version: u32,
    pub sessions: Vec<SessionEntry>,
}

impl Default for Registry {
    fn default() -> Self {
        Self {
            version: REGISTRY_VERSION,
            sessions: Vec::new(),
        }
    }
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn find(&self, id: Uuid) -> Option<&SessionEntry> {
        self.sessions.iter().find(|entry| entry.id == id)
    }

    pub fn find_mut(&mut self, id: Uuid) -> Option<&mut SessionEntry> {
        self.sessions.iter_mut().find(|entry| entry.id == id)
    }

    /// `true` se o nome normalizado já pertence a outra sessão (ignorando `except`).
    pub fn name_taken(&self, name_normalized: &str, except: Option<Uuid>) -> bool {
        self.sessions.iter().any(|entry| {
            entry.name_normalized == name_normalized && Some(entry.id) != except
        })
    }

    /// Insere uma entrada validando unicidade de nome e capacidade.
    pub fn insert(&mut self, entry: SessionEntry) -> Result<(), SessionError> {
        if self.sessions.len() >= MAX_SESSIONS {
            return Err(SessionError::CapacityExceeded);
        }
        if self.name_taken(&entry.name_normalized, None) {
            return Err(SessionError::DuplicateName);
        }
        self.sessions.push(entry);
        Ok(())
    }

    /// Renomeia uma sessão existente, revalidando nome e unicidade.
    pub fn rename(&mut self, id: Uuid, new_name: &str) -> Result<(), SessionError> {
        let name = validate_name(new_name)?;
        let normalized = normalize_name(&name);
        if self.name_taken(&normalized, Some(id)) {
            return Err(SessionError::DuplicateName);
        }
        let entry = self.find_mut(id).ok_or(SessionError::NotFound)?;
        entry.name = name;
        entry.name_normalized = normalized;
        Ok(())
    }

    /// Remove e retorna a entrada, se existir.
    pub fn remove(&mut self, id: Uuid) -> Option<SessionEntry> {
        let index = self.sessions.iter().position(|entry| entry.id == id)?;
        Some(self.sessions.remove(index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, mode: AuthMode) -> SessionEntry {
        SessionEntry::new(
            Uuid::new_v4(),
            name,
            mode,
            None,
            LockPolicy::default(),
            "2026-07-21T12:00:00Z".to_owned(),
        )
        .expect("entrada válida")
    }

    #[test]
    fn auth_mode_padrao_e_global() {
        assert_eq!(AuthMode::default(), AuthMode::Global);
    }

    #[test]
    fn normaliza_nome_case_insensitive_e_apara() {
        assert_eq!(normalize_name("  Trabalho "), "trabalho");
        assert_eq!(normalize_name("PESSOAL"), "pessoal");
    }

    #[test]
    fn entrada_apara_o_nome_e_computa_normalizado() {
        let e = entry("  Trabalho ", AuthMode::Global);
        assert_eq!(e.name, "Trabalho");
        assert_eq!(e.name_normalized, "trabalho");
    }

    #[test]
    fn nome_vazio_nulo_ou_grande_e_rejeitado() {
        assert_eq!(validate_name("   "), Err(SessionError::InvalidInput));
        assert_eq!(validate_name("a\0b"), Err(SessionError::InvalidInput));
        let big = "x".repeat(MAX_SESSION_NAME_BYTES + 1);
        assert_eq!(validate_name(&big), Err(SessionError::InvalidInput));
    }

    #[test]
    fn dica_grande_e_rejeitada_e_none_e_aceita() {
        let big = "x".repeat(MAX_HINT_BYTES + 1);
        let err = SessionEntry::new(
            Uuid::new_v4(),
            "Trabalho",
            AuthMode::Own,
            Some(big),
            LockPolicy::default(),
            "2026-07-21T12:00:00Z".to_owned(),
        )
        .unwrap_err();
        assert_eq!(err, SessionError::InvalidInput);
        assert!(entry("Trabalho", AuthMode::Own).hint.is_none());
    }

    #[test]
    fn politica_padrao_e_15_min_com_eventos_ativos() {
        let policy = LockPolicy::default();
        assert_eq!(policy.inactivity_secs, Some(DEFAULT_INACTIVITY_SECS));
        assert!(policy.on_windows_lock && policy.on_windows_suspend);
    }

    #[test]
    fn politica_aceita_nunca_e_rejeita_abaixo_do_minimo() {
        let never = LockPolicy {
            inactivity_secs: None,
            ..LockPolicy::default()
        };
        assert!(never.validate().is_ok());
        let too_low = LockPolicy {
            inactivity_secs: Some(MIN_INACTIVITY_SECS - 1),
            ..LockPolicy::default()
        };
        assert_eq!(too_low.validate(), Err(SessionError::InvalidInput));
        let at_min = LockPolicy {
            inactivity_secs: Some(MIN_INACTIVITY_SECS),
            ..LockPolicy::default()
        };
        assert!(at_min.validate().is_ok());
    }

    #[test]
    fn insert_rejeita_nome_duplicado_case_insensitive() {
        let mut reg = Registry::new();
        reg.insert(entry("Trabalho", AuthMode::Global)).unwrap();
        let dup = entry("trabalho", AuthMode::Own);
        assert_eq!(reg.insert(dup), Err(SessionError::DuplicateName));
        assert_eq!(reg.sessions.len(), 1);
    }

    #[test]
    fn rename_valida_unicidade_e_permite_o_proprio_nome() {
        let mut reg = Registry::new();
        let a = entry("Trabalho", AuthMode::Global);
        let id_a = a.id;
        reg.insert(a).unwrap();
        reg.insert(entry("Pessoal", AuthMode::Global)).unwrap();

        assert_eq!(reg.rename(id_a, "Pessoal"), Err(SessionError::DuplicateName));
        assert!(reg.rename(id_a, "Trabalho").is_ok()); // próprio nome
        assert!(reg.rename(id_a, "Projetos").is_ok());
        assert_eq!(reg.find(id_a).unwrap().name_normalized, "projetos");
    }

    #[test]
    fn rename_de_sessao_inexistente_falha() {
        let mut reg = Registry::new();
        assert_eq!(reg.rename(Uuid::new_v4(), "X"), Err(SessionError::NotFound));
    }

    #[test]
    fn remove_retorna_a_entrada() {
        let mut reg = Registry::new();
        let a = entry("Trabalho", AuthMode::Global);
        let id = a.id;
        reg.insert(a).unwrap();
        assert!(reg.remove(id).is_some());
        assert!(reg.find(id).is_none());
        assert!(reg.remove(id).is_none());
    }

    #[test]
    fn registry_novo_usa_versao_corrente() {
        assert_eq!(Registry::new().version, REGISTRY_VERSION);
    }
}
