use evdev::KeyCode;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::process::Command;

/// Configuration for the Grid Snap navigation system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridConfig {
    /// Whether Grid Snap mode is enabled.
    #[serde(default = "default_grid_enabled")]
    pub enabled: bool,

    /// Key used to activate or toggle Grid Snap inside Mouse Mode (e.g. "g").
    #[serde(default = "default_grid_activate")]
    pub activate: String,

    /// Configured display width in pixels. If 0 or omitted, auto-detection will be attempted.
    #[serde(default)]
    pub screen_width: u32,

    /// Configured display height in pixels. If 0 or omitted, auto-detection will be attempted.
    #[serde(default)]
    pub screen_height: u32,

    /// Number of grid rows (default: 3).
    #[serde(default = "default_grid_rows")]
    pub rows: u32,

    /// Number of grid columns (default: 3).
    #[serde(default = "default_grid_cols")]
    pub cols: u32,

    /// Whether consecutive grid key presses recursively subdivide the active cell.
    #[serde(default = "default_true")]
    pub recursive: bool,
}

fn default_grid_enabled() -> bool {
    true
}

fn default_grid_activate() -> String {
    "g".to_string()
}

fn default_grid_rows() -> u32 {
    3
}

fn default_grid_cols() -> u32 {
    3
}

fn default_true() -> bool {
    true
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            enabled: default_grid_enabled(),
            activate: default_grid_activate(),
            screen_width: 0,
            screen_height: 0,
            rows: default_grid_rows(),
            cols: default_grid_cols(),
            recursive: default_true(),
        }
    }
}

/// Tracks the estimated absolute cursor position on the screen based on accumulated relative deltas.
#[derive(Debug, Clone)]
pub struct CursorTracker {
    pub x: f64,
    pub y: f64,
    pub screen_width: u32,
    pub screen_height: u32,
}

impl CursorTracker {
    /// Initializes a new cursor tracker. If dimensions are 0, it attempts auto-detection.
    pub fn new(configured_w: u32, configured_h: u32) -> Self {
        let (w, h) = Self::resolve_resolution(configured_w, configured_h);
        Self {
            x: (w as f64) / 2.0,
            y: (h as f64) / 2.0,
            screen_width: w,
            screen_height: h,
        }
    }

    /// Resolves display resolution using explicit configuration or lightweight auto-detection.
    ///
    /// ponytail: Known ceiling - auto-detection parses niri or xrandr CLI outputs if available.
    /// If detection fails, defaults to 1920x1080 and warns user to set screen_width/screen_height in config.toml.
    pub fn resolve_resolution(configured_w: u32, configured_h: u32) -> (u32, u32) {
        if configured_w > 0 && configured_h > 0 {
            return (configured_w, configured_h);
        }

        if let Some((w, h)) = Self::detect_resolution() {
            info!("Auto-detected screen resolution: {}x{}", w, h);
            return (w, h);
        }

        warn!(
            "Failed to auto-detect screen resolution! Defaulting to 1920x1080. \
             Please set `screen_width` and `screen_height` under [grid] in ~/.config/fckmouse/config.toml."
        );
        (1920, 1080)
    }

    /// Attempts to query resolution via compositor tools (`niri msg outputs`) or `xrandr`.
    fn detect_resolution() -> Option<(u32, u32)> {
        // 1. Try niri
        if let Ok(output) = Command::new("niri").args(["msg", "outputs"]).output()
            && output.status.success()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                let trimmed = line.trim();
                if (trimmed.starts_with("Current mode:") || trimmed.starts_with("Logical size:"))
                    && let Some(res) = parse_dimensions(trimmed)
                {
                    return Some(res);
                }
            }
        }

        // 2. Try xrandr
        if let Ok(output) = Command::new("xrandr").output()
            && output.status.success()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if line.contains(" connected") {
                    for part in line.split_whitespace() {
                        if let Some(res) = parse_dimensions(part) {
                            return Some(res);
                        }
                    }
                }
            }
        }

        None
    }

    /// Accumulates relative movement deltas emitted to uinput.
    pub fn accumulate(&mut self, dx: i32, dy: i32) {
        self.x = (self.x + dx as f64).clamp(0.0, self.screen_width.saturating_sub(1) as f64);
        self.y = (self.y + dy as f64).clamp(0.0, self.screen_height.saturating_sub(1) as f64);
    }

    /// Computes the relative delta required to warp the cursor to `(target_x, target_y)`.
    pub fn delta_to(&mut self, target_x: f64, target_y: f64) -> (i32, i32) {
        let clamped_x = target_x.clamp(0.0, self.screen_width.saturating_sub(1) as f64);
        let clamped_y = target_y.clamp(0.0, self.screen_height.saturating_sub(1) as f64);

        let dx = (clamped_x - self.x).round() as i32;
        let dy = (clamped_y - self.y).round() as i32;

        self.x = clamped_x;
        self.y = clamped_y;

        (dx, dy)
    }
}

