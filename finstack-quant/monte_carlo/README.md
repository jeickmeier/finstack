# finstack-quant-monte-carlo

Monte Carlo simulation, pricing, and diagnostics for quantitative finance.
The crate supplies the pieces a pricing run is assembled from — random streams,
stochastic processes, discretization schemes, payoffs, a generic execution
engine, sensitivity estimators, and currency-tagged result types — plus a few
higher-level pricers for the common cases.

Four traits define the contracts: [`RandomStream`](src/traits.rs),
[`StochasticProcess`](src/traits.rs), [`Discretization`](src/traits.rs), and
[`Payoff`](src/traits.rs). [`McEngine`](src/engine/pricing.rs) composes them.
`prelude` re-exports the types most callers need.

## Position in the stack

Depends only on `finstack-quant-core` (plus `serde`, `serde_json`, `smallvec`,
`nalgebra`, `rayon`, `tracing`). It does not depend on `finstack-quant-valuations`,
so instrument pricing can build on it without a cycle.

Consumed by `finstack-quant-valuations` (Monte Carlo pricers for exotics and rate
products), re-exported from the umbrella crate as `finstack_quant::monte_carlo`,
and surfaced through both binding crates.

The crate has **no cargo features**; `[features] default = []`. Rayon is an
unconditional dependency. A few convenience entry points (`EuropeanPricer`,
`pricer::heston`, `greeks::gbm_european`) force serial execution under
`#[cfg(target_arch = "wasm32")]` because no thread pool exists there.

## Public surface

| Module | Contents |
|--------|----------|
| `traits` | `RandomStream`, `StochasticProcess`, `Discretization`, `Payoff`, `PathState`, `StateKey`, `state_keys`, `ProportionalDiffusion` |
| `engine` | `McEngine`, `McEngineBuilder`, `McEngineConfig`, `PathCaptureConfig`, `PathCaptureMode`, `MAX_NUM_PATHS`, `MAX_CAPTURED_PATHS` |
| `engine_fractional` | `simulate_path_fractional` — per-path loop with externally injected fractional noise, for rough-volatility processes |
| `time_grid` | Re-export of `finstack_quant_core::math::time_grid`: `TimeGrid`, `TimeGridError`, `map_date_to_step`, `map_dates_to_steps`, `map_exercise_dates_to_steps` |
| `rng` | `philox::PhiloxRng`, `sobol::SobolRng`, `fbm`, `volterra`; re-exports core's `brownian_bridge`, `poisson`, `BrownianBridge` |
| `process` | SDE definitions (see table below) and the `ProcessMetadata` trait |
| `discretization` | Time-stepping and exact transitions (see table below) |
| `payoff` | `vanilla` (`EuropeanCall`, `EuropeanPut`, `Digital`, `Forward`), `asian` (`AsianCall`, `AsianPut`, `AveragingMethod`, `geometric_asian_call_closed_form`), `barrier` (`BarrierOptionPayoff`, `BarrierType`, `OptionKind`), `lookback` (`Lookback`, `LookbackDirection`, `FloatingStrikeLookbackCall`, `FloatingStrikeLookbackPut`) |
| `pricer` | `european::EuropeanPricer`, `path_dependent::PathDependentPricer`, `lsmc::LsmcPricer`, `heston::{price_heston_call, price_heston_put}`, `basis` (`PolynomialBasis`, `LaguerreBasis`), `lsq` |
| `greeks` | `pathwise`, `lrm`, `finite_diff`, `gbm_european` |
| `barriers` | `bridge::{bridge_hit_probability, check_barrier_hit, BarrierDirection}`, `corrections::{gobet_miri_adjusted_barrier, half_step_adjusted_barrier}` |
| `variance_reduction` | `control_variate::{apply_control_variate, black_scholes_call, black_scholes_put}` |
| `estimate` / `online_stats` | `Estimate`, `OnlineStats`, `OnlineCovariance`, `required_samples` |
| `results` | `MoneyEstimate`, `MonteCarloResult`, `RunMetadata` |
| `paths` | `PathDataset`, `SimulatedPath`, `PathPoint`, `PathSamplingMethod`, `ProcessParams`, `CashflowType` |
| `registry` | Embedded runtime defaults and `MONTE_CARLO_DEFAULTS_EXTENSION_KEY` |
| `seed` | `derive_seed` — deterministic seeds from instrument id + scenario name |

