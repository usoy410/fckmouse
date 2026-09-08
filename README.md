<div align="center">

# 🖱️ fckmouse

**Zero-latency, zero-conflict keyboard-driven mouse navigation daemon for Linux tiling window managers.**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.85+-orange.svg)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/Platform-Linux%20(Wayland%20%2F%20X11)-blue.svg)](#)

*Never take your hands off the keyboard again.*

</div>

---

## 📖 Overview

`fckmouse` creates a Linux kernel-level virtual mouse device (`/dev/uinput`) that translates keyboard chords and modal keystrokes into fluid, pixel-perfect cursor movement, clicking, and scrolling.

It works natively across all **Wayland compositors** ([niri](https://github.com/YaLTeR/niri), Sway, Hyprland, River), **X11**, and Linux TTY without requiring `sudo` or micro-libraries.

---

## ✨ Features

- 🛡️ **Zero-Conflict Design**: Default keybindings (`Alt + Shift + ...`) avoid collisions with text selection (`Shift + Arrow`), word navigation (`Ctrl + Arrow`), browser commands (`Ctrl + W/A/S/D`), and window manager shortcuts.
- ⚡ **Zero-Latency Virtual Mouse (`/dev/uinput`)**: Emits hardware-level relative events (`REL_X`, `REL_Y`, `REL_WHEEL`) and button events (`BTN_LEFT`, `BTN_RIGHT`, `BTN_MIDDLE`).
- 🔒 **Exclusive Keyboard Grabbing**: When entering Mouse Mode, `fckmouse` grabs the keyboard via kernel `EVIOCGRAB`. Keystrokes control the mouse and **never leak or type into your open documents or browser**.
- 📈 **Progressive Acceleration Curve**: Single-pixel crawl ($1\text{–}2.5\text{px}$) on quick taps for clicking tiny links; smooth quadratic acceleration ramping up to $28\text{px/tick}$ on sustained hold.
- 🚀 **Turbo & Precision Multipliers**: Hold `Ctrl` for instant $3\times$ speed across multi-monitor setups, or `Shift` for $0.3\times$ precision crawl.
- 🪟 **Window Manager CLI**: Supports instant one-shot subcommands (`fckmouse move`, `fckmouse click`, `fckmouse scroll`) directly bindable in window manager configs.
- 🩺 **Built-in Doctor (`fckmouse doctor`)**: Instant diagnostics of `/dev/uinput`, `/dev/input/event*`, and user group permissions.

---

## 🎮 How to Use (Cheat Sheet)

You have **two ways** to control your cursor:

### Method 1: Instant Chords (No Mode Switching)
Move, click, and scroll on-the-fly while typing without entering any mode.

| Key Combination | Action | Description |
| :--- | :--- | :--- |
| **`Alt + Shift + Arrows`** | **Move Cursor** | Glides cursor with acceleration curve |
| **`Alt + Shift + WASD`** | **Move Cursor** | Left-hand ergonomic movement |
| **`Ctrl + Alt + Shift + Arrows`** | **Turbo Move** | $3\times$ speed boost to jump across displays |
| **`Alt + Shift + Space`** | **Left Click** | Standard left mouse click |
| **`Alt + Shift + C`** | **Right Click** | Context menu / right click |
| **`Alt + Shift + V`** | **Middle Click** | Open link in new tab / paste clipboard |
| **`Alt + Shift + R`** | **Scroll Up** | Scroll wheel up |
| **`Alt + Shift + F`** | **Scroll Down** | Scroll wheel down |

---

### Method 2: Modal Mouse Mode (Exclusive Full Mouse Lock)
For extended mouse navigation and web browsing without holding down multi-key chords.

1. Press **`Alt + Shift + M`** to toggle **Mouse Mode**.
2. The keyboard is exclusively locked for mouse control (**no letters will type into applications**):

| Key | Action | Description |
| :--- | :--- | :--- |
| **`Arrows` / `WASD` / `HJKL`** | **Move Cursor** | Smooth acceleration (supports diagonal motion) |
| **`Space`** | **Left Click** | Standard left mouse click |
| **`C`** | **Right Click** | Context menu / right click |
| **`V`** | **Middle Click** | Open links in new tab / terminal paste |
| **`R`** | **Scroll Up** | Wheel scroll up |
| **`F`** | **Scroll Down** | Wheel scroll down |
| **`Shift` (Hold)** | **Precision Crawl** | $0.3\times$ slow-motion speed for pixel-perfect targeting |
| **`Ctrl` (Hold)** | **Turbo Boost** | $3\times$ speed boost across multiple monitors |
| **`Esc`** or **`Alt + Shift + M`** | **Exit Mouse Mode** | Releases keyboard grab and returns immediately to normal typing |

---

## 🚀 Quick Start & Installation

### 1. Pre-built Binary or Source Build
```bash
# Clone the repository
git clone https://github.com/usoy410/fckmouse.git
cd fckmouse

# Build optimized release binary
cargo build --release

# Install to ~/.local/bin
install -m 755 target/release/fckmouse ~/.local/bin/
```

### 2. One-Time Permission Setup
To allow `fckmouse daemon` to read global keyboard events without `sudo`:
```bash
sudo usermod -aG input $USER
```
> [!IMPORTANT]
> Log out and log back in (or reboot) so Arch Linux / your distribution applies your new `input` group permissions.

### 3. Verify with Doctor
```bash
fckmouse doctor
```
Output:
```
🖥️  Session Environment:
   • Desktop / WM : niri
   • Session Type : wayland

🖱️  Virtual Mouse (/dev/uinput):
   ✅ /dev/uinput is WRITABLE. Virtual mouse works out of the box!

⌨️  Keyboard Devices (/dev/input/):
   ✅ Found 2 accessible keyboard device(s)
```

### 4. Run the Daemon
```bash
fckmouse daemon
```

---

## 🪟 Tiling Window Manager Integration (niri / Sway / Hyprland)

### Autostart via `niri` (`~/.config/niri/config.kdl`):
```kdl
spawn-at-startup "fckmouse" "daemon"
```

### Optional: Direct Window Manager Hotkeys in `binds.kdl`
If you prefer triggering cursor actions directly via compositor keybindings without running a background daemon:
```kdl
binds {
    // Cursor Movement
    Mod+Alt+Left  { spawn "fckmouse" "move" "--dx" "-30"; }
    Mod+Alt+Right { spawn "fckmouse" "move" "--dx" "30"; }
    Mod+Alt+Up    { spawn "fckmouse" "move" "--dy" "-30"; }
    Mod+Alt+Down  { spawn "fckmouse" "move" "--dy" "30"; }

    // Clicks
    Mod+Alt+Space { spawn "fckmouse" "click" "--button" "left"; }
    Mod+Alt+C     { spawn "fckmouse" "click" "--button" "right"; }
    Mod+Alt+V     { spawn "fckmouse" "click" "--button" "middle"; }

    // Scroll
    Mod+Alt+R     { spawn "fckmouse" "scroll" "--dy" "2"; }
    Mod+Alt+F     { spawn "fckmouse" "scroll" "--dy" "-2"; }
}
```

---

## ⚙️ Configuration (`~/.config/fckmouse/config.toml`)

Generate the default documented configuration:
```bash
fckmouse init-config
```

Customization options:
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
turbo_multiplier = 3.0      # Multiplier when Ctrl is held
precision_multiplier = 0.3  # Multiplier when Shift is held in modal mode
tick_rate_ms = 10           # Physics update frequency (10ms = 100 Hz)
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

Check logs anytime:
```bash
journalctl --user -u fckmouse -f
```

---

## 📄 License

MIT License. See [LICENSE](LICENSE) for details.
