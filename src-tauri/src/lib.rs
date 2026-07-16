pub mod crypto;
pub mod session;

use crypto::kdf::KdfParams;
use session::SessionManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default();

    // The updater is registered only in release builds, where the CI pipeline
    // injects the `plugins.updater` overlay. Without that config the plugin
    // initializer panics, so local dev/build omit it entirely.
    #[cfg(feature = "updater")]
    {
        builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    }

    builder
        // Fechar a janela bloqueia tudo (fail-closed; zeroiza GMK + sessões).
        .on_window_event(|window, event| {
            use tauri::Manager;
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if let Some(manager) = window.try_state::<SessionManager>() {
                    manager.lock_app();
                }
            }
        })
        .setup(|app| {
            use tauri::Manager;

            // Diretório de dados do app (%APPDATA%/…): raiz de registry/keyring/vaults.
            let root = app.path().app_data_dir()?;
            std::fs::create_dir_all(&root)?;
            // Parâmetros candidatos do Argon2id (⚠️ PT-01) para material novo.
            app.manage(SessionManager::new(root, KdfParams::CANDIDATE));

            // Relógio de inatividade: bloqueia sessões ociosas (VAULT-01 AC5).
            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
                if let Some(manager) = handle.try_state::<SessionManager>() {
                    manager.sweep_locks();
                }
            });

            #[cfg(feature = "updater")]
            {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = check_for_updates(handle).await {
                        eprintln!("Falha ao verificar atualizações: {error}");
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            session::commands::app_status,
            session::commands::create_global_password,
            session::commands::unlock_app,
            session::commands::lock_app,
            session::commands::change_global_password,
            session::commands::list_sessions,
            session::commands::create_session,
            session::commands::unlock_session,
            session::commands::lock_session,
            session::commands::lock_all,
            session::commands::change_master_password,
            session::commands::set_session_auth_mode,
            session::commands::set_lock_policy,
            session::commands::touch_session,
            session::commands::reveal_hint,
            session::commands::delete_session,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Secrets Storage");
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
