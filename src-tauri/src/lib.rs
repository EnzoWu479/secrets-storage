pub mod crypto;
pub mod platform;
#[cfg(feature = "security-proof")]
pub mod proof;
pub mod secrets;
pub mod security;
pub mod sessions;
pub mod storage;

use std::any::Any;
use std::sync::Arc;

use security::lock::{LockCoordinator, LockOutcome, LockReason};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitLifecycleEvent {
    ExitRequested,
    Exit,
    WindowCloseRequested,
}

pub struct ApplicationSensitiveState {
    _owned: Box<dyn Any + Send + 'static>,
}

impl ApplicationSensitiveState {
    pub fn new<T: Send + 'static>(state: T) -> Self {
        Self {
            _owned: Box::new(state),
        }
    }

    #[cfg(feature = "security-proof")]
    pub(crate) fn value_mut<T: Send + 'static>(&mut self) -> Option<&mut T> {
        self._owned.downcast_mut()
    }
}

pub type ApplicationLockCoordinator = LockCoordinator<ApplicationSensitiveState>;

pub struct ApplicationLifecycle {
    coordinator: Arc<ApplicationLockCoordinator>,
}

impl ApplicationLifecycle {
    pub fn new() -> Self {
        Self {
            coordinator: Arc::new(ApplicationLockCoordinator::default()),
        }
    }

    pub fn coordinator(&self) -> Arc<ApplicationLockCoordinator> {
        Arc::clone(&self.coordinator)
    }

    pub fn apply(&self, event: ExitLifecycleEvent) -> Option<LockOutcome> {
        apply_exit_lifecycle(self.coordinator.as_ref(), event)
    }
}

impl Default for ApplicationLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

pub fn apply_exit_lifecycle<T>(
    coordinator: &LockCoordinator<T>,
    event: ExitLifecycleEvent,
) -> Option<LockOutcome> {
    match event {
        ExitLifecycleEvent::ExitRequested | ExitLifecycleEvent::Exit => {
            Some(coordinator.lock(LockReason::Exiting))
        }
        ExitLifecycleEvent::WindowCloseRequested => None,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();

    // The updater is registered only in release builds, where the CI pipeline
    // injects the `plugins.updater` overlay. Without that config the plugin
    // initializer panics, so local dev/build omit it entirely.
    #[cfg(feature = "updater")]
    let builder = builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = check_for_updates(handle).await {
                    eprintln!("Falha ao verificar atualizações: {error}");
                }
            });
            Ok(())
        });

    let lifecycle = ApplicationLifecycle::new();
    let builder = builder.manage(lifecycle.coordinator());
    #[cfg(all(windows, feature = "security-proof"))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        proof::commands::proof_install_canary,
        proof::commands::proof_authorized_probe,
        proof::commands::proof_lock,
        proof::commands::proof_status
    ]);
    let app = builder
        .build(tauri::generate_context!())
        .expect("failed to build Secrets Storage");

    app.run(move |_app_handle, event| {
        let lifecycle_event = match event {
            tauri::RunEvent::ExitRequested { .. } => Some(ExitLifecycleEvent::ExitRequested),
            tauri::RunEvent::Exit => Some(ExitLifecycleEvent::Exit),
            _ => None,
        };
        if let Some(lifecycle_event) = lifecycle_event {
            let _ = lifecycle.apply(lifecycle_event);
        }
    });
}

#[cfg(feature = "updater")]
async fn check_for_updates(app: tauri::AppHandle) -> tauri_plugin_updater::Result<()> {
    use tauri_plugin_updater::UpdaterExt;

    if let Some(update) = app.updater()?.check().await? {
        update
            .download_and_install(|_downloaded, _total| {}, || {})
            .await?;
        app.restart();
    }

    Ok(())
}
