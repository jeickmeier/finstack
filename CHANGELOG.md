# Changelog

## Unreleased

### Changed

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
