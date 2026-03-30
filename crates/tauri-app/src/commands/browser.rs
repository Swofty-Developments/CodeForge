use std::collections::HashMap;
use std::sync::Arc;

use serde::Serialize;
use tauri::Emitter;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, Mutex};

fn browsers() -> &'static Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>> {
    static I: std::sync::OnceLock<Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>>> =
        std::sync::OnceLock::new();
    I.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

fn sidecar_path() -> Result<std::path::PathBuf, String> {
    let cwd = std::env::current_dir().unwrap_or_default();
    [
        cwd.join("crates/tauri-app/browser-sidecar/browser.mjs"),
        cwd.join("browser-sidecar/browser.mjs"),
    ]
    .into_iter()
    .find(|p| p.exists())
    .ok_or_else(|| "Browser sidecar not found".into())
}

async fn ensure(tid: &str, app: &tauri::AppHandle) -> Result<(), String> {
    let mut map = browsers().lock().await;
    if map.contains_key(tid) { return Ok(()); }

    let script = sidecar_path()?;
    let mut child = Command::new("node")
        .arg(&script)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("spawn: {e}"))?;

    let stdout = child.stdout.take().ok_or("no stdout")?;
    let stdin = child.stdin.take().ok_or("no stdin")?;
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    tokio::spawn(async move {
        let mut w = stdin;
        while let Some(msg) = rx.recv().await {
            let _ = w.write_all(msg.as_bytes()).await;
            let _ = w.write_all(b"\n").await;
            let _ = w.flush().await;
        }
        drop(child);
    });

    let app2 = app.clone();
    let id = tid.to_string();
    tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() { continue; }
            if let Ok(ev) = serde_json::from_str::<serde_json::Value>(&line) {
                #[derive(Serialize, Clone)]
                struct P { thread_id: String, #[serde(flatten)] d: serde_json::Value }
                let _ = app2.emit("browser-event", P { thread_id: id.clone(), d: ev });
            }
        }
    });

    map.insert(tid.to_string(), tx);
    Ok(())
}

fn cmd(map: &HashMap<String, mpsc::UnboundedSender<String>>, tid: &str, v: serde_json::Value) -> Result<(), String> {
    map.get(tid).ok_or("no browser")?.send(v.to_string()).map_err(|_| "closed".into())
}

#[tauri::command]
pub async fn browser_navigate(app: tauri::AppHandle, thread_id: String, url: String) -> Result<(), String> {
    ensure(&thread_id, &app).await?;
    let m = browsers().lock().await;
    cmd(&*m, &thread_id, serde_json::json!({"cmd":"navigate","url":url}))
}

#[tauri::command]
pub async fn browser_click(app: tauri::AppHandle, thread_id: String, x: f64, y: f64) -> Result<(), String> {
    ensure(&thread_id, &app).await?;
    cmd(&*browsers().lock().await, &thread_id, serde_json::json!({"cmd":"click","x":x,"y":y}))
}

#[tauri::command]
pub async fn browser_scroll(app: tauri::AppHandle, thread_id: String, x: f64, y: f64, delta_x: f64, delta_y: f64) -> Result<(), String> {
    ensure(&thread_id, &app).await?;
    cmd(&*browsers().lock().await, &thread_id, serde_json::json!({"cmd":"scroll","x":x,"y":y,"deltaX":delta_x,"deltaY":delta_y}))
}

#[tauri::command]
pub async fn browser_mouse_move(app: tauri::AppHandle, thread_id: String, x: f64, y: f64) -> Result<(), String> {
    ensure(&thread_id, &app).await?;
    cmd(&*browsers().lock().await, &thread_id, serde_json::json!({"cmd":"mouseMove","x":x,"y":y}))
}

#[tauri::command]
pub async fn browser_key_down(app: tauri::AppHandle, thread_id: String, key: String, text: String) -> Result<(), String> {
    ensure(&thread_id, &app).await?;
    cmd(&*browsers().lock().await, &thread_id, serde_json::json!({"cmd":"keyDown","key":key,"text":text}))
}

#[tauri::command]
pub async fn browser_key_up(app: tauri::AppHandle, thread_id: String, key: String) -> Result<(), String> {
    ensure(&thread_id, &app).await?;
    cmd(&*browsers().lock().await, &thread_id, serde_json::json!({"cmd":"keyUp","key":key}))
}

#[tauri::command]
pub async fn browser_type_text(app: tauri::AppHandle, thread_id: String, text: String) -> Result<(), String> {
    ensure(&thread_id, &app).await?;
    cmd(&*browsers().lock().await, &thread_id, serde_json::json!({"cmd":"type","text":text}))
}

#[tauri::command]
pub async fn browser_back(app: tauri::AppHandle, thread_id: String) -> Result<(), String> {
    ensure(&thread_id, &app).await?;
    cmd(&*browsers().lock().await, &thread_id, serde_json::json!({"cmd":"back"}))
}

#[tauri::command]
pub async fn browser_forward(app: tauri::AppHandle, thread_id: String) -> Result<(), String> {
    ensure(&thread_id, &app).await?;
    cmd(&*browsers().lock().await, &thread_id, serde_json::json!({"cmd":"forward"}))
}

#[tauri::command]
pub async fn browser_reload(app: tauri::AppHandle, thread_id: String) -> Result<(), String> {
    ensure(&thread_id, &app).await?;
    cmd(&*browsers().lock().await, &thread_id, serde_json::json!({"cmd":"reload"}))
}

#[tauri::command]
pub async fn browser_resize(app: tauri::AppHandle, thread_id: String, width: u32, height: u32) -> Result<(), String> {
    ensure(&thread_id, &app).await?;
    cmd(&*browsers().lock().await, &thread_id, serde_json::json!({"cmd":"resize","width":width,"height":height}))
}

#[tauri::command]
pub async fn browser_start_inspect(app: tauri::AppHandle, thread_id: String) -> Result<(), String> {
    ensure(&thread_id, &app).await?;
    cmd(&*browsers().lock().await, &thread_id, serde_json::json!({"cmd":"startInspect"}))
}

#[tauri::command]
pub async fn browser_stop_inspect(app: tauri::AppHandle, thread_id: String) -> Result<(), String> {
    ensure(&thread_id, &app).await?;
    cmd(&*browsers().lock().await, &thread_id, serde_json::json!({"cmd":"stopInspect"}))
}

#[tauri::command]
pub async fn browser_extract(app: tauri::AppHandle, thread_id: String) -> Result<(), String> {
    ensure(&thread_id, &app).await?;
    cmd(&*browsers().lock().await, &thread_id, serde_json::json!({"cmd":"extractElement"}))
}

#[tauri::command]
pub async fn browser_close(thread_id: String) -> Result<(), String> {
    let mut m = browsers().lock().await;
    if let Some(tx) = m.remove(&thread_id) {
        let _ = tx.send(r#"{"cmd":"close"}"#.into());
    }
    Ok(())
}
