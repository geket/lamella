//! Benchmarks for window operations

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use fluxway_core::config::Config;
use fluxway_core::state::State;
use fluxway_core::window::{Window, WindowId};

fn window_add_remove_benchmark(c: &mut Criterion) {
    c.bench_function("add_remove_100_windows", |b| {
        b.iter(|| {
            let config = Config::default();
            let mut state = State::new(config);
            for i in 1..=100u64 {
                let window = Window::new(WindowId(i), "test".into(), format!("Window {i}"));
                state.add_window(black_box(window));
            }
        });
    });
}

criterion_group!(benches, window_add_remove_benchmark);
criterion_main!(benches);
