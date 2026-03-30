use std::collections::HashSet;
use std::sync::Mutex;
use tauri::Manager;

fn created() -> &'static Mutex<HashSet<String>> {
    static I: std::sync::OnceLock<Mutex<HashSet<String>>> = std::sync::OnceLock::new();
    I.get_or_init(|| Mutex::new(HashSet::new()))
}

fn label(tid: &str) -> String {
    format!("bw{}", tid.replace(|c: char| !c.is_alphanumeric(), ""))
}

#[tauri::command]
pub fn browser_open(
    app: tauri::AppHandle,
    thread_id: String,
    url: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let lbl = label(&thread_id);
    let mut set = created().lock().unwrap();

    if set.contains(&lbl) {
        if let Some(wv) = app.get_webview(&lbl) {
            let _ = wv.set_position(tauri::LogicalPosition::new(x, y));
            let _ = wv.set_size(tauri::LogicalSize::new(width, height));
            if let Ok(u) = url.parse::<url::Url>() {
                let _ = wv.navigate(u);
            }
        }
        return Ok(());
    }

    let parsed: url::Url = url.parse().unwrap_or_else(|_| "about:blank".parse().unwrap());
    let windows = app.windows();
    let window = windows.values().next().ok_or("No window")?;

    let wv = window
        .add_child(
            tauri::webview::WebviewBuilder::new(&lbl, tauri::WebviewUrl::External(parsed)),
            tauri::LogicalPosition::new(x, y),
            tauri::LogicalSize::new(width, height),
        )
        .map_err(|e| format!("{e}"))?;

    #[cfg(debug_assertions)]
    { let _ = &wv; } // suppress unused warning when devtools not opened

    set.insert(lbl);
    Ok(())
}

#[tauri::command]
pub fn browser_navigate(app: tauri::AppHandle, thread_id: String, url: String) -> Result<(), String> {
    let wv = app.get_webview(&label(&thread_id)).ok_or("No browser")?;
    wv.navigate(url.parse::<url::Url>().map_err(|e: url::ParseError| e.to_string())?).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn browser_set_bounds(app: tauri::AppHandle, thread_id: String, x: f64, y: f64, w: f64, h: f64) -> Result<(), String> {
    let wv = app.get_webview(&label(&thread_id)).ok_or("No browser")?;
    wv.set_position(tauri::LogicalPosition::new(x, y)).map_err(|e| e.to_string())?;
    wv.set_size(tauri::LogicalSize::new(w, h)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn browser_eval(app: tauri::AppHandle, thread_id: String, js: String) -> Result<(), String> {
    app.get_webview(&label(&thread_id)).ok_or("No browser")?.eval(&js).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn browser_hide(app: tauri::AppHandle, thread_id: String) -> Result<(), String> {
    if let Some(wv) = app.get_webview(&label(&thread_id)) {
        let _ = wv.set_position(tauri::LogicalPosition::new(-9999.0_f64, -9999.0_f64));
    }
    Ok(())
}

#[tauri::command]
pub fn browser_close(app: tauri::AppHandle, thread_id: String) -> Result<(), String> {
    let lbl = label(&thread_id);
    created().lock().unwrap().remove(&lbl);
    if let Some(wv) = app.get_webview(&lbl) { let _ = wv.close(); }
    Ok(())
}

#[tauri::command]
pub fn browser_devtools(app: tauri::AppHandle, thread_id: String) -> Result<(), String> {
    app.get_webview(&label(&thread_id)).ok_or("No browser")?.open_devtools();
    Ok(())
}
