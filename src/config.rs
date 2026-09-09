use crate::physics::PhysicsConfig;
use anyhow::{Context, Result};
use evdev::KeyCode;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// A flexible key specification that accepts either a single key string or a list of keys.
///
/// Example in TOML:
/// ```toml
/// left_click = "space"
/// move_left = ["left", "a", "h"]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum KeyList {
    Single(String),
    Multiple(Vec<String>),
}

impl KeyList {
    pub fn single(s: &str) -> Self {
        KeyList::Single(s.to_string())
    }

    pub fn multiple(keys: &[&str]) -> Self {
        KeyList::Multiple(keys.iter().map(|s| s.to_string()).collect())
    }

    pub fn matches(&self, key: KeyCode) -> bool {
        match self {
            KeyList::Single(s) => key_matches(key, s),
            KeyList::Multiple(v) => v.iter().any(|s| key_matches(key, s)),
        }
    }

    pub fn is_any_pressed(&self, pressed_keys: &std::collections::HashSet<u16>) -> bool {
        match self {
            KeyList::Single(s) => is_key_string_pressed(s, pressed_keys),
            KeyList::Multiple(v) => v.iter().any(|s| is_key_string_pressed(s, pressed_keys)),
        }
    }
}

/// Matches an evdev `KeyCode` against a human-readable key string (case-insensitive).
pub fn key_matches(key: KeyCode, pattern: &str) -> bool {
    let p = pattern.trim().to_lowercase();
    match p.as_str() {
        "alt" => key == KeyCode::KEY_LEFTALT || key == KeyCode::KEY_RIGHTALT,
        "shift" => key == KeyCode::KEY_LEFTSHIFT || key == KeyCode::KEY_RIGHTSHIFT,
        "ctrl" | "control" => key == KeyCode::KEY_LEFTCTRL || key == KeyCode::KEY_RIGHTCTRL,
        "super" | "mod" | "meta" | "win" => {
            key == KeyCode::KEY_LEFTMETA || key == KeyCode::KEY_RIGHTMETA
        }
        "space" => key == KeyCode::KEY_SPACE,
        "esc" | "escape" => key == KeyCode::KEY_ESC,
        "enter" | "return" => key == KeyCode::KEY_ENTER,
        "tab" => key == KeyCode::KEY_TAB,
        "backspace" => key == KeyCode::KEY_BACKSPACE,
        "caps" | "capslock" => key == KeyCode::KEY_CAPSLOCK,
        "left" | "arrow_left" => key == KeyCode::KEY_LEFT,
        "right" | "arrow_right" => key == KeyCode::KEY_RIGHT,
        "up" | "arrow_up" => key == KeyCode::KEY_UP,
        "down" | "arrow_down" => key == KeyCode::KEY_DOWN,
        "pageup" | "page_up" | "pgup" => key == KeyCode::KEY_PAGEUP,
        "pagedown" | "page_down" | "pgdn" => key == KeyCode::KEY_PAGEDOWN,
        "home" => key == KeyCode::KEY_HOME,
        "end" => key == KeyCode::KEY_END,
        "insert" => key == KeyCode::KEY_INSERT,
        "delete" | "del" => key == KeyCode::KEY_DELETE,
        "a" => key == KeyCode::KEY_A,
        "b" => key == KeyCode::KEY_B,
        "c" => key == KeyCode::KEY_C,
        "d" => key == KeyCode::KEY_D,
        "e" => key == KeyCode::KEY_E,
        "f" => key == KeyCode::KEY_F,
        "g" => key == KeyCode::KEY_G,
        "h" => key == KeyCode::KEY_H,
        "i" => key == KeyCode::KEY_I,
        "j" => key == KeyCode::KEY_J,
        "k" => key == KeyCode::KEY_K,
        "l" => key == KeyCode::KEY_L,
        "m" => key == KeyCode::KEY_M,
        "n" => key == KeyCode::KEY_N,
        "o" => key == KeyCode::KEY_O,
        "p" => key == KeyCode::KEY_P,
        "q" => key == KeyCode::KEY_Q,
        "r" => key == KeyCode::KEY_R,
        "s" => key == KeyCode::KEY_S,
        "t" => key == KeyCode::KEY_T,
        "u" => key == KeyCode::KEY_U,
        "v" => key == KeyCode::KEY_V,
        "w" => key == KeyCode::KEY_W,
        "x" => key == KeyCode::KEY_X,
        "y" => key == KeyCode::KEY_Y,
        "z" => key == KeyCode::KEY_Z,
        "0" => key == KeyCode::KEY_0,
        "1" => key == KeyCode::KEY_1,
        "2" => key == KeyCode::KEY_2,
        "3" => key == KeyCode::KEY_3,
        "4" => key == KeyCode::KEY_4,
        "5" => key == KeyCode::KEY_5,
        "6" => key == KeyCode::KEY_6,
        "7" => key == KeyCode::KEY_7,
        "8" => key == KeyCode::KEY_8,
        "9" => key == KeyCode::KEY_9,
        "f1" => key == KeyCode::KEY_F1,
        "f2" => key == KeyCode::KEY_F2,
        "f3" => key == KeyCode::KEY_F3,
        "f4" => key == KeyCode::KEY_F4,
        "f5" => key == KeyCode::KEY_F5,
        "f6" => key == KeyCode::KEY_F6,
        "f7" => key == KeyCode::KEY_F7,
        "f8" => key == KeyCode::KEY_F8,
        "f9" => key == KeyCode::KEY_F9,
        "f10" => key == KeyCode::KEY_F10,
        "f11" => key == KeyCode::KEY_F11,
        "f12" => key == KeyCode::KEY_F12,
        "," | "comma" | "<" | "less" => key == KeyCode::KEY_COMMA,
        "." | "dot" | "period" | ">" | "greater" => key == KeyCode::KEY_DOT,
        "/" | "slash" => key == KeyCode::KEY_SLASH,
        ";" | "semicolon" => key == KeyCode::KEY_SEMICOLON,
        "'" | "apostrophe" => key == KeyCode::KEY_APOSTROPHE,
        "[" | "leftbrace" => key == KeyCode::KEY_LEFTBRACE,
        "]" | "rightbrace" => key == KeyCode::KEY_RIGHTBRACE,
        "-" | "minus" => key == KeyCode::KEY_MINUS,
        "=" | "equal" => key == KeyCode::KEY_EQUAL,
        "`" | "grave" => key == KeyCode::KEY_GRAVE,
        _ => false,
    }
}

