use std::collections::HashMap;
use std::sync::Arc;

use serde::Serialize;
use tauri::Emitter;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, Mutex};

fn browsers() -> &'static Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>> {
    static INSTANCE: std::sync::OnceLock<Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>>> =
        std::sync::OnceLock::new();
    INSTANCE.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

fn sidecar_path() -> Result<std::path::PathBuf, String> {
    let cwd = std::env::current_dir().unwrap_or_default();
    let candidates = [
        cwd.join("crates/tauri-app/browser-sidecar/browser.mjs"),
        cwd.join("browser-sidecar/browser.mjs"),
    ];
    candidates.into_iter().find(|p| p.exists())
        .ok_or_else(|| "Browser sidecar not found".into())
}

async fn ensure_browser(thread_id: &str, app: &tauri::AppHandle) -> Result<(), String> {
    let mut map = browsers().lock().await;
    if map.contains_key(thread_id) {
        return Ok(());
    }

    let script = sidecar_path()?;
    let mut child = Command::new("node")
        .arg(&script)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("Failed to spawn browser: {e}"))?;

    let stdout = child.stdout.take().ok_or("No stdout")?;
    let stdin = child.stdin.take().ok_or("No stdin")?;
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Stdin writer
    tokio::spawn(async move {
        let mut stdin = stdin;
        while let Some(msg) = rx.recv().await {
            let _ = stdin.write_all(msg.as_bytes()).await;
            let _ = stdin.write_all(b"\n").await;
            let _ = stdin.flush().await;
        }
        drop(child);
    });

    // Stdout reader — forward to frontend
    let app_handle = app.clone();
    let tid = thread_id.to_string();
    tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() { continue; }
            if let Ok(event) = serde_json::from_str::<serde_json::Value>(&line) {
                #[derive(Serialize, Clone)]
                struct Payload {
                    thread_id: String,
                    #[serde(flatten)]
                    data: serde_json::Value,
                }
                let _ = app_handle.emit("browser-event", Payload {
                    thread_id: tid.clone(),
                    data: event,
                });
            }
        }
    });

    map.insert(thread_id.to_string(), tx);
    Ok(())
}

fn send_cmd(map: &HashMap<String, mpsc::UnboundedSender<String>>, thread_id: &str, cmd: serde_json::Value) -> Result<(), String> {
    let tx = map.get(thread_id).ok_or("No browser for thread")?;
    tx.send(cmd.to_string()).map_err(|_| "Browser stdin closed".into())
}

#[tauri::command]
pub async fn browser_navigate(app: tauri::AppHandle, thread_id: String, url: String) -> Result<(), String> {
    ensure_browser(&thread_id, &app).await?;
    let map = browsers().lock().await;
    send_cmd(&map, &thread_id, serde_json::json!({"cmd": "navigate", "url": url}))
}

#[tauri::command]
pub async fn browser_click(app: tauri::AppHandle, thread_id: String, x: f64, y: f64) -> Result<(), String> {
    ensure_browser(&thread_id, &app).await?;
    let map = browsers().lock().await;
    send_cmd(&map, &thread_id, serde_json::json!({"cmd": "click", "x": x, "y": y}))
}

#[tauri::command]
pub async fn browser_scroll(app: tauri::AppHandle, thread_id: String, delta_y: f64) -> Result<(), String> {
    ensure_browser(&thread_id, &app).await?;
    let map = browsers().lock().await;
    send_cmd(&map, &thread_id, serde_json::json!({"cmd": "scroll", "deltaY": delta_y}))
}

#[tauri::command]
pub async fn browser_type_text(app: tauri::AppHandle, thread_id: String, text: String) -> Result<(), String> {
    ensure_browser(&thread_id, &app).await?;
    let map = browsers().lock().await;
    send_cmd(&map, &thread_id, serde_json::json!({"cmd": "type", "text": text}))
}

#[tauri::command]
pub async fn browser_keypress(app: tauri::AppHandle, thread_id: String, key: String) -> Result<(), String> {
    ensure_browser(&thread_id, &app).await?;
    let map = browsers().lock().await;
    send_cmd(&map, &thread_id, serde_json::json!({"cmd": "keypress", "key": key}))
}

#[tauri::command]
pub async fn browser_back(app: tauri::AppHandle, thread_id: String) -> Result<(), String> {
    ensure_browser(&thread_id, &app).await?;
    let map = browsers().lock().await;
    send_cmd(&map, &thread_id, serde_json::json!({"cmd": "back"}))
}

#[tauri::command]
pub async fn browser_forward(app: tauri::AppHandle, thread_id: String) -> Result<(), String> {
    ensure_browser(&thread_id, &app).await?;
    let map = browsers().lock().await;
    send_cmd(&map, &thread_id, serde_json::json!({"cmd": "forward"}))
}

#[tauri::command]
pub async fn browser_reload(app: tauri::AppHandle, thread_id: String) -> Result<(), String> {
    ensure_browser(&thread_id, &app).await?;
    let map = browsers().lock().await;
    send_cmd(&map, &thread_id, serde_json::json!({"cmd": "reload"}))
}

#[tauri::command]
pub async fn browser_resize(app: tauri::AppHandle, thread_id: String, width: u32, height: u32) -> Result<(), String> {
    ensure_browser(&thread_id, &app).await?;
    let map = browsers().lock().await;
    send_cmd(&map, &thread_id, serde_json::json!({"cmd": "resize", "width": width, "height": height}))
}

#[tauri::command]
pub async fn browser_close(thread_id: String) -> Result<(), String> {
    let mut map = browsers().lock().await;
    if let Some(tx) = map.remove(&thread_id) {
        let _ = tx.send(r#"{"cmd":"close"}"#.into());
    }
    Ok(())
}
