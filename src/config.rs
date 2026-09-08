use crate::physics::PhysicsConfig;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Keybinding configuration for Chord (instant on-the-fly) navigation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChordConfig {
    /// Enable instant chord movement without entering modal mode.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Require Alt key held.
    #[serde(default = "default_true")]
    pub require_alt: bool,

    /// Require Shift key held.
    #[serde(default = "default_true")]
    pub require_shift: bool,

    /// Require Super (Windows) key held.
    #[serde(default = "default_false")]
    pub require_super: bool,

    /// Allow Arrow keys for chord movement.
    #[serde(default = "default_true")]
    pub use_arrow_keys: bool,

    /// Allow WASD keys for chord movement.
    #[serde(default = "default_true")]
    pub use_wasd_keys: bool,

    /// Hold Ctrl for turbo speed multiplier.
    #[serde(default = "default_true")]
    pub ctrl_turbo: bool,
}

impl Default for ChordConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            require_alt: default_true(),
            require_shift: default_true(),
            require_super: default_false(),
            use_arrow_keys: default_true(),
            use_wasd_keys: default_true(),
            ctrl_turbo: default_true(),
        }
    }
}

/// Keybinding configuration for Modal (Vim-style Mouse Mode) navigation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalConfig {
    /// Enable modal toggle mode.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Allow Arrow keys to move cursor in modal mode.
    #[serde(default = "default_true")]
    pub use_arrow_keys: bool,

    /// Allow WASD keys to move cursor in modal mode.
    #[serde(default = "default_true")]
    pub use_wasd_keys: bool,

    /// Allow HJKL (Vim) keys to move cursor in modal mode.
    #[serde(default = "default_true")]
    pub use_hjkl_keys: bool,

    /// Scroll speed in discrete wheel notches per tick.
    #[serde(default = "default_scroll_speed")]
    pub scroll_speed: i32,
}

fn default_scroll_speed() -> i32 {
    1
}

impl Default for ModalConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            use_arrow_keys: default_true(),
            use_wasd_keys: default_true(),
            use_hjkl_keys: default_true(),
            scroll_speed: default_scroll_speed(),
        }
    }
}

/// Root application configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub chord: ChordConfig,

    #[serde(default)]
    pub modal: ModalConfig,

    #[serde(default)]
    pub physics: PhysicsConfig,
}

fn default_true() -> bool {
    true
}
fn default_false() -> bool {
    false
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

    /// Generates a well-documented default configuration string.
    pub fn generate_default_toml() -> String {
        r#"# fckmouse configuration file
# Place this at ~/.config/fckmouse/config.toml

[chord]
# Instant on-the-fly cursor movement using key chords.
enabled = true

# Modifiers required to initiate chord cursor movement.
# Default: Alt + Shift (Zero conflict with niri window manager & text editing).
require_alt = true
require_shift = true
require_super = false

# Direction keys recognized when holding the modifiers:
use_arrow_keys = true
use_wasd_keys = true

# Hold Ctrl simultaneously for Turbo Speed (3x multiplier):
ctrl_turbo = true


[modal]
# Vim-style Mouse Mode toggle (Default: Alt + Shift + M).
# In Mouse Mode, single keys control the mouse without holding modifiers:
# - Arrows / WASD / HJKL : Move cursor
# - Space                : Left Click
# - C                    : Right Click
# - V                    : Middle Click
# - R / F                : Scroll Up / Down
# - Shift                : Precision crawl (0.3x)
# - Ctrl                 : Turbo speed (3.0x)
# - Esc                  : Exit Mouse Mode
enabled = true
use_arrow_keys = true
use_wasd_keys = true
use_hjkl_keys = true
scroll_speed = 1


[physics]
# Movement physics and acceleration curve.
base_speed = 2.5            # Starting speed (pixels per 10ms tick)
max_speed = 28.0            # Maximum terminal speed under sustained hold
acceleration = 35.0         # Rate of acceleration per second
turbo_multiplier = 3.0      # Multiplier when Ctrl is held
precision_multiplier = 0.3  # Multiplier when Shift is held in modal mode
tick_rate_ms = 10           # Physics update frequency (10ms = 100 Hz)
"#.to_string()
    }
}
