use crate::config::AppConfig;
use crate::physics::MovementPhysics;
use crate::uinput::{MouseButton, MouseDevice};
use anyhow::{Context, Result};
use evdev::{Device, EventType, KeyCode};
use log::{debug, error, info, warn};
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
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            if file_name.starts_with("event") {
                match Device::open(&path) {
                    Ok(device) => {
                        if is_keyboard_device(&device) {
                            info!(
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

/// Tracks the active state of keyboard keys and handles chord/modal logic.
pub struct KeyboardController {
    config: AppConfig,
    mouse: MouseDevice,
    physics: MovementPhysics,
    pressed_keys: HashSet<u16>,
    modal_active: Arc<AtomicBool>,
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
        })
    }



    /// Returns true if the configured chord modifiers are currently satisfied.
    pub fn chord_modifiers_active(&self) -> bool {
        if !self.config.chord.enabled {
            return false;
        }

        if self.config.chord.modifiers.is_empty() {
            return false;
        }

        self.config.chord.modifiers.iter().all(|mod_name| {
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
        let is_modal = self.modal_active.load(Ordering::SeqCst);

        // 1. Toggle modal mode:
        // Triggered when chord modifiers are active and configured toggle key is pressed,
        // or when in modal mode and the toggle key is pressed.
        if (self.chord_modifiers_active() || is_modal) && self.config.modal.toggle.matches(key) {
            let next = !is_modal;
            self.modal_active.store(next, Ordering::SeqCst);

            if next {
                info!("=== [🖱️ MOUSE MODE ACTIVATED - Keyboard Grabbed] ===");
                println!("\n✨ [fckmouse] Mouse Mode ACTIVE! Keystrokes exclusively control the mouse.\n");
            } else {
                info!("=== [⌨️ MOUSE MODE DEACTIVATED - Keyboard Released] ===");
                println!("\n🔙 [fckmouse] Mouse Mode EXITED. Returned to standard typing.\n");
            }
            return Ok(());
        }

        // 2. Instant Chord Actions (when chord modifiers are held and NOT in modal mode)
        if !is_modal && self.chord_modifiers_active() {
            if self.config.chord.left_click.matches(key) {
                self.mouse.click(MouseButton::Left)?;
                return Ok(());
            }
            if self.config.chord.right_click.matches(key) {
                self.mouse.click(MouseButton::Right)?;
                return Ok(());
            }
            if self.config.chord.middle_click.matches(key) {
                self.mouse.click(MouseButton::Middle)?;
                return Ok(());
            }
            if self.config.chord.scroll_up.matches(key) {
                self.mouse.scroll_vertical(self.config.modal.scroll_speed)?;
                return Ok(());
            }
            if self.config.chord.scroll_down.matches(key) {
                self.mouse.scroll_vertical(-self.config.modal.scroll_speed)?;
                return Ok(());
            }
        }

        // 3. Modal Mouse Mode Actions (When exclusively in Mouse Mode)
        if is_modal {
            if self.config.modal.exit.matches(key) {
                self.modal_active.store(false, Ordering::SeqCst);
                println!("\n🔙 [fckmouse] Mouse Mode EXITED.\n");
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
            if self.config.modal.scroll_up.matches(key) {
                self.mouse.scroll_vertical(self.config.modal.scroll_speed)?;
                return Ok(());
            }
            if self.config.modal.scroll_down.matches(key) {
                self.mouse.scroll_vertical(-self.config.modal.scroll_speed)?;
                return Ok(());
            }
        }

        Ok(())
    }

    /// Calculates current desired direction vector based on active mode & keys.
    pub fn calculate_direction(&self) -> (i32, i32) {
        let mut dir_x = 0;
        let mut dir_y = 0;

        let is_modal = self.modal_active.load(Ordering::SeqCst);
        let is_chord = self.chord_modifiers_active();

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
        } else if is_chord {
            if self.config.chord.move_left.is_any_pressed(&self.pressed_keys) {
                dir_x -= 1;
            }
            if self.config.chord.move_right.is_any_pressed(&self.pressed_keys) {
                dir_x += 1;
            }
            if self.config.chord.move_up.is_any_pressed(&self.pressed_keys) {
                dir_y -= 1;
            }
            if self.config.chord.move_down.is_any_pressed(&self.pressed_keys) {
                dir_y += 1;
            }
        }

        (dir_x, dir_y)
    }

    /// Tick update loop: applies acceleration curve and moves the mouse.
    pub fn tick(&mut self) -> Result<()> {
        let (dir_x, dir_y) = self.calculate_direction();

        let is_modal = self.modal_active.load(Ordering::SeqCst);
        let is_chord = self.chord_modifiers_active();

        let turbo = if is_modal {
            self.config.modal.turbo.is_any_pressed(&self.pressed_keys)
        } else if is_chord {
            self.config.chord.turbo.is_any_pressed(&self.pressed_keys)
        } else {
            false
        };

        let precision = if is_modal {
            self.config.modal.precision.is_any_pressed(&self.pressed_keys)
        } else {
            false
        };

        let (dx, dy) = self.physics.step(dir_x, dir_y, turbo, precision);

        if dx != 0 || dy != 0 {
            self.mouse.move_relative(dx, dy)?;
        }

        Ok(())
    }

    /// Accessor for loop sleep duration.
    pub fn tick_interval(&self) -> Duration {
        Duration::from_millis(self.physics.tick_rate_ms())
    }
}

/// Spawns background listener threads for each keyboard and runs the central controller loop.
pub fn run_event_loop(config: AppConfig, running: Arc<AtomicBool>) -> Result<()> {
    let keyboards = discover_keyboards()?;
    if keyboards.is_empty() {
        return Err(anyhow::anyhow!(
            "No accessible keyboard devices found in /dev/input/.\n\
            Ensure your user belongs to the 'input' group:\n\
                sudo usermod -aG input $USER\n\
            Then log out and log back in (or run 'newgrp input')."
        ));
    }

    let modal_active = Arc::new(AtomicBool::new(false));
    let (tx, rx): (Sender<RawKeyEvent>, Receiver<RawKeyEvent>) = channel();

    // Spawn a listener thread for each detected keyboard device
    for path in keyboards {
        let tx_clone = tx.clone();
        let running_clone = running.clone();
        let modal_clone = modal_active.clone();
        let path_clone = path.clone();

        thread::Builder::new()
            .name(format!("kbd-{:?}", path.file_name()))
            .spawn(move || {
                listen_keyboard_worker(path_clone, tx_clone, running_clone, modal_clone);
            })
            .context("Failed to spawn keyboard listener thread")?;
    }

    let mut controller = KeyboardController::new(config, modal_active.clone())?;
    let tick_dur = controller.tick_interval();

    info!("fckmouse daemon is running. Press Ctrl+C to terminate.");
    println!("🚀 fckmouse daemon started successfully!");
    println!("👉 Instant Chord: Hold Alt + Shift + Arrow (or WASD) to move cursor");
    println!("👉 Instant Clicks: Hold Alt + Shift + Space (Left Click), C (Right Click), V (Middle Click)");
    println!("👉 Turbo Speed:   Hold Ctrl + Alt + Shift + Arrow for 3x speed");
    println!("👉 Modal Mode:    Press Alt + Shift + M for exclusive Mouse Mode (prevents typing in apps!)\n");

    while running.load(Ordering::SeqCst) {
        // Drain all pending keyboard events from workers
        while let Ok(event) = rx.try_recv() {
            if let Err(e) = controller.handle_key(event) {
                warn!("Error handling key event: {:?}", e);
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
    tx: Sender<RawKeyEvent>,
    running: Arc<AtomicBool>,
    modal_active: Arc<AtomicBool>,
) {
    let mut device = match Device::open(&path) {
        Ok(dev) => dev,
        Err(e) => {
            error!("Failed to open keyboard device {:?}: {}", path, e);
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
        match device.fetch_events() {
            Ok(events) => {
                for ev in events {
                    if ev.event_type() == EventType::KEY {
                        let raw = RawKeyEvent {
                            key: KeyCode(ev.code()),
                            value: ev.value(),
                        };
                        if tx.send(raw).is_err() {
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
                    debug!("Device {:?} fetch_events: {}", path, e);
                    sleep(Duration::from_millis(50));
                }
            }
        }

        if receiver_closed {
            let _ = device.ungrab();
            return;
        }
    }

    let _ = device.ungrab();
}