/// Checks if a key string pattern is satisfied by the currently pressed key set.
pub fn is_key_string_pressed(
    pattern: &str,
    pressed_keys: &std::collections::HashSet<u16>,
) -> bool {
    let p = pattern.trim().to_lowercase();
    match p.as_str() {
        "alt" => {
            pressed_keys.contains(&KeyCode::KEY_LEFTALT.code())
                || pressed_keys.contains(&KeyCode::KEY_RIGHTALT.code())
        }
        "shift" => {
            pressed_keys.contains(&KeyCode::KEY_LEFTSHIFT.code())
                || pressed_keys.contains(&KeyCode::KEY_RIGHTSHIFT.code())
        }
        "ctrl" | "control" => {
            pressed_keys.contains(&KeyCode::KEY_LEFTCTRL.code())
                || pressed_keys.contains(&KeyCode::KEY_RIGHTCTRL.code())
        }
        "super" | "mod" | "meta" | "win" => {
            pressed_keys.contains(&KeyCode::KEY_LEFTMETA.code())
                || pressed_keys.contains(&KeyCode::KEY_RIGHTMETA.code())
        }
        _ => {
            // Find keycode and check pressed set
            for code in pressed_keys {
                if key_matches(KeyCode(*code), &p) {
                    return true;
                }
            }
            false
        }
    }
}

