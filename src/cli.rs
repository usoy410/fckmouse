use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Command line arguments for fckmouse.
#[derive(Parser, Debug)]
#[command(
    name = "fckmouse",
    author = "usoy",
    version = "0.2.1",
    about = "Zero-latency, keyboard-driven mouse navigation daemon for Linux tiling window managers."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Path to custom config file (default: ~/.config/fckmouse/config.toml)
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,

    /// Verbose logging output
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the keyboard event listener daemon (default if no subcommand provided)
    Daemon,

    /// Move the mouse cursor by relative pixel offsets (useful for window manager keybinds)
    Move {
        /// Horizontal offset in pixels (positive = right, negative = left)
        #[arg(long, default_value_t = 0)]
        dx: i32,

        /// Vertical offset in pixels (positive = down, negative = up)
        #[arg(long, default_value_t = 0)]
        dy: i32,

        /// Apply turbo 3x multiplier
        #[arg(short, long)]
        turbo: bool,

        /// Apply precision 0.3x multiplier
        #[arg(short, long)]
        precision: bool,
    },

    /// Click a mouse button
    Click {
        /// Button to click (left, right, middle)
        #[arg(short, long, default_value = "left")]
        button: CliMouseButton,
    },

    /// Scroll the mouse wheel
    Scroll {
        /// Vertical scroll amount (positive = up, negative = down)
        #[arg(long, default_value_t = 1)]
        dy: i32,

        /// Horizontal scroll amount (positive = right, negative = left)
        #[arg(long, default_value_t = 0)]
        dx: i32,
    },

    /// Generate default configuration file at ~/.config/fckmouse/config.toml
    InitConfig {
        /// Overwrite configuration if file already exists
        #[arg(short, long)]
        force: bool,
    },

    /// Run system diagnostics to verify permissions and hardware detection
    Doctor,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliMouseButton {
    Left,
    Right,
    Middle,
}
