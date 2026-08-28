# Structured Credit — Stochastic Pricing

Valuations owns structured-credit calibration presets and the scenario-tree /
Monte Carlo orchestration that runs each simulated collateral path through the
deal waterfall. Product-independent default, prepayment, correlation, and
finite-pool copula engines live in
`finstack_quant_models::credit::pool`.

## Module layout

```text
stochastic/
├── calibrations.rs  # registry-backed RMBS/CLO/CMBS/ABS presets
├── tree/config.rs   # valuation-owned scenario-tree configuration
└── pricer/
    ├── config.rs    # PricingMode and internal pricer configuration
    ├── engine.rs    # path generation plus waterfall orchestration
    └── result.rs    # StochasticPricingResult and TranchePricingResult
```

## Configure a deal

```rust
use finstack_quant_cashflows::builder::PrepaymentModelSpec;
use finstack_quant_models::credit::pool::{
    CorrelationStructure, StochasticDefaultSpec, StochasticPrepaySpec,
};
use finstack_quant_valuations::instruments::fixed_income::structured_credit::StructuredCredit;

let mut clo = StructuredCredit::example();

// Applies valuation-owned, registry-backed presets for the deal type.
clo.enable_stochastic_defaults().expect("valid built-in stochastic defaults");

// Or supply explicit models-owned specifications.
clo.with_stochastic_prepay(StochasticPrepaySpec::factor_correlated(
    PrepaymentModelSpec::constant_cpr(0.15),
    0.40,
    0.25,
))
.with_stochastic_default(StochasticDefaultSpec::gaussian_copula(0.03, 0.20))
.with_correlation(CorrelationStructure::sectored(0.30, 0.10, -0.20).expect("valid correlation"));
```

`StochasticPrepaySpec`, `StochasticDefaultSpec`, `CorrelationStructure`, and
`PoolGranularity` are not re-exported by valuations. Import them from models.

## Pricing

```rust
use finstack_quant_valuations::instruments::fixed_income::structured_credit::PricingMode;

let result = clo.price_stochastic_with_mode(
    &market,
    as_of,
    PricingMode::MonteCarlo {
        num_paths: 50_000,
        antithetic: true,
    },
)?;
```

`PricingMode::default()` uses 10,000 antithetic Monte Carlo paths. Tree mode is
bounded by its non-recombining node count and is intended only for short
horizons. `StochasticPricingResult` and `TranchePricingResult` remain
valuation-owned outputs.

The path seed is fixed by the internal scenario configuration, so repeated
runs and bump-and-reprice sensitivities reuse common random numbers.

## Calibration ownership

`calibrations.rs` reads the v1 structured-credit assumption registry and
constructs explicit models-owned specs. RMBS, CLO, CMBS, and ABS preset policy
therefore remains in valuations; models has no dependency on the registry or
on instruments.

## Verification

```bash
mise run rust-test-filter -- finstack-quant-valuations structured_credit
mise run rust-test-crate -- finstack-quant-models
```
