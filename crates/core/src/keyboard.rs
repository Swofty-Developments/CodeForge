//! Key binding definitions and keymap management.
//!
//! Provides types for representing keyboard shortcuts with modifier keys,
//! mapping commands to key combinations, detecting conflicts, and
//! serializing keymaps for persistence.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// A physical key on the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Key {
    /// A letter key (stored uppercase).
    Char(char),
    /// Function key F1-F24.
    F(u8),
    /// Enter / Return.
    Enter,
    /// Escape.
    Escape,
    /// Tab.
    Tab,
    /// Space bar.
    Space,
    /// Backspace.
    Backspace,
    /// Delete / Forward-delete.
    Delete,
    /// Arrow up.
    Up,
    /// Arrow down.
    Down,
    /// Arrow left.
    Left,
    /// Arrow right.
    Right,
    /// Home.
    Home,
    /// End.
    End,
    /// Page up.
    PageUp,
    /// Page down.
    PageDown,
    /// Insert.
    Insert,
    /// Plus key.
    Plus,
    /// Minus key.
    Minus,
    /// Backtick / grave accent.
    Backtick,
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Key::Char(c) => write!(f, "{}", c.to_uppercase()),
            Key::F(n) => write!(f, "F{n}"),
            Key::Enter => write!(f, "Enter"),
            Key::Escape => write!(f, "Escape"),
            Key::Tab => write!(f, "Tab"),
            Key::Space => write!(f, "Space"),
            Key::Backspace => write!(f, "Backspace"),
            Key::Delete => write!(f, "Delete"),
            Key::Up => write!(f, "Up"),
            Key::Down => write!(f, "Down"),
            Key::Left => write!(f, "Left"),
            Key::Right => write!(f, "Right"),
            Key::Home => write!(f, "Home"),
            Key::End => write!(f, "End"),
            Key::PageUp => write!(f, "PageUp"),
            Key::PageDown => write!(f, "PageDown"),
            Key::Insert => write!(f, "Insert"),
            Key::Plus => write!(f, "+"),
            Key::Minus => write!(f, "-"),
            Key::Backtick => write!(f, "`"),
        }
    }
}

/// Modifier keys that can be combined with a key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Modifiers {
    /// Command key (macOS) / Windows key.
    pub cmd: bool,
    /// Control key.
    pub ctrl: bool,
    /// Alt / Option key.
    pub alt: bool,
    /// Shift key.
    pub shift: bool,
}

impl Modifiers {
    /// No modifiers pressed.
    pub const NONE: Self = Self {
        cmd: false,
        ctrl: false,
        alt: false,
        shift: false,
    };

    /// Only Cmd/Super.
    pub const CMD: Self = Self {
        cmd: true,
        ctrl: false,
        alt: false,
        shift: false,
    };

    /// Only Ctrl.
    pub const CTRL: Self = Self {
        cmd: false,
        ctrl: true,
        alt: false,
        shift: false,
    };

    /// Returns true if no modifiers are active.
    pub fn is_empty(&self) -> bool {
        !self.cmd && !self.ctrl && !self.alt && !self.shift
    }

    /// Return the platform-appropriate primary modifier (Cmd on macOS, Ctrl elsewhere).
    pub fn primary() -> Self {
        if cfg!(target_os = "macos") {
            Self::CMD
        } else {
            Self::CTRL
        }
    }
}

impl fmt::Display for Modifiers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.ctrl {
            write!(f, "Ctrl+")?;
        }
        if self.alt {
            write!(f, "Alt+")?;
        }
        if self.shift {
            write!(f, "Shift+")?;
        }
        if self.cmd {
            write!(f, "Cmd+")?;
        }
        Ok(())
    }
}

/// A key combination: a set of modifiers plus a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyCombo {
    /// Active modifier keys.
    pub modifiers: Modifiers,
    /// The primary key.
    pub key: Key,
}

impl KeyCombo {
    /// Create a key combo with no modifiers.
    pub fn key(key: Key) -> Self {
        Self {
            modifiers: Modifiers::NONE,
            key,
        }
    }

    /// Create a Cmd+key combo (macOS style).
    pub fn cmd(key: Key) -> Self {
        Self {
            modifiers: Modifiers::CMD,
            key,
        }
    }

    /// Create a Ctrl+key combo.
    pub fn ctrl(key: Key) -> Self {
        Self {
            modifiers: Modifiers::CTRL,
            key,
        }
    }

    /// Add Shift modifier to this combo.
    pub fn with_shift(mut self) -> Self {
        self.modifiers.shift = true;
        self
    }

    /// Add Alt modifier to this combo.
    pub fn with_alt(mut self) -> Self {
        self.modifiers.alt = true;
        self
    }

