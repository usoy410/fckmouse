use crate::config::AppConfig;
use crate::physics::MovementPhysics;
use crate::uinput::{MouseButton, MouseDevice};
use anyhow::{Context, Result};
use evdev::{Device, EventType, KeyCode};
use log::{debug, info, warn};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, sleep};
use std::time::Duration;

/// Internal event representing a keyboard key transition.
#[derive(Debug, Clone, Copy)]
pub struct RawKeyEvent {
    pub key: KeyCode,
    /// 0 = release, 1 = press, 2 = repeat
    pub value: i32,
}

/// Checks if a device is likely a real user-typing keyboard.
pub fn is_keyboard_device(device: &Device) -> bool {
    let supported_keys = match device.supported_keys() {
        Some(keys) => keys,
        None => return false,
    };

    // Must have standard alphanumeric typing keys
    supported_keys.contains(KeyCode::KEY_A)
        && supported_keys.contains(KeyCode::KEY_Z)
        && supported_keys.contains(KeyCode::KEY_SPACE)
        && supported_keys.contains(KeyCode::KEY_ENTER)
}

/// Discovers all available keyboard devices in `/dev/input/`.
pub fn discover_keyboards() -> Result<Vec<PathBuf>> {
    let mut keyboards = Vec::new();
    let mut permission_denied_count = 0;
    let entries = fs::read_dir("/dev/input")
        .context("Failed to read directory /dev/input. Check system input configuration.")?;

    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str())
            && file_name.starts_with("event")
        {
            match Device::open(&path) {
                Ok(device) => {
                    if is_keyboard_device(&device) {
                        debug!(
                            "Detected keyboard: {:?} ({})",
                            path,
                            device.name().unwrap_or("Unknown")
                        );
                        keyboards.push(path);
                    }
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::PermissionDenied {
                        permission_denied_count += 1;
                    }
                    debug!("Skipping device {:?}: {}", path, e);
                }
            }
        }
    }

    if keyboards.is_empty() && permission_denied_count > 0 {
        return Err(anyhow::anyhow!(
            "Permission denied when attempting to access /dev/input/event* devices ({} denied).\n\
            Your Linux user account does not have read permissions for raw evdev input.\n\n\
            💡 TO FIX THIS (One-time standard Linux setup):\n\
               1. Add your user to the 'input' group:\n\
                  sudo usermod -aG input $USER\n\
               2. Apply group changes by logging out and back in (or run 'newgrp input').\n\n\
            💡 ALTERNATIVELY, you can bind fckmouse commands directly in your window manager (niri) config without any group changes:\n\
               Mod+Alt+Left  {{ spawn \"fckmouse\" \"move\" \"--dx\" \"-25\"; }}\n\
               Mod+Alt+Right {{ spawn \"fckmouse\" \"move\" \"--dx\" \"25\"; }}\n\
               Mod+Alt+Up    {{ spawn \"fckmouse\" \"move\" \"--dy\" \"-25\"; }}\n\
               Mod+Alt+Down  {{ spawn \"fckmouse\" \"move\" \"--dy\" \"25\"; }}\n\
               Mod+Alt+Space {{ spawn \"fckmouse\" \"click\"; }}",
            permission_denied_count
        ));
    }

    Ok(keyboards)
}

/// Tracks the active state of keyboard keys and handles modal logic.
pub struct KeyboardController {
    config: AppConfig,
    mouse: MouseDevice,
    physics: MovementPhysics,
    pressed_keys: HashSet<u16>,
    modal_active: Arc<AtomicBool>,
    pending_grab: bool,
    pending_grab_start: Option<std::time::Instant>,
    scroll_tick_counter: u32,
}

impl KeyboardController {
    pub fn new(config: AppConfig, modal_active: Arc<AtomicBool>) -> Result<Self> {
        let mouse = MouseDevice::new()?;
        let physics = MovementPhysics::new(config.physics.clone());
        Ok(Self {
            config,
            mouse,
            physics,
            pressed_keys: HashSet::new(),
            modal_active,
            pending_grab: false,
            pending_grab_start: None,
            scroll_tick_counter: 0,
        })
    }

