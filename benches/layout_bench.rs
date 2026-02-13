//! Benchmarks for layout calculations

#![allow(clippy::many_single_char_names)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

use fluxway_core::config::Config;
use fluxway_core::layout::LayoutTree;
use fluxway_core::state::Geometry;
use fluxway_core::window::WindowId;

fn layout_calculation_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("layout");

    for num_windows in &[1, 5, 10, 20, 50, 100] {
        group.bench_with_input(
            BenchmarkId::new("calculate_tiling", num_windows),
            num_windows,
            |b, &n| {
                let config = Config::default();
                let mut tree = LayoutTree::new();
                for i in 0..n {
                    tree.add_window(WindowId(i as u64 + 1), &config);
                }
                let area = Geometry::new(0, 0, 1920, 1080);
                b.iter(|| {
                    tree.calculate_layout(black_box(area), 4);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, layout_calculation_benchmark);
criterion_main!(benches);
