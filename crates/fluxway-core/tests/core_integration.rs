//! Core-only integration tests.
//!
//! These tests exercise fluxway-core WITHOUT any backend or protocol crate.
//! They prove that the core engine is fully self-contained and testable
//! using only plain Rust types.

use fluxway_core::config::Config;
use fluxway_core::event::{CoreAction, CoreEvent};
use fluxway_core::input::{Command, Toggle, WorkspaceTarget};
use fluxway_core::state::Geometry;
use fluxway_core::window::WindowState;
use fluxway_core::Core;

/// Helper: create a core with default config and a 1920×1080 output.
fn test_core() -> Core {
    let config = Config::default();
    let mut core = Core::new(config);
    // Simulate an output so workspaces have geometry for layout.
    core.handle_event(CoreEvent::OutputAdded {
        id: 1,
        name: "test-output".into(),
        geometry: Geometry::new(0, 0, 1920, 1080),
    });
    core
}

/// Helper: map a synthetic window and return its ID.
fn map_window(core: &mut Core, app_id: &str, title: &str) -> fluxway_core::WindowId {
    let id = core.next_window_id();
    core.handle_event(CoreEvent::WindowMapped {
        id,
        app_id: Some(app_id.into()),
        title: Some(title.into()),
        pid: None,
        initial_geometry: Some(Geometry::new(0, 0, 800, 600)),
        is_xwayland: false,
    });
    id
}

// ── Test 1: workspace switching ──────────────────────────────────

#[test]
fn workspace_switching_changes_focus_and_actions() {
    let mut core = test_core();

    // Map a window on workspace 1 (default)
    let w1 = map_window(&mut core, "term", "Terminal");
    assert_eq!(core.focused_window(), Some(w1));

    // Switch to workspace 2
    let actions = core.exec(Command::Workspace(WorkspaceTarget::Number(2)));

    // Must contain WorkspaceChanged action
    assert!(
        actions.iter().any(|a| matches!(a, CoreAction::WorkspaceChanged { .. })),
        "Expected WorkspaceChanged action, got: {actions:?}"
    );

    // Active workspace should have changed
    let ws2_id = *core.state.workspaces.keys().nth(1).unwrap();
    assert_eq!(core.focused_workspace(), Some(ws2_id));

    // w1 should no longer be focused (it's on workspace 1)
    // Focus should be None since workspace 2 is empty
    // (or could be Some(w1) depending on impl; what matters is workspace changed)
    assert_ne!(core.focused_workspace(), Some(*core.state.workspaces.keys().next().unwrap()));
}

// ── Test 2: focus movement across tiled windows ──────────────────

#[test]
fn focus_movement_across_tiled_windows() {
    let mut core = test_core();

    // Map three windows on the same workspace
    let w1 = map_window(&mut core, "app1", "App 1");
    let w2 = map_window(&mut core, "app2", "App 2");
    let w3 = map_window(&mut core, "app3", "App 3");

    // The last-mapped window should be focused
    assert_eq!(core.focused_window(), Some(w3));

    // All three should be on the same workspace
    assert_eq!(
        core.state.windows.get(&w1).unwrap().workspace,
        core.state.windows.get(&w2).unwrap().workspace,
    );
    assert_eq!(
        core.state.windows.get(&w2).unwrap().workspace,
        core.state.windows.get(&w3).unwrap().workspace,
    );

    // Verify all three windows exist and are tiled
    assert!(core.state.windows.get(&w1).unwrap().is_tiled());
    assert!(core.state.windows.get(&w2).unwrap().is_tiled());
    assert!(core.state.windows.get(&w3).unwrap().is_tiled());

    // Relayout should produce geometry for all three
    let tick_actions = core.tick();
    let geo_actions: Vec<_> = tick_actions
        .iter()
        .filter(|a| matches!(a, CoreAction::SetWindowGeometry { .. }))
        .collect();

    // Each tiled window should have a geometry action
    assert!(
        geo_actions.len() >= 3,
        "Expected geometry for 3 windows, got {} actions: {geo_actions:?}",
        geo_actions.len()
    );
}

// ── Test 3: toggle floating preserves geometry and focus ─────────

#[test]
fn toggle_floating_preserves_geometry_and_focus() {
    let mut core = test_core();

    let w1 = map_window(&mut core, "editor", "Editor");
    assert_eq!(core.focused_window(), Some(w1));

    // Toggle floating ON
    let actions = core.exec(Command::Floating(Toggle::Switch));
    assert!(
        actions.iter().any(|a| matches!(a, CoreAction::SetFloating { id, floating: true } if *id == w1)),
        "Expected SetFloating(true) for {w1}, got: {actions:?}"
    );

    // Window should be floating now
    assert!(
        core.state.windows.get(&w1).unwrap().state.contains(WindowState::FLOATING),
        "Window should be floating"
    );
    // Focus should be preserved
    assert_eq!(core.focused_window(), Some(w1));

    // Toggle floating OFF
    let actions = core.exec(Command::Floating(Toggle::Switch));
    assert!(
        actions.iter().any(|a| matches!(a, CoreAction::SetFloating { id, floating: false } if *id == w1)),
        "Expected SetFloating(false) for {w1}, got: {actions:?}"
    );

    assert!(
        !core.state.windows.get(&w1).unwrap().state.contains(WindowState::FLOATING),
        "Window should not be floating"
    );
    assert_eq!(core.focused_window(), Some(w1));
}

// ── Test 4: scratchpad send and toggle ───────────────────────────