Crate root also re-exports `simulate_gbm_paths`, `GbmPathConfig`, and
`GbmPathSummary`: a compact captured-GBM-paths helper for plotting and
diagnostics that bypasses the payoff machinery.

Antithetic pairing is **not** in `variance_reduction` — it is implemented inline
in the engine loop and configured with `McEngineConfig::antithetic`.

### Processes

| Module | Types |
|--------|-------|
| `process::gbm` | `GbmParams`, `GbmProcess`, `MultiGbmProcess` |
| `process::gbm_dividends` | `Dividend`, `GbmWithDividends` (requires its dedicated scheme) |
| `process::brownian` | `BrownianParams`, `BrownianProcess`, `MultiBrownianProcess` |
| `process::heston` | `HestonParams`, `HestonProcess` |
| `process::cir` | `CirParams`, `CirProcess`, `CirPlusPlusProcess` |
| `process::ou` | `HullWhite1FParams`, `HullWhite1FProcess` (`HullWhite1FProcess::vasicek` for constant θ), `calibrate_theta_from_curve` |
| `process::multi_ou` | `MultiOuParams`, `MultiOuProcess` |
| `process::schwartz_smith` | `SchwartzSmithParams`, `SchwartzSmithProcess` |
| `process::lmm` | `LmmParams`, `LmmProcess` |
| `process::rough_bergomi` | `RoughBergomiParams`, `RoughBergomiProcess` |
| `process::rough_heston` | `RoughHestonParams`, `RoughHestonProcess` |
| `process::cheyette_rough` | `CheyetteRoughVolParams`, `CheyetteRoughVolProcess` |

There is no `process::correlation` module. Correlation helpers live in
`finstack_quant_core::math::linalg`; the prelude re-exports `apply_correlation`
and `cholesky_decomposition`, while `cholesky_correlation` (the engine's own
factoring entry point) must be imported from core directly. A process declares
its factor correlation via
`StochasticProcess::factor_correlation`; the engine Cholesky-factors it and
applies it to the raw shocks unless the scheme reports
`applies_correlation_internally()`.

### Discretization schemes

| Module | Types |
|--------|-------|
| `discretization::exact` | `ExactGbm`, `ExactMultiGbm`, `ExactMultiGbmCorrelated` |
| `discretization::exact_gbm_dividends` | `ExactGbmWithDividends` |
| `discretization::exact_hw1f` | `ExactHullWhite1F` |
| `discretization::euler` | `EulerMaruyama`, `LogEuler` |
| `discretization::milstein` | `Milstein`, `LogMilstein` |
| `discretization::qe_heston` | `QeHeston` |
| `discretization::qe_cir` | `QeCir` |
| `discretization::schwartz_smith` | `ExactSchwartzSmith` |
| `discretization::lmm_predictor_corrector` | `LmmPredictorCorrector` |
| `discretization::rough_bergomi` | `RoughBergomiEuler` |
| `discretization::rough_heston` | `RoughHestonHybrid` |
| `discretization::cheyette_rough` | `CheyetteRoughEuler` |

A process may declare `dedicated_scheme()`. Pairing it with anything else is
rejected at runtime rather than silently simulating the diffusion only (which is
what dropping discrete dividends or jumps would do).

## Pricing workflow

1. Build an `McEngine` from `McEngine::builder()` with a time grid and runtime
   options.
2. Construct an RNG. **The seed lives on the RNG, not on the engine** —
   `PhiloxRng::new(seed)`. `McEngineBuilder` has no `seed` method.
3. Pick a `StochasticProcess` and a compatible `Discretization`.
4. Supply the initial state as a raw `&[f64]` of length `process.dim()`.
5. Pick or implement a `Payoff`.
6. Call `price()` for the aggregate estimate, or `price_with_capture()` for the
   estimate plus a captured path dataset.

The engine owns the path loop, Welford online statistics, deterministic chunked
reduction, optional serial early stopping, and optional path capture.

### Generic engine

