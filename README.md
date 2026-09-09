<div align="center">

<img src="./fckmouse_logo.png" alt="fckmouse logo" width="280" />

# fckmouse

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.85+-orange.svg)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/Platform-Linux%20(Wayland%20%2F%20X11)-blue.svg)](#)

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
- 🔌 **Dynamic USB Hotplug & Multi-Device**: Automatically detects when keyboards are plugged, unplugged, or re-enumerated when connecting/disconnecting external USB devices without restarting the daemon.
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

## ⚙️ Configuration (`~/.config/fckmouse/config.toml`)

`fckmouse` is fully customizable. You can bind any keys you like for movements, clicks, scrolls, and modifiers.

Generate your configuration file:
```bash
fckmouse init-config
```

### Full Configuration Reference:
```toml
[chord]
# Instant on-the-fly cursor movement and clicking using key chords.
enabled = true

# Modifiers required to initiate chord cursor actions.
# Customize with any modifier combination: ["alt", "shift"], ["super", "alt"], ["ctrl", "alt"], etc.
modifiers = ["alt", "shift"]

# Direction keys recognized when holding the chord modifiers:
move_left = ["left", "a"]
move_right = ["right", "d"]
move_up = ["up", "w"]
move_down = ["down", "s"]

# Clicks and scrolls recognized when holding chord modifiers:
left_click = "space"
right_click = "c"
middle_click = "v"
scroll_up = "r"
scroll_down = "f"

# Speed multiplier key when holding chord:
turbo = "ctrl"


[modal]
# Vim-style Mouse Mode (exclusive keyboard lock via EVIOCGRAB):
enabled = true

# Key to toggle Mouse Mode (when chord modifiers are held):
toggle = "m"

# Key to exit Mouse Mode back to normal typing:
exit = "esc"

# Direction keys in Mouse Mode (e.g. arrows, WASD, and HJKL):
move_left = ["left", "a", "h"]
move_right = ["right", "d", "l"]
move_up = ["up", "w", "k"]
move_down = ["down", "s", "j"]

# Actions in Mouse Mode:
left_click = "space"
right_click = "c"
middle_click = "v"
scroll_up = "r"
scroll_down = "f"

# Modifiers inside Mouse Mode:
precision = "shift"    # Hold for 0.3x slow precision crawl
turbo = "ctrl"         # Hold for 3.0x turbo speed
scroll_speed = 1       # Scroll wheel notches per tick


[physics]
# Movement physics and acceleration curve:
base_speed = 2.5            # Starting speed (pixels per 10ms tick)
max_speed = 28.0            # Maximum terminal speed under sustained hold
acceleration = 35.0         # Rate of acceleration per second
turbo_multiplier = 3.0      # Multiplier when Turbo key is held
precision_multiplier = 0.3  # Multiplier when Precision key is held
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
