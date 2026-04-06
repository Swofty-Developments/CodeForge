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
        worktree_locks: tokio::sync::Mutex::new(HashMap::new()),
    };

    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init());

    #[cfg(any(target_os = "macos", windows, target_os = "linux"))]
    {
        builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    }

    builder
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
            commands::threads::delete_messages_after,
            commands::threads::fork_thread,
            commands::sessions::send_message,
            commands::sessions::interrupt_session,
            commands::sessions::stop_session,
            commands::sessions::respond_to_approval,
            commands::sessions::get_sessions_by_thread,
            commands::sessions::get_turn_checkpoints,
            commands::settings::get_setting,
            commands::settings::get_settings_batch,
            commands::settings::set_setting,
            commands::providers::get_provider_info,
            commands::worktree::create_worktree,
            commands::worktree::get_worktree,
            commands::worktree::merge_worktree,
            commands::worktree::create_pr_from_worktree,
            commands::worktree::get_pr_status,
            commands::worktree::get_pr_review_comments,
            commands::worktree::list_open_prs,
            commands::worktree::find_thread_for_pr,
            commands::worktree::checkout_pr_into_worktree,
            commands::worktree::link_pr_to_thread,
            commands::worktree::validate_worktrees,
            commands::worktree::repair_worktree,
            commands::worktree::cleanup_worktrees,
            commands::worktree::get_conflict_files,
            commands::worktree::get_conflict_markers,
            commands::worktree::resolve_conflict,
            commands::worktree::finalize_merge,
            commands::worktree::abort_merge,
            commands::worktree::undo_to_commit,
            commands::worktree::get_head_commit,
            commands::worktree::check_worktree_sync_status,
            commands::search::search_messages,
            commands::usage::get_usage_summary,
            commands::usage::get_thread_usage,
            commands::diff::get_changed_files,
            commands::diff::get_session_diff,
            commands::diff::get_file_diff,
            commands::diff::get_file_content,
            commands::diff::get_turn_diff,
            commands::diff::get_turn_changed_files,
            commands::blame::get_file_blame,
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
            commands::mcp::mcp_list_servers,
            commands::mcp::mcp_add_server,
            commands::mcp::mcp_remove_server,
            commands::mcp::list_slash_commands,
            commands::skills::list_skills,
            commands::skills::install_skill,
            commands::skills::uninstall_skill,
            commands::skills::enable_skill,
            commands::skills::disable_skill,
            commands::skills::list_marketplaces,
            commands::skills::add_marketplace,
            commands::onboarding::check_setup_status,
            commands::onboarding::complete_setup,
            commands::github::gh_auth_status,
            commands::github::gh_login,
            commands::github::list_prs,
            commands::github::get_pr_diff,
            commands::github::list_issues,
            commands::github::get_issue_context,
            commands::github::get_repo_info,
            commands::github::is_github_repo,
            commands::git::git_log,
            commands::git::git_branches,
            commands::git::git_checkout,
            commands::git::git_create_branch,
            commands::git::git_commit,
            commands::git::git_push,
            commands::git::git_fetch,
            commands::git::git_pull,
            commands::git::git_push_force,
            commands::git::git_delete_branch,
            commands::git::git_merge_branch,
            commands::git::git_stash,
            commands::git::git_stash_pop,
            commands::git::git_create_pr,
            commands::git::git_diff_branches,
            commands::git::git_status,
            commands::git::git_check_remote,
            commands::git::git_pull_branch,
            commands::git::git_init_repo,
            commands::git::git_repo_status,
            commands::themes::list_themes,
            commands::themes::import_theme,
            commands::themes::delete_custom_theme,
            commands::themes::export_theme,
            commands::filesystem::open_in_file_manager,
            commands::filesystem::open_external_url,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
