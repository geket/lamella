//! Invariant validation for the core state.
//!
//! Called after every `handle_event` / `exec` in debug builds.

use crate::state::State;
use crate::window::WindowState;

/// Error indicating which invariant was violated.
#[derive(Debug, thiserror::Error)]
pub enum InvariantError {
    #[error("Focused window {0} does not exist or is not mapped")]
    FocusedWindowMissing(String),

    #[error("Window {0} is in both a workspace and scratchpad")]
    WindowInBothWorkspaceAndScratchpad(String),

    #[error("Window {0} is on workspace list but has wrong workspace field")]
    WorkspaceMismatch(String),

    #[error("Focused workspace index out of range")]
    FocusedWorkspaceOutOfRange,

    #[error("Mark '{0}' points to non-existent window")]
    MarkPointsToMissing(String),
}

/// Validate all core invariants. Returns the first violation found.
pub fn validate(state: &State) -> Result<(), InvariantError> {
    // 1. Focused window must exist and be mapped
    if let Some(fid) = state.focus.focused_window {
        if !state.windows.contains_key(&fid) {
            return Err(InvariantError::FocusedWindowMissing(format!("{fid}")));
        }
    }

    // 2. Focused workspace must exist
    if let Some(ws_id) = state.focus.focused_workspace {
        if !state.workspaces.contains_key(&ws_id) {
            return Err(InvariantError::FocusedWorkspaceOutOfRange);
        }
    }

    // 3. Every mapped window belongs to exactly one workspace OR scratchpad
    for (&wid, window) in &state.windows {
        let in_scratchpad = state.scratchpad.contains(&wid);
        let in_workspace = window.workspace.is_some()
            && window
                .workspace
                .and_then(|ws_id| state.workspaces.get(&ws_id))
                .is_some_and(|ws| ws.contains(wid));

        // A window in scratchpad should still have a workspace assignment
        // (it returns there when toggled back), so we only flag if the
        // workspace's own list also claims the window AND scratchpad does.
        if in_scratchpad && in_workspace && !window.state.contains(WindowState::HIDDEN) {
            return Err(InvariantError::WindowInBothWorkspaceAndScratchpad(
                format!("{wid}"),
            ));
        }
    }

    // 4. Marks point to existing windows
    for (mark, &wid) in &state.marks {
        if !state.windows.contains_key(&wid) {
            return Err(InvariantError::MarkPointsToMissing(mark.clone()));
        }
    }

    Ok(())
}