    /// Return a human-readable label for this combo, appropriate for the platform.
    pub fn display_label(&self) -> String {
        let mut parts = Vec::new();
        if self.modifiers.ctrl {
            parts.push(if cfg!(target_os = "macos") {
                "\u{2303}"
            } else {
                "Ctrl"
            });
        }
        if self.modifiers.alt {
            parts.push(if cfg!(target_os = "macos") {
                "\u{2325}"
            } else {
                "Alt"
            });
        }
        if self.modifiers.shift {
            parts.push(if cfg!(target_os = "macos") {
                "\u{21E7}"
            } else {
                "Shift"
            });
        }
        if self.modifiers.cmd {
            parts.push(if cfg!(target_os = "macos") {
                "\u{2318}"
            } else {
                "Super"
            });
        }
        let key_label = self.key_label();
        parts.push(&key_label);
        if cfg!(target_os = "macos") {
            parts.join("")
        } else {
            parts.join("+")
        }
    }

    fn key_label(&self) -> String {
        self.key.to_string()
    }
}

impl fmt::Display for KeyCombo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.modifiers, self.key)
    }
}

/// Error returned when parsing a key combo string fails.
#[derive(Debug, Clone, thiserror::Error)]
#[error("invalid key binding: {reason}")]
pub struct KeyBindingParseError {
    /// Explanation of why the parse failed.
    pub reason: String,
}

impl FromStr for KeyCombo {
    type Err = KeyBindingParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();
        if parts.is_empty() {
            return Err(KeyBindingParseError {
                reason: "empty key binding".to_string(),
            });
        }

        let mut modifiers = Modifiers::NONE;
        let mut key_part = None;

        for part in &parts {
            match part.to_lowercase().as_str() {
                "cmd" | "super" | "meta" => modifiers.cmd = true,
                "ctrl" | "control" => modifiers.ctrl = true,
                "alt" | "option" | "opt" => modifiers.alt = true,
                "shift" => modifiers.shift = true,
                _ => {
                    if key_part.is_some() {
                        return Err(KeyBindingParseError {
                            reason: format!("multiple non-modifier keys found: {s}"),
                        });
                    }
                    key_part = Some(*part);
                }
            }
        }

        let key_str = key_part.ok_or_else(|| KeyBindingParseError {
            reason: "no key specified".to_string(),
        })?;

        let key = parse_key(key_str)?;
        Ok(KeyCombo { modifiers, key })
    }
}

fn parse_key(s: &str) -> Result<Key, KeyBindingParseError> {
    match s.to_lowercase().as_str() {
        "enter" | "return" => Ok(Key::Enter),
        "escape" | "esc" => Ok(Key::Escape),
        "tab" => Ok(Key::Tab),
        "space" => Ok(Key::Space),
        "backspace" => Ok(Key::Backspace),
        "delete" | "del" => Ok(Key::Delete),
        "up" => Ok(Key::Up),
        "down" => Ok(Key::Down),
        "left" => Ok(Key::Left),
        "right" => Ok(Key::Right),
        "home" => Ok(Key::Home),
        "end" => Ok(Key::End),
        "pageup" => Ok(Key::PageUp),
        "pagedown" => Ok(Key::PageDown),
        "insert" => Ok(Key::Insert),
        "plus" => Ok(Key::Plus),
        "minus" => Ok(Key::Minus),
        "backtick" => Ok(Key::Backtick),
        other => {
            if let Some(n) = other.strip_prefix('f') {
                let num: u8 = n.parse().map_err(|_| KeyBindingParseError {
                    reason: format!("invalid function key: {other}"),
                })?;
                Ok(Key::F(num))
            } else if other.len() == 1 {
                Ok(Key::Char(other.chars().next().unwrap().to_ascii_uppercase()))
            } else {
                Err(KeyBindingParseError {
                    reason: format!("unknown key: {other}"),
                })
            }
        }
    }
}

/// A complete key binding mapping a key combo to a command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBinding {
    /// The key combination that triggers this binding.
    pub combo: KeyCombo,
    /// The command identifier to execute.
    pub command: String,
    /// Optional context / when-clause (e.g., "editor.focus").
    pub when: Option<String>,
    /// Human-readable description.
    pub description: Option<String>,
}

/// A keymap managing a collection of key bindings with conflict detection.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KeyMap {
    /// All registered bindings.
    bindings: Vec<KeyBinding>,
}

impl KeyMap {
    /// Create an empty keymap.
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    /// Add a binding to the keymap.
    pub fn add(&mut self, binding: KeyBinding) {
        self.bindings.push(binding);
    }

    /// Remove all bindings for the given command.
    pub fn remove_command(&mut self, command: &str) {
        self.bindings.retain(|b| b.command != command);
    }

    /// Look up the command bound to a key combo in a given context.
    pub fn lookup(&self, combo: &KeyCombo, context: Option<&str>) -> Option<&KeyBinding> {
        self.bindings.iter().rev().find(|b| {
            b.combo == *combo
                && match (&b.when, context) {
                    (Some(w), Some(c)) => w == c,
                    (None, _) => true,
                    (Some(_), None) => false,
                }
        })
    }

