# Core source layout

Navigation aid for people working *inside* `finstack-quant-core`. It maps
directories and files to responsibilities and records which items are public API
and which are `pub(crate)` plumbing. At ~100k lines it is the second-largest
source tree in the workspace (after `valuations/src`), and nearly every other
crate compiles against it; the split between "public vocabulary" and "internal
helper" is not obvious from a directory listing.

The public-surface map — what a downstream crate should reach for, conventions
that bite, generated assets, bindings — is [`../README.md`](../README.md). Do not
read this file for that; read it for *where things live*. Per-item detail is in
the rustdoc (`cargo doc -p finstack-quant-core --open`).

## Crate-wide lint gates

`lib.rs` sets these for the whole crate, and they shape how code in every
directory below must be written:

```rust
#![forbid(unsafe_code)]
#![warn(clippy::float_cmp)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
```

All four `deny`s are allowed back under `#[cfg(test)]`, along with `float_cmp`
and `indexing_slicing` — the latter is not enabled anywhere in the workspace
today, so its test-scoped `allow` is currently vestigial (see the comment above
`[workspace.lints.rust]` in the root `Cargo.toml`). The practical effect is that
an `unwrap()` which compiles in a `mod tests` block will not compile in the
module above it. Fallible paths return `crate::Result<T>`; every `unreachable!()`
call site that remains lives under `dates/` (in `imm.rs`, `calendar/algo.rs` and
`calendar/generated.rs`), and each enclosing function carries a local
`#[allow(clippy::unreachable)]` plus a justification comment.

## Top-level directories

