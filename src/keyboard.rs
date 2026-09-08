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

    /// Whether the Alt modifier is currently depressed.
    pub fn is_alt_pressed(&self) -> bool {
        self.pressed_keys.contains(&KeyCode::KEY_LEFTALT.code())
            || self.pressed_keys.contains(&KeyCode::KEY_RIGHTALT.code())
    }

    /// Whether the Shift modifier is currently depressed.
    pub fn is_shift_pressed(&self) -> bool {
        self.pressed_keys.contains(&KeyCode::KEY_LEFTSHIFT.code())
            || self.pressed_keys.contains(&KeyCode::KEY_RIGHTSHIFT.code())
    }

    /// Whether the Ctrl modifier is currently depressed.
    pub fn is_ctrl_pressed(&self) -> bool {
        self.pressed_keys.contains(&KeyCode::KEY_LEFTCTRL.code())
            || self.pressed_keys.contains(&KeyCode::KEY_RIGHTCTRL.code())
    }

    /// Whether the Super/Meta (Windows) key is currently depressed.
    pub fn is_super_pressed(&self) -> bool {
        self.pressed_keys.contains(&KeyCode::KEY_LEFTMETA.code())
            || self.pressed_keys.contains(&KeyCode::KEY_RIGHTMETA.code())
    }

    /// Returns true if the configured chord modifiers are currently satisfied.
    pub fn chord_modifiers_active(&self) -> bool {
        let c = &self.config.chord;
        if !c.enabled {
            return false;
        }

        if c.require_alt && !self.is_alt_pressed() {
            return false;
        }
        if c.require_shift && !self.is_shift_pressed() {
            return false;
        }
        if c.require_super && !self.is_super_pressed() {
            return false;
        }

        true
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
        // Toggle modal mode: Alt + Shift + M (or Mod + Alt + M)
        if key == KeyCode::KEY_M && (self.is_alt_pressed() && (self.is_shift_pressed() || self.is_super_pressed())) {
            let current = self.modal_active.load(Ordering::SeqCst);
            let next = !current;
            self.modal_active.store(next, Ordering::SeqCst);

            if next {
                info!("=== [🖱️ MOUSE MODE ACTIVATED - Keyboard Grabbed] ===");
                println!("\n✨ [fckmouse] Mouse Mode ACTIVE! Use Arrows/WASD/HJKL to move, Space to click, Esc to exit.\n");
            } else {
                info!("=== [⌨️ MOUSE MODE DEACTIVATED - Keyboard Released] ===");
                println!("\n🔙 [fckmouse] Mouse Mode EXITED. Returned to standard typing.\n");
            }
            return Ok(());
        }

        // 1. Instant Chord Actions (No need to enter Mouse Mode!)
        // When Alt + Shift is held, allow direct clicking & scrolling on the fly:
        if self.chord_modifiers_active() {
            match key {
                KeyCode::KEY_SPACE => {
                    self.mouse.click(MouseButton::Left)?;
                    return Ok(());
                }
                KeyCode::KEY_C => {
                    self.mouse.click(MouseButton::Right)?;
                    return Ok(());
                }
                KeyCode::KEY_V => {
                    self.mouse.click(MouseButton::Middle)?;
                    return Ok(());
                }
                KeyCode::KEY_R => {
                    self.mouse.scroll_vertical(self.config.modal.scroll_speed)?;
                    return Ok(());
                }
                KeyCode::KEY_F => {
                    self.mouse.scroll_vertical(-self.config.modal.scroll_speed)?;
                    return Ok(());
                }
                _ => {}
            }
        }

        // 2. Modal Mouse Mode Actions (When exclusively in Mouse Mode)
        if self.modal_active.load(Ordering::SeqCst) {
            match key {
                KeyCode::KEY_ESC => {
                    self.modal_active.store(false, Ordering::SeqCst);
                    println!("\n🔙 [fckmouse] Mouse Mode EXITED.\n");
                }
                KeyCode::KEY_SPACE => {
                    self.mouse.click(MouseButton::Left)?;
                }
                KeyCode::KEY_C => {
                    self.mouse.click(MouseButton::Right)?;
                }
                KeyCode::KEY_V => {
                    self.mouse.click(MouseButton::Middle)?;
                }
                KeyCode::KEY_R => {
                    self.mouse.scroll_vertical(self.config.modal.scroll_speed)?;
                }
                KeyCode::KEY_F => {
                    self.mouse.scroll_vertical(-self.config.modal.scroll_speed)?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Calculates current desired direction vector based on active mode & keys.
    pub fn calculate_direction(&self) -> (i32, i32) {
        let mut dir_x = 0;
        let mut dir_y = 0;

        let is_modal = self.modal_active.load(Ordering::SeqCst);
        let allow_arrows = (is_modal && self.config.modal.use_arrow_keys)
            || (self.chord_modifiers_active() && self.config.chord.use_arrow_keys);

        let allow_wasd = (is_modal && self.config.modal.use_wasd_keys)
            || (self.chord_modifiers_active() && self.config.chord.use_wasd_keys);

        let allow_hjkl = is_modal && self.config.modal.use_hjkl_keys;

        // Left
        if (allow_arrows && self.pressed_keys.contains(&KeyCode::KEY_LEFT.code()))
            || (allow_wasd && self.pressed_keys.contains(&KeyCode::KEY_A.code()))
            || (allow_hjkl && self.pressed_keys.contains(&KeyCode::KEY_H.code()))
        {
            dir_x -= 1;
        }

        // Right
        if (allow_arrows && self.pressed_keys.contains(&KeyCode::KEY_RIGHT.code()))
            || (allow_wasd && self.pressed_keys.contains(&KeyCode::KEY_D.code()))
            || (allow_hjkl && self.pressed_keys.contains(&KeyCode::KEY_L.code()))
        {
            dir_x += 1;
        }

        // Up
        if (allow_arrows && self.pressed_keys.contains(&KeyCode::KEY_UP.code()))
            || (allow_wasd && self.pressed_keys.contains(&KeyCode::KEY_W.code()))
            || (allow_hjkl && self.pressed_keys.contains(&KeyCode::KEY_K.code()))
        {
            dir_y -= 1;
        }

        // Down
        if (allow_arrows && self.pressed_keys.contains(&KeyCode::KEY_DOWN.code()))
            || (allow_wasd && self.pressed_keys.contains(&KeyCode::KEY_S.code()))
            || (allow_hjkl && self.pressed_keys.contains(&KeyCode::KEY_J.code()))
        {
            dir_y += 1;
        }

        (dir_x, dir_y)
    }

    /// Tick update loop: applies acceleration curve and moves the mouse.
    pub fn tick(&mut self) -> Result<()> {
        let (dir_x, dir_y) = self.calculate_direction();

        let turbo = self.is_ctrl_pressed();
        let precision = self.modal_active.load(Ordering::SeqCst) && self.is_shift_pressed();

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
