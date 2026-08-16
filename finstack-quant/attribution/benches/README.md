# finstack-quant-attribution benchmarks

Criterion benchmarks for the P&L attribution entry points. The crate sets
`autobenches = false` in [`../Cargo.toml`](../Cargo.toml), so a new file here does
nothing until it is added as a `[[bench]]` target. Two targets are registered, both
`harness = false`:

| Target | Scope |
|--------|-------|
| `attribution` | Fixed-size cost of `attribute_pnl_parallel` and `attribute_pnl_waterfall` on one bond and on five bonds |
| `attribution_scale` | Five methodologies — `simple_pnl_bridge`, `attribute_pnl_metrics_based`, `attribute_pnl_parallel`, `attribute_pnl_waterfall`, `attribute_pnl_taylor` — swept over portfolio sizes N ∈ {10, 100, 1000}, plus a 200-instrument credit-factor-model case. `attribute_return_contribution` is public and not covered here. |

Both targets attribute across the same shape of market move: a flat `USD-OIS`
`DiscountCurve` at 4%, shifted between `market_t0` and `market_t1`. `attribution.rs`
uses a 5 bp shift; `attribution_scale.rs` uses 1 bp. Instruments are vanilla
fixed-coupon `Bond`s (1M USD notional, 5% coupon), which keeps the pricing path warm
without pulling volatility machinery into the measurement.

## `attribution` benchmarks

Top-level `c.bench_function` ids (no `benchmark_group`), one Criterion group per bench:

| Id | Measures |
|----|----------|
| `parallel_1_bond` | `attribute_pnl_parallel` with `ExecutionPolicy::Parallel`, single 5y bond |
| `waterfall_1_bond` | `attribute_pnl_waterfall` over `default_waterfall_order()`, single 5y bond |
| `parallel_5_bonds` | The parallel path looped over 5 bonds with maturities spread 3y–11y |

## `attribution_scale` benchmarks

| Group | Ids | Measures |
|-------|-----|----------|
| `attribution` | `<method>/{10,100,1000}` for `simple_bridge`, `metrics_based`, `parallel`, `waterfall`, `taylor` | Per-instrument cost of each methodology at three portfolio sizes, `Throughput::Elements(n)` so ns-per-instrument is comparable across sizes |
| `attribution_credit` | `parallel_with_credit_model/200` | `AttributionEnvelope::execute` over 200 `AttributionSpec`s carrying a `CreditFactorModel`, i.e. the JSON-spec path rather than the direct function call |

`simple_pnl_bridge` is the intended baseline: two reprices, no factor loop. The other
four methodologies add factor iteration on top, so read them as a multiple of the
bridge rather than in isolation. `metrics_based` additionally pays for two
`price_with_metrics` calls per instrument (`Dv01`, `Theta`, `Convexity`) inside the
measured region.

Both groups set `sample_size(10)`. Per the rationale in `attribution_scale.rs`, at
N = 1000 the waterfall and parallel paths would take minutes per size at Criterion's
default of 100 samples; 10 samples is enough to see a scaling trend, not enough for a
tight confidence interval. Treat single-run deltas here with suspicion and use
`--save-baseline` / `--baseline` instead of eyeballing.

## Run

```bash
cargo bench -p finstack-quant-attribution --bench attribution
cargo bench -p finstack-quant-attribution --bench attribution_scale
cargo bench -p finstack-quant-attribution -- --quick
cargo bench -p finstack-quant-attribution -- taylor            # filter by id substring
cargo bench -p finstack-quant-attribution -- --save-baseline before
cargo bench -p finstack-quant-attribution -- --baseline before
```

Benchmarks are measurement tasks, not gates: they are not run by `mise run rust-test`
(nextest), not by `mise run all-test`, and not by PR CI. What CI enforces is that they
compile — `mise run rust-lint` runs `clippy --workspace --all-targets --all-features --
-D warnings`. Workspace-wide measurement goes through `mise run rust-bench` (reduced
sampling, tunable via `FQ_BENCH_SAMPLE_SIZE`, `FQ_BENCH_WARM_UP_TIME`,
`FQ_BENCH_MEASUREMENT_TIME`, `FQ_BENCH_NRESAMPLES`), with
`mise run rust-bench-baseline` and `mise run rust-bench-compare` (fails above a 10%
median regression).

Criterion writes to `target/criterion/<group>/<id>/report/index.html` for
`attribution_scale`. `attribution` has no `benchmark_group`, so its ids sit at the top
level: `target/criterion/<id>/report/index.html`. The `mise run rust-bench*` tasks pass
`--noplot`.

## Conventions when adding a case

- Build fixtures outside `b.iter`. `attribution_scale.rs` does this through the
  `Fixture` / `CreditFixture` structs so curve construction is not folded into the
  attribution measurement.
- Set `group.throughput(Throughput::Elements(n))` on any size sweep; a scaling bench
  without throughput cannot be read as ns-per-instrument.
- Register the function in the target's `criterion_group!`. `attribution.rs` uses one
  `criterion_group!` per function and lists all three in `criterion_main!`; a new
  function added to neither runs silently as a no-op.

## See also

- [`../README.md`](../README.md) — crate overview, methodology table, and the test tree
  under [`../tests/`](../tests)
- [`../../portfolio/benches/README.md`](../../portfolio/benches/README.md) — book-level
  attribution, including the method-owned controls and Rayon thresholds
- [`../../valuations/benches/README.md`](../../valuations/benches/README.md) — the
  underlying instrument pricing these benchmarks call twice per attribution
