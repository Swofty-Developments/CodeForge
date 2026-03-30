#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;
mod streaming;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use codeforge_persistence::Database;
use codeforge_session::SessionManager;
use state::TauriState;

fn db_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".codeforge")
        .join("codeforge.db")
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("codeforge=debug,info")
        .init();

    let db_path = db_path();
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let db = Database::open(&db_path).expect("Failed to open database");
    let tauri_state = TauriState {
        db: Arc::new(std::sync::Mutex::new(db)),
        session_manager: tokio::sync::Mutex::new(SessionManager::new()),
        thread_sessions: tokio::sync::Mutex::new(HashMap::new()),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(tauri_state)
        .invoke_handler(tauri::generate_handler![
            commands::projects::get_all_projects,
            commands::projects::get_threads_by_project,
            commands::projects::get_messages_by_thread,
            commands::projects::create_project,
            commands::projects::rename_project,
            commands::projects::delete_project,
            commands::threads::create_thread,
            commands::threads::rename_thread,
            commands::threads::set_thread_color,
            commands::threads::delete_thread,
            commands::threads::move_thread_to_project,
            commands::threads::persist_user_message,
            commands::sessions::send_message,
            commands::sessions::stop_session,
            commands::sessions::respond_to_approval,
            commands::settings::get_setting,
            commands::settings::set_setting,
            commands::providers::get_provider_info,
            commands::worktree::create_worktree,
            commands::worktree::get_worktree,
            commands::worktree::merge_worktree,
            commands::search::search_messages,
            commands::usage::get_usage_summary,
            commands::usage::get_thread_usage,
            commands::diff::get_changed_files,
            commands::diff::get_session_diff,
            commands::diff::get_file_diff,
            commands::diff::get_file_content,
            commands::naming::auto_name_thread,
            commands::browser::browser_navigate,
            commands::browser::browser_click,
            commands::browser::browser_scroll,
            commands::browser::browser_mouse_move,
            commands::browser::browser_key_down,
            commands::browser::browser_key_up,
            commands::browser::browser_type_text,
            commands::browser::browser_back,
            commands::browser::browser_forward,
            commands::browser::browser_reload,
            commands::browser::browser_resize,
            commands::browser::browser_start_inspect,
            commands::browser::browser_stop_inspect,
            commands::browser::browser_extract,
            commands::browser::browser_close,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
