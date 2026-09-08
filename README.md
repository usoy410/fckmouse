# 🖱️ fckmouse

> **Zero-latency, zero-conflict keyboard-driven mouse navigation daemon for Linux tiling window managers (Wayland & X11).**

Never take your hands off the keyboard again. `fckmouse` creates a Linux kernel-level virtual mouse using `/dev/uinput` and translates keyboard chords and modal keystrokes into fluid, pixel-perfect cursor movement and clicks.

---

## ⚡ Features

- **Zero Input Conflict**: Carefully engineered defaults (`Alt + Shift + Arrows / WASD`) that never clash with your text editors, terminal readline, browser shortcuts, or window manager bindings.
- **True Linux Kernel Virtual Mouse (`/dev/uinput`)**: Emits genuine hardware mouse events. Universally supported across **Wayland** (niri, Sway, Hyprland, River), **X11**, and Linux TTY without requiring root or `sudo`.
- **Dual-Layered Navigation**:
  - **Instant Chords**: Nudge the cursor on-the-fly without changing modes.
  - **Modal Mouse Mode**: Tap a hotkey to enter full mouse mode with clicks, middle clicks, and scrolling.
- **Progressive Physics & Acceleration**:
  - Single-pixel micro-stepping on initial tap for precise button/link targeting.
  - Quadratic acceleration curve while held down to glide effortlessly across 1440p / 4K displays.
  - Turbo multiplier (**Ctrl**) for flying across multi-monitor workspaces.
- **Window Manager CLI Integration**: Also supports instant one-shot commands (`fckmouse move`, `fckmouse click`, `fckmouse scroll`) directly bindable in compositor configs like `niri` or `sway`.
- **System Doctor**: Built-in `fckmouse doctor` command to diagnose input permissions and display sessions automatically.

---

## 🎮 Keybindings & Controls

### 1. Instant Chord Mode (No Mode Switching Needed!)
You do **not** need to enter Mouse Mode to click or move! While holding `Alt + Shift`:

| Shortcut | Action |
| :--- | :--- |
| **`Alt + Shift + Arrows`** | Move cursor (smooth acceleration) |
| **`Alt + Shift + WASD`** | Move cursor (left-hand friendly) |
| **`Ctrl + Alt + Shift + Arrows`** | **Turbo Boost** (3x speed across monitors) |
| **`Alt + Shift + Space`** | **Left Click** |
| **`Alt + Shift + C`** | **Right Click** |
| **`Alt + Shift + V`** | **Middle Click** (open links in new tab) |
| **`Alt + Shift + R`** | **Scroll Up** |
| **`Alt + Shift + F`** | **Scroll Down** |

> **Why `Alt + Shift`?**
> Standard combinations like `Shift + Arrow` break text selection/highlighting, `Ctrl + Arrow` breaks word navigation, and `Ctrl + WASD` catastrophically closes tabs (`Ctrl+W`) or selects all (`Ctrl+A`). `Alt + Shift` is completely untouched by standard editors, shells, and desktop applications.

---

### 2. Modal Mouse Mode (Vim-Style)
For continuous mouse navigation and web browsing without holding down multi-key chords.

- **Toggle Mouse Mode**: Press **`Alt + Shift + M`** (or `Mod + Alt + M`).
- When active, your keyboard becomes a dedicated mouse controller:

| Key | Action |
| :--- | :--- |
| **`Arrows` / `WASD` / `HJKL`** | Move cursor with acceleration |
| **`Space`** | **Left Click** |
| **`C`** | **Right Click** |
| **`V`** | **Middle Click** |
| **`R`** | **Scroll Up** |
| **`F`** | **Scroll Down** |
| **`Shift` (Hold)** | **Precision Crawl** ($0.3\times$ slow-motion) |
| **`Ctrl` (Hold)** | **Turbo Boost** ($3\times$ speed) |
| **`Esc`** or **`Alt + Shift + M`** | **Exit Mouse Mode** back to standard typing |

---

## ⚙️ Configuration (`~/.config/fckmouse/config.toml`)

Generate the default documented configuration:
```bash
fckmouse init-config
```

Example configuration:
```toml
[chord]
enabled = true
require_alt = true
require_shift = true
require_super = false
use_arrow_keys = true
use_wasd_keys = true
ctrl_turbo = true

[modal]
enabled = true
use_arrow_keys = true
use_wasd_keys = true
use_hjkl_keys = true
scroll_speed = 1

[physics]
base_speed = 2.5            # Starting speed (pixels per 10ms tick)
max_speed = 28.0            # Terminal speed under sustained hold
acceleration = 35.0         # Rate of acceleration per second
turbo_multiplier = 3.0      # Speed multiplier when Ctrl is held
precision_multiplier = 0.3  # Speed multiplier when Shift is held in modal mode
tick_rate_ms = 10           # Physics update frequency (10ms = 100 Hz)
```

---

## 🚀 Installation & Setup

### 1. Build & Install
```bash
cargo build --release
cp target/release/fckmouse ~/.local/bin/
```

### 2. Check Permissions (`doctor`)
Run diagnostics to verify system access:
```bash
fckmouse doctor
```

- `/dev/uinput` is typically writable out-of-the-box on modern Linux via systemd ACLs.
- To enable global keyboard reading for the background daemon without root, ensure your user is in the `input` group:
  ```bash
  sudo usermod -aG input $USER
  ```
  *(Then log out and log back in, or run `newgrp input`)*.

---

## 🪟 Niri / Tiling Window Manager Integration

You can autostart `fckmouse daemon` or bind direct CLI actions in your window manager config.

### Autostarting Daemon in `niri` (`~/.config/niri/config.kdl`):
```kdl
spawn-at-startup "fckmouse" "daemon"
```

### Optional: Direct Window Manager Hotkeys in `binds.kdl`
If you prefer triggering mouse actions directly from `niri` without running a background listener:
```kdl
binds {
    // Quick cursor nudges
    Mod+Alt+Left  { spawn "fckmouse" "move" "--dx" "-25"; }
    Mod+Alt+Right { spawn "fckmouse" "move" "--dx" "25"; }
    Mod+Alt+Up    { spawn "fckmouse" "move" "--dy" "-25"; }
    Mod+Alt+Down  { spawn "fckmouse" "move" "--dy" "25"; }

    // Left click & Right click
    Mod+Alt+Space { spawn "fckmouse" "click" "--button" "left"; }
    Mod+Alt+C     { spawn "fckmouse" "click" "--button" "right"; }

    // Scroll
    Mod+Alt+Page_Up   { spawn "fckmouse" "scroll" "--dy" "2"; }
    Mod+Alt+Page_Down { spawn "fckmouse" "scroll" "--dy" "-2"; }
}
```

---

## 🛠️ Systemd User Service

To run `fckmouse` as a background user service:
```bash
mkdir -p ~/.config/systemd/user/
cp systemd/fckmouse.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now fckmouse
```

---

## 🧪 Testing

Run unit tests:
```bash
cargo test
```

---

## 📄 License

MIT
