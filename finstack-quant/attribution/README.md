# finstack-quant-attribution

Multi-period P&L attribution for individual instruments. Decomposes mark-to-market
change between two dates (T₀ → T₁) into contributions from carry, rates curves,
credit curves, inflation, correlations, FX, volatility, model parameters, and
market scalars.

Attribution is layered by cost and fidelity. The lightest entry points reprice
once per date, the heaviest perform per-factor bump-and-reprice loops. Pick the
cheapest method that answers your question — every additional tier adds
repricing cost and operational moving parts.

## Methodologies

| Tier         | Entry point                                                | Behavior                                                                                       |
|--------------|------------------------------------------------------------|------------------------------------------------------------------------------------------------|
| Minimal      | [`simple_pnl_bridge`](src/lib.rs)                          | Scalar `value(T₁) − value(T₀)` in target currency. No decomposition.                          |
| Linear       | [`attribute_pnl_metrics_based`](src/metrics_based/)      | Linear (and optional second-order) approximation from precomputed metrics. No extra repricing. |
| Parallel     | [`attribute_pnl_parallel`](src/parallel.rs)                | Isolate one factor at a time (T₀ for that factor, T₁ elsewhere). Residual carries cross-effects. |
| Waterfall    | [`attribute_pnl_waterfall`](src/waterfall.rs)              | Apply factors in order; per-factor P&Ls sum to total P&L up to tolerance. Order matters.       |
| Taylor       | [`attribute_pnl_taylor`](src/taylor.rs)                    | First- and optional second-order sensitivity expansion from bump-and-reprice Greeks.           |

Default waterfall order (from [`default_waterfall_order`](src/waterfall.rs)):

```
Carry → RatesCurves → CreditCurves → InflationCurves → Correlations
      → Fx → Volatility → ModelParameters → MarketScalars
```

## Factors

`AttributionFactor` in [`types.rs`](src/types.rs) enumerates the nine factor
families. Each populates a top-level field on `PnlAttribution` (`carry`,
`rates_curves_pnl`, …) and, when requested, an optional `*_detail` struct with
finer breakdowns:

- **Carry** — theta, accrual, pull-to-par, financing.
- **RatesCurves** — per-curve and optional per-tenor IR risk
  (`RatesCurvesAttribution`).
- **CreditCurves** — per-hazard-curve spread P&L, with optional generic /
  per-level / adder decomposition via a calibrated `CreditFactorModel`
  (`CreditFactorAttribution`).
- **InflationCurves** — real-rate and CPI curve moves.
- **Correlations** — base correlation curve changes for structured credit.
- **Fx** — spot FX revaluation in the target reporting currency.
- **Volatility** — implied-vol surface moves (`VolAttribution`).
- **ModelParameters** — prepayment, default, recovery, conversion-policy and
  other model inputs snapshotted via
  `finstack_quant_valuations::instruments::model_params::ModelParamsSnapshot`.
- **MarketScalars** — dividends, equity/commodity spots, inflation index fixings.

## Layout

```
attribution/
├── lib.rs                  # Module docs, simple_pnl_bridge, public re-exports
├── types.rs                # AttributionFactor, PnlAttribution, AttributionMeta, *Detail structs
├── factors.rs              # MarketSnapshot, restore flags, per-factor market mutation
├── helpers.rs              # reprice_instrument, compute_pnl, compute_pnl_with_fx
├── parallel.rs             # attribute_pnl_parallel
├── waterfall.rs            # attribute_pnl_waterfall, default_waterfall_order
├── metrics_based/          # attribute_pnl_metrics_based (linear from metrics)
├── taylor.rs               # attribute_pnl_taylor, TaylorAttributionConfig
├── model_params.rs         # extract/replace model params, measure_*_shift
├── credit_factor.rs        # compute_credit_factor_attribution, model wiring
├── credit_cascade.rs       # Waterfall credit-factor cascade
├── credit_decomposition.rs # Generic / per-level / adder decomposition
├── execution.rs            # AttributionSpec::execute dispatcher
├── target_currency.rs           # translate_to_target_currency (native → reporting currency)
└── spec.rs                 # JSON envelope, AttributionSpec, AttributionResult
```

## Dependencies

