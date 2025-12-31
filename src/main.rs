//! Fluxway - A lightweight, highly scalable tiling window manager
//!
//! Combines the best features of i3, Sway, and Fluxbox into a modern
//! Wayland-native compositor with excellent performance and usability.
//!
//! # Features
//! - Tree-based tiling (i3-style) with split containers
//! - Tabbed and stacked layouts (Fluxbox-inspired)
//! - Full floating window support with mouse interactions
//! - Workspace management with named workspaces
//! - i3-compatible IPC protocol for tooling integration
//! - TOML configuration with runtime reload
//! - GPU-accelerated rendering via Smithay
//! - XWayland support (optional)
//! - Scratchpad for hidden floating windows
//! - Window marks (vim-style named references)
//! - Comprehensive keybinding system with modes

use anyhow::Result;
use clap::Parser;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

mod compositor;
mod config;
mod input;
mod ipc;
mod layout;
mod render;
mod state;
mod window;
mod workspace;
mod x11_compat;

use config::Config;
use x11_compat::{detect_session_type, SessionType};

/// Fluxway - A modern tiling window manager
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

    // Load configuration
    let config = match Config::load(args.config.as_deref()) {
        Ok(cfg) => {
            info!("Configuration loaded successfully");
            cfg
        },
        Err(e) => {
            warn!("Failed to load config: {}, using defaults", e);
            Config::default()
        },
    };

    if args.validate {
        info!("Configuration is valid");
        return Ok(());
    }

    // Detect session type
    let session_type = detect_session_type();
    info!("Detected session type: {}", session_type);

    // Determine backend
    let use_winit = args.nested || args.backend == "winit" || {
        // Auto-detect: use winit if already in a graphical session
        matches!(session_type, SessionType::Wayland | SessionType::X11)
    };

    // Run the compositor
    match args.backend.as_str() {
        "x11" => {
            #[cfg(feature = "x11")]
            {
                info!("Using native X11 backend");
                // X11 backend would be started here
                warn!("Native X11 backend not yet fully implemented");
                compositor::run_winit(config)
            }
            #[cfg(not(feature = "x11"))]
            {
                anyhow::bail!("X11 feature not compiled in. Rebuild with --features x11");
            }
        },
        "drm" => {
            // DRM backend for production use
            warn!("DRM backend not yet implemented, falling back to winit");
            compositor::run_winit(config)
        },
        _ if use_winit => {
            info!("Using winit backend (nested/development mode)");
            compositor::run_winit(config)
        },
        _ => {
            warn!("No suitable backend detected, using winit");
            compositor::run_winit(config)
        },
    }
}
