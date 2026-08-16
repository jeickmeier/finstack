# Calibration

Plan-driven construction of discount, forward, hazard, inflation, cross-currency
basis, parametric, volatility, and base-correlation structures from market
quotes. This is the canonical path from raw quotes to a
`finstack_quant_core::market_data::context::MarketContext`.

`MarketContext::try_from(MarketContextState)` is the *snapshot* deserializer — it
rehydrates a previously saved context. It does not build one from quotes. Use it
only to replay an already-calibrated context.

## Layout

| Path | Visibility | Contents |
|------|-----------|----------|
| `api/` | public | `CalibrationEnvelope` schema, `engine::execute`, envelope validation, market-datum and prior-market inputs |
| `bumps/` | public | Re-calibration helpers for what-if risk (`BumpRequest`, `VolBumpRequest`) |
| `hull_white/` | public | Hull-White 1F calibration to swaptions and cap/floors |
| `defaults.rs` | public | Embedded calibration defaults and their config-extension override key |
| `solver.rs` + `solver/` | crate-private | Sequential bootstrap, global fit (Newton/LM), multi-start, shared helpers; re-exports `SolverConfig` |
| `targets/` | crate-private | Per-step targets: discount, forward, hazard, inflation, vol, swaption, SVI, LMM, base correlation, Student-t, XCCY basis, parametric |
| `validation/` | crate-private | Curve/surface validators, preflight checks, rate bounds |
| `config.rs` | re-exported | `CalibrationConfig` and the per-curve solve configs |
| `report.rs` | re-exported | `CalibrationReport`, `CalibrationDiagnostics`, `QuoteQuality` |
| `step_runtime.rs` | crate-private | Step execution plumbing |
| `prepared.rs` | crate-private | Calibration-side wrappers around `market::build::PreparedQuote` |
| `constants.rs` | crate-private | Shared numerical thresholds for the solvers and targets |

Public re-exports from `crate::calibration`: `CalibrationConfig`,
`CalibrationMethod`, `DiscountCurveSolveConfig`, `HazardCurveSolveConfig`,
`InflationCurveSolveConfig`, `RatesStepConventions`, `ResidualWeightingScheme`,
`SolverConfig`, `CurveValidator`, the surface no-arbitrage validators
(`validate_surface`, `validate_calendar_spread`, `validate_butterfly_spread`,
and friends), `RateBounds`, `RateBoundsPolicy`, `ValidationConfig`,
`ValidationMode`, `CalibrationReport`, `CalibrationDiagnostics`, `QuoteQuality`.

## Envelope structure

A `CalibrationEnvelope` (schema marker `finstack_quant.calibration/1`) carries
quotes in two complementary tracks:

- **Track A — bootstrapping.** `plan.quote_sets` names ID lists resolved against
  `market_data`; `plan.steps` consume them. Step kinds mirror the `StepParams`
  variants: `discount`, `forward`, `hazard`, `inflation`, `vol_surface`,
  `swaption_vol`, `base_correlation`, `student_t`, `hull_white`,
  `cap_floor_hull_white`, `svi_surface`, `xccy_basis`, `parametric`.
- **Track B — snapshot data.** FX matrices, bond prices, equity spots, and
  dividend schedules are not bootstrapped. Supply them as `market_data` entries
  (`fx_spot`, `price`, `dividend_schedule`), or pass already-calibrated objects
  in `prior_market`.

Both tracks may appear in the same envelope; the engine merges `market_data` and
`prior_market` into the working context before running steps.

## Executing a plan

```rust
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_valuations::calibration::api::schema::{
    CalibrationEnvelope, CalibrationPlan,
};
use finstack_quant_valuations::calibration::api::engine;

fn run(plan: CalibrationPlan) -> finstack_quant_core::Result<MarketContext> {
    let envelope = CalibrationEnvelope::new(plan, Vec::new(), Vec::new());

    let result = engine::execute(&envelope)?;
    println!("executed {} steps", result.result.step_reports.len());

    MarketContext::try_from(result.result.final_market)
}
```

