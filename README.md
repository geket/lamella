# Fluxway

<div align="center">

![Fluxway Logo](https://img.shields.io/badge/Fluxway-Window%20Manager-blue?style=for-the-badge)

**A Bridge Between Worlds**

[![CI](https://github.com/geket/lamella/actions/workflows/ci.yml/badge.svg)](https://github.com/geket/lamella/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Wayland](https://img.shields.io/badge/Wayland-ready-green.svg)](https://wayland.freedesktop.org/)
[![X11](https://img.shields.io/badge/X11-compatible-blue.svg)](https://www.x.org/)

*Fluxway is a research-oriented tiling window manager designed to bridge the gap between X11 and Wayland ecosystems while serving as a reference implementation for window manager developers.*

[Features](#features) • [Philosophy](#philosophy) • [Installation](#installation) • [Configuration](#configuration) • [Architecture](#architecture) • [Roadmap](#roadmap)

</div>

---

## Philosophy

> **Fluxway does not seek to replace your window manager.**

The Linux desktop ecosystem is experiencing a significant transition. Major distributions are rapidly migrating from X11 to Wayland, often leaving users and developers caught between two worlds. Legacy applications break, muscle memory conflicts with new paradigms, and the rich ecosystem of X11 tools faces an uncertain future.

**Fluxway exists to address three critical needs:**

### 1. Bridge the Compatibility Gap

As distributions like Ubuntu, Fedora, and others make Wayland their default, millions of users find themselves navigating compatibility issues. Fluxway provides:

- **XWayland integration** for running X11 applications seamlessly within Wayland
- **Native X11 mode** for systems not yet ready for full Wayland adoption  
- **Unified configuration** that works across both display protocols
- **i3-compatible IPC** allowing existing tools to work regardless of backend

### 2. Assist Window Manager Development

Fluxway serves as a **living reference implementation** for window manager developers:

- Clean, well-documented Rust codebase demonstrating modern WM architecture
- Modular design allowing developers to study or extract individual components
- Comprehensive test suite showing how to validate WM behavior
- CI/CD pipeline with WM-specific linting and safety checks

Whether you're building the next great compositor or improving an existing one, Fluxway's codebase is designed to be studied, forked, and learned from.

### 3. Preserve What Works

The best ideas from classic window managers shouldn't be lost in transition:

- **i3's** elegant tree-based tiling and powerful IPC
- **Fluxbox's** intuitive tabbed containers and lightweight philosophy  
- **Sway's** Wayland-native approach and modern architecture

Fluxway synthesizes these proven concepts into a unified implementation that respects the past while embracing the future.

---

## Features

### Current Implementation ✅

#### Core Window Management
- [x] **Tree-based tiling layout** — i3-style container hierarchy with splits
- [x] **Tabbed containers** — Fluxbox-inspired window grouping
- [x] **Stacked layout mode** — Overlapping windows with easy switching
- [x] **Floating window support** — Full floating mode with mouse interactions
- [x] **Focus-follows-mouse** — Optional automatic focus on hover
- [x] **Window marks** — Vim-style named references for quick navigation
- [x] **Scratchpad** — Hidden floating windows accessible via keybinding

#### Workspace Management
- [x] **10 default workspaces** — Numbered 1-10 with custom naming
- [x] **Workspace back-and-forth** — Quick toggle to previous workspace
- [x] **Per-output workspaces** — Multi-monitor aware workspace assignment
- [x] **Focus history tracking** — Intelligent focus restoration
- [x] **Urgent window handling** — Visual indicators for attention requests

#### Configuration System
- [x] **TOML configuration** — Human-readable, version-control friendly
- [x] **i3-compatible syntax** — Familiar commands for i3/Sway users
- [x] **Runtime reload** — Apply changes without restart
- [x] **Window rules** — Match windows by class, title, or criteria
- [x] **Startup commands** — Launch applications on WM start
- [x] **Mode system** — Binding modes like i3's resize mode

#### Input Handling
- [x] **Comprehensive keybindings** — Full keyboard control
- [x] **Mouse bindings** — Button + modifier combinations
- [x] **Mod+drag operations** — Move and resize with modifier keys
- [x] **XKB keyboard configuration** — Layout, variant, and options
- [x] **Pointer configuration** — Speed, acceleration, natural scroll

#### Display Protocol Support
- [x] **Wayland compositor** — Native Wayland via Smithay
- [x] **XWayland bridge** — Run X11 applications in Wayland
- [x] **X11 compatibility module** — Foundation for native X11 mode
- [x] **Winit backend** — Nested mode for development/testing

#### IPC & Integration
- [x] **i3-compatible IPC** — Unix socket with JSON protocol
- [x] **Command execution** — Run arbitrary commands
- [x] **Event subscription** — Monitor WM state changes
- [x] **External tool support** — Works with i3status, polybar, etc.

#### Visual Features
- [x] **Configurable borders** — Width, color, and style per state
- [x] **Gap support** — Inner and outer gaps with per-edge control
- [x] **Smart borders/gaps** — Hide when single window
- [x] **Animation system** — Smooth transitions with easing curves
- [x] **Color theming** — Full color customization

---

### Planned Features 🚧

#### Short-term (v0.2.0)

| Feature | Description | Status |
|---------|-------------|--------|
| Full Wayland rendering | Complete surface rendering pipeline | In Progress |
| DRM backend | Direct hardware rendering for production | Planned |
| Bar rendering | Built-in status bar or i3bar support | Planned |
| Wallpaper support | Background image/color rendering | Planned |
| Shadow rendering | Window shadows with customization | Planned |

#### Medium-term (v0.3.0)

| Feature | Description | Status |
|---------|-------------|--------|
| Native X11 mode | Full X11 WM without Wayland | Planned |
| Session management | Proper session save/restore | Planned |
| Layer shell | Support for panels, overlays, backgrounds | Planned |
| Screen recording | wlr-screencopy protocol | Planned |
| Gamma control | Night light and color temperature | Planned |

#### Long-term (v1.0.0)

| Feature | Description | Status |
|---------|-------------|--------|
| Plugin system | Extend functionality via plugins | Research |
| Scripting API | Lua or Rhai scripting support | Research |
| Remote IPC | Network-accessible IPC for remote control | Research |
| Accessibility | Screen reader and a11y support | Research |
| Touchscreen | Touch and gesture support | Research |

---

## The Wayland Transition

### The Problem

The shift from X11 to Wayland is necessary for the future of Linux desktop, but it's happening faster than the ecosystem can adapt:

```
2023-2025: The Great Migration
├── Ubuntu 24.04 → Wayland default
├── Fedora 40+ → Wayland only for GNOME
├── Debian 13 → Wayland default planned
├── Many others following suit...
│
└── Meanwhile:
    ├── Legacy applications break
    ├── Remote desktop tools struggle
    ├── Screen sharing requires workarounds
    ├── Accessibility tools need rewrites
    └── 20 years of X11 tooling becomes obsolete
```

### Fluxway's Approach

Rather than forcing users to choose sides, Fluxway embraces both worlds:

```
┌─────────────────────────────────────────────────────────────┐
│                      User Applications                       │
├─────────────────────────────────────────────────────────────┤
│                    Fluxway Core (Rust)                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │   Layout    │  │   Window    │  │    Input    │         │
│  │   Engine    │  │    State    │  │   Handler   │         │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘         │
│         └────────────────┼────────────────┘                 │
│                          │                                   │
│  ┌───────────────────────┴───────────────────────┐         │
│  │          Display Server Abstraction            │         │
│  └───────────────────────┬───────────────────────┘         │
│         ┌────────────────┼────────────────┐                 │
│         ▼                ▼                ▼                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │   Wayland   │  │  XWayland   │  │  Native X11 │         │
│  │   Backend   │  │   Bridge    │  │   Backend   │         │
│  │  (Smithay)  │  │             │  │  (x11rb)    │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
└─────────────────────────────────────────────────────────────┘
```

**This means:**

- Run Wayland-native when your system supports it
- Fall back to X11 when needed (older hardware, specific requirements)
- Mix X11 and Wayland applications seamlessly via XWayland
- Use the same configuration, keybindings, and muscle memory everywhere

---

## Installation

### Prerequisites

**System Dependencies (Ubuntu/Debian):**
```bash
sudo apt install -y \
    libudev-dev \
    libwayland-dev \
    libxkbcommon-dev \
    libinput-dev \
    libdrm-dev \
    libgbm-dev \
    libegl-dev \
    libgles2-mesa-dev \
    libseat-dev \
    libx11-dev \
    libxcb1-dev \
    libxcb-composite0-dev \
    libxcb-randr0-dev
```

**System Dependencies (Fedora):**
```bash
sudo dnf install -y \
    systemd-devel \
    wayland-devel \
    libxkbcommon-devel \
    libinput-devel \
    mesa-libdrm-devel \
    mesa-libgbm-devel \
    mesa-libEGL-devel \
    mesa-libGLES-devel \
    libseat-devel \
    libX11-devel \
    libxcb-devel
```

**System Dependencies (Arch):**
```bash
sudo pacman -S \
    wayland \
    libxkbcommon \
    libinput \
    libdrm \
    mesa \
    seatd \
    libx11 \
    libxcb
```

### Building from Source

```bash
# Clone the repository
git clone https://github.com/geket/lamella.git
cd fluxway

# Build with default features (Wayland + XWayland)
cargo build --release

# Or build with specific features
cargo build --release --features "wayland,xwayland"  # Default
cargo build --release --features "x11"                # X11 only
cargo build --release --features "full"               # Everything

# Install to ~/.cargo/bin
cargo install --path .
```

### Running

```bash
# Generate default configuration
fluxway --print-default-config > ~/.config/fluxway/config.toml

# Run in nested mode (for testing inside another WM)
fluxway --nested

# Run with specific backend
fluxway --backend winit    # Nested/testing mode
fluxway --backend drm      # Production (TTY)
fluxway --backend x11      # Native X11 mode

# Validate configuration
fluxway --validate

# Enable debug logging
fluxway --debug
```

---

## Configuration

Fluxway uses TOML for configuration, located at:
- `$XDG_CONFIG_HOME/fluxway/config.toml`
- `~/.config/fluxway/config.toml`
- `~/.fluxway/config.toml`
- `/etc/fluxway/config.toml`

### Example Configuration

```toml
# General settings
[general]
focus_follows_mouse = "yes"
mouse_warping = "output"
workspace_back_and_forth = true
floating_modifier = "Mod4"      # Super/Windows key
default_layout = "split"

# Gaps between windows
[gaps]
inner = 8
outer = 4

[gaps.per_edge]
top = 0      # Space for external bar
bottom = 0
left = 0
right = 0

# Border styling
[border]
width = 2

[colors.focused]
border = "#5294e2"
background = "#383c4a"
text = "#d3dae3"
indicator = "#5294e2"
child_border = "#5294e2"

[colors.unfocused]
border = "#2d2d2d"
background = "#2d2d2d"
text = "#888888"
child_border = "#2d2d2d"

# Font for title bars (when enabled)
[font]
family = "JetBrains Mono"
size = 10.0

# Input configuration
[input]
repeat_delay = 300
repeat_rate = 50
xkb_layout = "us"
natural_scroll = false
tap = true
pointer_speed = 0.0

# Keybindings
[[bindings]]
keys = "Mod4+Return"
command = "exec alacritty"

[[bindings]]
keys = "Mod4+d"
command = "exec wofi --show drun"

[[bindings]]
keys = "Mod4+Shift+q"
command = "kill"

[[bindings]]
keys = "Mod4+h"
command = "focus left"

[[bindings]]
keys = "Mod4+j"
command = "focus down"

[[bindings]]
keys = "Mod4+k"
command = "focus up"

[[bindings]]
keys = "Mod4+l"
command = "focus right"

[[bindings]]
keys = "Mod4+Shift+h"
command = "move left"

[[bindings]]
keys = "Mod4+Shift+j"
command = "move down"

[[bindings]]
keys = "Mod4+Shift+k"
command = "move up"

[[bindings]]
keys = "Mod4+Shift+l"
command = "move right"

[[bindings]]
keys = "Mod4+v"
command = "split vertical"

[[bindings]]
keys = "Mod4+b"
command = "split horizontal"

[[bindings]]
keys = "Mod4+f"
command = "fullscreen toggle"

[[bindings]]
keys = "Mod4+Shift+space"
command = "floating toggle"

[[bindings]]
keys = "Mod4+w"
command = "layout tabbed"

[[bindings]]
keys = "Mod4+e"
command = "layout toggle split"

[[bindings]]
keys = "Mod4+1"
command = "workspace 1"

[[bindings]]
keys = "Mod4+Shift+1"
command = "move container to workspace 1"

# ... workspaces 2-10 follow same pattern

[[bindings]]
keys = "Mod4+minus"
command = "scratchpad show"

[[bindings]]
keys = "Mod4+Shift+minus"
command = "move scratchpad"

[[bindings]]
keys = "Mod4+Shift+c"
command = "reload"

[[bindings]]
keys = "Mod4+Shift+e"
command = "exit"

# Window rules
[[rules]]
criteria = { app_id = "firefox" }
commands = ["move container to workspace 2"]

[[rules]]
criteria = { class = "Spotify" }
commands = ["move container to workspace 10"]

[[rules]]
criteria = { window_type = "dialog" }
commands = ["floating enable"]

# Startup commands
[[startup]]
command = "waybar"

[[startup]]
command = "dunst"

[[startup]]
command = "nm-applet --indicator"
```

### i3 Users: Migration Guide

Most i3 configuration concepts translate directly:

| i3 Config | Fluxway Config |
|-----------|----------------|
| `set $mod Mod4` | `floating_modifier = "Mod4"` |
| `bindsym $mod+Return exec alacritty` | `[[bindings]]` with `keys = "Mod4+Return"` |
| `gaps inner 10` | `[gaps] inner = 10` |
| `for_window [class="Firefox"]` | `[[rules]] criteria = { class = "Firefox" }` |
| `exec_always polybar` | `[[startup]] command = "polybar"` |

---

## Architecture

Fluxway follows a modular architecture designed for clarity and extensibility:

```
src/
├── main.rs           # Entry point, CLI parsing, backend selection
├── compositor.rs     # Smithay integration, event loop, input handling
├── state.rs          # Central state management, focus tracking
├── layout.rs         # Tree-based tiling engine, container management
├── window.rs         # Window properties, state flags, criteria matching
├── workspace.rs      # Virtual desktop management, focus history
├── config.rs         # TOML configuration parsing, defaults
├── input.rs          # Keybindings, mouse bindings, command parsing
├── ipc.rs            # i3-compatible IPC protocol implementation
├── render.rs         # Animation system, border rendering, damage tracking
└── x11_compat.rs     # X11/XWayland compatibility layer
```

### Key Design Decisions

**Why Rust?**
- Memory safety without garbage collection (critical for WMs)
- Zero-cost abstractions for performance
- Excellent concurrency primitives
- Strong type system catches bugs at compile time
- Growing ecosystem for Wayland development (Smithay)

**Why Smithay?**
- Pure Rust Wayland compositor library
- No C dependencies for core functionality
- Actively maintained with good documentation
- Modular design matches our philosophy

**Why i3-compatible IPC?**
- Massive ecosystem of existing tools
- Users don't need to learn new protocols
- Eases migration from i3/Sway
- Battle-tested protocol design

---

## For Window Manager Developers

Fluxway is designed to be studied and learned from. Here's how different components might help your project:

### Layout Engine (`layout.rs`)
- Tree-based container hierarchy
- Recursive layout calculation
- Direction-aware focus/move operations
- Ratio-based resizing with minimum size enforcement

### State Management (`state.rs`)
- Centralized state with clear ownership
- Focus history for intelligent focus restoration
- Scratchpad implementation
- Window marks (vim-style named references)

### Input Handling (`input.rs`)
- Modifier parsing (`Mod4+Shift+Return`)
- Command parsing (i3-compatible syntax)
- Binding modes for context-sensitive keys

### IPC Protocol (`ipc.rs`)
- i3-compatible message format
- Event subscription system
- JSON serialization for responses

### CI/CD Pipeline (`.github/workflows/`)
- WM-specific Clippy lints
- Anti-pattern detection (blocking in event handlers)
- Feature matrix testing (Wayland/X11/XWayland)
- Security auditing (cargo-audit, cargo-deny)

---

## Compatibility Matrix

| Feature | Wayland | XWayland | X11 Native |
|---------|---------|----------|------------|
| Tiling | ✅ | ✅ | 🚧 |
| Floating | ✅ | ✅ | 🚧 |
| Keybindings | ✅ | ✅ | 🚧 |
| Mouse bindings | ✅ | ✅ | 🚧 |
| IPC | ✅ | ✅ | 🚧 |
| Multi-monitor | ✅ | ✅ | 🚧 |
| HiDPI | ✅ | ⚠️ | 🚧 |
| Screen sharing | 🚧 | ❌ | N/A |
| Legacy X11 apps | via XWayland | ✅ | ✅ |

Legend: ✅ Supported | ⚠️ Partial | 🚧 In Development | ❌ Not Available

---

## Roadmap

### Phase 1: Foundation (Current)
- [x] Core tiling engine
- [x] Configuration system
- [x] Input handling
- [x] IPC protocol
- [ ] Complete Wayland rendering
- [ ] DRM backend for production

### Phase 2: Feature Parity
- [ ] Full X11 native backend
- [ ] Bar support (built-in or external)
- [ ] Session management
- [ ] Screen recording/sharing

### Phase 3: Innovation
- [ ] Plugin system
- [ ] Scripting API
- [ ] Advanced animations
- [ ] Accessibility features

### Phase 4: Ecosystem
- [ ] Distribution packages
- [ ] Comprehensive documentation
- [ ] Migration tools from other WMs
- [ ] Community themes and configs

---

## Contributing

Fluxway welcomes contributions! Whether you're fixing bugs, adding features, or improving documentation, your help is appreciated.

### Development Setup

```bash
# Clone and build
git clone https://github.com/geket/lamella.git
cd fluxway
cargo build

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run -- --nested

# Check formatting
cargo fmt --check

# Run Clippy
cargo clippy --all-features
```

### Code Style

- Follow Rust standard formatting (`cargo fmt`)
- All public items should have documentation
- Prefer `?` over `.unwrap()` in non-test code
- Add tests for new functionality
- Keep commits focused and well-described

### Areas Needing Help

- **Testing**: Real-world testing on different hardware/distros
- **Documentation**: Improving guides and examples
- **X11 Backend**: Implementing native X11 support
- **Accessibility**: Screen reader and a11y support
- **Packaging**: Distribution-specific packages

---

## FAQ

**Q: Why another window manager?**

A: Fluxway isn't trying to compete with i3, Sway, or others. It's a bridge and reference implementation to help the ecosystem navigate the X11→Wayland transition.

**Q: Should I use this as my daily driver?**

A: Not yet. Fluxway is still in early development. It's best used for testing, learning, or contributing to WM development.

**Q: Will Fluxway support Nvidia?**

A: Wayland+Nvidia support depends on Smithay and driver improvements. We'll support it as the ecosystem matures.

**Q: Can I use my i3 config?**

A: Not directly, but the configuration concepts are similar. A migration tool is planned.

**Q: How does this compare to Sway?**

A: Sway is a production-ready i3 replacement for Wayland. Fluxway is a research/bridge project that also supports X11 natively.

---

## Acknowledgments

Fluxway builds on the shoulders of giants:

- **[i3](https://i3wm.org/)** — The tiling paradigm that inspired a generation
- **[Sway](https://swaywm.org/)** — Proving Wayland can match X11 functionality  
- **[Fluxbox](http://fluxbox.org/)** — Elegant simplicity and tabbed windows
- **[Smithay](https://smithay.github.io/)** — Making Wayland accessible to Rust
- **[wlroots](https://gitlab.freedesktop.org/wlroots/wlroots)** — Pioneering modular compositor libraries

---

## License

Fluxway is licensed under the MIT License. See [LICENSE](LICENSE) for details.

---

<div align="center">

**Fluxway** — *Bridging the gap, one window at a time.*

[Report Bug](https://github.com/geket/lamella/issues) • [Request Feature](https://github.com/geket/lamella/issues) • [Discussions](https://github.com/geket/lamella/discussions)

</div>
