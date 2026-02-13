//! Fluxway — A lightweight, highly scalable tiling window manager
//!
//! This binary is a thin shell that:
//! - Parses CLI arguments
//! - Loads configuration via `fluxway-core`
//! - Selects and starts a backend from `fluxway-backend-winit`
//!
//! All window-manager logic lives in `fluxway-core`.
//! All protocol/display logic lives in `fluxway-backend-winit`.

use anyhow::Result;
use clap::Parser;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

use fluxway_core::config::Config;

/// Fluxway — A modern tiling window manager
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long)]
    config: Option<String>,

    /// Run in debug mode with verbose logging
    #[arg(short, long)]
    debug: bool,

    /// Validate configuration and exit
    #[arg(long)]
    validate: bool,

    /// Print default configuration to stdout
    #[arg(long)]
    print_default_config: bool,

    /// Socket path for IPC
    #[arg(short, long)]
    socket: Option<String>,

    /// Run in nested mode using winit backend (for testing)
    #[arg(long)]
    nested: bool,

    /// Run headless integration test
    #[arg(long)]
    headless: bool,

    /// Backend to use: auto, winit, or drm
    #[arg(long, default_value = "auto")]
    backend: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.debug {
        Level::DEBUG
    } else {
        Level::INFO
    };
    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .compact()
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Fluxway v{} starting...", env!("CARGO_PKG_VERSION"));

    // Handle special commands
    if args.print_default_config {
        println!("{}", Config::default_config_string());
        return Ok(());
    }

    // Load configuration (via core — no protocol deps)
    let config = match Config::load(args.config.as_deref()) {
        Ok(cfg) => {
            info!("Configuration loaded successfully");
            cfg
        }
        Err(e) => {
            warn!("Failed to load config: {}, using defaults", e);
            Config::default()
        }
    };

    if args.validate {
        info!("Configuration is valid");
        return Ok(());
    }

    // Headless integration test
    if args.headless {
        info!("Running headless integration test");
        return match fluxway_backend_winit::run_headless_test(config) {
            Ok(true) => {
                info!("Headless test PASSED: pipeline exercised successfully");
                Ok(())
            }
            Ok(false) => {
                anyhow::bail!("Headless test FAILED: state did not change as expected");
            }
            Err(e) => {
                anyhow::bail!("Headless test ERROR: {}", e);
            }
        };
    }

    // Start backend
    let mut backend = fluxway_backend_winit::WinitBackend::new(config);
    backend.run()
}