    /// Detect conflicting bindings (same combo and context).
    pub fn find_conflicts(&self) -> Vec<(&KeyBinding, &KeyBinding)> {
        let mut conflicts = Vec::new();
        for (i, a) in self.bindings.iter().enumerate() {
            for b in self.bindings.iter().skip(i + 1) {
                if a.combo == b.combo && a.when == b.when {
                    conflicts.push((a, b));
                }
            }
        }
        conflicts
    }

    /// Return all bindings.
    pub fn bindings(&self) -> &[KeyBinding] {
        &self.bindings
    }

    /// Return all bindings for a specific command.
    pub fn bindings_for(&self, command: &str) -> Vec<&KeyBinding> {
        self.bindings
            .iter()
            .filter(|b| b.command == command)
            .collect()
    }

    /// Return the number of bindings.
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    /// Return true if there are no bindings.
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    /// Create a default keymap with common editor bindings.
    pub fn with_defaults() -> Self {
        let mut map = Self::new();
        let bindings = vec![
            ("Cmd+S", "file.save", "Save the current file"),
            ("Cmd+Z", "edit.undo", "Undo last action"),
            ("Cmd+Shift+Z", "edit.redo", "Redo last action"),
            ("Cmd+C", "edit.copy", "Copy selection"),
            ("Cmd+V", "edit.paste", "Paste from clipboard"),
            ("Cmd+X", "edit.cut", "Cut selection"),
            ("Cmd+A", "edit.select_all", "Select all"),
            ("Cmd+F", "search.find", "Find in file"),
            ("Cmd+Shift+F", "search.find_in_files", "Find in files"),
            ("Cmd+P", "palette.open", "Open command palette"),
            ("Cmd+N", "thread.new", "New thread"),
            ("Cmd+W", "tab.close", "Close tab"),
            ("Cmd+T", "tab.new", "New tab"),
            ("Cmd+K", "chat.clear", "Clear chat"),
            ("Cmd+Enter", "chat.send", "Send message"),
        ];
        for (combo_str, command, desc) in bindings {
            if let Ok(combo) = combo_str.parse::<KeyCombo>() {
                map.add(KeyBinding {
                    combo,
                    command: command.to_string(),
                    when: None,
                    description: Some(desc.to_string()),
                });
            }
        }
        map
    }

    /// Merge another keymap into this one, with the other taking precedence.
    pub fn merge(&mut self, other: &KeyMap) {
        for binding in &other.bindings {
            // Remove existing binding for the same combo+context.
            self.bindings
                .retain(|b| !(b.combo == binding.combo && b.when == binding.when));
            self.bindings.push(binding.clone());
        }
    }

    /// Serialize the keymap to a JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.bindings)
    }

    /// Deserialize a keymap from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let bindings: Vec<KeyBinding> = serde_json::from_str(json)?;
        Ok(Self { bindings })
    }
}

/// A human-readable cheatsheet row.
#[derive(Debug, Clone)]
pub struct CheatsheetEntry {
    /// Category grouping (e.g., "Editor", "Navigation").
    pub category: String,
    /// The command description.
    pub description: String,
    /// The key combo display label.
    pub shortcut: String,
}

/// Generate a cheatsheet from a keymap, grouping by command prefix.
pub fn generate_cheatsheet(keymap: &KeyMap) -> Vec<CheatsheetEntry> {
    let mut entries = Vec::new();
    for binding in keymap.bindings() {
        let category = binding
            .command
            .split('.')
            .next()
            .unwrap_or("other")
            .to_string();
        let description = binding
            .description
            .clone()
            .unwrap_or_else(|| binding.command.clone());
        entries.push(CheatsheetEntry {
            category,
            description,
            shortcut: binding.combo.display_label(),
        });
    }
    entries.sort_by(|a, b| a.category.cmp(&b.category).then(a.description.cmp(&b.description)));
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_key_combo() {
        let combo: KeyCombo = "Cmd+Shift+S".parse().unwrap();
        assert!(combo.modifiers.cmd);
        assert!(combo.modifiers.shift);
        assert_eq!(combo.key, Key::Char('S'));
    }

    #[test]
    fn conflict_detection() {
        let mut keymap = KeyMap::new();
        keymap.add(KeyBinding {
            combo: "Cmd+S".parse().unwrap(),
            command: "file.save".to_string(),
            when: None,
            description: None,
        });
        keymap.add(KeyBinding {
            combo: "Cmd+S".parse().unwrap(),
            command: "file.save_as".to_string(),
            when: None,
            description: None,
        });
        let conflicts = keymap.find_conflicts();
        assert_eq!(conflicts.len(), 1);
    }

    #[test]
    fn default_keymap() {
        let keymap = KeyMap::with_defaults();
        assert!(!keymap.is_empty());
        let combo: KeyCombo = "Cmd+S".parse().unwrap();
        let binding = keymap.lookup(&combo, None);
        assert!(binding.is_some());
        assert_eq!(binding.unwrap().command, "file.save");
    }
}