```toml
[dependencies]
finstack-quant-attribution = { path = "../finstack-quant/attribution" }
finstack-quant-core        = { path = "../finstack-quant/core" }
finstack-quant-valuations  = { path = "../finstack-quant/valuations" }
```

Import path uses underscores:

```rust
use finstack_quant_attribution::{
    attribute_pnl_parallel, attribute_pnl_waterfall, default_waterfall_order,
    AttributionFactor, ExecutionPolicy, PnlAttribution,
};
```

## Quick start

A runnable parallel-attribution example, plus sign / carry / residual
conventions, lives in the crate rustdoc
(`cargo doc -p finstack-quant-attribution --open`).

Metrics-based attribution needs `ValuationResult`s priced at both dates with
the metrics in [`default_attribution_metrics`](src/spec.rs) (or a caller-chosen
subset). When parallel or waterfall runs request curve detail, optional
`rates_detail` exposes per-`(curve_id, tenor)` P&L.

## JSON specification

[`AttributionEnvelope`](src/spec.rs) / [`AttributionSpec`](src/spec.rs) define a
schema-versioned (`finstack_quant.attribution/1`) JSON contract used by bindings and
batch pipelines. A spec carries an `InstrumentJson` payload, two
`MarketContextState` snapshots, both `as_of` dates, the methodology, and
optional config / credit-factor-model overrides.

```rust,ignore
use finstack_quant_attribution::{AttributionEnvelope, AttributionSpec, ATTRIBUTION_SCHEMA_V1};

let envelope: AttributionEnvelope = serde_json::from_str(&json)?;
assert_eq!(envelope.schema, ATTRIBUTION_SCHEMA_V1);

let result_envelope = envelope.execute()?;
let result = &result_envelope.result; // AttributionResult { attribution, results_meta }
```

`AttributionSpec::from_json_inputs` is the binding-friendly constructor used by
the Python and WASM layers. Schemas live under `schemas/attribution/1/`.

## Public API

| Item                                                                              | Module           | Notes                                       |
|-----------------------------------------------------------------------------------|------------------|---------------------------------------------|
| `simple_pnl_bridge`                                                               | `lib`            | Total P&L, no decomposition                 |
| `attribute_pnl_parallel`                                                          | `parallel`       | Factor isolation, residual reports cross-effects |
| `attribute_pnl_waterfall`, `default_waterfall_order`                              | `waterfall`      | Sum-preserving ordered decomposition        |
| `attribute_pnl_metrics_based`                                                     | `metrics_based`  | Linear approximation from precomputed metrics |
| `attribute_pnl_taylor`, `TaylorAttributionConfig`                                | `taylor`         | Sensitivity-based expansion mapped to `PnlAttribution` |
| `PnlAttribution`, `AttributionFactor`, `AttributionMethod`, `AttributionMeta`     | `types`          | Result envelope and factor enums            |
| `CarryDetail`, `RatesCurvesAttribution`, `CreditCurvesAttribution`, `CreditFactorAttribution`, `InflationCurvesAttribution`, `CorrelationsAttribution`, `FxAttribution`, `VolAttribution`, `ModelParamsAttribution`, `ScalarsAttribution`, `CrossFactorDetail`, `CreditCarryDecomposition`, `CreditCarryByLevel`, `LevelCarry`, `LevelPnl`, `SourceLine` | `types` | Per-factor detail structs                   |
| `MarketSnapshot`, `MarketRestoreFlags`                                             | `factors`        | T₀/T₁ snapshot and per-factor restore primitives |
| `compute_pnl`, `compute_pnl_with_fx`                                              | `helpers`        | Money/FX arithmetic for P&L computation     |
| `translate_to_target_currency`                                                         | `target_currency`     | Post-hoc translation of a native-currency `PnlAttribution` into a reporting currency, adding `fx_translation_pnl` |
| `extract_model_params`, `with_model_params`, `measure_prepayment_shift`, `measure_default_shift`, `measure_recovery_shift`, `measure_conversion_shift` | `model_params` | Model-parameter snapshotting and shift attribution; use `finstack_quant_valuations::instruments::model_params::ModelParamsSnapshot` for the snapshot type |
| `compute_credit_factor_attribution`, `CreditAttributionInput`, `CreditFactorDetailOptions`, `credit_factor_model_id` | `credit_factor` | Calibrated credit-factor decomposition of `credit_curves_pnl`; the model type is `finstack_quant_factor_model::credit::hierarchy::CreditFactorModel` |
| `AttributionEnvelope`, `AttributionSpec`, `AttributionConfig`, `AttributionResult`, `AttributionResultEnvelope`, `ATTRIBUTION_SCHEMA_V1`, `default_attribution_metrics` | `spec` | JSON contract |