/// Keybinding configuration for Chord (instant on-the-fly) navigation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChordConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Modifiers required to activate chord movement (e.g. ["alt", "shift"] or ["super", "alt"])
    #[serde(default = "default_chord_modifiers")]
    pub modifiers: Vec<String>,

    #[serde(default = "default_chord_left")]
    pub move_left: KeyList,

    #[serde(default = "default_chord_right")]
    pub move_right: KeyList,

    #[serde(default = "default_chord_up")]
    pub move_up: KeyList,

    #[serde(default = "default_chord_down")]
    pub move_down: KeyList,

    #[serde(default = "default_left_click")]
    pub left_click: KeyList,

    #[serde(default = "default_right_click")]
    pub right_click: KeyList,

    #[serde(default = "default_middle_click")]
    pub middle_click: KeyList,

    #[serde(default = "default_scroll_up")]
    pub scroll_up: KeyList,

    #[serde(default = "default_scroll_down")]
    pub scroll_down: KeyList,

    #[serde(default = "default_turbo_key")]
    pub turbo: KeyList,
}

fn default_chord_modifiers() -> Vec<String> {
    vec!["alt".to_string(), "shift".to_string()]
}
fn default_chord_left() -> KeyList {
    KeyList::multiple(&["left", "a"])
}
fn default_chord_right() -> KeyList {
    KeyList::multiple(&["right", "d"])
}
fn default_chord_up() -> KeyList {
    KeyList::multiple(&["up", "w"])
}
fn default_chord_down() -> KeyList {
    KeyList::multiple(&["down", "s"])
}
fn default_left_click() -> KeyList {
    KeyList::single("space")
}
fn default_right_click() -> KeyList {
    KeyList::single("c")
}
fn default_middle_click() -> KeyList {
    KeyList::single("v")
}
fn default_scroll_up() -> KeyList {
    KeyList::single("r")
}
fn default_scroll_down() -> KeyList {
    KeyList::single("f")
}
fn default_turbo_key() -> KeyList {
    KeyList::single("ctrl")
}

impl Default for ChordConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            modifiers: default_chord_modifiers(),
            move_left: default_chord_left(),
            move_right: default_chord_right(),
            move_up: default_chord_up(),
            move_down: default_chord_down(),
            left_click: default_left_click(),
            right_click: default_right_click(),
            middle_click: default_middle_click(),
            scroll_up: default_scroll_up(),
            scroll_down: default_scroll_down(),
            turbo: default_turbo_key(),
        }
    }
}

/// Keybinding configuration for Modal (Vim-style Mouse Mode) navigation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Modifiers required to toggle Mouse Mode (e.g. ["alt", "shift"])
    #[serde(default = "default_toggle_modifiers")]
    pub toggle_modifiers: Vec<String>,

    /// Key that toggles mouse mode when toggle modifiers are held (e.g. "m")
    #[serde(default = "default_modal_toggle")]
    pub toggle: KeyList,

    /// Key to exit modal mode back to normal typing (e.g. "esc")
    #[serde(default = "default_modal_exit")]
    pub exit: KeyList,

    #[serde(default = "default_modal_left")]
    pub move_left: KeyList,

    #[serde(default = "default_modal_right")]
    pub move_right: KeyList,

    #[serde(default = "default_modal_up")]
    pub move_up: KeyList,

    #[serde(default = "default_modal_down")]
    pub move_down: KeyList,

    #[serde(default = "default_left_click")]
    pub left_click: KeyList,

    #[serde(default = "default_right_click")]
    pub right_click: KeyList,

    #[serde(default = "default_middle_click")]
    pub middle_click: KeyList,

    #[serde(default = "default_modal_scroll_up")]
    pub scroll_up: KeyList,

    #[serde(default = "default_modal_scroll_down")]
    pub scroll_down: KeyList,

    #[serde(default = "default_modal_scroll_left")]
    pub scroll_left: KeyList,

    #[serde(default = "default_modal_scroll_right")]
    pub scroll_right: KeyList,

    #[serde(default = "default_precision_key")]
    pub precision: KeyList,

    #[serde(default = "default_turbo_key")]
    pub turbo: KeyList,

    #[serde(default = "default_scroll_speed")]
    pub scroll_speed: i32,
}

