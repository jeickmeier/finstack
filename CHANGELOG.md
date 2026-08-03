# Changelog

## Unreleased

### Changed

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
