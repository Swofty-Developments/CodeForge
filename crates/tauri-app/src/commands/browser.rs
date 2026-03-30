use std::collections::HashMap;
use std::sync::Arc;

use tauri::{LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewBuilder};
use tokio::sync::Mutex;

struct BrowserWebview {
    label: String,
}

fn browsers() -> &'static Arc<Mutex<HashMap<String, BrowserWebview>>> {
    static INSTANCE: std::sync::OnceLock<Arc<Mutex<HashMap<String, BrowserWebview>>>> =
        std::sync::OnceLock::new();
    INSTANCE.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

fn webview_label(thread_id: &str) -> String {
    // Labels must be alphanumeric + hyphens
    format!("browser-{}", thread_id.replace(|c: char| !c.is_alphanumeric() && c != '-', ""))
}

#[tauri::command]
pub async fn browser_open(
    app: tauri::AppHandle,
    thread_id: String,
    url: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let label = webview_label(&thread_id);
    let mut map = browsers().lock().await;

    // If already exists, just navigate and reposition
    if map.contains_key(&thread_id) {
        if let Some(wv) = app.get_webview(&label) {
            let parsed: url::Url = url.parse().map_err(|e: url::ParseError| e.to_string())?;
            wv.navigate(parsed).map_err(|e| e.to_string())?;
            wv.set_position(LogicalPosition::new(x, y)).map_err(|e| e.to_string())?;
            wv.set_size(LogicalSize::new(width, height)).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }

    let parsed: url::Url = url.parse().unwrap_or_else(|_| "about:blank".parse().unwrap());

    // Get the underlying Window (not WebviewWindow) to call add_child
    let windows = app.windows();
    let window = windows.values().next().ok_or("No window found")?;

    let builder = WebviewBuilder::new(&label, WebviewUrl::External(parsed));

    window
        .add_child(
            builder,
            LogicalPosition::new(x, y),
            LogicalSize::new(width, height),
        )
        .map_err(|e| format!("Failed to create browser webview: {e}"))?;

    map.insert(thread_id, BrowserWebview { label });
    Ok(())
}

#[tauri::command]
pub async fn browser_navigate(
    app: tauri::AppHandle,
    thread_id: String,
    url: String,
) -> Result<(), String> {
    let map = browsers().lock().await;
    let bw = map.get(&thread_id).ok_or("No browser for this thread")?;
    let wv = app.get_webview(&bw.label).ok_or("Webview not found")?;
    let parsed: url::Url = url.parse().map_err(|e: url::ParseError| e.to_string())?;
    wv.navigate(parsed).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn browser_set_bounds(
    app: tauri::AppHandle,
    thread_id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let map = browsers().lock().await;
    let bw = map.get(&thread_id).ok_or("No browser for this thread")?;
    let wv = app.get_webview(&bw.label).ok_or("Webview not found")?;
    wv.set_position(LogicalPosition::new(x, y)).map_err(|e| e.to_string())?;
    wv.set_size(LogicalSize::new(width, height)).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn browser_eval(
    app: tauri::AppHandle,
    thread_id: String,
    js: String,
) -> Result<(), String> {
    let map = browsers().lock().await;
    let bw = map.get(&thread_id).ok_or("No browser for this thread")?;
    let wv = app.get_webview(&bw.label).ok_or("Webview not found")?;
    wv.eval(&js).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn browser_hide(
    app: tauri::AppHandle,
    thread_id: String,
) -> Result<(), String> {
    let map = browsers().lock().await;
    let bw = map.get(&thread_id).ok_or("No browser for this thread")?;
    if let Some(wv) = app.get_webview(&bw.label) {
        // Move off-screen to hide
        let _ = wv.set_position(LogicalPosition::new(-9999.0, -9999.0));
    }
    Ok(())
}

#[tauri::command]
pub async fn browser_show(
    app: tauri::AppHandle,
    thread_id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let map = browsers().lock().await;
    let bw = map.get(&thread_id).ok_or("No browser for this thread")?;
    if let Some(wv) = app.get_webview(&bw.label) {
        wv.set_position(LogicalPosition::new(x, y)).map_err(|e| e.to_string())?;
        wv.set_size(LogicalSize::new(width, height)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn browser_close(
    app: tauri::AppHandle,
    thread_id: String,
) -> Result<(), String> {
    let mut map = browsers().lock().await;
    if let Some(bw) = map.remove(&thread_id) {
        if let Some(wv) = app.get_webview(&bw.label) {
            let _ = wv.close();
        }
    }
    Ok(())
}
