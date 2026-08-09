# Changelog

## Unreleased

### Added

- **Python:** ~90 `to_dataframe` / `to_*_dataframe` exports across portfolio,
  statements, statements-analytics, margin, scenarios, analytics, core.credit,
  core.market_data, monte_carlo, factor-model and valuations.correlation. Every
  frame documents its columns; `Money` becomes a float column plus one
  `currency` column, never a nested cell. 52 result types also gained
  `_repr_html_`, so they render as tables in Jupyter.
- **Python:** pickle support on every wrapper that round-trips through JSON, so
  results survive `multiprocessing` / `joblib` / `dask`. `copy.deepcopy` works
  through the same path. `PortfolioOptimizationResult` is the one exception: its
  Rust type has no `Deserialize`.
- **Python:** `finstack_quant.<domain>.schema` for `valuations`, `statements`,
  `factor_model` and `cashflows`, exposing the JSON Schemas compiled into the
  extension, so they can never drift from the installed wheel.
- **Python:** `finstack_quant.__version__`, sourced from the crate version.
- **Python:** `FinstackError` (`finstack_quant.core`) as the common base for the
  library's named exceptions. It derives from `ValueError`, so every existing
  `except ValueError` is unaffected. `CalibrationEnvelopeError` stays outside the
  tree because it is a `RuntimeError` and PyO3 cannot express two bases.
- **Rust, JSON:** clean-price CDS-option strikes (`CDSOptionStrike::CleanPricePct`),
  the CDX HY market convention, alongside the existing forward-spread strikes.
  Price strikes carry `strike_index_factor` (the strike's original index factor
  `f0`) and are validated as index-only, no-knockout, with an explicit positive
  coupon, factors in `(0, 1]`, `f <= f0`, and realized loss bounded by the removed
  original notional. Delta and gamma branch by strike kind: a clean price is not a
  valid argument to the Black `d₁`, so price strikes use a curve-reprice hedge
  ratio (option CS01 over underlying spread DV01 under a symmetric ±1 bp par-quote
  bump with hazard rebootstrap, sticky native strike, sticky surface volatility),
  and gamma is the change in that delta across the ±5 bp screen bump.
- **Rust:** a two-factor rates-credit lattice (`models::trees::two_factor_rates_credit`)
  for callable credit-risky bonds and term loans, with public `ModelConfig` inputs
  `hazard_volatility`, `hazard_mean_reversion` and `rate_credit_correlation`. One
  `resolve_rates_credit_config()` owns the public-config to lattice-input mapping
  for both engines. Correlation feasibility is proved against the per-node Fréchet
  bounds at calibration time rather than clamped during pricing, and
  `HazardFloorSaturation` reports how much state-price mass sat on the zero-hazard
  floor. Mean-reversion speeds above `KAPPA_MAX` are rejected — use `HullWhiteTree`.
- **Rust:** future floating-coupon resets valued at lattice nodes (`NodeCoupon`).
  The deterministic projection stays booked as before; the lattice adds only the
  node-dependent increment, which vanishes identically as `rate_vol → 0`, so an
  option-free floater's PV is invariant to rate volatility. Caps and floors on a
  floating leg are therefore priced as the caplet/floorlet strip they are. Call
  dates strictly inside a future floating period, and future floating PIK, are
  rejected rather than mispriced.
- **Rust:** `models::credit::market_anchored`, the shared fractional-to-absolute
  credit-volatility mapping (credit triangle `s = (1 − R)·λ`), used by both the
  callable lattice and the revolving-credit CIR path so they cannot drift apart.
  `CreditVolatilityConversion` carries every input and output together, so a 35%
  CDS-option quote cannot reach an additive hazard lattice without the 1.05%
  absolute figure sitting beside it. This is an explicit first-order local
  mapping, not a calibration.
- **Rust:** `market::credit_option_vol`, which resolves a CDX/iTraxx option-vol
  surface point into that additive hazard volatility. The surface is queried
  strictly at the strike's **native displayed** coordinate — a decimal spread for
  CDX IG and iTraxx, a clean price in percentage points (`107.0`, never `1.07`
  and never a spread equivalent) for CDX HY — and the index-derived *fractional*
  vol is anchored on the *target* curve's own reference hazard, so a single-name
  bond is never anchored to the index's hazard level. Selectors are `Strike`,
  `Moneyness` (against the native ATM-forward coordinate) and `Delta`.