    /// Checks if the configured toggle chord modifiers (e.g. ["alt", "shift"]) are all pressed.
    pub fn is_toggle_chord_active(&self) -> bool {
        if self.config.modal.toggle_modifiers.is_empty() {
            return false;
        }

        self.config.modal.toggle_modifiers.iter().all(|mod_name| {
            crate::config::is_key_string_pressed(mod_name, &self.pressed_keys)
        })
    }

    /// Handles an incoming key event.
    pub fn handle_key(&mut self, event: RawKeyEvent) -> Result<()> {
        let code = event.key.code();
        match event.value {
            0 => {
                // Key release
                self.pressed_keys.remove(&code);
            }
            1 => {
                // Key press
                self.pressed_keys.insert(code);
                self.on_key_press(event.key)?;
            }
            2 => {
                // Key repeat
                self.pressed_keys.insert(code);
            }
            _ => {}
        }
        Ok(())
    }

    /// Handles one-shot actions triggered on key down.
    fn on_key_press(&mut self, key: KeyCode) -> Result<()> {
        let is_modal = self.modal_active.load(Ordering::SeqCst) || self.pending_grab;

        // 1. Toggle modal mode:
        // Triggered when toggle modifiers (e.g. Alt+Shift) are held and toggle key (e.g. M) is pressed,
        // or when in modal mode and the toggle key is pressed.
        if (self.is_toggle_chord_active() || is_modal) && self.config.modal.toggle.matches(key) {
            if is_modal {
                self.pending_grab = false;
                self.pending_grab_start = None;
                self.modal_active.store(false, Ordering::SeqCst);
                info!("=== [⌨️ MOUSE MODE DEACTIVATED - Keyboard Released] ===");
                println!("\n🔙 [fckmouse] Mouse Mode EXITED. Returned to standard typing.\n");
            } else {
                // Clean grab transition: wait until Alt and Shift are released before grabbing.
                // This ensures Wayland/X11 receives key-up events, preventing stuck modifiers and duplicate window clicks!
                self.pending_grab = true;
                self.pending_grab_start = Some(std::time::Instant::now());
                info!("=== [🖱️ MOUSE MODE ACTIVATING - Waiting for modifier release] ===");
                println!("\n✨ [fckmouse] Mouse Mode activating... Release Alt+Shift to engage exclusive lock.\n");
            }
            return Ok(());
        }

        // 2. Modal Mouse Mode Actions (When exclusively in Mouse Mode or pending grab)
        if is_modal {
            if self.config.modal.exit.matches(key) {
                self.pending_grab = false;
                self.pending_grab_start = None;
                self.modal_active.store(false, Ordering::SeqCst);
                println!("\n🔙 [fckmouse] Mouse Mode EXITED. Returned to standard typing.\n");
                return Ok(());
            }
            if self.config.modal.left_click.matches(key) {
                self.mouse.click(MouseButton::Left)?;
                return Ok(());
            }
            if self.config.modal.right_click.matches(key) {
                self.mouse.click(MouseButton::Right)?;
                return Ok(());
            }
            if self.config.modal.middle_click.matches(key) {
                self.mouse.click(MouseButton::Middle)?;
                return Ok(());
            }
            // Immediate response on key press for responsive single taps
            if self.config.modal.scroll_up.matches(key) {
                self.mouse.scroll_vertical(self.config.modal.scroll_speed)?;
                return Ok(());
            }
            if self.config.modal.scroll_down.matches(key) {
                self.mouse.scroll_vertical(-self.config.modal.scroll_speed)?;
                return Ok(());
            }
            if self.config.modal.scroll_left.matches(key) {
                self.mouse.scroll_horizontal(-self.config.modal.scroll_speed)?;
                return Ok(());
            }
            if self.config.modal.scroll_right.matches(key) {
                self.mouse.scroll_horizontal(self.config.modal.scroll_speed)?;
                return Ok(());
            }
        }

        Ok(())
    }

    /// Calculates current desired direction vector based on active keys in Mouse Mode.
    pub fn calculate_direction(&self) -> (i32, i32) {
        let mut dir_x = 0;
        let mut dir_y = 0;

        let is_modal = self.modal_active.load(Ordering::SeqCst) || self.pending_grab;

        if is_modal {
            if self.config.modal.move_left.is_any_pressed(&self.pressed_keys) {
                dir_x -= 1;
            }
            if self.config.modal.move_right.is_any_pressed(&self.pressed_keys) {
                dir_x += 1;
            }
            if self.config.modal.move_up.is_any_pressed(&self.pressed_keys) {
                dir_y -= 1;
            }
            if self.config.modal.move_down.is_any_pressed(&self.pressed_keys) {
                dir_y += 1;
            }
        }

        (dir_x, dir_y)
    }

