use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Physics configuration controlling mouse acceleration and responsiveness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsConfig {
    /// Initial speed (pixels per tick) when a direction key is first pressed.
    #[serde(default = "default_base_speed")]
    pub base_speed: f64,

    /// Maximum terminal speed (pixels per tick) under sustained hold.
    #[serde(default = "default_max_speed")]
    pub max_speed: f64,

    /// Rate of acceleration per second while holding a direction key.
    #[serde(default = "default_acceleration")]
    pub acceleration: f64,

    /// Speed multiplier when the Turbo key (Ctrl) is held.
    #[serde(default = "default_turbo_multiplier")]
    pub turbo_multiplier: f64,

    /// Speed multiplier when the Precision key (Shift) is held in modal mode.
    #[serde(default = "default_precision_multiplier")]
    pub precision_multiplier: f64,

    /// Engine loop tick rate in milliseconds (e.g. 10ms = 100 Hz update frequency).
    #[serde(default = "default_tick_rate_ms")]
    pub tick_rate_ms: u64,
}

fn default_base_speed() -> f64 {
    2.5
}
fn default_max_speed() -> f64 {
    28.0
}
fn default_acceleration() -> f64 {
    35.0
}
fn default_turbo_multiplier() -> f64 {
    3.0
}
fn default_precision_multiplier() -> f64 {
    0.3
}
fn default_tick_rate_ms() -> u64 {
    10
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            base_speed: default_base_speed(),
            max_speed: default_max_speed(),
            acceleration: default_acceleration(),
            turbo_multiplier: default_turbo_multiplier(),
            precision_multiplier: default_precision_multiplier(),
            tick_rate_ms: default_tick_rate_ms(),
        }
    }
}

/// Dynamic velocity tracker that calculates pixel deltas on each loop tick.
#[derive(Debug)]
pub struct MovementPhysics {
    config: PhysicsConfig,
    press_start: Option<Instant>,
    accumulated_x: f64,
    accumulated_y: f64,
}

impl MovementPhysics {
    /// Initializes a new physics engine with the given configuration.
    pub fn new(config: PhysicsConfig) -> Self {
        Self {
            config,
            press_start: None,
            accumulated_x: 0.0,
            accumulated_y: 0.0,
        }
    }

    /// Resets the movement duration (called when all direction keys are released).
    pub fn reset(&mut self) {
        self.press_start = None;
        self.accumulated_x = 0.0;
        self.accumulated_y = 0.0;
    }

    /// Computes the discrete pixel delta `(dx, dy)` for the current tick.
    ///
    /// - `dir_x`: -1 for left, 1 for right, 0 for none
    /// - `dir_y`: -1 for up, 1 for down, 0 for none
    /// - `turbo`: true if turbo multiplier should be applied
    /// - `precision`: true if precision dampener should be applied
    pub fn step(
        &mut self,
        dir_x: i32,
        dir_y: i32,
        turbo: bool,
        precision: bool,
    ) -> (i32, i32) {
        if dir_x == 0 && dir_y == 0 {
            self.reset();
            return (0, 0);
        }

        let now = Instant::now();
        let elapsed_secs = match self.press_start {
            Some(start) => now.duration_since(start).as_secs_f64(),
            None => {
                self.press_start = Some(now);
                0.0
            }
        };

        // Linear + quadratic smooth acceleration ramp
        let raw_speed = self.config.base_speed + (self.config.acceleration * elapsed_secs);
        let speed = raw_speed.min(self.config.max_speed);

        // Apply modifiers
        let mut multiplier = 1.0;
        if turbo {
            multiplier *= self.config.turbo_multiplier;
        }
        if precision {
            multiplier *= self.config.precision_multiplier;
        }

        let final_speed = speed * multiplier;

        // Normalize diagonals so diagonal motion isn't 1.414x faster
        let (norm_x, norm_y) = if dir_x != 0 && dir_y != 0 {
            let inv_sqrt2 = 0.7071067811865475;
            (dir_x as f64 * inv_sqrt2, dir_y as f64 * inv_sqrt2)
        } else {
            (dir_x as f64, dir_y as f64)
        };

        // Sub-pixel accumulation to avoid truncation loss
        self.accumulated_x += norm_x * final_speed;
        self.accumulated_y += norm_y * final_speed;

        let dx = self.accumulated_x.trunc() as i32;
        let dy = self.accumulated_y.trunc() as i32;

        self.accumulated_x -= dx as f64;
        self.accumulated_y -= dy as f64;

        (dx, dy)
    }

    /// Accessor for tick interval.
    pub fn tick_rate_ms(&self) -> u64 {
        self.config.tick_rate_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_movement() {
        let mut physics = MovementPhysics::new(PhysicsConfig::default());
        let (dx, dy) = physics.step(0, 0, false, false);
        assert_eq!((dx, dy), (0, 0));
    }

    #[test]
    fn test_initial_nudge_speed() {
        let mut physics = MovementPhysics::new(PhysicsConfig {
            base_speed: 3.0,
            ..Default::default()
        });
        let (dx, dy) = physics.step(1, 0, false, false);
        assert_eq!(dx, 3);
        assert_eq!(dy, 0);
    }

    #[test]
    fn test_turbo_multiplier() {
        let mut physics = MovementPhysics::new(PhysicsConfig {
            base_speed: 2.0,
            turbo_multiplier: 4.0,
            ..Default::default()
        });
        let (dx, _dy) = physics.step(1, 0, true, false);
        assert_eq!(dx, 8);
    }

    #[test]
    fn test_precision_multiplier() {
        let mut physics = MovementPhysics::new(PhysicsConfig {
            base_speed: 10.0,
            precision_multiplier: 0.3,
            ..Default::default()
        });
        let (_dx, dy) = physics.step(0, 1, false, true);
        assert_eq!(dy, 3);
    }
}