### Changed

- **Breaking (Rust, JSON, Python, WASM):** the CDS-option strike is a typed enum
  instead of a bare decimal. `CDSOption.strike` and `CDSOptionParams.strike` are
  now `CDSOptionStrike`, whose canonical JSON is externally tagged, and **the old
  scalar wire shape is rejected with no compatibility fallback**:

  ```json
  { "strike": "0.0325" }                      // before — now rejected
  { "strike": { "spread": "0.0325" } }        // after, forward-spread strike
  { "strike": { "clean_price_pct": "107.0" } } // after, CDX HY clean-price strike
  ```

  Persisted `CDSOption` payloads must be migrated; Python and WASM reach CDS
  options through this JSON, so they are affected identically. `Spread` stays a
  decimal annual rate (`0.0325` = 325 bp) and `CleanPricePct` is quoted in
  percentage-price points (`107.0` = fraction `1.07`) — use
  `clean_price_fraction()` rather than re-dividing by 100. Spread strikes reject
  `strike_index_factor` as inert. `effective_underlying_cds_coupon` is now
  fallible, since a clean-price strike can never serve as the running coupon, and
  `settlement` is explicit on `CDSOptionParams` (default `Cash`).
- **Breaking (Rust behavior):** the callable rates-credit path reads only
  `hw1f_sigma` / `hw1f_mean_reversion` and **rejects** the legacy
  `implied_volatility` / `mean_reversion` channel when its canonical counterpart
  is absent, so a configuration cannot silently flip pricing regime; `hw1f_*` wins
  when both are set. Hazard inputs supplied without a `credit_curve_id` are
  rejected rather than ignored.
- **Breaking (Rust behavior, PV-affecting):** `RatesCreditConfig::default()` is
  now deterministic in both factors. The callable-bond path had been inheriting an
  undeclared `hazard_vol = 0.20` — worth roughly 33% of PV on the reference
  fixture — from `..Default::default()` construction. Callers that want a
  stochastic factor must now declare it explicitly, so previously-priced callable
  credit-risky bonds will change value.
- **Breaking (Rust behavior):** CDS-option volatility resolution is strict. An
  instrument-level implied-vol override wins; otherwise the surface is looked up
  at the native strike coordinate with a required `VolSurfaceAxis::Strike` and
  `VolQuoteType::BlackLognormal`, and out-of-grid coordinates error instead of
  clamping to the nearest edge. Delta and gamma screen metrics share this
  resolver. Valuing a physically-settled option at or after expiry now fails
  explicitly; the exercise/delivery lifecycle is not modelled.
- **Fixed (Rust):** bond rate vega bumped `market_quotes.implied_volatility`,
  which the rates-credit path no longer reads, producing a silently **zero** vega
  on every credit-risky callable (or an error when `hw1f_sigma` was unset). Vega
  now bumps whichever channel the instrument's own routing consumes.
- **Breaking (Python):** 40 `Performance` metrics return a `pandas.Series`
  indexed by ticker (with `.name` set to the metric) instead of `list[float]`;
  `skew_kurt` and `value_at_risk_and_es` return a tuple of two Series. Positional
  access must become `.iloc[i]` — `perf.sharpe()[0]` warns on pandas 2 and raises
  on pandas 3. `perf.sharpe()["FUND"]` is the preferred form, and
  `pd.concat([...], axis=1)` now yields correctly named columns.
- **Breaking (Python):** the pricers return typed results instead of JSON
  strings — `price_instrument` / `price_instrument_with_metrics` →
  `ValuationResult`, and `structured_credit_tranche_oas` / `_metrics` /
  `_scenario_table` → `OasResult` / `TrancheMetrics` / `ScenarioTable`. Replace
  `ValuationResult.from_json(price_instrument(...))` with the call itself, and
  `json.loads(result)` with `result.to_json()` where the wire payload is still
  wanted. `instrument_cashflows_json` is unchanged and still returns `str`.