`engine::execute` returns a `CalibrationResultEnvelope` whose `result` carries
`final_market` (a `MarketContextState`), a merged plan-level `report`,
`step_reports` keyed by step id, and `results_meta`. Use
`engine::execute_with_diagnostics` when you want the structured `ExecuteError`
with `worst_quote_id`, tolerance, and related detail preserved.

Both hosts take the same envelope JSON but hand back different shapes.
`finstack_quant.valuations.calibrate(envelope_json)` returns a
`CalibrationResult` object whose `.market` getter rebuilds a `MarketContext`,
alongside `.success`, `.report_json`, `.step_ids`, `.iterations`,
`.max_residual`, and `.rmse`. The WASM `valuations.calibrate(envelope)` has no
such wrapper: it returns a plain `CalibrationResultEnvelope` object mirroring
the Rust wire type, so a caller writes `calibrate(env).result.final_market` and
re-ingests that sub-document rather than reading a `.market` property. Runnable
envelope examples live in
[`../../examples/market_bootstrap/`](../../examples/market_bootstrap/README.md).

## Configuration

### Two tolerances

1. **Solver tolerance** (`config.solver.tolerance()`, default `1e-12`) — when
   the numerical root finder stops. `SolverConfig` wraps
   `finstack_quant_core::math::solver::BrentSolver`; this is convergence in
   parameter space, not economic fit.
2. **Validation tolerance** (`config.discount_curve.validation_tolerance` and
   the hazard/inflation equivalents, default `1e-8`) — whether the calibration
   counts as successful. After the solver converges, final residuals are
   compared against it; any residual above the threshold marks the report
   failed.

A precise root is not the same as an accurate reprice, which is why both exist.
Validation tolerances are per-unit-notional residuals.

`CalibrationConfig::fail_on_bad_fit` defaults to `true`: a step whose
`report.success` is false is propagated as
`finstack_quant_core::Error::Calibration` and its output is **not** installed
into the context. Diagnostic workflows that want the report without aborting can
set it to `false`.

`CalibrationConfig::use_parallel` defaults to `false` for determinism.

### Precedence

1. **Step-level** — `CalibrationStep.params` (a `StepParams` variant carrying,
   for example, `DiscountCurveParams { method, .. }`). Highest priority.
2. **Plan-level** — `CalibrationPlan.settings` (`CalibrationConfig`).
3. **Global defaults** — `CalibrationConfig::default()`.

Step-level `method` always wins over the plan-level `calibration_method`, which
serves mainly as runtime state passed from targets to solvers.

`CalibrationMethod` is `Bootstrap` (default) or
`GlobalSolve { use_analytical_jacobian: bool }`.

### Forward-curve method policy

Forward-curve steps **require** `GlobalSolve`. `Bootstrap` is rejected with a
validation error rather than silently reinterpreted: projection discount factors
chain the actual contractual reset/end-date grid, so calendar-adjusted periods
couple adjacent reset rates and must be solved simultaneously.

Forward-curve interpolation knots remain simple fixed-tenor rate controls.
Calibrated curves separately store a validated contractual `projection_grid` of
reset/end-date boundaries, keeping `rate(reset)` and DF-implied
`rate_between(reset, end)` coherent for off-grid 3M periods such as 91- or
92-day Act/360 accruals. Sparse or older curves without that optional grid fall
back to fixed numeric-tenor stepping from zero.

The global forward target enforces `CalibrationConfig::effective_rate_bounds`
per fitted reset-rate parameter, and — until a dedicated forward solve config
exists — borrows `discount_curve.weighting_scheme` and
`discount_curve.validation_tolerance`.

### Rate bounds

`rate_bounds_policy` defaults to `RateBoundsPolicy::AutoCurrency`, deriving
bounds per currency. Set it to `Explicit` to use the `rate_bounds` field
verbatim. `effective_rate_bounds(currency)` resolves the pair.

### Recommended settings