fn default_toggle_modifiers() -> Vec<String> {
    vec!["alt".to_string(), "shift".to_string()]
}
fn default_modal_toggle() -> KeyList {
    KeyList::single("m")
}
fn default_modal_exit() -> KeyList {
    KeyList::single("esc")
}
fn default_modal_left() -> KeyList {
    KeyList::multiple(&["a", "h"])
}
fn default_modal_right() -> KeyList {
    KeyList::multiple(&["d", "l"])
}
fn default_modal_up() -> KeyList {
    KeyList::multiple(&["w", "k"])
}
fn default_modal_down() -> KeyList {
    KeyList::multiple(&["s", "j"])
}
fn default_modal_scroll_up() -> KeyList {
    KeyList::multiple(&["up", "pageup", ",", "r"])
}
fn default_modal_scroll_down() -> KeyList {
    KeyList::multiple(&["down", "pagedown", ".", "f"])
}
fn default_modal_scroll_left() -> KeyList {
    KeyList::single("left")
}
fn default_modal_scroll_right() -> KeyList {
    KeyList::single("right")
}
fn default_precision_key() -> KeyList {
    KeyList::single("shift")
}
fn default_scroll_speed() -> i32 {
    1
}

impl Default for ModalConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            toggle_modifiers: default_toggle_modifiers(),
            toggle: default_modal_toggle(),
            exit: default_modal_exit(),
            move_left: default_modal_left(),
            move_right: default_modal_right(),
            move_up: default_modal_up(),
            move_down: default_modal_down(),
            left_click: default_left_click(),
            right_click: default_right_click(),
            middle_click: default_middle_click(),
            scroll_up: default_modal_scroll_up(),
            scroll_down: default_modal_scroll_down(),
            scroll_left: default_modal_scroll_left(),
            scroll_right: default_modal_scroll_right(),
            precision: default_precision_key(),
            turbo: default_turbo_key(),
            scroll_speed: default_scroll_speed(),
        }
    }
}

/// Root application configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub chord: Option<ChordConfig>,

    #[serde(default)]
    pub modal: ModalConfig,

    #[serde(default)]
    pub physics: PhysicsConfig,
}

fn default_true() -> bool {
    true
}

impl AppConfig {
    /// Returns the standard configuration path (`~/.config/fckmouse/config.toml`).
    pub fn default_config_path() -> Result<PathBuf> {
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        Ok(PathBuf::from(home).join(".config").join("fckmouse").join("config.toml"))
    }

    /// Loads configuration from the given path, or falls back to default if it does not exist.
    pub fn load_or_default(path: Option<&Path>) -> Result<Self> {
        let target_path = match path {
            Some(p) => p.to_path_buf(),
            None => Self::default_config_path()?,
        };

        if target_path.exists() {
            let content = fs::read_to_string(&target_path)
                .with_context(|| format!("Failed to read config file at {:?}", target_path))?;
            let config: AppConfig = toml::from_str(&content)
                .with_context(|| format!("Failed to parse TOML configuration at {:?}", target_path))?;
            log::info!("Loaded configuration from {:?}", target_path);
            Ok(config)
        } else {
            log::info!("Config file not found at {:?}, using zero-conflict defaults.", target_path);
            Ok(AppConfig::default())
        }
    }