- **Breaking (Python):** `ModelBuilder` / `MixedNodeBuilder` configuration
  methods return the builder instead of `None`, so calls chain. Statement-per-line
  code is unaffected (the object is mutated in place and the returned value *is*
  the same builder); the only visible change is that a notebook cell ending on a
  setter now echoes a builder repr. `build()` and `mixed()` remain terminal.
- **Python:** every date-valued `as_of` parameter accepts a `datetime.date` /
  `pandas.Timestamp` as well as an ISO string — the pricers, the scenario entry
  points, the `structured_credit_tranche_*` family, the portfolio sensitivity,
  stress, what-if and aggregation functions, `accrued_interest_json`,
  `evaluate_engine`, `decompose_levels` and `run_corporate_analysis`. The two
  functions that take a fiscal *period* rather than a date
  (`credit_assessment`, `credit_assessment_report`, e.g. `"2025Q4"`) still take
  a string, since a calendar date has no meaning there. A malformed date string
  is now rejected by the shared extractor, so the `ValueError` names the
  offending value and the reason and reads the same at every entry point.
- **Python:** zero-row result frames now carry their real dtypes instead of
  `object`. Concatenating an empty frame with a populated one previously
  downgraded every column, so a numeric column silently stopped being numeric
  and `groupby().sum()`, arithmetic and `to_parquet` broke — on the common path
  of iterating a book where some entries produce no rows.
- **Python:** `PnlAttribution.to_dataframe()` now raises `ValueError` when a
  factor's currency differs from `total_pnl`'s. The single-row frame carries one
  `currency` label for every factor, so a mixed-currency attribution would have
  made `df[factors].sum(axis=1)` add unlike units. Use `to_long_dataframe()`,
  which carries currency per row, for genuinely mixed input.

- Canonicalized editor and agent rules under `.agents/rules`. Cursor and
  Claude now resolve the same reviewed rule tree through checkout-relative
  `.cursor/rules` and `.claude/rules` symlinks.
- **Breaking (Rust, Python, JSON):** Removed `VolSurfaceKind` and the redundant
  `surface_kind` field from direct and hierarchy volatility-surface shocks;
  `vol_surface_id` now fully identifies the target surface.
- **Breaking (Rust, Python, JSON):** Removed the unused
  `OperationSpec::BaseCorrBucketPts.maturities` field and Python argument;
  base-correlation shocks target the supported detachment dimension only.
- **Breaking (Rust, JSON):** Removed the unused
  `IndexUnderlyingParams.convexity_id` field and its `with_convexity` builder;
  fixed-income index TRS pricing never consumed the identifier.
- **Breaking (Rust, JSON, WASM):** Removed the unwired
  `RateQuote::Futures.vol_surface_id` field and its persisted
  `RateCalibrationQuote` copy. Futures calibration continues to use the
  explicit `convexity_adjustment` value.
- **Breaking (Rust, Python):** Removed statement formula alias and fuzzy-name
  rewriting, including `ModelBuilder::with_name_normalization`. Formulas and
  `where` clauses now require exact node IDs; unknown IDs retain the dependency
  graph's nearest-name diagnostics.
- **Breaking (Python):** Removed the 13-argument `SaCcrTrade` constructor,
  which inferred supervisory delta and option classification from direction.
  Construct trades with `SaCcrTrade.from_json` and the complete canonical
  schema; deserialization now immediately applies the Rust SA-CCR regulatory
  validator.
- **Breaking (Rust, ECL policy JSON):** Simplified ECL staging and
  schedule-based calculation now use canonical `EclStageRequest` and
  `EclRequest` surfaces. The duplicate binding-default policy block, its seven
  Rust getters, and `compute_ecl_weighted_from_schedules` were removed; Python
  retains its established signatures and error types while delegating all
  policy, exposure, scenario, and configuration construction to Rust.
- **Breaking (Rust, JSON, Python):** Removed the unused
  `ScenarioDefinition.model_id` field and Python `ScenarioSet.model_ids`
  constructor input. Scenario-set JSON containing `model_id` is now rejected
  by the existing unknown-field validation.
