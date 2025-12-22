# Contributing to Fluxway

Thank you for your interest in contributing to Fluxway! This document provides guidelines and information for contributors.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Coding Standards](#coding-standards)
- [Testing](#testing)
- [Pull Request Process](#pull-request-process)
- [Areas Needing Help](#areas-needing-help)

## Code of Conduct

This project follows the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). Please be respectful and constructive in all interactions.

## Getting Started

### Prerequisites

1. **Rust toolchain** (1.70+)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup component add rustfmt clippy
   ```

2. **System dependencies** (Ubuntu/Debian)
   ```bash
   sudo apt install -y \
       libudev-dev libwayland-dev libxkbcommon-dev libinput-dev \
       libdrm-dev libgbm-dev libegl-dev libgles2-mesa-dev libseat-dev \
       libx11-dev libxcb1-dev
   ```

3. **Fork and clone**
   ```bash
   git clone https://github.com/YOUR_USERNAME/fluxway.git
   cd fluxway
   ```

### Building

```bash
# Debug build (faster compilation)
cargo build

# Release build (optimized)
cargo build --release

# Build with all features
cargo build --all-features

# Build specific features
cargo build --no-default-features --features "wayland"
cargo build --no-default-features --features "x11"
```

### Running

```bash
# Run in nested mode (inside existing compositor/X11)
cargo run -- --nested

# Run with debug logging
RUST_LOG=debug cargo run -- --nested

# Validate configuration
cargo run -- --validate
```

## Development Workflow

### Branching Strategy

- `main` — Stable, release-ready code
- `develop` — Integration branch for features
- `feature/*` — New features
- `fix/*` — Bug fixes
- `docs/*` — Documentation updates

### Making Changes

1. Create a branch from `develop`:
   ```bash
   git checkout develop
   git pull origin develop
   git checkout -b feature/my-feature
   ```

2. Make your changes with clear, focused commits

3. Ensure all checks pass:
   ```bash
   cargo fmt --check
   cargo clippy --all-features
   cargo test
   ```

4. Push and create a pull request

## Coding Standards

### Formatting

We use `rustfmt` with the configuration in `rustfmt.toml`:

```bash
# Format all code
cargo fmt

# Check formatting without changes
cargo fmt --check
```

### Linting

We use strict Clippy lints appropriate for window manager development:

```bash
# Run Clippy
cargo clippy --all-features

# With all warnings as errors
cargo clippy --all-features -- -D warnings
```

Key linting rules:
- **No `.unwrap()` or `.expect()`** in non-test code — use proper error handling
- **No `.ok()` silently ignoring errors** — handle or log errors
- **No `panic!()` in production code** — window managers must not crash
- **Prefer `.get()` over indexing** — avoid panics on out-of-bounds

### Documentation

- All public items must have documentation
- Use `///` for item documentation
- Use `//!` for module-level documentation
- Include examples where helpful

```rust
/// Calculates the layout for all windows in a workspace.
///
/// This function traverses the container tree and assigns geometry
/// to each window based on the current layout mode.
///
/// # Arguments
///
/// * `workspace` - The workspace to calculate layout for
/// * `outer_gap` - Gap between windows and screen edges
///
/// # Returns
///
/// A vector of window IDs with their calculated geometries.
pub fn calculate_layout(workspace: &Workspace, outer_gap: u32) -> Vec<(WindowId, Geometry)> {
    // ...
}
```

### Error Handling

Use `thiserror` for error types and `anyhow` for error propagation:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),
    
    #[error("Failed to parse config: {0}")]
    ParseError(#[from] toml::de::Error),
    
    #[error("Invalid value for {field}: {message}")]
    InvalidValue { field: String, message: String },
}
```

### Unsafe Code

Unsafe code requires a `// SAFETY:` comment explaining why it's safe:

```rust
// SAFETY: We've verified that ptr is non-null and properly aligned,
// and the memory it points to is valid for the lifetime of this reference.
unsafe { &*ptr }
```

## Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_layout_calculation

# Run tests with output
cargo test -- --nocapture

# Run tests for specific feature
cargo test --features "x11"
```

### Writing Tests

Place tests in the same file as the code they test:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_container_split() {
        let mut container = Container::new(ContainerType::Split);
        container.add_child(child1);
        container.add_child(child2);
        
        assert_eq!(container.children().len(), 2);
    }
    
    #[test]
    fn test_geometry_intersection() {
        let a = Geometry::new(0, 0, 100, 100);
        let b = Geometry::new(50, 50, 100, 100);
        
        let intersection = a.intersection(&b);
        assert!(intersection.is_some());
    }
}
```

### Integration Tests

Place integration tests in `tests/`:

```rust
// tests/ipc_test.rs
use fluxway::ipc::{IpcClient, IpcMessage};

#[test]
fn test_ipc_round_trip() {
    // Test IPC message serialization/deserialization
}
```

### Benchmarks

Add benchmarks to `benches/`:

```rust
// benches/layout_bench.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn layout_benchmark(c: &mut Criterion) {
    c.bench_function("calculate_layout_100_windows", |b| {
        b.iter(|| {
            // Benchmark code
        });
    });
}

criterion_group!(benches, layout_benchmark);
criterion_main!(benches);
```

## Pull Request Process

### Before Submitting

1. **Rebase on latest develop**
   ```bash
   git fetch origin
   git rebase origin/develop
   ```

2. **Ensure all checks pass**
   ```bash
   cargo fmt --check
   cargo clippy --all-features -- -D warnings
   cargo test
   ```

3. **Update documentation** if needed

4. **Write a clear PR description**

### PR Template

```markdown
## Description
Brief description of the changes.

## Type of Change
- [ ] Bug fix (non-breaking change fixing an issue)
- [ ] New feature (non-breaking change adding functionality)
- [ ] Breaking change (fix or feature causing existing functionality to change)
- [ ] Documentation update

## Testing
Describe the tests you ran and how to reproduce.

## Checklist
- [ ] My code follows the project's style guidelines
- [ ] I have added tests covering my changes
- [ ] All new and existing tests pass
- [ ] I have updated documentation as needed
- [ ] My commits are focused and have clear messages
```

### Review Process

1. At least one maintainer must approve
2. All CI checks must pass
3. No unresolved conversations
4. Squash and merge preferred for clean history

## Areas Needing Help

### High Priority

- **X11 Native Backend**: Implementing full X11 window manager support
- **DRM Backend**: Direct rendering for production use on TTY
- **Testing**: Real-world testing on various hardware and distros

### Medium Priority

- **Documentation**: Improving guides, examples, and API docs
- **Accessibility**: Screen reader support, high contrast themes
- **Packaging**: Creating packages for major distributions

### Good First Issues

Look for issues labeled `good first issue`:
- Documentation improvements
- Adding tests for existing functionality
- Small bug fixes
- Code cleanup and refactoring

### Research Areas

- Plugin/extension system design
- Scripting API (Lua, Rhai, or other)
- Wayland protocol extensions
- Performance optimization

## Getting Help

- **GitHub Issues**: Bug reports and feature requests
- **GitHub Discussions**: Questions and general discussion
- **Code Review**: Ask questions in PR comments

## Recognition

Contributors are recognized in:
- The project's CONTRIBUTORS file
- Release notes for significant contributions
- GitHub's contributor graph

Thank you for contributing to Fluxway! 🎉
