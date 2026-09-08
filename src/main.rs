mod cli;
mod config;
mod keyboard;
mod physics;
mod uinput;

use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, CliMouseButton, Commands};
use config::AppConfig;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use uinput::{MouseButton, MouseDevice};

fn main() -> Result<()> {
    let args = Cli::parse();

    // Initialize logger
    let log_level = if args.verbose { "debug" } else { "info" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    // Load configuration
    let config = AppConfig::load_or_default(args.config.as_deref())?;

    match args.command.unwrap_or(Commands::Daemon) {
        Commands::Daemon => {
            let running = Arc::new(AtomicBool::new(true));
            let r = running.clone();

            // Set up clean Ctrl+C signal handler
            ctrlc::set_handler(move || {
                r.store(false, Ordering::SeqCst);
            })
            .context("Error setting Ctrl-C handler")?;

            keyboard::run_event_loop(config, running)?;
        }
        Commands::Move {
            dx,
            dy,
            turbo,
            precision,
        } => {
            let mut mouse = MouseDevice::new()
                .context("Failed to initialize virtual mouse for one-shot movement")?;
            let mut final_dx = dx as f64;
            let mut final_dy = dy as f64;

            if turbo {
                final_dx *= config.physics.turbo_multiplier;
                final_dy *= config.physics.turbo_multiplier;
            }
            if precision {
                final_dx *= config.physics.precision_multiplier;
                final_dy *= config.physics.precision_multiplier;
            }

            mouse.move_relative(final_dx.round() as i32, final_dy.round() as i32)?;
        }
        Commands::Click { button } => {
            let mut mouse = MouseDevice::new()
                .context("Failed to initialize virtual mouse for click action")?;
            let btn = match button {
                CliMouseButton::Left => MouseButton::Left,
                CliMouseButton::Right => MouseButton::Right,
                CliMouseButton::Middle => MouseButton::Middle,
            };
            mouse.click(btn)?;
        }
        Commands::Scroll { dy, dx } => {
            let mut mouse = MouseDevice::new()
                .context("Failed to initialize virtual mouse for scroll action")?;
            if dy != 0 {
                mouse.scroll_vertical(dy)?;
            }
            if dx != 0 {
                mouse.scroll_horizontal(dx)?;
            }
        }
        Commands::InitConfig { force } => {
            init_config_file(force)?;
        }
        Commands::Doctor => {
            run_doctor()?;
        }
    }

    Ok(())
}

fn init_config_file(force: bool) -> Result<()> {
    let config_path = AppConfig::default_config_path()?;
    if config_path.exists() && !force {
        println!(
            "Configuration file already exists at {:?}.\nUse --force to overwrite.",
            config_path
        );
        return Ok(());
    }

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory {:?}", parent))?;
    }

    let default_content = AppConfig::generate_default_toml();
    fs::write(&config_path, default_content)
        .with_context(|| format!("Failed to write configuration to {:?}", config_path))?;

    println!("✅ Created default configuration file at {:?}", config_path);
    Ok(())
}

fn run_doctor() -> Result<()> {
    println!("🔍 Running fckmouse System Diagnostics...\n");

    // 1. Check Display & Session
    let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".to_string());
    let current_desktop =
        std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "unknown".to_string());
    println!("🖥️  Session Environment:");
    println!("   • Desktop / WM : {}", current_desktop);
    println!("   • Session Type : {}", session_type);

    // 2. Check /dev/uinput
    println!("\n🖱️  Virtual Mouse (/dev/uinput):");
    let uinput_path = Path::new("/dev/uinput");
    if uinput_path.exists() {
        match fs::OpenOptions::new().write(true).open(uinput_path) {
            Ok(_) => println!("   ✅ /dev/uinput is WRITABLE. Virtual mouse works out of the box!"),
            Err(e) => {
                println!("   ❌ /dev/uinput permission error: {}", e);
                println!("   💡 Fix: Ensure udev rules grant access to uinput or run: sudo chmod 666 /dev/uinput");
            }
        }
    } else {
        println!("   ❌ /dev/uinput does NOT exist. Load kernel module: sudo modprobe uinput");
    }

    // 3. Check /dev/input permissions & Keyboards
    println!("\n⌨️  Keyboard Devices (/dev/input/):");
    match keyboard::discover_keyboards() {
        Ok(keyboards) => {
            if keyboards.is_empty() {
                println!("   ⚠️  No keyboard devices detected in /dev/input/");
            } else {
                println!(
                    "   ✅ Found {} accessible keyboard device(s):",
                    keyboards.len()
                );
                for kbd in keyboards {
                    println!("      • {:?}", kbd);
                }
            }
        }
        Err(e) => {
            println!("   ❌ {}", e);
        }
    }

    println!("\n✨ Diagnostics complete.\n");
    Ok(())
}