    /// Calculates 2D scroll request (sx, sy) based on pressed keys in Mouse Mode.
    pub fn calculate_scroll(&self) -> (i32, i32) {
        let mut sx = 0;
        let mut sy = 0;

        let is_modal = self.modal_active.load(Ordering::SeqCst) || self.pending_grab;

        if is_modal {
            if self.config.modal.scroll_up.is_any_pressed(&self.pressed_keys) {
                sy += 1;
            }
            if self.config.modal.scroll_down.is_any_pressed(&self.pressed_keys) {
                sy -= 1;
            }
            if self.config.modal.scroll_left.is_any_pressed(&self.pressed_keys) {
                sx -= 1;
            }
            if self.config.modal.scroll_right.is_any_pressed(&self.pressed_keys) {
                sx += 1;
            }
        }

        (sx, sy)
    }

    /// Tick update loop: applies acceleration curve, continuous 2D scrolling, and handles clean grab.
    pub fn tick(&mut self) -> Result<()> {
        // 1. Process clean grab transition
        if self.pending_grab {
            let elapsed_ms = self
                .pending_grab_start
                .map(|t| t.elapsed().as_millis())
                .unwrap_or(0);

            // Once toggle modifiers are released, or after 350ms safety timeout:
            if !self.is_toggle_chord_active() || elapsed_ms > 350 {
                self.pending_grab = false;
                self.pending_grab_start = None;
                self.modal_active.store(true, Ordering::SeqCst);
                info!("Clean grab transition complete (modifiers released). Mouse Mode active.");
                println!("🔒 [fckmouse] Keyboard exclusively grabbed (zero modifier leakage).");
            }
        }

        let is_modal = self.modal_active.load(Ordering::SeqCst) || self.pending_grab;
        if !is_modal {
            return Ok(());
        }

        let turbo = self.config.modal.turbo.is_any_pressed(&self.pressed_keys);
        let precision = self.config.modal.precision.is_any_pressed(&self.pressed_keys);

        // 2. Cursor movement
        let (dir_x, dir_y) = self.calculate_direction();
        let (dx, dy) = self.physics.step(dir_x, dir_y, turbo, precision);

        if dx != 0 || dy != 0 {
            self.mouse.move_relative(dx, dy)?;
        }

        // 3. Continuous 2D scrolling
        let (sx, sy) = self.calculate_scroll();
        if sx != 0 || sy != 0 {
            self.scroll_tick_counter += 1;
            let interval = if turbo { 2 } else if precision { 8 } else { 4 };
            if self.scroll_tick_counter >= interval {
                self.scroll_tick_counter = 0;
                let base_speed = self.config.modal.scroll_speed;
                let mult = if turbo { 2 } else { 1 };
                if sy != 0 {
                    self.mouse.scroll_vertical(sy * base_speed * mult)?;
                }
                if sx != 0 {
                    self.mouse.scroll_horizontal(sx * base_speed * mult)?;
                }
            }
        } else {
            self.scroll_tick_counter = 0;
        }

        Ok(())
    }

    /// Accessor for loop sleep duration.
    pub fn tick_interval(&self) -> Duration {
        Duration::from_millis(self.physics.tick_rate_ms())
    }
}

/// Internal event sent from keyboard workers to the central daemon.
#[derive(Debug, Clone)]
pub enum DaemonEvent {
    Key(RawKeyEvent),
    Disconnected(PathBuf),
}