```rust
use finstack_quant_core::currency::Currency;
use finstack_quant_monte_carlo::prelude::*;

let engine = McEngine::builder()
    .num_paths(25_000)
    .uniform_grid(1.0, 252)
    .parallel(true)
    .build()
    .expect("valid Monte Carlo configuration");

let rng = PhiloxRng::new(11);
let process = GbmProcess::with_params(0.03, 0.01, 0.20).expect("valid GBM parameters");
let disc = ExactGbm::new();
let payoff = EuropeanCall::new(100.0, 1.0, 252);
let discount_factor = (-0.03_f64).exp();

let result = engine
    .price(
        &rng,
        &process,
        &disc,
        &[100.0],
        &payoff,
        Currency::USD,
        discount_factor,
    )
    .expect("pricing should succeed");

println!("{} +/- {}", result.mean, result.stderr);
```

`discount_factor` is a scalar, not a rate: the engine imposes no compounding or
day-count convention. Callers holding a flat continuously compounded rate and a
year fraction should build it with
`finstack_quant_core::cashflow::flat_discount_factor`.

### Pricing with captured paths

`price_with_capture()` returns the aggregate estimate plus a subset of paths for
plotting, debugging, or cashflow inspection. It takes one extra argument,
`ProcessParams`, which names the raw state-vector positions so downstream
consumers can interpret them.

```rust
use finstack_quant_core::currency::Currency;
use finstack_quant_monte_carlo::prelude::*;

let engine = McEngine::builder()
    .num_paths(10_000)
    .uniform_grid(1.0, 12)
    .path_capture(PathCaptureConfig::sample(200, 17).with_payoffs())
    .parallel(false)
    .build()
    .expect("valid Monte Carlo configuration");

let rng = PhiloxRng::new(11);
let process = GbmProcess::with_params(0.03, 0.01, 0.20).expect("valid GBM parameters");
let disc = ExactGbm::new();
let payoff = EuropeanCall::new(100.0, 1.0, 12);
let process_params = ProcessParams::new("GBM").with_factors(vec!["spot".to_string()]);

let result = engine
    .price_with_capture(
        &rng,
        &process,
        &disc,
        &[100.0],
        &payoff,
        Currency::USD,
        (-0.03_f64).exp(),
        process_params,
    )
    .expect("pricing with capture should succeed");

println!("estimate={}", result.estimate.mean);

if let Some(paths) = result.paths.as_ref() {
    println!("captured={} of {}", paths.num_captured(), paths.num_paths_total);
    println!("sampling={:?}", paths.sampling_method);
    println!("state_keys={:?}", paths.state_var_keys());
}
```

`MonteCarloResult::paths` and `MonteCarloResult::run` are public fields, not
accessor methods.

### Unbiased American pricing (two-pass LSMC)

Single-pass Longstaff-Schwartz fits the regression and prices on the same path
set, biasing the price upward. `LsmcPricer::price_unbiased` trains the policy on
one path set and replays it on a second, independent set. It rejects a
`pricing_seed` equal to the configured training seed.

```rust
use finstack_quant_core::currency::Currency;
use finstack_quant_monte_carlo::prelude::*;

let cfg = LsmcConfig::new(50_000, vec![25, 50, 75, 100], 100)
    .expect("valid LSMC config")
    .with_seed(42);
let pricer = LsmcPricer::new(cfg);
let process = GbmProcess::with_params(0.05, 0.0, 0.3).expect("valid GBM parameters");
let put = AmericanPut::new(100.0).expect("valid strike");
let basis = PolynomialBasis::new(2);

let unbiased = pricer
    .price_unbiased(
        &process,
        100.0,
        1.0,
        100,
        &put,
        &basis,
        Currency::USD,
        0.05,
        /* pricing_seed = */ 4243,
    )
    .expect("two-pass pricing should succeed");

assert!(unbiased.mean.amount() > 0.0);
```

For finer control, `LsmcPricer::fit_exercise_policy` returns an `ExercisePolicy`
that `LsmcPricer::price_with_policy` can replay across multiple scenarios.
`LsmcConfig::every_step(num_paths, num_steps)` builds the American exercise grid
`1..=num_steps`. Note that terminal exercise at `num_steps` is always applied
whether or not it is listed, and immediate exercise at `t = 0` floors the
reported price, so it cannot print below intrinsic.

### Finite-difference Greeks with common random numbers

