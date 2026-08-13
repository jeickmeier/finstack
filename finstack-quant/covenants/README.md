# Covenants

Evaluate financial and non-financial covenants, track breaches and cure periods, apply consequences, and project compliance with headroom analytics.

## Layout

```
covenants/
├── engine/      # CovenantEngine, specs, consequences, breach tracking
├── forward.rs   # Forward projection (deterministic or analytic stochastic)
├── schedule.rs  # Piecewise threshold schedules
├── metric.rs    # CovenantMetricId, metric-source traits
├── templates.rs # Preset covenant packages (lbo_standard, cov_lite, ...)
├── json.rs      # Serde-first JSON binding surface
└── report.rs    # CovenantReport
```

## Evaluation

```rust
use finstack_quant_core::dates::{create_date, Month, Tenor};
use finstack_quant_covenants::{
    Covenant, CovenantEngine, CovenantSpec, CovenantType, HashMapMetricSource,
};

# fn main() -> finstack_quant_core::Result<()> {
let covenant = Covenant::new(
    CovenantType::MaxDebtToEbitda { threshold: 4.5 },
    Tenor::quarterly(),
);
let mut engine = CovenantEngine::new();
engine.add_spec(CovenantSpec::with_metric(covenant, "debt_to_ebitda"));

let mut metrics = HashMapMetricSource::from_pairs([("debt_to_ebitda", 3.2)]);
let test_date = create_date(2025, Month::March, 31)?;
let reports = engine.evaluate(&mut metrics, test_date)?;

assert!(reports["max_debt_ebitda"].passed);
assert_eq!(reports["max_debt_ebitda"].threshold, Some(4.5));
# Ok(())
# }
```

Built-in financial types include leverage, coverage, and asset-coverage tests. `CovenantType::Custom` and non-financial affirmative/negative covenants use registered metrics or `CovenantSpec::with_evaluator`.

## Consequences

After cure expiry, `apply_consequences` can apply `RateIncrease`, `CashSweep`, `BlockDistributions`, `AccelerateMaturity`, `Default`, and related variants on instruments implementing `InstrumentMutator`.

`evaluate_and_track` treats one continuous uncured breach of a covenant instance as one breach episode. The cure deadline is anchored to the original breach date, and metric recovery before that deadline marks the episode cured.

## Forward projection

`forecast_covenant_generic` projects metric values through a `ModelTimeSeries` adapter (no direct dependency on the statements crate). `CovenantForecastConfig` selects deterministic output or analytic lognormal per-date breach probabilities. The stochastic output is an independent marginal probability for each test date, not a first-passage distribution; `num_paths`, `random_seed`, and `antithetic` are retained for source compatibility and future path-consistent simulation.

Forecast IDs use `Covenant::instance_key()` so outputs can join to engine reports and breach history. Human-readable text is carried separately in `covenant_description`. Nullable forecast fields serialize as JSON `null` when a value is inactive or not meaningful, such as springing covenants outside activation periods or negative-EBITDA leverage ratios.

## Threshold schedules

`ThresholdSchedule` supports step-down limits used by covenant evaluation. `ThresholdSchedule::new` validates finite values and rejects duplicate dates.

## Windows

`CovenantWindow` restricts which specs apply between `start` and `end`; active windows override base specs. If windows exist but the test date falls outside every window, evaluation falls back to the base `specs`. Overlapping windows are rejected by engine validation.

## Templates

`templates` provides preset covenant packages that return `Vec<CovenantSpec>`
ready for `CovenantEngine`: `lbo_standard`, `cov_lite`, `real_estate`, and
`project_finance`.

## JSON

`json` is a serde-first binding surface: `evaluate_engine_json`, the
`validate_*_json` validators, and JSON template builders (`lbo_standard_json`,
`cov_lite_json`, `project_finance_json`, `real_estate_json`).

Inbound JSON denies unknown fields and runs domain validation, including non-negative cure periods, finite thresholds, valid waiver dates, non-overlapping windows, and valid threshold schedules.

Amount-style covenants such as `MaxCapex`, `MinLiquidity`, and `Basket` use bare `f64` thresholds in the deal currency agreed by the caller; currency-typed thresholds are outside this crate's current JSON surface.

## Related

- `finstack-quant-statements` — statement node IDs commonly provide covenant metric inputs
- `finstack-quant-valuations` — `ValuationResult` can attach `CovenantReport` outputs
