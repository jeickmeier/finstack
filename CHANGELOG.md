# Changelog

## Unreleased

### Changed

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