- `finite_diff_delta` / `finite_diff_gamma` honour the engine's `use_parallel`
  flag and report a **conservative independence-bound** stderr — an upper bound,
  since CRN correlates the legs.
- `finite_diff_delta_crn` / `finite_diff_gamma_crn` pair the legs per path and
  report the true paired stderr. Serial only.

All four require a splittable RNG and fail closed with `SobolRng`.

```rust
use finstack_quant_core::currency::Currency;
use finstack_quant_monte_carlo::prelude::*;

let engine = McEngine::builder()
    .num_paths(20_000)
    .uniform_grid(1.0, 50)
    .parallel(false)
    .build()
    .expect("valid config");
let rng = PhiloxRng::new(42);
let gbm = GbmProcess::with_params(0.05, 0.0, 0.2).expect("valid GBM parameters");
let disc = ExactGbm::new();
let call = EuropeanCall::new(100.0, 1.0, 50);

let (delta, paired_stderr) = finite_diff_delta_crn(
    &engine,
    &rng,
    &gbm,
    &disc,
    /* initial_spot   = */ 100.0,
    &call,
    Currency::USD,
    /* discount_factor= */ (-0.05_f64).exp(),
    /* relative bump  = */ 0.01,
)
.expect("CRN delta should succeed");
```

For GBM European contracts specifically, host bindings must go through
`greeks::gbm_european` rather than assembling the engine themselves.

### Compact entry points

`pricer::european::EuropeanPricer` wires a uniform time grid, `PhiloxRng`,
`ExactGbm`, and `McEngine` internally for European-style payoffs under GBM:

```rust
use finstack_quant_core::currency::Currency;
use finstack_quant_monte_carlo::payoff::vanilla::EuropeanCall;
use finstack_quant_monte_carlo::pricer::european::EuropeanPricer;
use finstack_quant_monte_carlo::process::gbm::GbmProcess;

let pricer = EuropeanPricer::new(25_000).with_seed(19).with_parallel(false);
let process = GbmProcess::with_params(0.03, 0.01, 0.20).expect("valid GBM parameters");
let payoff = EuropeanCall::new(100.0, 1.0, 252);

let result = pricer
    .price(&process, 100.0, 1.0, 252, &payoff, Currency::USD, (-0.03_f64).exp())
    .expect("pricing should succeed");
```

`PathDependentPricer` covers path-dependent contracts (with optional Sobol and
Brownian-bridge construction), and `pricer::heston::{price_heston_call,
price_heston_put}` are the canonical Heston European entry points shared with the
host bindings.

## Determinism and parallelism

The determinism guarantee is stronger than "reproducible across runs":

- Every path maps to its own substream via `rng.split(path_id)`, so per-path
  values do not depend on execution order or thread count.
- The default chunk size is a **pure function of `num_paths`**
  (`(num_paths / 64).clamp(100, 10_000)`), never of `rayon::current_num_threads()`.
  The chunk partition fixes the `OnlineStats::merge` reduction tree, and
  floating-point merges are order-sensitive.
- Consequently, with a splittable RNG and auto-stopping disabled, serial and
  parallel runs are **bit-identical**, and so are runs across machines,
  `RAYON_NUM_THREADS` settings, and native-vs-wasm hosts.
  `src/engine/tests.rs` pins this with `to_bits()` comparisons across 1-thread
  and 8-thread rayon pools.
- Captured paths are sorted by `path_id` before being returned, so dataset
  ordering is stable across serial and parallel runs.

Constraints:

- Parallel pricing requires `RandomStream::supports_splitting() == true`.
  `PhiloxRng` satisfies this; `SobolRng` does not and must run serially.
- `target_ci_half_width` auto-stopping is serial-only. It is an optional-stopping
  rule, so the stopped estimator carries a small bias; a 5 000-sample warm-up
  keeps the half-width estimate stable before the rule can fire.
- Passing an explicit `chunk_size(n)` overrides the adaptive default and changes
  the reduction tree — results stay deterministic but will not match a default-
  chunked run bit-for-bit.

## Runtime validation

The engine rejects these configurations instead of proceeding:

