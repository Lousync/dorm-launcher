mod catalog;
mod commands;
mod config;
mod downloads;
mod error;
mod github;
mod install;
mod model;
mod paths;
mod state;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(downloads::DownloadManager::default())
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::get_config,
            commands::set_config,
            commands::default_install_root,
            commands::app_data_dir,
            commands::get_state,
            commands::get_catalog,
            commands::install_game,
            commands::cancel_task,
            commands::launch_game,
            commands::set_current_version,
            commands::uninstall_game,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
