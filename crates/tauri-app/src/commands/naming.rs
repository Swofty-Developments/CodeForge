use tauri::State;
use crate::state::TauriState;

/// Spawns a quick separate CLI call to generate a short thread name.
/// Uses whatever provider the thread is using (claude or codex).
/// Runs independently — does not interfere with the main session.
#[tauri::command]
pub async fn auto_name_thread(
    state: State<'_, TauriState>,
    thread_id: String,
    messages_summary: String,
    provider: String,
) -> Result<String, String> {
    let prompt = format!(
        "Given this conversation, write a short title (max 5 words, no quotes, no punctuation at the end). Just output the title, nothing else.\n\n{}",
        messages_summary
    );

    let (cmd, args): (&str, Vec<&str>) = match provider.as_str() {
        "codex" => ("codex", vec!["-q", &prompt]),
        _ => ("claude", vec!["-p", "--model", "haiku", &prompt]),
    };

    let output = tokio::process::Command::new(cmd)
        .args(&args)
        .output()
        .await
        .map_err(|e| format!("Failed to run {cmd} for naming: {e}"))?;

    if !output.status.success() {
        return Err(format!("{cmd} naming call failed"));
    }

    let name = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_string();

    if name.is_empty() {
        return Err("Empty name returned".into());
    }

    // Persist the rename
    let tid = uuid::Uuid::parse_str(&thread_id).map_err(|e| e.to_string())?;
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let _ = codeforge_persistence::queries::update_thread_title(db.conn(), tid, &name);
    }

    Ok(name)
}