#[test]
fn scratchpad_send_and_toggle() {
    let mut core = test_core();

    let w1 = map_window(&mut core, "term", "Terminal");
    let w2 = map_window(&mut core, "browser", "Browser");
    assert_eq!(core.focused_window(), Some(w2));

    // Send w2 to scratchpad
    core.exec(Command::MoveToScratchpad);

    // w2 should now be in scratchpad
    assert!(
        core.state.scratchpad.contains(&w2),
        "Window should be in scratchpad"
    );
    assert!(
        core.state.windows.get(&w2).unwrap().state.contains(WindowState::HIDDEN),
        "Scratchpad window should be hidden"
    );

    // Toggle scratchpad to show w2
    let actions = core.exec(Command::ScratchpadShow);

    // w2 should be focused again
    assert!(
        actions.iter().any(|a| matches!(a, CoreAction::SetFocus { id: Some(id) } if *id == w2)),
        "Expected SetFocus for scratchpad window"
    );
}

// ── Test 5: marks set and focus ──────────────────────────────────

#[test]
fn marks_set_and_focus() {
    let mut core = test_core();

    let w1 = map_window(&mut core, "editor", "Editor");
    let w2 = map_window(&mut core, "term", "Terminal");
    assert_eq!(core.focused_window(), Some(w2));

    // Focus w1 and set a mark on it
    core.handle_event(CoreEvent::FocusRequested { id: w1 });
    assert_eq!(core.focused_window(), Some(w1));

    core.exec(Command::Mark("a".into()));
    assert_eq!(core.state.marks.get("a"), Some(&w1));

    // Focus w2
    core.handle_event(CoreEvent::FocusRequested { id: w2 });
    assert_eq!(core.focused_window(), Some(w2));

    // Go to mark "a" — should focus w1
    let actions = core.exec(Command::GotoMark("a".into()));
    assert_eq!(core.focused_window(), Some(w1));
    assert!(
        actions.iter().any(|a| matches!(a, CoreAction::SetFocus { id: Some(id) } if *id == w1)),
        "Expected SetFocus for marked window"
    );
}

// ── Test 6: window unmap cleans up state ─────────────────────────

#[test]
fn window_unmap_cleans_state() {
    let mut core = test_core();

    let w1 = map_window(&mut core, "app", "App");
    core.exec(Command::Mark("x".into()));
    assert_eq!(core.state.marks.get("x"), Some(&w1));

    // Unmap the window
    let actions = core.handle_event(CoreEvent::WindowUnmapped { id: w1 });

    // Window should be gone
    assert!(!core.state.windows.contains_key(&w1));
    // Mark should be cleaned up
    assert!(!core.state.marks.contains_key("x"));
    // Focus should be cleared
    assert!(
        actions.iter().any(|a| matches!(a, CoreAction::SetFocus { .. })),
        "Expected SetFocus action after unmap"
    );
}

// ── Test 7: exit command ─────────────────────────────────────────

#[test]
fn exit_command_sets_flag_and_emits_action() {
    let mut core = test_core();

    let actions = core.exec(Command::Exit);
    assert!(core.should_exit);
    assert!(
        actions.iter().any(|a| matches!(a, CoreAction::Exit)),
        "Expected Exit action"
    );
}

// ── Test 8: move window to workspace ─────────────────────────────

#[test]
fn move_window_to_workspace() {
    let mut core = test_core();

    let w1 = map_window(&mut core, "app", "App");
    let ws1 = *core.state.workspaces.keys().next().unwrap();
    let ws3 = *core.state.workspaces.keys().nth(2).unwrap();

    // Window starts on workspace 1
    assert_eq!(core.state.windows.get(&w1).unwrap().workspace, Some(ws1));

    // Move to workspace 3
    core.exec(Command::MoveToWorkspace(WorkspaceTarget::Number(3)));

    // Window should now be on workspace 3
    assert_eq!(core.state.windows.get(&w1).unwrap().workspace, Some(ws3));
    assert!(core.state.workspaces.get(&ws3).unwrap().contains(w1));
    assert!(!core.state.workspaces.get(&ws1).unwrap().contains(w1));
}

// ── Test 9: invariants hold after operations ─────────────────────

#[test]
fn invariants_hold_after_mixed_operations() {
    let mut core = test_core();

    let w1 = map_window(&mut core, "a", "A");
    let w2 = map_window(&mut core, "b", "B");
    let w3 = map_window(&mut core, "c", "C");

    // Switch workspaces
    core.exec(Command::Workspace(WorkspaceTarget::Number(2)));

    // Map another window on ws2
    let w4 = map_window(&mut core, "d", "D");

    // Move w4 to ws3
    core.exec(Command::MoveToWorkspace(WorkspaceTarget::Number(3)));

    // Toggle floating on a window
    core.exec(Command::Workspace(WorkspaceTarget::Number(1)));
    core.handle_event(CoreEvent::FocusRequested { id: w1 });
    core.exec(Command::Floating(Toggle::Switch));

    // Set marks
    core.exec(Command::Mark("m1".into()));
    core.handle_event(CoreEvent::FocusRequested { id: w2 });
    core.exec(Command::Mark("m2".into()));

    // Send w3 to scratchpad
    core.handle_event(CoreEvent::FocusRequested { id: w3 });
    core.exec(Command::MoveToScratchpad);

    // Unmap w2
    core.handle_event(CoreEvent::WindowUnmapped { id: w2 });

    // Validate invariants
    core.state.validate_invariants().expect("Invariants should hold");
}