/// Spawns background listener threads for each keyboard and runs the central controller loop.
/// Dynamically detects newly connected or re-enumerated USB keyboards.
pub fn run_event_loop(config: AppConfig, running: Arc<AtomicBool>) -> Result<()> {
    let modal_active = Arc::new(AtomicBool::new(false));
    let (tx, rx): (Sender<DaemonEvent>, Receiver<DaemonEvent>) = channel();
    let mut active_keyboards: HashSet<PathBuf> = HashSet::new();

    let mut controller = KeyboardController::new(config, modal_active.clone())?;
    let tick_dur = controller.tick_interval();

    info!("fckmouse daemon is running. Press Ctrl+C to terminate.");
    println!("🚀 fckmouse daemon started successfully!");
    println!("👉 Toggle Mouse Mode: Press Alt + Shift + M (Exclusive keyboard lock)");
    println!("👉 Left Hand:         WASD moves cursor, Space clicks, C right click, V middle click");
    println!("👉 Right Hand:        Arrows for 2D scrolling, PageUp/PageDown for page scroll");
    println!("👉 Multipliers:       Hold Ctrl for Turbo (3x), Shift for Precision (0.3x)");
    println!("👉 Exit Mouse Mode:   Press Esc or Alt + Shift + M\n");

    let mut tick_counter: u64 = 0;

    while running.load(Ordering::SeqCst) {
        // Periodic rescan for newly connected keyboards or USB re-enumeration (every 100 ticks = ~1 second)
        if (tick_counter.is_multiple_of(100) || active_keyboards.is_empty())
            && let Ok(discovered) = discover_keyboards()
        {
            for path in discovered {
                if !active_keyboards.contains(&path) {
                    info!("Hotplugged keyboard listener for: {:?}", path);
                    active_keyboards.insert(path.clone());
                    let tx_clone = tx.clone();
                    let running_clone = running.clone();
                    let modal_clone = modal_active.clone();
                    let path_clone = path.clone();

                    let _ = thread::Builder::new()
                        .name(format!("kbd-{:?}", path.file_name()))
                        .spawn(move || {
                            listen_keyboard_worker(path_clone, tx_clone, running_clone, modal_clone);
                        });
                }
            }
        }
        tick_counter = tick_counter.wrapping_add(1);

        // Drain all pending events from workers
        while let Ok(event) = rx.try_recv() {
            match event {
                DaemonEvent::Key(raw_key) => {
                    if let Err(e) = controller.handle_key(raw_key) {
                        warn!("Error handling key event: {:?}", e);
                    }
                }
                DaemonEvent::Disconnected(path) => {
                    active_keyboards.remove(&path);
                    info!("Keyboard removed from active set: {:?}", path);
                }
            }
        }

        // Run physics & move cursor if movement keys are pressed
        if let Err(e) = controller.tick() {
            warn!("Error during cursor tick: {:?}", e);
        }

        sleep(tick_dur);
    }

    // Ensure all exclusive grabs are released on exit
    modal_active.store(false, Ordering::SeqCst);
    sleep(Duration::from_millis(15));

    info!("fckmouse daemon shutting down cleanly.");
    Ok(())
}