| Use case | Solver tolerance | Validation tolerance | Method |
|----------|------------------|----------------------|--------|
| Forward curves | `1e-12` | `1e-8` | `GlobalSolve` (required) |
| Discount curves | `1e-12` | `1e-8` | `Bootstrap` or `GlobalSolve` |
| Smooth curve fitting | `1e-10` | `1e-8` | `GlobalSolve` |
| Distressed credit | `1e-10` | `1e-6` | `Bootstrap` |
| Real-time pricing | `1e-6` | `1e-4` | target-dependent |
| Interactive exploration | `1e-4` | `1e-2` | target-dependent |

### `compute_diagnostics`

Defaults to `false` to keep solver runs lean. Enabling it adds one
finite-difference Jacobian evaluation per parameter after convergence and
produces:

- per-quote sensitivity: max `|d residual / d param|` per quote
- condition number: `cond(J^T J)` for the finite-difference Jacobian
- consistent RMS / max residual reporting across solvers

Global-solve failures always include the three worst-fit quotes in
`convergence_reason`, diagnostics on or off.

## Reports

`CalibrationReport` carries `success`, `residuals` (a `BTreeMap` keyed by
instrument id, so ordering is stable), `iterations`, `objective_value`,
`max_residual`, `rmse`, `validation_passed`, `validation_error`,
`convergence_reason`, `metadata`, `solver_config`, and `results_meta`, plus five
optional fields: `diagnostics` (populated only when `compute_diagnostics` is
on), `worst_quote_id` and `worst_quote_residual`, `explanation`, and
`model_version`.

## Bumps

`calibration::bumps` is the supported what-if surface: apply a `BumpRequest`
(parallel in basis points, or per-tenor) to a calibrated object and re-run the
matching step. Used by `finstack-quant-scenarios`, CS01, key-rate DV01, and vega.

| Asset class | Entry points |
|-------------|--------------|
| Discount rates | `bump_discount_curve`, `bump_discount_curve_from_rate_calibration`, `bump_discount_curve_with_config`, `bump_discount_curve_synthetic` |
| Forward rates | `bump_forward_curve_from_rate_calibration` |
| Credit hazard | `bump_hazard_spreads`, `bump_hazard_shift` |
| Inflation | `bump_inflation_rates` |
| Volatility | `bump_vol_surface` (takes `VolBumpRequest`) |

Rate and spread bumps are in basis points. Vol bumps use a separate request type
because absolute vol points and relative shifts are different operations.

Synthetic helpers operate directly on curve knots and do **not** recalibrate.
For recalibrated bumps, both base and bumped curves reprice every input quote to
within their calibration tolerances, so residual leakage into a sensitivity is
bounded by roughly (sum of the two repricing tolerances) / (bump size) — on the
order of `2e-10 / 1e-4 = 2e-6` of the PV unit at a 1bp bump, negligible versus
the bump.

## Determinism

- Fixed inputs give identical outputs: Halton multi-start, no system RNG.
- Residual keys use `BTreeMap` ordering.
- Solver loops reuse buffers; parallelism is opt-in via `use_parallel`.

## Extending

**New calibration target**: implement the target trait in `solver/` (bootstrap
targets implement `BootstrapTarget`; global targets implement
`GlobalSolveTarget`), add the target module under `targets/`, add a `StepParams`
variant plus its params struct in `api/schema.rs`, and wire it through
`targets/mod.rs` and the engine.

**New quote-driven instrument**: define the quote type under
[`../market/quotes/`](../market/README.md), add a builder under
`market/build/`, then teach the relevant target to build and price it.

Regenerate schemas after any public wire change: `mise run rust-gen-schemas`,
verify with `mise run rust-check-schemas`.

## Verification

```bash
cargo nextest run -p finstack-quant-valuations --test calibration
cargo nextest run -p finstack-quant-valuations --test credit_calibration
cargo bench -p finstack-quant-valuations --bench calibration
cargo bench -p finstack-quant-valuations --bench global_calibration
```
