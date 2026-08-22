//! Benchmarks for Brownian bridge path construction.
//!
//! `construct_path` runs once per Monte Carlo path, so its per-call cost is a
//! direct multiplier on path-dependent pricers. Measures both the uniform-grid
//! and irregular-grid variants across the step counts typical for daily and
//! intraday simulation grids.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use finstack_quant_core::math::random::BrownianBridge;
use std::hint::black_box;

fn shocks(num_steps: usize) -> Vec<f64> {
    (0..num_steps)
        .map(|i| ((i % 17) as f64 * 0.031) - 0.25)
        .collect()
}

fn bench_construct_path(c: &mut Criterion) {
    let mut group = c.benchmark_group("brownian_bridge_construct_path");
    for &num_steps in &[16usize, 64, 256] {
        let bridge = BrownianBridge::new(num_steps);
        let z = shocks(num_steps);
        group.bench_with_input(
            BenchmarkId::new("uniform", num_steps),
            &num_steps,
            |b, _| {
                let mut w = vec![0.0; num_steps + 1];
                b.iter(|| {
                    bridge
                        .construct_path(black_box(&z), &mut w, 1.0 / 252.0)
                        .expect("valid bridge inputs");
                    black_box(w[num_steps])
                });
            },
        );
    }
    group.finish();
}

fn bench_construct_path_irregular(c: &mut Criterion) {
    let mut group = c.benchmark_group("brownian_bridge_construct_path_irregular");
    for &num_steps in &[16usize, 64, 256] {
        let bridge = BrownianBridge::new(num_steps);
        let z = shocks(num_steps);
        let times: Vec<f64> = (0..=num_steps)
            .map(|i| i as f64 / num_steps as f64)
            .collect();
        group.bench_with_input(
            BenchmarkId::new("irregular", num_steps),
            &num_steps,
            |b, _| {
                let mut w = vec![0.0; num_steps + 1];
                b.iter(|| {
                    bridge
                        .construct_path_irregular(black_box(&z), &mut w, black_box(&times))
                        .expect("valid bridge inputs");
                    black_box(w[num_steps])
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_construct_path,
    bench_construct_path_irregular
);
criterion_main!(benches);