- **Breaking (Rust, Python):** Corporate analysis now stores direct
  `CreditContextMetrics` per instrument instead of the speculative
  `CreditInstrumentAnalysis` wrapper. Non-positive DCF enterprise-value
  suppression is reported once at the top level as
  `ev_suppressed_non_positive`.
- **Breaking (Rust):** Removed the unused
  `CreditScoringError::OutOfRange` variant. Credit scoring input failures
  continue to use `NonFiniteInput` and `InvalidBinaryIndicator`.
- **Breaking (Rust):** Removed the unused `MigrationError::NotSquare` variant.
  Credit migration matrix shape failures continue to use `DimensionMismatch`.
- **Breaking (Rust):** Removed the unused
  `InputError::JointCalendarNonConvergent` variant. Active joint-calendar safety
  failures continue to use `JointCalendarIterationLimitExceeded`.
- **Breaking (Rust):** `ThresholdSchedule::new` now returns `Result` and is the
  sole threshold-schedule constructor; `try_new` was removed. Deserialization
  routes through the same finite-value and unique-date validation.
- **Breaking (Python):** Removed the module-level `price_european_call` and
  `price_european_put` functions. Use `EuropeanPricer.price_call` /
  `price_put` or `McEngine.price_european_call` / `price_european_put`.
  WASM retains `priceEuropeanCall` and `priceEuropeanPut` as
  function-oriented browser entry points.
- **Breaking (Rust, Python, WASM):** Kyle calibration now requires an explicit
  `reference_price` and returns price-space lambda. The working `*_with_mid`
  implementations now own the canonical `KyleLambdaModel::lambda_from_series`
  and `from_amihud` names; the legacy fail-closed signatures were deleted.
- **Breaking (Rust):** `almgren_chriss_uniform_impact` now returns the canonical
  `ImpactEstimate`; the duplicate `AlmgrenChrissImpactView` was removed.
  Python and WASM retain the established four-key host payload, mapping
  `total_cost` to `total_impact` and `cost_bp` to `expected_cost_bp`.
- **Breaking (Rust):** Removed the no-op `JumpEuler::with_max_jumps`
  constructor. Use `JumpEuler::new`; the aggregate jump sampler remains
  uncapped.
- **Breaking (Rust behavior):** Statement formula checks now use the canonical
  statements DSL evaluator, including time-series functions and `cs.*`
  references. `CheckSuiteSpec::resolve()` materializes formula checks directly;
  the duplicate analytics resolver was removed, and missing references or
  evaluation failures are returned instead of being silently skipped.
- Comparable-company flat field construction, named-field access, and metric
  selector parsing now share one canonical Rust implementation across scoring,
  Python, and WASM; accepted fields and scoring behavior are unchanged.
- **Breaking (Rust, Python, WASM):** Statement-model Monte Carlo now belongs
  exclusively to `finstack-quant-statements`. Import `MonteCarloConfig`,
  `MonteCarloResults`, and `run_monte_carlo` from `finstack_quant.statements`,
  or call `statements.runMonteCarlo` in WASM; the duplicate
  `statements_analytics` facade was removed without changing JSON payloads.
- **Breaking (Python):** `reporting.attribution_tearsheet` is now
  presentation-only and accepts only a precomputed `PnlAttribution` or its
  canonical JSON/dict payload. Inline instrument/market attribution parameters
  were removed; compute attribution through `finstack_quant.attribution` before
  rendering.
- **Breaking (Python):** `reporting.instrument_tearsheet` is now presentation-only
  and requires a precomputed `ValuationResult`; its `market`, `as_of`, `model`,
  and `market_price` parameters were removed. The presentation-owned
  `recommended_metrics` helper was also removed, so callers select metrics in
  the valuations API before rendering.
- **Breaking (Rust/Python):** Removed the unused SA-CCR engine
  `reporting_currency` field, builder option, and Python constructor argument.
  SA-CCR monetary inputs must already use one consistent currency; the engine
  does not perform currency conversion.