| Path | Role | Visibility |
|------|------|------------|
| `canonical/` | `value_serializer.rs` — a bespoke `serde::Serializer` into `serde_json::Value` that rejects non-finite `f32`/`f64` at serialization time instead of emitting `null` | private `mod` of `canonical.rs`; items are `pub(super)` |
| `cashflow/` | `discounting.rs` (`npv` and friends over a `Discounting` curve), `primitives.rs` (`CashFlow`, `CFKind`, `CashFlowAccrual`), `xirr.rs` (`irr`, `xirr`, day-count-aware variants) | all three are private `mod`s; the whole surface is re-exported from `cashflow/mod.rs` |
| `contract/` | Vocabulary for loading persisted artifacts under resource limits: `descriptor.rs`, `diagnostics.rs`, `limits.rs`, `load.rs` | all private `mod`s; re-exported from `contract/mod.rs` *and* again at the crate root |
| `dates/` | Dates, calendars, day-count, schedules, tenors, IMM/CDS rolls, periods — see [below](#dates) | `calendar` and `fx` are `pub mod`; everything else is private with re-exports at `dates::` |
| `error/` | `inputs.rs` (`InputError`, `NonFiniteKind`), `suggestions.rs` (fuzzy "did you mean" matching for unknown ids) | `inputs` re-exported publicly; `suggestions`' `format_suggestions`/`fuzzy_suggestions` are `pub(crate)` |
| `expr/` | Scalar DAG expression engine over `&[f64]` columns: `ast.rs`, `ast_walk.rs`, `context.rs`, `dag.rs`, `eval.rs`, `eval_functions.rs` | **all six are private `mod`s**; the public surface is only `Expr`, `ExprNode`, `BinOp`, `UnaryOp`, `Function`, `EvaluationResult`, `SimpleContext`, `CompiledExpr`, `EvalOpts` |
| `generated/` | **Not a module.** Four committed generated tables pulled in with `include!` — see [Generated code](#generated-code) | n/a |
| `market_data/` | Curves, surfaces, scalars, context, bumps — see [below](#market_data) | mixed; most leaf curve modules are private |
| `math/` | Numerics — see [below](#math) | mixed; see the placement policy |
| `money/` | `types.rs` (`Money`, `FormatOpts`), `rounding.rs` (Decimal representation and rounding), `fx/` (`matrix.rs`, `provider.rs`, `providers.rs`, `types.rs`) | `fx` is `pub mod`; `types` and `rounding` are private. `rounding::AmountRepr = rust_decimal::Decimal` and every `repr_*` helper is `pub(crate)` — this is where the Decimal invariant is enforced |
| `types/` | `id.rs` (phantom-typed ids), `rates.rs` (`Rate`/`Bps`/`Percentage`), `ratings.rs`, `attributes.rs`, `barrier.rs` | all private `mod`s, re-exported from `types/mod.rs` |
| `bin/` | `gen_core_schemas.rs` — the `gen_core_schemas` binary that writes `schemas/market_data/*` and `schemas/index.json` | binary target, not part of the library |

## Top-level files

| File | Role | Notes |
|------|------|-------|
| `lib.rs` | Lint gates, module declarations, crate docs, crate-root re-exports | Root re-exports are `HashMap`, `HashSet`, `Error`, `InputError`, `NonFiniteKind`, `Result`, the four `canonical::*` items, and all of `contract::*` except the three `load` helpers (`check_json_limits`, `deserialize_json_value`, `parse_json_value`) |
| `canonical.rs` | `to_canonical_bytes`, `canonical_bytes_of_value`, `content_hash`, `CANONICAL_VERSION` | Objects sorted recursively by UTF-8 key bytes; **array order is preserved**; non-finite floats are an error, not `null` |
| `collections.rs` | `HashMap`/`HashSet` aliases over `rustc_hash` | **`pub(crate) mod`** — deliberately not a public submodule. Import the aliases from the crate root (`finstack_quant_core::HashMap`) |
| `config.rs` | `FinstackConfig`, rounding/scale policy, `RoundingContext`, `ResultsMeta`, `NumericMode`, `results_meta*` | There is no global state; every helper takes a caller-supplied `&FinstackConfig` |
| `currency.rs` | `Currency` behavior, parsing, metadata | The enum itself is `include!`d from `OUT_DIR/currency_generated.rs` |
| `decimal.rs` | `f64_to_decimal`, `decimal_to_f64` | The only sanctioned bridge; both return `Result` rather than collapsing non-finite input |
| `embedded_registry.rs` | `EmbeddedJsonRegistry<T>` — `OnceLock`-cached parse+validate of a compile-time JSON asset, with a `FinstackConfig` extension-key override path | **`pub(crate) mod`**. Its methods are written `pub`, but the module gate makes the whole type crate-internal. Used by `rating_scales` |
| `explain.rs` | `ExplainOpts`, `ExplanationTrace`, `TraceEntry` | Opt-in tracing; off by default |
| `prelude.rs` | Curated re-export list | A convenience layer over canonical paths, not a second source of truth |
| `rating_scales.rs` | `RatingScaleRegistry`, `ScorecardScale`, `RatingLevel`, `UnknownScalePolicy`, `embedded_registry()`, `registry_from_config()` | Backed by `data/rating_scales/` through `EmbeddedJsonRegistry` |
| `schema.rs` | Deterministic JSON Schema assembly: `SerdeSchema`, `SchemaArtifact`, `ARTIFACTS`, `run_schema_generator`, `run_schema_index_generator`, `project_llm`, `externalize_schema_definitions` | 2.9k lines, the largest single file. `ARTIFACTS` is the registration list the `gen_core_schemas` binary walks |
| `serde_guard.rs` | `UnknownFieldGuard` | Restores `deny_unknown_fields` for structs that use `#[serde(flatten)]`, which serde cannot do natively. Must be the **final** flattened field and `#[schemars(skip)]` |
| `table.rs` | `TableEnvelope`, `TableColumn`, `TableColumnData`, `TableColumnRole` | The canonical columnar interchange type. Arrow export lives in the separate `finstack-quant-arrow` crate, which is not re-exported by the umbrella |
| `validation.rs` | `require`, `require_or`, `require_with` | Convention-agnostic invariant checks returning `Result` |
| `versions.rs` | Model-version string constants stamped into calibration reports | Add a constant here rather than a string literal at the call site |
| `wire.rs` | Newtype wrappers whose storage form cannot describe its own JSON contract to `schemars`: `SchemaVersion`, `DateWire`, `DecimalWire`, `PositiveF64Wire`, `NonNegativeF64Wire`, `ClosedUnitIntervalF64Wire`, `OpenUnitIntervalF64Wire`, `CorrelationWire`, `PercentageQuantityWire` | Reach for one of these before hand-writing `#[schemars(with = ...)]` on a field |

<a name="math"></a>

## `math/` (~26.9k LOC)

### Placement policy

From `.agents/rules/rust/code-standards.md`, "Mathematical Utilities
Organization": **mathematics belongs in `core::math` unless it is exclusively
useful to one domain.** A Cholesky factorization, a streaming variance
accumulator, or a Sobol sequence is general and lives here; a GBM path simulator
or a European-call payoff is Monte-Carlo-specific and lives in
`finstack-quant-models`. Before adding a numerical helper anywhere else,
check whether `core::math` already has it, and if you are putting it in a domain
crate be ready to say in the PR why it cannot be generalized.

`monte_carlo`'s `time_grid.rs` and `online_stats.rs` are re-export shims over
`core::math::time_grid` and `core::math::stats` — that is the pattern to follow,
not duplication.

### Module split

| Module | Contents | Notes |
|--------|----------|-------|
| `mod.rs` | `ZERO_TOLERANCE`, `round_half_away`, `clamp_or_nan`, plus the flat `pub use` list | `round_half_away` and `clamp_or_nan` are the shared implementations behind the expression-language `round`/`clamp`; the core vector evaluator and the statements scalar evaluator both delegate here so semantics cannot drift |
| `stats.rs` | `mean`, `variance`, `covariance`, `correlation`, `quantile`, NaN-tolerant `*_or_nan` variants, `OnlineStats`, `OnlineCovariance`, `required_samples` | |
| `special_functions.rs` | `norm_cdf`, `norm_pdf`, `erf`, `ln_gamma`, `standard_normal_inv_cdf`, `student_t_cdf`, `student_t_inv_cdf` | |
| `linalg.rs` | `cholesky_decomposition`, `cholesky_correlation`, `symmetric_eigen`, `build_correlation_matrix`, `validate_correlation_matrix`, `apply_correlation`, `apply_lower_triangular`, `CholeskyError`, `CorrelationFactor` | |
| `random.rs` + `random/` | `random.rs` holds `RandomNumberGenerator`, `Pcg64Rng`, `box_muller_transform`; the sibling directory holds `sobol.rs` (`SobolRng`, `MAX_SOBOL_DIMENSION`), `brownian_bridge.rs`, `poisson.rs`, all `pub mod` | Both a `random.rs` and a `random/` directory exist; the file is the module root |
| `integration.rs` | `gauss_legendre_integrate` (fixed / composite / adaptive), `GaussHermiteQuadrature`, `GaussLaguerreQuadrature` | |
| `solver.rs` | `Solver` trait, `NewtonSolver`, `BrentSolver`, `BracketHint` | The selection guide in `math/mod.rs` rustdoc says which to use per problem |
| `solver_multi.rs` | `LevenbergMarquardtSolver`, `AnalyticalDerivatives` | Systems and multi-dimensional minimization |
| `summation.rs` | `kahan_sum`, `neumaier_sum`, `NeumaierAccumulator` | |
| `interp/` | `generic.rs` (`Interpolator`), `traits.rs` (`InterpFn`, `InterpolationStrategy`), `strategies.rs` (five strategies), `types.rs` (`InterpStyle`, `ExtrapolationPolicy`, `ValidationPolicy`), `utils.rs` (`validate_knots`, `interp_knots_flat`) | `generic` and `traits` are private `mod`s; `strategies`, `types`, `utils` are `pub(crate) mod` with selected items re-exported. So `math::interp::ExtrapolationPolicy` is public API, while the path it is actually defined at — `math::interp::types::ExtrapolationPolicy` — is not reachable outside the crate |
| `distributions.rs` | `binomial_pmf_all`, `binomial_probability`, `chi_squared_quantile`, `log_factorial` | |
| `probability.rs` | `CorrelatedBernoulli`, `correlation_bounds`, `joint_probabilities` | |
| `volatility/` | See below | |
| `compounding.rs` | `Compounding` conversions between simple, periodic, and continuous | |
| `time_grid.rs` | `TimeGrid`, `TimeGridError`, `map_date_to_step`, `map_dates_to_steps`, `map_exercise_dates_to_steps` | |
| `piecewise.rs` | `PiecewiseConstantCurve` — validated left-continuous piecewise-constant curve | |
| `fractional.rs` | `HurstExponent`, `RiemannLiouvilleKernel`, `fbm_covariance*`, `mittag_leffler` | Rough-volatility support |
| `consecutive.rs` | `count_consecutive` | Longest win/loss streaks for return series |

Volatility pricing, fitting, evaluation, extrapolation, and convention
conversion live in `finstack-quant-models`. Core retains only the neutral
market-data artifacts and generic mathematical primitives they consume.

<a name="market_data"></a>

## `market_data/` (~28.7k LOC)

| Path | Role | Visibility |
|------|------|------------|
| `mod.rs` | Module docs; re-exports `MarketContext`, `DiscountCurve`, and the dividend types | |
| `traits.rs` | `TermStructure`, `Discounting`, `Forward`, `Survival` — the minimal polymorphic curve surface, all `Send + Sync` | `pub mod` |
| `context/` | `MarketContext` and its operations: `curve_storage.rs`, `getters.rs`, `insert.rs`, `ops_bump.rs`, `ops_roll.rs`, `state_serde.rs`, `stats.rs` | All seven are private `mod`s. Public: `CurveStorage`, `ContextStats`, and the `state_serde` set (`MarketContextState`, `CurveState`, `CreditIndexState`, `build_snapshot_fx_matrix`, `MARKET_CONTEXT_STATE_CONTRACT`, `MARKET_CONTEXT_STATE_VERSION`). `for_each_context_curve` is `pub(crate)` |
| `term_structures/` | One file (or directory) per curve family: `base_correlation`, `basis_spread_curve`, `credit_index`, `discount_curve/`, `flat`, `forward_curve`, `forward_variance`, `hazard_curve`, `inflation`, `parametric_curve`, `price_curve` (prices and volatility-index levels, selected by `PriceCurveKind`), `rate_calibration` | **Every leaf is a private `mod` except `forward_variance` (`pub mod`).** `common/` is `pub(crate) mod` (shared conventions, interp glue, knot ops — all `pub(crate) use`). Curve types are re-exported from `term_structures/mod.rs`; that file is the single canonical path |
| `term_structures/discount_curve/` | `mod.rs` defines the `DiscountCurve` struct and `DEFAULT_MIN_FORWARD_TENOR`; `builder.rs`, `curve.rs`, `traits.rs`, `transform.rs`, `validation.rs` are private `mod`s | Only `DiscountCurveBuilder` and `ValidationMode` are re-exported here; the struct itself is declared in `mod.rs` |
| `surfaces/` | `vol_surface.rs`, `vol_cube.rs`, `delta_vol_surface.rs`, `fx_delta_vol_surface.rs` | `fx_delta_vol_surface` is `pub mod`; the other three are private. `FxDeltaVolSurfaceBuilder` comes from `delta_vol_surface`, `FxDeltaVolSurface` from `fx_delta_vol_surface` |
| `scalars/` | `primitives.rs` (`MarketScalar`, `ScalarTimeSeries`, `SeriesInterpolation`), `inflation_index.rs`, `storage.rs` (`TimeSeriesStorage`) | All private `mod`s; `storage` has no public re-export at all |
| `bumps.rs` | `BumpSpec`, `BumpType`, `BumpUnits`, `BumpMode`, `MarketBump`, `Bumpable` — the scenario/greeks perturbation vocabulary | `pub mod` |
| `diff.rs` | `measure_*_shift` — realized shift between two contexts, used to verify a bump did what it claimed and for metrics-based attribution | `pub mod` |
| `dividends.rs` | `DividendEvent`, `DividendKind`, `DividendSchedule`, `DividendScheduleBuilder` | `pub mod`, also re-exported at `market_data::` |
| `fixings.rs` | The `FIXING:{forward_curve_id}` lookup convention over `ScalarTimeSeries` in a `MarketContext` | `pub mod` |
| `hierarchy/` | `builder.rs`, `completeness.rs`, `resolution.rs` — tagged tree over `CurveId`s for scenario targeting | All private `mod`s |

<a name="dates"></a>

## `dates/` (~11.7k LOC)

`dates/mod.rs` is a facade: it re-exports the `time` crate types (`Date`,
`Duration`, `Month`, `OffsetDateTime`, `PrimitiveDateTime`) rather than wrapping
them, and pulls almost every child module's surface up to `dates::`.

| Path | Role | Visibility |
|------|------|------------|
| `mod.rs` | Facade, `create_date`, `parse_iso_date`, `days_since_epoch`, `date_from_epoch_days`, year-length constants | |
| `calendar/` | `types.rs` (`Calendar`, `WeekendRule`), `rule.rs` (`Rule`, `Direction`, `Observed`), `business_days.rs` (`adjust`, `BusinessDayConvention`, `HolidayCalendar`, `available_calendars`), `composite.rs`, `algo.rs` (lunar-festival algorithms), `generated.rs` (`BASE_YEAR`/`END_YEAR` + `nth_weekday_of_month`) | `pub mod calendar`, but **all six children are `pub(crate) mod`** — reach items through `dates::calendar::*` or `dates::*` re-exports. The private `calendars_generated` module `include!`s `OUT_DIR/calendars.rs` and is glob-re-exported |
| `daycount/` | `act_act.rs`, `thirty360.rs`, `other.rs`, `context.rs` | All four private `mod`s; `DayCount`, `DayCountContext`, `Thirty360Convention` etc. surface at `dates::` |
| `schedule_iter.rs` | `Schedule`, `ScheduleBuilder`, `ScheduleSpec`, `StubKind`, `ScheduleWarning`, `ScheduleErrorPolicy` | Private `mod`, fully re-exported |
| `schedule_gen.rs` | Generation internals behind the builder: `is_cds_roll_date`, `is_imm_roll_date`, `generate_imm_dates`, `enforce_monotonic_and_dedup`, `BuilderInternal` | Private `mod`, `pub(super)` items — **no public surface** |
| `imm.rs` | IMM / CDS / SIFMA / equity-expiry roll helpers; `include!`s `OUT_DIR/sifma_settlements_generated.rs` | Private `mod`, fully re-exported |
| `tenor.rs` | `Tenor`, `TenorUnit` | Private `mod` |
| `periods.rs` | `Period`, `PeriodId`, `PeriodKind`, `PeriodPlan`, `FiscalConfig`, `build_periods`, `build_fiscal_periods` | Private `mod` |
| `date_extensions.rs` | `DateExt` | Private `mod` |
| `fx.rs` | Joint two-calendar FX settlement: `resolve_calendar`, `adjust_joint_calendar`, `add_joint_business_days`, `fx_spot_date`, `fx_standard_spot_lag_days`, `ResolvedCalendarPair` | **`pub mod fx`** — one of only two public child modules under `dates`. Consumed by `finstack-quant-valuations`' FX instruments |

<a name="generated-code"></a>

## Generated code

`src/generated/` is **not a Rust module** — it holds four committed files pulled
in by `include!`, and nothing declares `mod generated;`.

| File | Included by | Produced by |
|------|-------------|-------------|
| `currency_generated.rs` (`Currency` enum, `MINOR_UNITS`) | `src/currency.rs`, via `OUT_DIR` | No in-repo generator; `build/currency_build.rs` only *copies* it into `OUT_DIR` so rust-analyzer can see it |
| `holiday_generated.rs` (`BASE_YEAR`, `END_YEAR`) | `src/dates/calendar/generated.rs` | No in-repo generator |
| `cny_generated.rs` (Chinese New Year dates) | `src/dates/calendar/algo.rs` | No in-repo generator; paired with `data/chinese_new_year.csv` |
| `festivals_generated.rs` (Dragon Boat, Mid-Autumn) | `src/dates/calendar/algo.rs` | `data/gen_lunar_festivals.py`, which rewrites the table and both CSVs |

Two more generated files never land in `src/` — `build.rs` writes them straight
to `OUT_DIR` and they *are* rebuilt when their inputs change:
`calendars.rs` (from `data/calendars/*.json`, `include!`d by
`dates/calendar/mod.rs`) and `sifma_settlements_generated.rs` (from
`data/sifma_settlements.csv`, `include!`d by `dates/imm.rs`).

The practical consequence: editing `data/iso_4217.csv` or
`data/chinese_new_year.csv` alone changes nothing. Editing
`data/calendars/*.json` changes the next build. See
[`../README.md`](../README.md#generated-assets) for the full table.

## Conventions when adding to this tree

- **One canonical path per item.** Leaf modules stay private and their `mod.rs`
  owns the `pub use` list. `prelude.rs` is a curated convenience layer on top of
  those paths, never a second definition site.
- **Errors.** Everything fallible returns `crate::Result<T>` (`error::Error`).
  Add a variant with context rather than formatting a string at the call site;
  `Error` and `InputError` are `#[non_exhaustive]`. Downstream crates
  `analytics`, `attribution`, `cashflows`, `covenants`, `features`, `margin`,
  `monte_carlo` and `statements-analytics` reuse this `Error` directly, so a
  new variant is a workspace-visible change.
- **Numerics.** `f64` for model internals; `Money` (Decimal) for monetary
  amounts. `decimal::{f64_to_decimal, decimal_to_f64}` is the only sanctioned
  bridge. No `F` type alias exists in this workspace.
- **Determinism.** `HashMap`/`HashSet` are `rustc_hash` aliases with no random
  seed. Use `BTreeMap` when you need sorted iteration for serialization or
  goldens. Never read the system clock — take dates as parameters.
- **Serde.** New inbound types deny unknown fields; if the struct uses
  `#[serde(flatten)]`, add `serde_guard::UnknownFieldGuard` as the final
  flattened field and `#[schemars(skip)]` it. Reach for a `wire.rs` newtype
  before hand-writing `#[schemars(with = ...)]`.
- **New schema artifact.** Implement `SerdeSchema` and register the type in
  `schema::ARTIFACTS`; unregistered types are silently absent from the generated
  index. Regenerate with `mise run rust-gen-schemas`, verify with
  `mise run rust-check-schemas`.
- **New math helper.** Read the placement policy above before putting it in a
  domain crate.
- **New dependency.** Every domain crate in the workspace depends on this one —
  the only exceptions are `finstack-quant-valuations-macros` (proc-macro) and
  `finstack-quant-test-utils` — so adding to `Cargo.toml` here is a
  workspace-wide decision.

## Tests and benchmarks

Unit tests stay in `#[cfg(test)] mod tests` next to the code. Everything else
lives outside `src/`:

- Integration tests: [`../tests/README.md`](../tests/README.md)
- Golden fixtures: [`../tests/golden/README.md`](../tests/golden/README.md)
- Criterion suites (11 registered targets; `autobenches = false`, so a new file
  does nothing until it is added as a `[[bench]]` in `Cargo.toml`):
  [`../benches/README.md`](../benches/README.md)

```bash
mise run rust-test                     # cargo nextest, workspace minus finstack-quant-py
cargo nextest run -p finstack-quant-core
mise run rust-lint                     # cargo fmt --check + clippy -D warnings
mise run rust-bench                    # all Criterion targets, reduced timing
mise run rust-doc                      # rustdoc build + the workspace doc tests
```

Do not run a bare `cargo test` — it pulls in doc tests across the workspace.
`mise run rust-doc` is the sanctioned path when you want the rustdoc examples
checked.

Nothing in this README is compiled. No README in this repository is wired into
rustdoc, so any symbol named here can rot silently; grep before trusting it.

## Related

- [`../README.md`](../README.md) — crate README: public surface, conventions,
  generated assets, bindings, verification
- [`../../monte_carlo/src/README.md`](../../monte_carlo/src/README.md) — the
  sibling source-layout README this file is modelled on
- [`../../valuations/src/market/README.md`](../../valuations/src/market/README.md)
  — the valuations-side consumer of `market_data`
