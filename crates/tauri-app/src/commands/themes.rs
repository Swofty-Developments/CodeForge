use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub preview: Vec<String>,
    pub vars: HashMap<String, String>,
    #[serde(default)]
    pub is_custom: bool,
}

fn builtin_themes_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("themes")
}

fn custom_themes_dir() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".codeforge").join("themes")
}

fn read_themes_from_dir(dir: &PathBuf, is_custom: bool) -> Vec<Theme> {
    let mut themes = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return themes,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "json") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(mut theme) = serde_json::from_str::<Theme>(&content) {
                    theme.is_custom = is_custom;
                    themes.push(theme);
                }
            }
        }
    }
    themes
}

#[tauri::command]
pub fn list_themes() -> Result<Vec<Theme>, String> {
    let mut all = read_themes_from_dir(&builtin_themes_dir(), false);
    let custom = read_themes_from_dir(&custom_themes_dir(), true);
    all.extend(custom);
    Ok(all)
}

#[tauri::command]
pub fn import_theme(json_content: String) -> Result<Theme, String> {
    let mut theme: Theme =
        serde_json::from_str(&json_content).map_err(|e| format!("Invalid theme JSON: {e}"))?;

    if theme.id.is_empty() {
        return Err("Theme must have an id".to_string());
    }
    if theme.name.is_empty() {
        return Err("Theme must have a name".to_string());
    }
    if theme.vars.is_empty() {
        return Err("Theme must have vars".to_string());
    }

    // Sanitize id for use as filename
    let safe_id: String = theme
        .id
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    theme.id = safe_id.clone();
    theme.is_custom = true;

    let dir = custom_themes_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create themes dir: {e}"))?;

    let path = dir.join(format!("{safe_id}.json"));
    let json = serde_json::to_string_pretty(&theme).map_err(|e| format!("Serialize error: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("Failed to write theme: {e}"))?;

    Ok(theme)
}

#[tauri::command]
pub fn delete_custom_theme(id: String) -> Result<(), String> {
    let dir = custom_themes_dir();
    let path = dir.join(format!("{id}.json"));
    if !path.exists() {
        return Err("Theme not found or is a built-in theme".to_string());
    }
    std::fs::remove_file(&path).map_err(|e| format!("Failed to delete theme: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn export_theme(id: String) -> Result<String, String> {
    // Check built-in first, then custom
    let builtin_path = builtin_themes_dir().join(format!("{id}.json"));
    if builtin_path.exists() {
        return std::fs::read_to_string(&builtin_path)
            .map_err(|e| format!("Failed to read theme: {e}"));
    }

    let custom_path = custom_themes_dir().join(format!("{id}.json"));
    if custom_path.exists() {
        return std::fs::read_to_string(&custom_path)
            .map_err(|e| format!("Failed to read theme: {e}"));
    }

    Err(format!("Theme '{id}' not found"))
}
