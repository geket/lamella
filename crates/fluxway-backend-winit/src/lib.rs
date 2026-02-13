//! Fluxway Winit Backend — Adapter between Smithay/Winit and fluxway-core.
//!
//! This crate:
//! - Owns all Smithay/Winit/protocol types.
//! - Maintains a mapping from protocol surface handles to `WindowId`.
//! - Translates protocol events → `CoreEvent`, feeds them to `Core`.
//! - Applies returned `CoreAction`s back to the protocol world.
//!
//! **No Smithay/Winit types leak into `fluxway-core`.**

use std::collections::HashMap;
use std::process::Command as ProcessCommand;

use anyhow::Result;
use tracing::{error, info, warn};

use fluxway_core::config::Config;
use fluxway_core::event::{CoreAction, CoreEvent};
use fluxway_core::{Command, Core, Geometry, WindowId};

/// Protocol-side handle for a window surface.
///
/// In a real Smithay integration this would wrap a `WlSurface` or
/// `X11Surface`. For now it's a placeholder demonstrating the mapping
/// pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SurfaceHandle(pub u64);

/// The backend adapter.
///
/// Owns the event loop, protocol connections, and the core engine.
pub struct WinitBackend {
    /// The protocol-agnostic core.
    pub core: Core,
    /// Protocol surface → core WindowId mapping.
    surface_map: HashMap<SurfaceHandle, WindowId>,
    /// Reverse mapping for applying actions.
    window_to_surface: HashMap<WindowId, SurfaceHandle>,
    /// Next surface handle counter (placeholder).
    next_surface: u64,
}

impl WinitBackend {
    pub fn new(config: Config) -> Self {
        Self {
            core: Core::new(config),
            surface_map: HashMap::new(),
            window_to_surface: HashMap::new(),
            next_surface: 1,
        }
    }

    /// Register a new surface and return the core `WindowId`.
    pub fn register_surface(&mut self) -> (SurfaceHandle, WindowId) {
        let handle = SurfaceHandle(self.next_surface);
        self.next_surface += 1;
        let wid = self.core.next_window_id();
        self.surface_map.insert(handle, wid);
        self.window_to_surface.insert(wid, handle);
        (handle, wid)
    }

    /// Remove a surface mapping.
    pub fn unregister_surface(&mut self, handle: SurfaceHandle) -> Option<WindowId> {
        let wid = self.surface_map.remove(&handle)?;
        self.window_to_surface.remove(&wid);
        Some(wid)
    }

    /// Translate a protocol event into a `CoreEvent` and process it.
    pub fn handle_protocol_event(&mut self, event: CoreEvent) -> Vec<CoreAction> {
        self.core.handle_event(event)
    }

    /// Apply a list of core actions to the protocol world.
    pub fn apply_actions(&mut self, actions: &[CoreAction]) {
        for action in actions {
            match action {
                CoreAction::SetWindowGeometry { id, x, y, w, h } => {
                    if let Some(_surface) = self.window_to_surface.get(id) {
                        // In real backend: configure the Wayland/X11 surface
                        tracing::trace!(
                            "Configure surface for {id}: {x},{y} {w}x{h}"
                        );
                    }
                }
                CoreAction::SetFocus { id } => {
                    if let Some(wid) = id {
                        if let Some(_surface) = self.window_to_surface.get(wid) {
                            // In real backend: set keyboard focus
                            tracing::trace!("Focus surface for {wid}");
                        }
                    }
                }
                CoreAction::RequestClose { id } => {
                    if let Some(_surface) = self.window_to_surface.get(id) {
                        // In real backend: send close request to client
                        tracing::trace!("Close request for {id}");
                    }
                }
                CoreAction::SpawnProcess { command } => {
                    info!("Spawning: {}", command);
                    if let Err(e) = ProcessCommand::new("sh").arg("-c").arg(command).spawn() {
                        error!("Failed to spawn '{}': {}", command, e);
                    }
                }
                CoreAction::ReloadConfig => {
                    info!("Reloading configuration");
                    match Config::load(None) {
                        Ok(config) => self.core.reload_config(config),
                        Err(e) => error!("Failed to reload config: {}", e),
                    }
                }
                CoreAction::Exit => {
                    info!("Exit requested by core");
                }
                CoreAction::SetFloating { id, floating } => {
                    tracing::trace!("Floating changed for {id}: {floating}");
                }
                CoreAction::WorkspaceChanged { active } => {
                    tracing::trace!("Workspace changed: {active:?}");
                }
            }
        }
    }

    /// Run the backend event loop (placeholder — real impl would use calloop).
    pub fn run(&mut self) -> Result<()> {
        info!("Starting Fluxway (winit backend)");
        info!(
            "  - Workspaces: {}",
            self.core.state.workspaces.len()
        );

        // In headless/test mode, just tick a few times
        info!("Running in headless mode (no display backend)");
        for _ in 0..10 {
            let actions = self.core.tick();
            self.apply_actions(&actions);
            if self.core.should_exit {
                break;
            }
        }

        info!("Fluxway shutdown complete");
        Ok(())
    }
}

/// Run headless integration test through the backend adapter.
pub fn run_headless_test(config: Config) -> Result<bool> {
    let mut backend = WinitBackend::new(config);

    assert!(
        !backend.core.state.workspaces.is_empty(),
        "No workspaces created"
    );
    let initial_workspace = backend.core.focused_workspace();

    // Simulate workspace switch
    let actions = backend
        .core
        .exec(Command::Workspace(fluxway_core::input::WorkspaceTarget::Number(2)));
    backend.apply_actions(&actions);
    let actions = backend.core.tick();
    backend.apply_actions(&actions);

    let after_switch = backend.core.focused_workspace();
    let switched = initial_workspace != after_switch;

    // Simulate exit
    let actions = backend.core.exec(Command::Exit);
    backend.apply_actions(&actions);
    assert!(backend.core.should_exit, "Exit command did not set should_exit");

    Ok(switched)
}
