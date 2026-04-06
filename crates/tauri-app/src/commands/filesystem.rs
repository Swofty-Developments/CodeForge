use std::process::Command;

/// Open an http/https URL in the user's default browser.
///
/// This is the escape hatch for external links (PR banners, docs, etc.) where
/// `window.open` is a no-op in the Tauri webview and the frontend doesn't have
/// the `@tauri-apps/plugin-shell` package installed.
#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), String> {
    // Guard against arbitrary command injection — only allow http(s).
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(format!("refusing to open non-http url: {url}"));
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&url)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "windows")]
    {
        // cmd /c start "" "<url>" — the empty quoted title is required so the
        // URL isn't interpreted as the window title.
        Command::new("cmd")
            .args(["/c", "start", "", &url])
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub fn open_in_file_manager(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}
