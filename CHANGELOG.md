# Changelog

## Unreleased

### Changed

- Canonicalized editor and agent rules under `.agents/rules`. Cursor and
  Claude now resolve the same reviewed rule tree through checkout-relative
  `.cursor/rules` and `.claude/rules` symlinks.
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