- **Breaking (Rust):** Removed the unused FRTB SBA engine
  `reporting_currency` field and builder option. Currency remains explicit on
  `FrtbSensitivities`, where it participates in the regulatory input contract.
- **Breaking (Rust):** Removed the speculative FRTB parameter-bundle and
  revision APIs, including `FrtbParams`, `FrtbRevision`, the JSON-overlay
  registry, and the corresponding `FrtbSbaEngine` builder and accessors. FRTB
  SBA continues to use the fixed BCBS d457 constants under
  `regulatory::frtb::params`.
- **Breaking (Rust):** Removed the duplicate scenario tenor and period parsing
  helpers, including their context wrappers. Use
  `finstack_quant_core::dates::Tenor` directly for parsing, simple year/day
  approximations, and calendar-aware year fractions.
- **Breaking (Rust):** `InterpolationResult` and
  `calculate_interpolation_weights` are now crate-private scenario adapter
  details and no longer part of the public or serialized API.
- **Breaking (Rust):** `TemplateRegistry` now has one validated registration
  path. Use the fallible `TemplateRegistry::with_embedded_builtins`,
  `register_json_template_str`, or `load_json_dir`; the builder-factory
  `register` / `register_with_components` methods and panicking `Default`
  implementation were removed.
- **Breaking (Rust):** Simplified the valuations surface. Use
  `schema::instrument_schema("bond")`, `TreePricer::calculate_oas`, and the
  free `solve_ytm` function in place of the removed bond wrappers and YTM
  solver objects. Tranche loss methods now use their stored balance, the
  valuations prelude no longer re-exports the core prelude, unused constants
  were removed, and rate-exotic Monte Carlo settings must be constructed as
  typed `RateExoticMcConfig` values.
- **Breaking (Rust):** Removed low-value public aliases and helpers from the
  foundational crates: use `analytics::regression::constrained_least_squares`
  and the correlation crate-root exports; construct `DatedSeries` with its
  public fields or `Default`; evaluate covenant schedules through covenant
  APIs; and import core `Error` / `Result` directly. The unused credit-calibrator
  `config` and `diagnostics` accessors were also removed.
- **Breaking (Rust, JSON, Python, WASM):** Removed the duplicate feature-operation
  names `clip_by_quantile` / `ClipByQuantile` and
  `dollar_neutral_weights` / `DollarNeutralWeights`. Use `winsorize` /
  `CrossSectionalOp::Winsorize` and `long_short_weights` /
  `CrossSectionalOp::LongShortWeights` in typed Rust calls, JSON panel specs,
  and Python or WASM string-dispatched calls.
- **Breaking (Rust):** Removed the public `CagrBasis` and
  `AnnualizationConvention` configuration types. `Performance::cagr()` keeps
  its existing date-based Act/365.25 behavior.
- **Breaking (Rust):** Factor-model configuration, covariance, envelope,
  primitive, and sensitivity types are now exposed only through their existing
  crate-root paths. The `matching`, `credit`, and `schema` modules remain public.
- **Breaking (Rust):** Removed the unused `FactorModelError` hierarchy. Factor-model
  workflows continue to return their canonical core or portfolio errors, and
  `UnmatchedPolicy` remains available at the factor-model crate root with the
  same `snake_case` JSON representation.
- **Breaking (Rust):** Renamed the serialize-only scenario outputs
  `ScenarioRevalueEnvelope` and `ScenarioPnlEnvelope` to
  `ScenarioRevalueView` and `ScenarioPnlView`, and renamed their helpers to
  `apply_and_revalue_view` and `scenario_pnl_view`. No deprecated aliases are
  provided because these pre-1.0 outputs are not round-trip persistence
  envelopes.
- **Breaking (JSON):** Attribution request deserialization now rejects missing
  or mismatched `schema` markers, matching the published schema `const` and
  existing attribution-result behavior.
- **Breaking (JSON):** Margin serialization and deserialization now enforce the
  published bounds for MPOR, collateral maturities, haircuts, concentration
  limits, default haircuts, and notification hours. Invalid publicly
  constructible values fail serialization instead of producing JSON that the
  same type cannot deserialize.