fn listen_keyboard_worker(
    path: PathBuf,
    tx: Sender<DaemonEvent>,
    running: Arc<AtomicBool>,
    modal_active: Arc<AtomicBool>,
) {
    let mut device = match Device::open(&path) {
        Ok(dev) => dev,
        Err(e) => {
            debug!("Failed to open keyboard device {:?}: {}", path, e);
            let _ = tx.send(DaemonEvent::Disconnected(path));
            return;
        }
    };

    let _ = device.set_nonblocking(true);

    while running.load(Ordering::SeqCst) {
        let is_modal = modal_active.load(Ordering::SeqCst);
        if is_modal != device.is_grabbed() {
            if is_modal {
                if let Err(e) = device.grab() {
                    warn!("Failed to exclusively grab keyboard {:?}: {}", path, e);
                } else {
                    info!("Exclusive grab active on {:?}", path);
                }
            } else {
                let _ = device.ungrab();
                info!("Exclusive grab released on {:?}", path);
            }
        }

        let mut receiver_closed = false;
        let mut disconnected = false;

        match device.fetch_events() {
            Ok(events) => {
                for ev in events {
                    if ev.event_type() == EventType::KEY {
                        let raw = RawKeyEvent {
                            key: KeyCode(ev.code()),
                            value: ev.value(),
                        };
                        if tx.send(DaemonEvent::Key(raw)).is_err() {
                            receiver_closed = true;
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    sleep(Duration::from_millis(4));
                } else {
                    // Fatal device error (e.g. ENODEV on unplug/reset)
                    debug!("Keyboard {:?} disconnected ({})", path, e);
                    disconnected = true;
                }
            }
        }

        if receiver_closed || disconnected {
            let _ = device.ungrab();
            if disconnected {
                let _ = tx.send(DaemonEvent::Disconnected(path));
            }
            return;
        }
    }

    let _ = device.ungrab();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toggle_chord_detection() {
        let config = AppConfig::default();
        let modal_active = Arc::new(AtomicBool::new(false));
        if let Ok(mut controller) = KeyboardController::new(config, modal_active) {
            assert!(!controller.is_toggle_chord_active());

            // Press Alt
            controller.pressed_keys.insert(KeyCode::KEY_LEFTALT.code());
            assert!(!controller.is_toggle_chord_active());

            // Press Shift
            controller.pressed_keys.insert(KeyCode::KEY_LEFTSHIFT.code());
            assert!(controller.is_toggle_chord_active());

            // Release Shift
            controller.pressed_keys.remove(&KeyCode::KEY_LEFTSHIFT.code());
            assert!(!controller.is_toggle_chord_active());
        }
    }

    #[test]
    fn test_wasd_direction_calculation() {
        let config = AppConfig::default();
        let modal_active = Arc::new(AtomicBool::new(true));
        if let Ok(mut controller) = KeyboardController::new(config, modal_active) {
            // W -> Up (dy = -1)
            controller.pressed_keys.insert(KeyCode::KEY_W.code());
            assert_eq!(controller.calculate_direction(), (0, -1));

            // W + D -> Up-Right (dx = 1, dy = -1)
            controller.pressed_keys.insert(KeyCode::KEY_D.code());
            assert_eq!(controller.calculate_direction(), (1, -1));

            controller.pressed_keys.clear();

            // A -> Left (dx = -1)
            controller.pressed_keys.insert(KeyCode::KEY_A.code());
            assert_eq!(controller.calculate_direction(), (-1, 0));

            // S -> Down (dy = 1)
            controller.pressed_keys.clear();
            controller.pressed_keys.insert(KeyCode::KEY_S.code());
            assert_eq!(controller.calculate_direction(), (0, 1));
        }
    }

    #[test]
    fn test_arrow_keys_2d_scroll_calculation() {
        let config = AppConfig::default();
        let modal_active = Arc::new(AtomicBool::new(true));
        if let Ok(mut controller) = KeyboardController::new(config, modal_active) {
            // Arrow Up -> sy = +1
            controller.pressed_keys.insert(KeyCode::KEY_UP.code());
            assert_eq!(controller.calculate_scroll(), (0, 1));

            // Arrow Down -> sy = -1
            controller.pressed_keys.clear();
            controller.pressed_keys.insert(KeyCode::KEY_DOWN.code());
            assert_eq!(controller.calculate_scroll(), (0, -1));

            // Arrow Left -> sx = -1
            controller.pressed_keys.clear();
            controller.pressed_keys.insert(KeyCode::KEY_LEFT.code());
            assert_eq!(controller.calculate_scroll(), (-1, 0));

            // Arrow Right -> sx = +1
            controller.pressed_keys.clear();
            controller.pressed_keys.insert(KeyCode::KEY_RIGHT.code());
            assert_eq!(controller.calculate_scroll(), (1, 0));

            // PageUp -> sy = +1
            controller.pressed_keys.clear();
            controller.pressed_keys.insert(KeyCode::KEY_PAGEUP.code());
            assert_eq!(controller.calculate_scroll(), (0, 1));

            // PageDown -> sy = -1
            controller.pressed_keys.clear();
            controller.pressed_keys.insert(KeyCode::KEY_PAGEDOWN.code());
            assert_eq!(controller.calculate_scroll(), (0, -1));

            // Comma (scroll up) and Dot (scroll down)
            controller.pressed_keys.clear();
            controller.pressed_keys.insert(KeyCode::KEY_COMMA.code());
            assert_eq!(controller.calculate_scroll(), (0, 1));

            controller.pressed_keys.clear();
            controller.pressed_keys.insert(KeyCode::KEY_DOT.code());
            assert_eq!(controller.calculate_scroll(), (0, -1));
        }
    }
}
