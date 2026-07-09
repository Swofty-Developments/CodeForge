#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod db;
mod events;
mod migrations;
mod queries;
mod runtime;
mod state;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use forge_session::SessionManager;

use crate::db::Database;
use crate::state::AppState;

/// Locate the app DB under the user's home. A missing home is a real,
/// surfaced error — never a silent `.` fallback that scatters the DB into
/// whatever cwd the app happened to launch from.
fn db_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| "cannot locate home directory (neither HOME nor USERPROFILE is set)".to_string())?;
    Ok(PathBuf::from(home).join(".featureforge").join("featureforge.db"))
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "featureforge=debug,forge=debug,info".into()),
        )
        .init();

    let db_path = match db_path() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("featureforge: {e}");
            std::process::exit(1);
        }
    };
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let db = Database::open(&db_path).expect("failed to open app database");

    let app_state = AppState {
        db: Arc::new(Mutex::new(db)),
        repos: tokio::sync::Mutex::new(HashMap::new()),
        sessions: tokio::sync::Mutex::new(SessionManager::new()),
        session_repos: tokio::sync::Mutex::new(HashMap::new()),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::repo::open_repo,
            commands::repo::close_repo,
            commands::repo::reindex_repo,
            commands::repo::daemon_status,
            commands::features::get_features,
            commands::features::get_feature,
            commands::features::get_feature_doc,
            commands::features::pin_feature,
            commands::features::update_feature,
            commands::timeline::get_timeline,
            commands::timeline::get_diff_by_feature,
            commands::sessions::start_session,
            commands::sessions::send_session_input,
            commands::sessions::approve_session,
            commands::sessions::stop_session,
            commands::sessions::list_sessions,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