    /// Generates a well-documented default configuration string with customizable keybinds.
    pub fn generate_default_toml() -> String {
        r#"# fckmouse configuration file
# Place this at ~/.config/fckmouse/config.toml

[modal]
# Mouse Mode.
# Single keys control the mouse exclusively without holding modifiers.
# The keyboard is exclusively locked (EVIOCGRAB) so letters never type into open apps!
enabled = true

# Modifiers required to toggle Mouse Mode:
toggle_modifiers = ["alt", "shift"]

# Key to toggle Mouse Mode (when toggle modifiers are pressed):
toggle = "m"

# Key to exit Mouse Mode back to normal typing:
exit = "esc"

# Direction keys in Mouse Mode (Left Hand: WASD and HJKL):
move_left = ["a", "h"]
move_right = ["d", "l"]
move_up = ["w", "k"]
move_down = ["s", "j"]

# Actions in Mouse Mode:
left_click = "space"
right_click = "c"
middle_click = "v"

# 2D Scrolling Engine (Right Hand: Arrow Keys, PageUp/PageDown, Comma/Dot):
scroll_up = ["up", "pageup", ",", "r"]
scroll_down = ["down", "pagedown", ".", "f"]
scroll_left = "left"
scroll_right = "right"

# Modifiers inside Mouse Mode:
precision = "shift"    # Hold for 0.3x slow precision crawl
turbo = "ctrl"         # Hold for 3.0x turbo speed
scroll_speed = 1       # Scroll wheel notches per tick


[physics]
# Movement physics and acceleration curve.
base_speed = 2.5            # Starting speed (pixels per 10ms tick)
max_speed = 28.0            # Maximum terminal speed under sustained hold
acceleration = 35.0         # Rate of acceleration per second
turbo_multiplier = 3.0      # Multiplier when Turbo key is held
precision_multiplier = 0.3  # Multiplier when Precision key is held
tick_rate_ms = 10           # Physics update frequency (10ms = 100 Hz)
"#.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_matching() {
        assert!(key_matches(KeyCode::KEY_SPACE, "space"));
        assert!(key_matches(KeyCode::KEY_ESC, "esc"));
        assert!(key_matches(KeyCode::KEY_LEFTALT, "alt"));
        assert!(key_matches(KeyCode::KEY_RIGHTALT, "alt"));
        assert!(key_matches(KeyCode::KEY_LEFTCTRL, "ctrl"));
        assert!(key_matches(KeyCode::KEY_LEFTMETA, "super"));
        assert!(key_matches(KeyCode::KEY_A, "a"));
        assert!(key_matches(KeyCode::KEY_H, "h"));
        assert!(key_matches(KeyCode::KEY_COMMA, "<"));
        assert!(key_matches(KeyCode::KEY_DOT, ">"));
        assert!(key_matches(KeyCode::KEY_PAGEUP, "pgup"));
        assert!(key_matches(KeyCode::KEY_PAGEDOWN, "pgdn"));
    }

    #[test]
    fn test_key_list_matching() {
        let single = KeyList::single("space");
        assert!(single.matches(KeyCode::KEY_SPACE));
        assert!(!single.matches(KeyCode::KEY_A));

        let multiple = KeyList::multiple(&["a", "h"]);
        assert!(multiple.matches(KeyCode::KEY_A));
        assert!(multiple.matches(KeyCode::KEY_H));
        assert!(!multiple.matches(KeyCode::KEY_D));
    }

    #[test]
    fn test_custom_toml_deserialization() {
        let toml_str = r#"
        [modal]
        enabled = true
        toggle_modifiers = ["super", "alt"]
        toggle = "grave"
        exit = "tab"
        move_left = ["a", "h"]
        move_right = ["d", "l"]
        move_up = ["w", "k"]
        move_down = ["s", "j"]
        left_click = "enter"
        right_click = "m"
        middle_click = "n"
        scroll_up = ["up", "pgup"]
        scroll_down = ["down", "pgdn"]
        scroll_left = "left"
        scroll_right = "right"
        precision = "shift"
        turbo = "ctrl"
        scroll_speed = 2
        "#;

        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.modal.toggle_modifiers, vec!["super", "alt"]);
        assert!(config.modal.toggle.matches(KeyCode::KEY_GRAVE));
        assert!(config.modal.exit.matches(KeyCode::KEY_TAB));
        assert!(config.modal.left_click.matches(KeyCode::KEY_ENTER));
        assert!(config.modal.scroll_up.matches(KeyCode::KEY_PAGEUP));
        assert!(config.modal.scroll_left.matches(KeyCode::KEY_LEFT));
        assert_eq!(config.modal.scroll_speed, 2);
    }
}
