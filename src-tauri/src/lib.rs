//! Thin Tauri shell over `attestrum-transcription-core`: managed state,
//! command registration, and event plumbing. Contract:
//! `docs/diagrams/architecture/ipc-transcribe-sequence.md`.

mod commands;
mod error;
mod state;

use attestrum_transcription_core::store::Store;
use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let default_root = app.path().app_data_dir()?;
            // Settings can point the store somewhere else; bootstrap from the
            // default location to find out.
            let bootstrap = Store::open(&default_root)?;
            let root = match bootstrap.load_settings().storage_dir {
                Some(dir) => std::path::PathBuf::from(dir),
                None => default_root,
            };
            let store = Store::open(&root)?;
            app.manage(AppState::new(store));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::product_info,
            commands::list_models,
            commands::download_model,
            commands::cancel_download,
            commands::delete_model,
            commands::list_input_devices,
            commands::start_recording,
            commands::stop_recording,
            commands::transcribe,
            commands::cancel_job,
            commands::list_transcripts,
            commands::get_transcript,
            commands::update_transcript,
            commands::rename_transcript,
            commands::delete_transcript,
            commands::player_load,
            commands::player_play,
            commands::player_pause,
            commands::player_seek,
            commands::player_peaks,
            commands::export_transcript,
            commands::get_settings,
            commands::set_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
