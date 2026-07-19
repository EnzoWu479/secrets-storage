use thiserror::Error;
use uuid::Uuid;

use crate::secrets::model::SecretRecordV1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MoveState {
    PendingMove {
        move_id: Uuid,
        target_session_id: Uuid,
        original_revision: u64,
    },
    Staged {
        move_id: Uuid,
        source_session_id: Uuid,
        original_revision: u64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryAction {
    NoAction,
    RollbackSource,
    CompleteFromPending,
    CommitDestination,
    Conflict,
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum MoveStateError {
    #[error("origem e destino devem ser diferentes")]
    SameSession,
    #[error("revisão do segredo está obsoleta")]
    RevisionConflict,
    #[error("segredo já participa de uma movimentação")]
    MoveAlreadyPending,
    #[error("marker de movimentação incompatível")]
    MarkerMismatch,
    #[error("revisão do segredo esgotada")]
    RevisionOverflow,
}

impl SecretRecordV1 {
    pub fn is_visible(&self) -> bool {
        !matches!(self.move_state, Some(MoveState::Staged { .. }))
    }
}

pub fn begin_move(
    record: &mut SecretRecordV1,
    source_session_id: Uuid,
    target_session_id: Uuid,
    move_id: Uuid,
    expected_revision: u64,
) -> Result<(), MoveStateError> {
    if source_session_id == target_session_id {
        return Err(MoveStateError::SameSession);
    }
    if record.revision != expected_revision {
        return Err(MoveStateError::RevisionConflict);
    }
    if record.move_state.is_some() {
        return Err(MoveStateError::MoveAlreadyPending);
    }
    record.move_state = Some(MoveState::PendingMove {
        move_id,
        target_session_id,
        original_revision: record.revision,
    });
    Ok(())
}

pub fn staged_copy(
    source: &SecretRecordV1,
    source_session_id: Uuid,
) -> Result<SecretRecordV1, MoveStateError> {
    let Some(MoveState::PendingMove {
        move_id,
        original_revision,
        ..
    }) = source.move_state
    else {
        return Err(MoveStateError::MarkerMismatch);
    };
    if source.revision != original_revision {
        return Err(MoveStateError::RevisionConflict);
    }
    let revision = original_revision
        .checked_add(1)
        .ok_or(MoveStateError::RevisionOverflow)?;
    let mut staged = source.clone();
    staged.revision = revision;
    staged.move_state = Some(MoveState::Staged {
        move_id,
        source_session_id,
        original_revision,
    });
    Ok(staged)
}

pub fn commit_staged(
    target: &mut SecretRecordV1,
    expected_move_id: Uuid,
) -> Result<(), MoveStateError> {
    match target.move_state {
        Some(MoveState::Staged { move_id, .. }) if move_id == expected_move_id => {
            target.move_state = None;
            Ok(())
        }
        _ => Err(MoveStateError::MarkerMismatch),
    }
}

pub fn rollback_pending(
    source: &mut SecretRecordV1,
    expected_move_id: Uuid,
) -> Result<(), MoveStateError> {
    match source.move_state {
        Some(MoveState::PendingMove { move_id, .. }) if move_id == expected_move_id => {
            source.move_state = None;
            Ok(())
        }
        _ => Err(MoveStateError::MarkerMismatch),
    }
}

pub fn recovery_action(
    source: Option<&SecretRecordV1>,
    target: Option<&SecretRecordV1>,
    expected_move_id: Uuid,
) -> RecoveryAction {
    let pending_revision = source.and_then(|record| match record.move_state {
        Some(MoveState::PendingMove {
            move_id,
            original_revision,
            ..
        }) if move_id == expected_move_id && record.revision == original_revision => {
            Some(original_revision)
        }
        _ => None,
    });
    let staged_revision = target.and_then(|record| match record.move_state {
        Some(MoveState::Staged {
            move_id,
            original_revision,
            ..
        }) if move_id == expected_move_id
            && record.revision == original_revision.saturating_add(1) =>
        {
            Some(original_revision)
        }
        _ => None,
    });

    match (source, target, pending_revision, staged_revision) {
        (None, None, _, _) => RecoveryAction::NoAction,
        (Some(_), None, Some(_), _) => RecoveryAction::RollbackSource,
        (None, Some(_), _, Some(_)) => RecoveryAction::CommitDestination,
        (Some(_), Some(_), Some(source_revision), Some(target_revision))
            if source_revision == target_revision =>
        {
            RecoveryAction::CompleteFromPending
        }
        _ => RecoveryAction::Conflict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::model::{
        validate_new, CreateSecretInput, SecretDataInput, SecretRecordV1, SECRET_RECORD_VERSION,
    };

    fn record() -> SecretRecordV1 {
        let validated = validate_new(CreateSecretInput {
            name: "Origem".into(),
            data: SecretDataInput::SecureNote {
                text: "canary".into(),
            },
        })
        .expect("fixture válida");
        SecretRecordV1 {
            version: SECRET_RECORD_VERSION,
            id: Uuid::from_u128(10),
            revision: 7,
            name: validated.name,
            created_at_ms: 100,
            updated_at_ms: 200,
            move_state: None,
            data: validated.data,
        }
    }

    #[test]
    fn primeiro_commit_marca_pending_e_mantem_origem_visivel() {
        let source = Uuid::from_u128(1);
        let target = Uuid::from_u128(2);
        let move_id = Uuid::from_u128(3);
        let mut record = record();

        begin_move(&mut record, source, target, move_id, 7).expect("pending");

        assert!(record.is_visible());
        assert!(matches!(
            record.move_state,
            Some(MoveState::PendingMove {
                move_id: id,
                target_session_id,
                original_revision: 7
            }) if id == move_id && target_session_id == target
        ));
        assert_eq!(record.revision, 7);
    }

    #[test]
    fn origem_igual_ao_destino_e_rejeitada_sem_mutacao() {
        let session = Uuid::from_u128(1);
        let mut record = record();

        let result = begin_move(&mut record, session, session, Uuid::from_u128(3), 7);

        assert!(matches!(result, Err(MoveStateError::SameSession)));
        assert!(record.move_state.is_none());
        assert_eq!(record.revision, 7);
    }

    #[test]
    fn revisao_obsoleta_e_rejeitada_sem_mutacao() {
        let mut record = record();

        let result = begin_move(
            &mut record,
            Uuid::from_u128(1),
            Uuid::from_u128(2),
            Uuid::from_u128(3),
            6,
        );

        assert!(matches!(result, Err(MoveStateError::RevisionConflict)));
        assert!(record.move_state.is_none());
    }

    #[test]
    fn segundo_commit_cria_staged_oculto_com_nova_revisao() {
        let source = Uuid::from_u128(1);
        let target = Uuid::from_u128(2);
        let move_id = Uuid::from_u128(3);
        let mut original = record();
        begin_move(&mut original, source, target, move_id, 7).expect("pending");

        let staged = staged_copy(&original, source).expect("staged");

        assert!(!staged.is_visible());
        assert_eq!(staged.revision, 8);
        assert!(matches!(
            staged.move_state,
            Some(MoveState::Staged {
                move_id: id,
                source_session_id,
                original_revision: 7
            }) if id == move_id && source_session_id == source
        ));
    }

    #[test]
    fn quarto_commit_torna_destino_visivel() {
        let source = Uuid::from_u128(1);
        let target = Uuid::from_u128(2);
        let move_id = Uuid::from_u128(3);
        let mut original = record();
        begin_move(&mut original, source, target, move_id, 7).expect("pending");
        let mut staged = staged_copy(&original, source).expect("staged");

        commit_staged(&mut staged, move_id).expect("committed");

        assert!(staged.is_visible());
        assert!(staged.move_state.is_none());
        assert_eq!(staged.revision, 8);
    }

    #[test]
    fn rollback_remove_pending_sem_mudar_revisao() {
        let move_id = Uuid::from_u128(3);
        let mut source = record();
        begin_move(
            &mut source,
            Uuid::from_u128(1),
            Uuid::from_u128(2),
            move_id,
            7,
        )
        .expect("pending");

        rollback_pending(&mut source, move_id).expect("rollback");

        assert!(source.is_visible());
        assert!(source.move_state.is_none());
        assert_eq!(source.revision, 7);
    }

    #[test]
    fn recovery_reverte_quando_so_origem_pending_existe() {
        let move_id = Uuid::from_u128(3);
        let mut source = record();
        begin_move(
            &mut source,
            Uuid::from_u128(1),
            Uuid::from_u128(2),
            move_id,
            7,
        )
        .expect("pending");

        assert_eq!(
            recovery_action(Some(&source), None, move_id),
            RecoveryAction::RollbackSource
        );
    }

    #[test]
    fn recovery_completa_quando_pending_e_staged_correspondem() {
        let move_id = Uuid::from_u128(3);
        let mut source = record();
        begin_move(
            &mut source,
            Uuid::from_u128(1),
            Uuid::from_u128(2),
            move_id,
            7,
        )
        .expect("pending");
        let staged = staged_copy(&source, Uuid::from_u128(1)).expect("staged");

        assert_eq!(
            recovery_action(Some(&source), Some(&staged), move_id),
            RecoveryAction::CompleteFromPending
        );
    }

    #[test]
    fn recovery_confirma_destino_quando_origem_ja_foi_removida() {
        let move_id = Uuid::from_u128(3);
        let mut source = record();
        begin_move(
            &mut source,
            Uuid::from_u128(1),
            Uuid::from_u128(2),
            move_id,
            7,
        )
        .expect("pending");
        let staged = staged_copy(&source, Uuid::from_u128(1)).expect("staged");

        assert_eq!(
            recovery_action(None, Some(&staged), move_id),
            RecoveryAction::CommitDestination
        );
    }

    #[test]
    fn recovery_falha_fechado_para_markers_incompativeis() {
        let move_id = Uuid::from_u128(3);
        let mut source = record();
        begin_move(
            &mut source,
            Uuid::from_u128(1),
            Uuid::from_u128(2),
            move_id,
            7,
        )
        .expect("pending");
        let mut staged = staged_copy(&source, Uuid::from_u128(1)).expect("staged");
        if let Some(MoveState::Staged {
            original_revision, ..
        }) = &mut staged.move_state
        {
            *original_revision = 99;
        }

        assert_eq!(
            recovery_action(Some(&source), Some(&staged), move_id),
            RecoveryAction::Conflict
        );
    }
}
