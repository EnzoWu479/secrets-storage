//! Camada de comandos Tauri: adaptadores finos IPC → [`SessionManager`].
//!
//! Só convertem argumentos (parse de UUID, `String` → bytes) e delegam. Toda
//! autorização/validação vive no manager (C-10). Senhas chegam como `String`
//! (única forma de material que entra pelo IPC); a zeroização dessas cópias é
//! endurecida em PT-04.

use tauri::State;
use uuid::Uuid;

use crate::session::error::{Result, SessionError};
use crate::session::manager::{AppStatus, SessionInfo, SessionManager};
use crate::session::model::{AuthMode, LockPolicy};

fn parse_id(id: &str) -> Result<Uuid> {
    Uuid::parse_str(id).map_err(|_| SessionError::NotFound)
}

#[tauri::command]
pub fn app_status(state: State<'_, SessionManager>) -> AppStatus {
    state.app_status()
}

#[tauri::command]
pub fn create_global_password(state: State<'_, SessionManager>, password: String) -> Result<()> {
    state.create_global_password(password.as_bytes())
}

#[tauri::command]
pub fn unlock_app(state: State<'_, SessionManager>, password: String) -> Result<()> {
    state.unlock_app(password.as_bytes())
}

#[tauri::command]
pub fn lock_app(state: State<'_, SessionManager>) {
    state.lock_app();
}

#[tauri::command]
pub fn change_global_password(
    state: State<'_, SessionManager>,
    current: String,
    new: String,
) -> Result<()> {
    state.change_global_password(current.as_bytes(), new.as_bytes())
}

#[tauri::command]
pub fn list_sessions(state: State<'_, SessionManager>) -> Vec<SessionInfo> {
    state.list_sessions()
}

#[tauri::command]
pub fn create_session(
    state: State<'_, SessionManager>,
    name: String,
    auth_mode: AuthMode,
    password: Option<String>,
    hint: Option<String>,
    lock_policy: LockPolicy,
) -> Result<SessionInfo> {
    state.create_session(
        &name,
        auth_mode,
        password.as_deref().map(str::as_bytes),
        hint,
        lock_policy,
    )
}

#[tauri::command]
pub fn unlock_session(
    state: State<'_, SessionManager>,
    id: String,
    password: String,
) -> Result<()> {
    state.unlock_session(&parse_id(&id)?, password.as_bytes())
}

#[tauri::command]
pub fn lock_session(state: State<'_, SessionManager>, id: String) -> Result<()> {
    state.lock_session(&parse_id(&id)?);
    Ok(())
}

#[tauri::command]
pub fn lock_all(state: State<'_, SessionManager>) {
    state.lock_all();
}

#[tauri::command]
pub fn change_master_password(
    state: State<'_, SessionManager>,
    id: String,
    current: String,
    new: String,
) -> Result<()> {
    state.change_master_password(&parse_id(&id)?, current.as_bytes(), new.as_bytes())
}

#[tauri::command]
pub fn set_session_auth_mode(
    state: State<'_, SessionManager>,
    id: String,
    new_mode: AuthMode,
    secret: String,
) -> Result<()> {
    state.set_session_auth_mode(&parse_id(&id)?, new_mode, secret.as_bytes())
}

#[tauri::command]
pub fn set_lock_policy(
    state: State<'_, SessionManager>,
    id: String,
    lock_policy: LockPolicy,
) -> Result<()> {
    state.set_lock_policy(&parse_id(&id)?, lock_policy)
}

#[tauri::command]
pub fn touch_session(state: State<'_, SessionManager>, id: String) -> Result<()> {
    state.touch_session(&parse_id(&id)?)
}

#[tauri::command]
pub fn reveal_hint(state: State<'_, SessionManager>, id: String) -> Result<Option<String>> {
    state.reveal_hint(&parse_id(&id)?)
}

#[tauri::command]
pub fn delete_session(
    state: State<'_, SessionManager>,
    id: String,
    password: String,
) -> Result<()> {
    state.delete_session(&parse_id(&id)?, password.as_bytes())
}
