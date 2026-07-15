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

    builder
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