- `num_paths == 0`, or `num_paths > MAX_NUM_PATHS` (`10_000_000`).
- `chunk_size == Some(0)`.
- A time grid with zero steps.
- `process.dim() == 0`, or `initial_state.len() != process.dim()`.
- A process that reports `requires_injected_noise()` (rough-volatility models):
  the generic loop would fill those factor slots with i.i.d. normals. Drive them
  through `engine_fractional::simulate_path_fractional` or a dedicated pricer.
- A process with a `dedicated_scheme()` paired with a different scheme.
- A payoff whose `max_event_step()` exceeds the grid — the fixing would silently
  never fire.
- `discount_factor` non-finite or negative; any non-finite discounted path value.
- `target_ci_half_width` non-finite, non-positive, or combined with
  `use_parallel = true`.
- `use_parallel = true` with a non-splittable RNG.
- Path capture combined with `antithetic = true`.
- Requested captured paths above `MAX_CAPTURED_PATHS` (`100_000`), or a sample
  `count` outside `1..=num_paths`.
- An invalid `ProcessParams` (`price_with_capture` only).

`PathCaptureMode::Sample { count, seed }` uses deterministic hash-based Bernoulli
sampling, so the realized number of retained paths is close to `count`, not
exactly `count`.

## Conventions and units

- Rates, dividend yields, and volatilities are decimals; times and time-grid
  coordinates are year fractions.
- `Payoff::value` returns an **undiscounted** `Money`; the engine applies the
  caller-supplied `discount_factor` per path.
- Confidence intervals are reported on discounted path values.
- Captured-path statistics (`median`, `percentile_25`, `percentile_75`, `min`,
  `max`) describe the **retained subset**, not the full Monte Carlo population.
- With `antithetic = true`, `num_paths` counts independent estimators, while
  `num_simulated_paths` counts raw simulated paths (`2 * num_paths`). Both are
  reported on `Estimate` and `MoneyEstimate`.
- Payoffs read named values from `PathState` (`spot`, `variance`, `short_rate`,
  indexed spots) rather than raw state-vector positions. The built-in payoffs
  panic on a missing or non-finite named input rather than defaulting to `0.0`,
  because that default turns a wiring bug into a systematically wrong price
  (puts paying full strike, down-barriers knocking out at step 0).
- `#![forbid(unsafe_code)]`; `unwrap`/`expect`/`panic`/`unreachable` are denied
  outside tests.

See [`INVARIANTS.md`](../../INVARIANTS.md) for the workspace-wide Decimal/f64 and
determinism rules.

## Results and captured diagnostics

`MoneyEstimate` carries `mean: Money`, `stderr`, `ci_95: (Money, Money)`,
`num_paths`, `num_simulated_paths`, and optional `std_dev`, `median`,
`percentile_25`, `percentile_75`, `min`, `max`.

`MonteCarloResult` adds `paths: Option<PathDataset>` and
`run: Option<RunMetadata>`. `RunMetadata` stamps `seed`, `use_parallel`,
`antithetic`, the resolved `chunk_size`, and `num_steps` so a run is auditable
and replayable. The engine cannot observe the seed (it receives a constructed
stream), so `run.seed` is filled by pricers that derive the stream from a seed.

`PathDataset` holds the retained `SimulatedPath` values, `num_paths_total`, the
`PathSamplingMethod`, and the `ProcessParams`. Each `SimulatedPath` carries
`PathPoint` entries per captured step (time, raw state vector, optional payoff
snapshot, typed cashflows), a `final_value`, and an optional `irr`.

Supplying `ProcessParams::with_factors(...)` lets consumers call
`PathDataset::state_var_keys()` to recover stable names for each state-vector
position.

## Runtime defaults registry

Default path counts, seeds, and parallel/antithetic flags are versioned JSON in
[`data/defaults/pricer_defaults.v1.json`](data/defaults/pricer_defaults.v1.json),
embedded at build time and read by `McEngineConfig::new`, `McEngineBuilder::new`,
`LsmcConfig::new`, `PathDependentPricerConfig`, and the convenience pricers.
Overlays use the `FinstackConfig` extension key
`monte_carlo.defaults.v1` (`registry::MONTE_CARLO_DEFAULTS_EXTENSION_KEY`).

## Extending the crate

Extend through the traits; do not modify the engine loop.

