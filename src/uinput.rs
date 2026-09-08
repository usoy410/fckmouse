use anyhow::{Context, Result};
use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, EventType, InputEvent, KeyCode, RelativeAxisCode};
use std::thread::sleep;
use std::time::Duration;

/// Mouse button identifier for click events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

impl MouseButton {
    pub fn to_keycode(self) -> KeyCode {
        match self {
            MouseButton::Left => KeyCode::BTN_LEFT,
            MouseButton::Right => KeyCode::BTN_RIGHT,
            MouseButton::Middle => KeyCode::BTN_MIDDLE,
        }
    }
}

/// Linux `/dev/uinput` Virtual Mouse Driver.
///
/// Creates a kernel-level virtual mouse device that emits relative movements,
/// button clicks, and scroll wheel ticks. Compatible with Wayland, X11, and TTY.
pub struct MouseDevice {
    device: VirtualDevice,
}

impl MouseDevice {
    /// Creates and registers a new virtual mouse with the Linux kernel via `/dev/uinput`.
    pub fn new() -> Result<Self> {
        let keys = AttributeSet::from_iter([
            KeyCode::BTN_LEFT,
            KeyCode::BTN_RIGHT,
            KeyCode::BTN_MIDDLE,
            KeyCode::BTN_SIDE,
            KeyCode::BTN_EXTRA,
        ]);

        let rel_axes = AttributeSet::from_iter([
            RelativeAxisCode::REL_X,
            RelativeAxisCode::REL_Y,
            RelativeAxisCode::REL_WHEEL,
            RelativeAxisCode::REL_HWHEEL,
        ]);

        let device = VirtualDevice::builder()
            .context("Failed to initialize VirtualDevice builder")?
            .name("fckmouse Virtual Mouse")
            .with_keys(&keys)
            .context("Failed to configure mouse button keys")?
            .with_relative_axes(&rel_axes)
            .context("Failed to configure relative mouse axes")?
            .build()
            .context("Failed to build virtual uinput mouse device. Ensure /dev/uinput is writable.")?;

        // Give the kernel and Wayland compositor a moment to attach the new device
        sleep(Duration::from_millis(60));

        Ok(Self { device })
    }

    /// Emits relative cursor movement `(dx, dy)` in pixels.
    pub fn move_relative(&mut self, dx: i32, dy: i32) -> Result<()> {
        if dx == 0 && dy == 0 {
            return Ok(());
        }

        let mut events = Vec::with_capacity(2);
        if dx != 0 {
            events.push(InputEvent::new_now(
                EventType::RELATIVE.0,
                RelativeAxisCode::REL_X.0,
                dx,
            ));
        }
        if dy != 0 {
            events.push(InputEvent::new_now(
                EventType::RELATIVE.0,
                RelativeAxisCode::REL_Y.0,
                dy,
            ));
        }

        self.device
            .emit(&events)
            .context("Failed to emit relative cursor movement")?;

        Ok(())
    }

    /// Presses a mouse button (down).
    pub fn press_button(&mut self, button: MouseButton) -> Result<()> {
        let key = button.to_keycode();
        let event = InputEvent::new_now(EventType::KEY.0, key.code(), 1);
        self.device
            .emit(&[event])
            .context("Failed to emit mouse button press")?;
        Ok(())
    }

    /// Releases a mouse button (up).
    pub fn release_button(&mut self, button: MouseButton) -> Result<()> {
        let key = button.to_keycode();
        let event = InputEvent::new_now(EventType::KEY.0, key.code(), 0);
        self.device
            .emit(&[event])
            .context("Failed to emit mouse button release")?;
        Ok(())
    }

    /// Performs a full click (down then up) with a brief 20ms hold delay.
    pub fn click(&mut self, button: MouseButton) -> Result<()> {
        self.press_button(button)?;
        sleep(Duration::from_millis(20));
        self.release_button(button)?;
        Ok(())
    }

    /// Emits vertical scroll wheel ticks.
    /// Positive `dy` scrolls up, negative `dy` scrolls down.
    pub fn scroll_vertical(&mut self, dy: i32) -> Result<()> {
        if dy == 0 {
            return Ok(());
        }
        let event = InputEvent::new_now(
            EventType::RELATIVE.0,
            RelativeAxisCode::REL_WHEEL.0,
            dy,
        );
        self.device
            .emit(&[event])
            .context("Failed to emit vertical scroll")?;
        Ok(())
    }

    /// Emits horizontal scroll wheel ticks.
    pub fn scroll_horizontal(&mut self, dx: i32) -> Result<()> {
        if dx == 0 {
            return Ok(());
        }
        let event = InputEvent::new_now(
            EventType::RELATIVE.0,
            RelativeAxisCode::REL_HWHEEL.0,
            dx,
        );
        self.device
            .emit(&[event])
            .context("Failed to emit horizontal scroll")?;
        Ok(())
    }
}