Sign, carry, currency, and residual conventions are documented in the crate
rustdoc.

## Numerical behavior

- All four methodologies guard against missing curves/surfaces and report
  zero contribution rather than panicking when a factor is absent from both
  market snapshots.
- Per-factor bump-and-reprice paths in `parallel`, `waterfall`, and `taylor`
  reuse a single `MarketSnapshot` and apply targeted restore flags
  (`MarketRestoreFlags`) to avoid full-context cloning.
- `ExecutionPolicy::Parallel` is the standalone default for parallel and Taylor
  attribution. Portfolio-level callers use `ExecutionPolicy::Serial` for those
  inner per-factor repricings so the outer position loop owns Rayon and avoids
  nested thread-pool contention.
- Taylor attribution uses central differences by default; bump sizes are
  configurable via `TaylorAttributionConfig`.
- Strict mode (`AttributionConfig::strict_validation = true`) propagates per-factor
  pricing errors; otherwise they are logged via `tracing` and the factor's P&L
  is set to zero.
- Output rounding follows `FinstackConfig::rounding`; `AttributionConfig::rounding_scale`
  overrides the per-currency scale for a single run.

## Extending

Adding a new factor requires coordinated updates to:

1. `AttributionFactor` and `PnlAttribution` in [`types.rs`](src/types.rs).
2. The factor-isolation / restore logic in [`factors.rs`](src/factors.rs).
3. All four methodology modules (`parallel`, `waterfall`, `metrics_based`,
   `taylor`).
4. `default_waterfall_order` in [`waterfall.rs`](src/waterfall.rs).
5. The JSON schema under `schemas/attribution/1/` and parity tests under
   `tests/attribution/`.

Follow an existing factor (e.g. `Fx` or `Volatility`) end-to-end as a template.

## Bindings

- **Python**: `AttributionSpec`-based JSON pipeline; result types serialize via
  serde and are exposed under `finstack_quant.attribution`. See
  `finstack-quant-py/parity_contract.toml`.
- **WASM**: attribution is exposed as a JSON-first surface under
  `finstack-quant-wasm/exports/attribution.js`. It intentionally mirrors the Python
  JSON/spec entry points (`attribute_pnl`, `attribute_pnl_from_spec`,
  `validate_attribution_json`, and the default-list helpers) rather than the
  full Rust type surface. The agreed WASM facade is pinned in
  `[wasm_attribution_subset]` in `finstack-quant-py/parity_contract.toml`.

## Related

- [`finstack-quant-valuations`](../valuations/README.md) — instrument repricing used at T₀ and T₁.
- [`finstack-quant-cashflows`](../cashflows/README.md) — accrual and carry inputs.
- [`finstack-quant-factor-model`](../factor-model/README.md) — calibrated credit-factor models consumed via `CreditFactorModel`.
- [`finstack-quant-portfolio`](../portfolio/README.md) — aggregates per-instrument `PnlAttribution`s into book-level views.

## References

Quantitative references: [`docs/REFERENCES.md`](../../docs/REFERENCES.md).

- Fixed-income sensitivity intuition: `docs/REFERENCES.md#tuckman-serrat-fixed-income`
- Risk decomposition and factor attribution: `docs/REFERENCES.md#meucci-risk-and-asset-allocation`

## Verification

```bash
cargo fmt -p finstack-quant-attribution
cargo clippy -p finstack-quant-attribution --all-features -- -D warnings
cargo test  -p finstack-quant-attribution
cargo test  -p finstack-quant-attribution --doc
RUSTDOCFLAGS='-D warnings' cargo doc -p finstack-quant-attribution --no-deps --all-features
```
