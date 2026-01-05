//! Benchmarks for window operations
//!
//! These benchmarks measure the performance of window management operations,
//! including creation, destruction, and state changes.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

fn window_creation_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("window_creation");

    group.bench_function("create_window", |b| {
        b.iter(|| {
            // Placeholder for window creation
            black_box(uuid::Uuid::new_v4())
        });
    });

    group.bench_function("create_window_with_properties", |b| {
        b.iter(|| {
            // Placeholder for window creation with full properties
            let _title = black_box("Window Title".to_string());
            let _app_id = black_box("app.example".to_string());
            black_box(uuid::Uuid::new_v4())
        });
    });

    group.finish();
}

fn window_state_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("window_state");

    group.bench_function("toggle_floating", |b| {
        b.iter(|| {
            let mut state = black_box(0u32);
            state ^= 1 << 4; // Toggle floating bit
            black_box(state)
        });
    });

    group.bench_function("toggle_fullscreen", |b| {
        b.iter(|| {
            let mut state = black_box(0u32);
            state ^= 1 << 1; // Toggle fullscreen bit
            black_box(state)
        });
    });

    group.bench_function("check_multiple_states", |b| {
        b.iter(|| {
            let state = black_box(0b11010101u32);
            let focused = (state & (1 << 0)) != 0;
            let fullscreen = (state & (1 << 1)) != 0;
            let hidden = (state & (1 << 3)) != 0;
            black_box((focused, fullscreen, hidden))
        });
    });

    group.finish();
}

fn window_geometry_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("geometry");

    group.bench_function("contains_point", |b| {
        b.iter(|| {
            let (x, y, w, h) = black_box((100, 100, 800, 600));
            let (px, py) = black_box((450, 350));
            let contains = px >= x && px < x + w && py >= y && py < y + h;
            black_box(contains)
        });
    });

    group.bench_function("intersects", |b| {
        b.iter(|| {
            let (x1, y1, w1, h1) = black_box((100, 100, 800, 600));
            let (x2, y2, w2, h2) = black_box((500, 400, 400, 300));
            let intersects = x1 < x2 + w2 && x1 + w1 > x2 && y1 < y2 + h2 && y1 + h1 > y2;
            black_box(intersects)
        });
    });

    group.bench_function("split_horizontal", |b| {
        b.iter(|| {
            let (x, y, w, h) = black_box((0, 0, 1920, 1080));
            let ratio = black_box(0.5f64);
            let left_w = (w as f64 * ratio) as i32;
            let right_w = w - left_w;
            black_box(((x, y, left_w, h), (x + left_w, y, right_w, h)))
        });
    });

    group.finish();
}

fn window_lookup_benchmark(c: &mut Criterion) {
    use std::collections::HashMap;

    let mut group = c.benchmark_group("window_lookup");

    for size in [10, 50, 100, 500, 1000].iter() {
        group.bench_with_input(BenchmarkId::new("hashmap_lookup", size), size, |b, &n| {
            let mut map: HashMap<uuid::Uuid, u32> = HashMap::new();
            let mut keys = Vec::new();
            for i in 0..n {
                let id = uuid::Uuid::new_v4();
                map.insert(id, i);
                keys.push(id);
            }
            let lookup_key = keys[n as usize / 2];

            b.iter(|| black_box(map.get(&lookup_key)));
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    window_creation_benchmark,
    window_state_benchmark,
    window_geometry_benchmark,
    window_lookup_benchmark,
);
criterion_main!(benches);
