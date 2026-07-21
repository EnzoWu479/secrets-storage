//! Atraso progressivo (backoff exponencial com teto) para tentativas de
//! desbloqueio, mantido em memória. O tempo é injetado (`now_ms`), seguindo o
//! padrão do coordenador de clipboard; nunca apaga sessão nem cofre.

use std::collections::BTreeMap;

use uuid::Uuid;

use crate::sessions::model::SessionError;

/// Atraso base após a primeira falha (1 s).
pub const BASE_BACKOFF_MS: u64 = 1_000;
/// Teto do atraso progressivo (5 min).
pub const MAX_BACKOFF_MS: u64 = 5 * 60 * 1_000;

/// Estado de tentativas de um único alvo (a GMP ou uma sessão específica).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AttemptState {
    failures: u32,
    next_allowed_ms: u64,
}

impl AttemptState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn failures(&self) -> u32 {
        self.failures
    }

    /// Atraso que será aplicado após `failures` falhas consecutivas.
    pub fn backoff_ms(failures: u32) -> u64 {
        if failures == 0 {
            return 0;
        }
        let shift = failures - 1;
        if shift >= 63 {
            return MAX_BACKOFF_MS;
        }
        BASE_BACKOFF_MS
            .checked_shl(shift)
            .map_or(MAX_BACKOFF_MS, |value| value.min(MAX_BACKOFF_MS))
    }

    /// `Ok` se já é permitido tentar; caso contrário, `TooManyAttempts`.
    pub fn check(&self, now_ms: u64) -> Result<(), SessionError> {
        if now_ms >= self.next_allowed_ms {
            Ok(())
        } else {
            Err(SessionError::TooManyAttempts)
        }
    }

    /// Registra uma falha e agenda a próxima tentativa permitida.
    pub fn record_failure(&mut self, now_ms: u64) {
        self.failures = self.failures.saturating_add(1);
        let backoff = Self::backoff_ms(self.failures);
        self.next_allowed_ms = now_ms.saturating_add(backoff);
    }

    /// Zera o estado após um desbloqueio bem-sucedido.
    pub fn record_success(&mut self) {
        self.failures = 0;
        self.next_allowed_ms = 0;
    }
}

/// Alvo do atraso: o gate global (GMP) ou uma sessão específica.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttemptKey {
    Global,
    Session(Uuid),
}

/// Rastreador com estados independentes para a GMP e para cada sessão.
#[derive(Debug, Default)]
pub struct AttemptTracker {
    global: AttemptState,
    sessions: BTreeMap<Uuid, AttemptState>,
}

impl AttemptTracker {
    pub fn new() -> Self {
        Self::default()
    }

    fn state(&self, key: AttemptKey) -> AttemptState {
        match key {
            AttemptKey::Global => self.global,
            AttemptKey::Session(id) => self.sessions.get(&id).copied().unwrap_or_default(),
        }
    }

    fn state_mut(&mut self, key: AttemptKey) -> &mut AttemptState {
        match key {
            AttemptKey::Global => &mut self.global,
            AttemptKey::Session(id) => self.sessions.entry(id).or_default(),
        }
    }

    pub fn check(&self, key: AttemptKey, now_ms: u64) -> Result<(), SessionError> {
        self.state(key).check(now_ms)
    }

    pub fn record_failure(&mut self, key: AttemptKey, now_ms: u64) {
        self.state_mut(key).record_failure(now_ms);
    }

    pub fn record_success(&mut self, key: AttemptKey) {
        self.state_mut(key).record_success();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nova_tentativa_e_permitida_imediatamente() {
        assert!(AttemptState::new().check(0).is_ok());
    }

    #[test]
    fn falha_agenda_proxima_tentativa() {
        let mut state = AttemptState::new();
        state.record_failure(10_000);
        assert_eq!(state.check(10_000 + BASE_BACKOFF_MS - 1), Err(SessionError::TooManyAttempts));
        assert!(state.check(10_000 + BASE_BACKOFF_MS).is_ok());
    }

    #[test]
    fn backoff_cresce_e_atinge_o_teto() {
        assert_eq!(AttemptState::backoff_ms(0), 0);
        assert_eq!(AttemptState::backoff_ms(1), BASE_BACKOFF_MS);
        assert_eq!(AttemptState::backoff_ms(2), BASE_BACKOFF_MS * 2);
        assert_eq!(AttemptState::backoff_ms(3), BASE_BACKOFF_MS * 4);
        assert_eq!(AttemptState::backoff_ms(100), MAX_BACKOFF_MS);
        // Monotônico e limitado ao teto.
        assert!(AttemptState::backoff_ms(20) <= MAX_BACKOFF_MS);
    }

    #[test]
    fn sucesso_reseta_o_estado() {
        let mut state = AttemptState::new();
        state.record_failure(0);
        state.record_failure(0);
        assert_eq!(state.failures(), 2);
        state.record_success();
        assert_eq!(state.failures(), 0);
        assert!(state.check(0).is_ok());
    }

    #[test]
    fn global_e_sessao_sao_independentes() {
        let mut tracker = AttemptTracker::new();
        let session = Uuid::new_v4();

        // Muitas falhas globais não devem atrasar a sessão.
        tracker.record_failure(AttemptKey::Global, 0);
        tracker.record_failure(AttemptKey::Global, 0);
        assert_eq!(
            tracker.check(AttemptKey::Global, 0),
            Err(SessionError::TooManyAttempts)
        );
        assert!(tracker.check(AttemptKey::Session(session), 0).is_ok());
    }

    #[test]
    fn sessoes_distintas_nao_compartilham_atraso() {
        let mut tracker = AttemptTracker::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        tracker.record_failure(AttemptKey::Session(a), 0);
        assert_eq!(
            tracker.check(AttemptKey::Session(a), 0),
            Err(SessionError::TooManyAttempts)
        );
        assert!(tracker.check(AttemptKey::Session(b), 0).is_ok());
    }
}