**New process** — implement `StochasticProcess`: `dim()` is the raw state-vector
length, `num_factors()` the number of independent shocks, `drift()` and
`diffusion()` the SDE. Implement `populate_path_state()` to map raw entries onto
semantic keys (`spot`, `variance`, `short_rate`, indexed spots), and declare
`factor_correlation()`, `dedicated_scheme()`, or `requires_injected_noise()`
where they apply.

**New discretization** — implement `Discretization<P>`. Prefer an exact scheme
when an analytical transition exists; otherwise document stability and positivity
assumptions. Override `work_size()` for scratch space, `prepare()` for
grid-dependent precomputation, `applies_correlation_internally()` when the scheme
handles correlation itself, and `scheme_id()` for dedicated-scheme pairing.

**New payoff** — implement `Payoff`: `on_path_start()` for per-path random setup,
`on_event()` to consume each path state, `value()` for the final undiscounted
amount, `reset()` to clear per-path state. Emit diagnostics with
`state.add_cashflow()` / `state.add_typed_cashflow()`, and report the last fixing
via `max_event_step()` so grid mismatches are caught up front.

## Bindings

- **Python** — `finstack_quant.monte_carlo` exposes `McEngine`, `TimeGrid`,
  `EuropeanPricer`, `PathDependentPricer`, `LsmcPricer`, `MoneyEstimate`,
  `Estimate`, `GbmPathSummary`, `simulate_gbm_paths`, `price_heston_call` /
  `price_heston_put`, `heston_satisfies_feller`, `black_scholes_call` /
  `black_scholes_put`, and the four finite-difference Greek functions.
- **WASM** — the `monte_carlo` namespace in
  [`exports/monte_carlo.js`](../../finstack-quant-wasm/exports/monte_carlo.js)
  exposes the convenience pricers only: `priceEuropeanCall/Put`,
  `priceHestonCall/Put`, `priceAsianCall/Put`, `priceAmericanCall/Put`,
  `priceAmericanCallUnbiased` / `priceAmericanPutUnbiased`, and
  `blackScholesCall/Put`.

Neither binding exposes the generic trait surface; custom processes, schemes, and
payoffs are Rust-only.

## Verification

```bash
cargo nextest run -p finstack-quant-monte-carlo --lib
cargo nextest run -p finstack-quant-monte-carlo --lib --run-ignored only
cargo clippy -p finstack-quant-monte-carlo --all-targets -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc -p finstack-quant-monte-carlo --no-deps
cargo bench -p finstack-quant-monte-carlo --bench mc_hot_paths
```

Workspace gates: `mise run rust-test`, `mise run rust-test-slow` (the
`#[ignore]`d rough-volatility convergence cases), `mise run rust-lint`,
`mise run rust-doc`, `mise run rust-bench`. Do not run `cargo test` directly —
it pulls in doc tests the workspace gates run separately.

Benchmarks in [`benches/mc_hot_paths.rs`](benches/mc_hot_paths.rs) cover
`european_pricer`, `lsmc_pricer`, `lsq_regression`, `heston_qe_pricer`,
`rough_heston_step`, and `hw1f_pricer`.

## References

Canonical references live in [`docs/REFERENCES.md`](../../docs/REFERENCES.md).
Anchors used across this crate:

- [`#glasserman-2004-monte-carlo`](../../docs/REFERENCES.md#glasserman-2004-monte-carlo)
- [`#welford-1962`](../../docs/REFERENCES.md#welford-1962)
- [`#salmon-2011-philox`](../../docs/REFERENCES.md#salmon-2011-philox)
- [`#joe-kuo-2008-sobol`](../../docs/REFERENCES.md#joe-kuo-2008-sobol)
- [`#owen-1995-scrambling`](../../docs/REFERENCES.md#owen-1995-scrambling)
- [`#heston-1993`](../../docs/REFERENCES.md#heston-1993)
- [`#schwartz-smith-2000`](../../docs/REFERENCES.md#schwartz-smith-2000)
- [`#black-scholes-1973`](../../docs/REFERENCES.md#black-scholes-1973)
- [`#hull-options-futures`](../../docs/REFERENCES.md#hull-options-futures)

Process- and scheme-specific assumptions live in the module docs
(`cargo doc -p finstack-quant-monte-carlo --open`). See
[`src/README.md`](src/README.md) for the source-directory map.
