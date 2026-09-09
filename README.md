<div align="center">

<img src="./fckmouse_logo.png" alt="fckmouse logo" width="280" />

# fckmouse

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.85+-orange.svg)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/Platform-Linux%20(Wayland%20%2F%20X11)-blue.svg)](#)

</div>

---

## 📖 Overview

`fckmouse` creates a Linux kernel-level virtual mouse device (`/dev/uinput`) that translates modal keystrokes into fluid, pixel-perfect cursor movement, clean clicking, and full 2D scrolling.

It works in **all desktop environments** (GNOME, KDE, XFCE, etc.), **all Wayland compositors** (niri, Sway, Hyprland), and **X11** on **any Linux distribution** — except non-Linux OSes (macOS/Windows).

### 📦 Dependencies & Requirements
- **Linux Kernel**: with `/dev/uinput` enabled (standard on all modern distros).
- **User Permissions**: user in the `input` group (`sudo usermod -aG input $USER`).
- **Zero Runtime Dependencies**: self-contained binary (no external C libraries, Python, or compositor-specific tools required).

---

## ✨ Features

- ⚡ **Zero-Latency Virtual Mouse (`/dev/uinput`)**: Emits hardware-level relative events (`REL_X`, `REL_Y`, `REL_WHEEL`, `REL_HWHEEL`) and button events (`BTN_LEFT`, `BTN_RIGHT`, `BTN_MIDDLE`).
- 🔒 **Exclusive Keyboard Grabbing**: When entering Mouse Mode, `fckmouse` grabs the keyboard via kernel `EVIOCGRAB`. Keystrokes control the mouse and **never leak or type into your open documents or browser**.
- 🛡️ **Clean Grab Transition**: Waits for activation keys (`Alt`, `Shift`) to release before locking the device, ensuring the compositor receives key-up events and completely preventing stuck `Shift` / duplicate window link clicks.
- 🧭 **2D Scrolling Engine**: Smooth vertical and horizontal scrolling mapped to Arrow keys (`Up`/`Down` and `Left`/`Right`) plus `PageUp`/`PageDown`, completely avoiding browser shortcut conflicts (like Brave's Read Aloud player).
- 📈 **Progressive Acceleration Curve**: Single-pixel crawl ($1\text{–}2.5\text{px}$) on quick taps for clicking tiny links; smooth quadratic acceleration ramping up to $28\text{px/tick}$ on sustained hold.
- 🚀 **Turbo & Precision Multipliers**: Hold `Ctrl` for instant $3\times$ speed across multi-monitor setups, or `Shift` for $0.3\times$ precision crawl.
- 🪟 **Desktop & CLI Shortcuts**: Supports instant one-shot subcommands (`fckmouse move`, `fckmouse click`, `fckmouse scroll`) bindable to desktop or window manager hotkeys.
- 🔌 **Dynamic USB Hotplug & Multi-Device**: Automatically detects when keyboards are plugged, unplugged, or re-enumerated when connecting/disconnecting external USB devices without restarting the daemon.
- 🩺 **Built-in Doctor (`fckmouse doctor`)**: Instant diagnostics of `/dev/uinput`, `/dev/input/event*`, and user group permissions.

---

## 🎮 How to Use (Cheat Sheet)

`fckmouse` uses **Modal Mouse Mode** with an exclusive Linux kernel keyboard lock (`EVIOCGRAB`). When active, keystrokes exclusively control the mouse without leaking letters into open documents, triggering browser shortcuts, or holding down sticky modifiers.

1. Press **`Alt + Shift + M`** to toggle **Mouse Mode**.
2. Control cursor gliding, clicking, and 2D scrolling effortlessly:

| Key(s) | Action | Hand / Group | Description |
| :--- | :--- | :--- | :--- |
| **`WASD`** / `HJKL` | **Move Cursor** | Left Hand | Smooth acceleration (supports diagonal motion) |
| **`Space`** | **Left Click** | Left Hand | Standard clean left click (no duplicate window!) |
| **`C`** | **Right Click** | Left Hand | Context menu / right click |
| **`V`** | **Middle Click** | Left Hand | Open links in new tab / terminal paste |
| **`Shift` (Hold)** | **Precision Crawl** | Left Hand | $0.3\times$ slow-motion speed for pixel-perfect targeting |
| **`Ctrl` (Hold)** | **Turbo Boost** | Left Hand | $3\times$ speed boost across multiple monitors |
| **`Up` / `Down`** | **Vertical Scroll** | Right Hand | Wheel scroll up / down (smooth continuous hold) |
| **`Left` / `Right`** | **Horizontal Scroll** | Right Hand | Horizontal wheel scroll for wide code blocks & sheets |
| **`PageUp` / `PageDown`** | **Page Scroll** | Right Hand | Fast page-up / page-down scrolling |
| **`,` / `.`** | **Alternate Scroll** | Right Hand | Alternate scroll up / down keys |
| **`Esc`** or **`Alt + Shift + M`** | **Exit Mouse Mode** | Either Hand | Releases keyboard grab and returns immediately to normal typing |

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
[modal]
# Mouse Mode (exclusive keyboard lock via EVIOCGRAB):
enabled = true

# Modifiers required to toggle Mouse Mode:
toggle_modifiers = ["alt", "shift"]

# Key to toggle Mouse Mode (when toggle modifiers are held):
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