/// Helper function to parse `WIDTHxHEIGHT` strings like `1920x1080` or `1366x768`.
fn parse_dimensions(s: &str) -> Option<(u32, u32)> {
    for token in s.split(|c: char| !c.is_alphanumeric()) {
        let parts: Vec<&str> = token.split('x').collect();
        if parts.len() == 2
            && let Ok(w) = parts[0].parse::<u32>()
            && let Ok(h) = parts[1].parse::<u32>()
            && w >= 320
            && h >= 240
        {
            return Some((w, h));
        }
    }
    None
}

/// Active bounding box for recursive grid subdivision.
#[derive(Debug, Clone, Copy)]
pub struct GridRegion {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl GridRegion {
    /// Full screen region based on dimensions.
    pub fn full(width: u32, height: u32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: width as f64,
            height: height as f64,
        }
    }

    /// Returns the center point of the current region.
    pub fn center(&self) -> (f64, f64) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Subdivides the region into rows x cols and returns the sub-region at (row, col).
    pub fn sub_cell(&self, row: u32, col: u32, total_rows: u32, total_cols: u32) -> Self {
        let cell_w = self.width / total_rows.max(1) as f64;
        let cell_h = self.height / total_cols.max(1) as f64;
        Self {
            x: self.x + (col as f64 * cell_w),
            y: self.y + (row as f64 * cell_h),
            width: cell_w,
            height: cell_h,
        }
    }
}

/// Maps standard 3x3 keys (QWE/ASD/ZXC or Numpad 1-9) to `(row, col)` index (0-indexed).
pub fn key_to_grid_cell(key: KeyCode) -> Option<(u32, u32)> {
    match key {
        // Row 0: Top
        KeyCode::KEY_Q | KeyCode::KEY_KP7 | KeyCode::KEY_7 => Some((0, 0)),
        KeyCode::KEY_W | KeyCode::KEY_KP8 | KeyCode::KEY_8 => Some((0, 1)),
        KeyCode::KEY_E | KeyCode::KEY_KP9 | KeyCode::KEY_9 => Some((0, 2)),

        // Row 1: Middle
        KeyCode::KEY_A | KeyCode::KEY_KP4 | KeyCode::KEY_4 => Some((1, 0)),
        KeyCode::KEY_S | KeyCode::KEY_KP5 | KeyCode::KEY_5 => Some((1, 1)),
        KeyCode::KEY_D | KeyCode::KEY_KP6 | KeyCode::KEY_6 => Some((1, 2)),

        // Row 2: Bottom
        KeyCode::KEY_Z | KeyCode::KEY_KP1 | KeyCode::KEY_1 => Some((2, 0)),
        KeyCode::KEY_X | KeyCode::KEY_KP2 | KeyCode::KEY_2 => Some((2, 1)),
        KeyCode::KEY_C | KeyCode::KEY_KP3 | KeyCode::KEY_3 => Some((2, 2)),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dimensions() {
        assert_eq!(parse_dimensions("Current mode: 1366x768 @ 60Hz"), Some((1366, 768)));
        assert_eq!(parse_dimensions("eDP-1 connected 1920x1080+0+0"), Some((1920, 1080)));
        assert_eq!(parse_dimensions("invalid string"), None);
    }

    #[test]
    fn test_cursor_tracker_accumulation_and_delta() {
        let mut tracker = CursorTracker {
            x: 500.0,
            y: 500.0,
            screen_width: 1920,
            screen_height: 1080,
        };

        tracker.accumulate(100, -50);
        assert_eq!(tracker.x, 600.0);
        assert_eq!(tracker.y, 450.0);

        let (dx, dy) = tracker.delta_to(1000.0, 500.0);
        assert_eq!(dx, 400);
        assert_eq!(dy, 50);
        assert_eq!(tracker.x, 1000.0);
        assert_eq!(tracker.y, 500.0);
    }

    #[test]
    fn test_grid_subdivision() {
        let region = GridRegion::full(900, 900);
        assert_eq!(region.center(), (450.0, 450.0));

        // Center cell (1, 1) in 3x3
        let center_cell = region.sub_cell(1, 1, 3, 3);
        assert_eq!(center_cell.x, 300.0);
        assert_eq!(center_cell.y, 300.0);
        assert_eq!(center_cell.width, 300.0);
        assert_eq!(center_cell.height, 300.0);
        assert_eq!(center_cell.center(), (450.0, 450.0));

        // Top-left cell (0, 0)
        let top_left = region.sub_cell(0, 0, 3, 3);
        assert_eq!(top_left.x, 0.0);
        assert_eq!(top_left.y, 0.0);
        assert_eq!(top_left.center(), (150.0, 150.0));
    }

    #[test]
    fn test_key_to_grid_cell() {
        assert_eq!(key_to_grid_cell(KeyCode::KEY_Q), Some((0, 0)));
        assert_eq!(key_to_grid_cell(KeyCode::KEY_S), Some((1, 1)));
        assert_eq!(key_to_grid_cell(KeyCode::KEY_C), Some((2, 2)));
        assert_eq!(key_to_grid_cell(KeyCode::KEY_KP5), Some((1, 1)));
        assert_eq!(key_to_grid_cell(KeyCode::KEY_SPACE), None);
    }
}
