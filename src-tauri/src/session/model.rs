//! Tipos persistidos do registro de sessões (metadados não secretos).
//!
//! O `registry.json` é legível com o app bloqueado (AD-013 / VAULT-01 AC9): guarda
//! id, nome, `auth_mode`, dica, política e data de criação — **nunca** material de
//! chave. O nome também vai autenticado na AAD do `.vault`; no `unlock` o core
//! confere que batem (detecta adulteração do registro).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Versão do formato do `registry.json` (fail-closed em versão futura).
pub const REGISTRY_VERSION: u16 = 1;

/// Modo de autenticação da sessão (também autenticado na AAD do `.vault`, D-04).
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum AuthMode {
    /// Aberta em conjunto ao desbloquear a GMP (padrão).
    Global,
    /// Isolada por senha própria (opt-out).
    Own,
}

/// Política de bloqueio automático por sessão (VAULT-01 AC4/AC5).
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct LockPolicy {
    /// Inatividade até bloquear, em segundos; `None` = "nunca".
    pub inactivity_secs: Option<u64>,
    /// Bloquear ao bloquear o Windows (best-effort nesta fatia).
    pub on_windows_lock: bool,
    /// Bloquear ao suspender o Windows (best-effort nesta fatia).
    pub on_windows_suspend: bool,
}

impl Default for LockPolicy {
    fn default() -> Self {
        // Padrão do design: 15 min, reage a lock/suspend do Windows.
        Self {
            inactivity_secs: Some(900),
            on_windows_lock: true,
            on_windows_suspend: true,
        }
    }
}

/// Entrada de uma sessão no registro (metadado não secreto).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SessionEntry {
    /// Identificador estável da sessão (também na AAD do `.vault`).
    pub id: Uuid,
    /// Nome exibido, escolhido pelo usuário.
    pub name: String,
    /// Nome normalizado para unicidade case-insensitive (VAULT-01 AC13).
    pub name_normalized: String,
    /// Modo de autenticação corrente.
    pub auth_mode: AuthMode,
    /// Dica opcional; metadado não secreto (VAULT-04 AC3).
    pub hint: Option<String>,
    /// Política de bloqueio automático.
    pub lock_policy: LockPolicy,
    /// Criação, em segundos desde a época Unix (o frontend formata).
    pub created_at_unix: u64,
}

/// Registro persistido de todas as sessões.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Registry {
    /// Versão do formato do registro.
    pub version: u16,
    /// Sessões conhecidas (bloqueadas ou não).
    pub sessions: Vec<SessionEntry>,
}

impl Registry {
    /// Registro vazio da versão corrente (1º uso).
    pub fn empty() -> Self {
        Self {
            version: REGISTRY_VERSION,
            sessions: Vec::new(),
        }
    }

    /// Encontra uma sessão pelo id.
    pub fn find(&self, id: &Uuid) -> Option<&SessionEntry> {
        self.sessions.iter().find(|s| &s.id == id)
    }

    /// Encontra uma sessão pelo id (mutável).
    pub fn find_mut(&mut self, id: &Uuid) -> Option<&mut SessionEntry> {
        self.sessions.iter_mut().find(|s| &s.id == id)
    }

    /// Indica se o nome normalizado já existe (ignorando uma sessão, ex.: renomeação).
    pub fn name_taken(&self, normalized: &str, except: Option<&Uuid>) -> bool {
        self.sessions
            .iter()
            .any(|s| s.name_normalized == normalized && except != Some(&s.id))
    }
}

/// Normaliza um nome de sessão para comparação de unicidade case-insensitive.
///
/// Apara espaços das pontas, colapsa espaços internos e converte para minúsculas.
pub fn normalize_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizacao_ignora_caixa_e_espacos() {
        assert_eq!(normalize_name("  Trabalho  "), "trabalho");
        assert_eq!(normalize_name("Meu   Projeto"), "meu projeto");
        assert_eq!(normalize_name("TRABALHO"), normalize_name("trabalho"));
    }

    #[test]
    fn name_taken_respeita_a_excecao() {
        let id = Uuid::new_v4();
        let reg = Registry {
            version: REGISTRY_VERSION,
            sessions: vec![SessionEntry {
                id,
                name: "Trabalho".into(),
                name_normalized: "trabalho".into(),
                auth_mode: AuthMode::Global,
                hint: None,
                lock_policy: LockPolicy::default(),
                created_at_unix: 0,
            }],
        };
        assert!(reg.name_taken("trabalho", None));
        assert!(!reg.name_taken("trabalho", Some(&id)));
        assert!(!reg.name_taken("pessoal", None));
    }
}
