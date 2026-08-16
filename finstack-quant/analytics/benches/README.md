# finstack-quant-analytics benchmarks

Criterion benchmarks for the `finstack-quant-analytics` hot paths. The crate sets
`autobenches = false` in [`../Cargo.toml`](../Cargo.toml), so a new file here is inert
until it is added as a `[[bench]]` target. One target is registered:
`analytics_hot_paths` (`harness = false`, Criterion owns `main`).

Every case is driven through [`Performance`](../src/performance/mod.rs), the crate's
canonical public entry point. Almost every per-metric building block is `pub(crate)`;
the narrow public exceptions listed in [`../src/lib.rs`](../src/lib.rs) — `beta`,
`regression::constrained_least_squares`, and the `correlation` matrix helpers — exist
for cross-crate use, not as a measurement surface. If you add a benchmark, go through
`Performance` too — measuring an internal helper measures something users cannot call.

## Cases

`analytics_hot_paths.rs` uses top-level `c.bench_function` calls rather than
`benchmark_group`, so each id below is its own Criterion directory.

| Criterion fn | Benchmark ids |
|--------------|---------------|
| `bench_tail_risk` | `Performance::value_at_risk 2.5k`, `Performance::expected_shortfall 2.5k`, `Performance::value_at_risk 100k`, `Performance::expected_shortfall 100k` |
| `bench_return_based` | `Performance::volatility 2.5k`, `Performance::sharpe 2.5k` |
| `bench_drawdown` | `Performance::drawdown_series 10k` |
| `bench_performance` | `Performance::new 750x2 daily` (construction from price levels), `Performance::sharpe 750x2`, `Performance::value_at_risk 750x2` |
| `bench_rolling_greeks` | `Performance::rolling_greeks 2.5k window=63` |
| `bench_multi_factor_greeks` | `Performance::multi_factor_greeks 2.5k k=3` — OLS against three factor columns |
| `bench_correlation_matrix` | `Performance::correlation_matrix 1k x 50` |
| `bench_period_stats` | `Performance::period_stats 2.5k monthly` |

The 2.5k / 100k pair in `bench_tail_risk` is the only size sweep; it exists because the
tail metrics sort, so per-observation cost is expected to grow with `n log n`.

## Fixtures

Panel inputs are generated in-file and are deterministic — no RNG crate, no clock:

- `synthetic_returns(n, seed)` — a splitmix64-style iteration mapped into
  `(-0.02, 0.02)`.
- `synthetic_dates(n)` — consecutive calendar days from 2020-01-01 via
  `Date::next_day`.
- `perf_from_returns(n, seed)` — single-ticker `Performance::from_returns` at
  `PeriodKind::Daily`.
- `perf_panel(n_obs, n_tickers, seed)` — multi-ticker panel where column 0 (`T0`) is the
  benchmark ticker; used by the rolling-greeks, multi-factor, and correlation cases.

Keep new fixtures deterministic. A seeded generator is a hard requirement here, not a
style preference: benchmark inputs feed the same code paths the correctness tests pin.

## Run

```bash
cargo bench -p finstack-quant-analytics --bench analytics_hot_paths
cargo bench -p finstack-quant-analytics --bench analytics_hot_paths -- --quick
cargo bench -p finstack-quant-analytics -- value_at_risk        # filter by id substring
cargo bench -p finstack-quant-analytics -- --save-baseline before
cargo bench -p finstack-quant-analytics -- --baseline before
```

Benchmarks are measurement tasks, not gates: they are not run by `mise run rust-test`
(nextest), not by `mise run all-test`, and not by PR CI. What CI does enforce is that
they compile — `mise run rust-lint` runs `clippy --workspace --all-targets --all-features
-- -D warnings`. Workspace-wide measurement goes through `mise run rust-bench` (reduced
sampling, tunable via `FQ_BENCH_SAMPLE_SIZE`, `FQ_BENCH_WARM_UP_TIME`,
`FQ_BENCH_MEASUREMENT_TIME`, `FQ_BENCH_NRESAMPLES`), with
`mise run rust-bench-baseline` and `mise run rust-bench-compare` for regression gating
(the compare task fails above a 10% median regression).

Criterion writes to `target/criterion/<id>/report/index.html`; the `mise run rust-bench*`
tasks pass `--noplot`, so run `cargo bench` directly if you want the HTML.

## See also

- [`../README.md`](../README.md) — crate overview, public surface, and the correctness
  tests under [`../tests/`](../tests)
- [`../../core/benches/README.md`](../../core/benches/README.md) — core numerics
  benchmarks. Note that the core primitives analytics actually calls
  (`math::stats::quantile`, `math::summation`, `math::special_functions`,
  `math::linalg`) have no dedicated core bench target, so their cost surfaces here
  through `Performance` rather than there.
