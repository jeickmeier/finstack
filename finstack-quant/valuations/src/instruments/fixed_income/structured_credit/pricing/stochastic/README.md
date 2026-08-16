# Structured Credit — Stochastic Models

Stochastic prepayment and default models for `StructuredCredit` deals, plus the
Monte Carlo / scenario-tree pricing engine that runs the deterministic
waterfall over simulated paths.

Use the deterministic path ([`../../README.md`](../../README.md)) for
day-to-day valuation. Reach for this module when you need a loss distribution:
VaR / expected shortfall, correlation risk, tranche tail behavior.

## Module layout

```
stochastic/
├── mod.rs             # re-exports and module overview
├── calibrations.rs    # deal-type calibration constants, sourced from the assumption registry
├── prepayment/
│   ├── spec.rs             # StochasticPrepaySpec (public)
│   ├── traits.rs           # StochasticPrepayment
│   ├── factor_correlated.rs
│   ├── richard_roll.rs
│   └── regime_switching.rs
├── default/
│   ├── spec.rs             # StochasticDefaultSpec (public)
│   ├── traits.rs           # StochasticDefault, MacroCreditFactors
│   ├── copula_based.rs     # Gaussian / Student-t copula
│   ├── per_name.rs         # PerNameCopulaDefault + PoolGranularity (public)
│   ├── factor_correlated.rs
│   ├── intensity_process.rs
│   └── hazard_curve_adapter.rs  # wraps core HazardCurve
├── correlation/
│   └── structure.rs        # CorrelationStructure (public)
├── tree/
│   └── config.rs           # ScenarioTreeConfig (crate-internal)
└── pricer/
    ├── config.rs           # PricingMode (public), StochasticPricerConfig (crate-internal)
    ├── engine.rs           # StochasticPricer (crate-internal)
    └── result.rs           # StochasticPricingResult, TranchePricingResult (public)
```

## Public surface

Everything below is re-exported from the parent module,
`finstack_quant_valuations::instruments::fixed_income::structured_credit`.
`StochasticPricer`, `StochasticPricerConfig` and `ScenarioTreeConfig` are
**crate-internal** — drive them through `StructuredCredit::price_stochastic`.

| Item | Purpose |
|------|---------|
| `StochasticPrepaySpec` | Prepayment model selection and parameters. |
| `StochasticDefaultSpec` | Default model selection and parameters. |
| `CorrelationStructure` | Flat, sectored or explicit-matrix correlation. |
| `PoolGranularity` | `PerName` (default) or `LargeHomogeneous`. |
| `PricingMode` | `Tree`, `MonteCarlo { num_paths, antithetic }`, `Hybrid { tree_periods, mc_paths }`. |
| `StochasticPricingResult`, `TranchePricingResult` | Pricing output. |

## Enabling stochastic models on a deal

```rust
use finstack_quant_valuations::instruments::fixed_income::structured_credit::{
    CorrelationStructure, StochasticDefaultSpec, StochasticPrepaySpec, StructuredCredit,
};

let mut clo = StructuredCredit::example();

// Auto-calibrate for the deal type.
clo.enable_stochastic_defaults();

// Or configure explicitly. These setters take &mut self and return &mut Self.
clo.with_stochastic_prepay(StochasticPrepaySpec::clo_standard())
    .with_stochastic_default(StochasticDefaultSpec::clo_standard())
    .with_correlation(CorrelationStructure::clo_standard());
```

## Pricing

```rust
use finstack_quant_valuations::instruments::fixed_income::structured_credit::PricingMode;

// Defaults to Monte Carlo with 10,000 antithetic paths
// (or `instrument_pricing_overrides.model_config.mc_paths` when set).
let result = clo.price_stochastic(&market, as_of)?;

// Or pick the mode explicitly.
let result = clo.price_stochastic_with_mode(
    &market,
    as_of,
    PricingMode::MonteCarlo { num_paths: 50_000, antithetic: true },
)?;

println!("NPV:              {}", result.npv);
println!("expected loss:    {}", result.expected_loss);
println!("unexpected loss:  {}", result.unexpected_loss);
println!("expected shortfall @ {}: {}", result.es_confidence, result.expected_shortfall);
```

### Pricing modes

| Mode | Behavior |
|------|----------|
| `MonteCarlo { num_paths, antithetic }` | **Default.** The only mode that reaches a realistic horizon. Antithetic variates negate the raw innovations, which negates the whole evolved factor path. |
| `Tree` | Exact enumeration of the scenario lattice. Bounded to roughly ten periods by the `3^n` node count, so it is not usable at deal horizon. |
| `Hybrid { tree_periods, mc_paths }` | Tree over the near term, Monte Carlo for the tail. |

`PricingMode::default()` is `MonteCarlo { num_paths: 10_000, antithetic: true }`.

### Result

`StochasticPricingResult` carries `npv`, `clean_price`, `dirty_price`,
`expected_loss`, `unexpected_loss`, `expected_shortfall`, `es_confidence`,
`pv_std_error`, `pv_confidence_interval`, `num_paths`, the `pricing_mode`
actually used, and `tranche_results: Vec<TranchePricingResult>`.

`TranchePricingResult` carries `tranche_id`, `seniority`, `attachment`,
`detachment`, `npv`, `expected_loss`, `unexpected_loss`,
`expected_shortfall`, `average_life`, `spread` and `credit_duration`.

**`clean_price` equals `dirty_price`** at the deal level: the stochastic result
carries no per-tranche interest flows, so accrued cannot be computed here and
is reported unadjusted rather than fabricated. Use `calculate_tranche_metrics`
when you need an accrued-adjusted clean price.

## Models

### Prepayment (`StochasticPrepaySpec`)

| Constructor | Model | Use |
|-------------|-------|-----|
| `deterministic(PrepaymentModelSpec)` | pass-through | disable stochastic prepayment |
| `factor_correlated(..)` | base CPR shocked by the systematic factor | general ABS/CLO |
| `richard_roll(..)` | refi incentive, burnout, seasonality | agency RMBS |
| `regime_switching(..)` | two-state prepayment | regime-dependent pools |
| `rmbs_agency(pool_coupon)` | Richard-Roll with registry calibration | agency RMBS |
| `clo_standard()` | factor-correlated with CLO calibration | leveraged-loan pools |

### Default (`StochasticDefaultSpec`)

| Constructor | Model | Use |
|-------------|-------|-----|
| `deterministic(DefaultModelSpec)` | pass-through | disable stochastic defaults |
| `gaussian_copula(base_cdr, correlation)` | Gaussian copula | corporate CLO |
| `student_t_copula(base_cdr, correlation, degrees_of_freedom)` | Student-t copula | fatter joint tails |
| `intensity_process(..)` | Cox process, mean-reverting intensity | CDS-like modeling |
| `factor_correlated(..)` | factor-shocked CDR | general |
| `from_hazard_curve(curve, factor_sensitivity)` / `from_hazard_curve_full(..)` | market-calibrated hazard curve | credit-curve-driven deals |
| `rmbs_standard()`, `clo_standard()` | registry calibration | deal-type defaults |

### Pool granularity

`PoolGranularity::PerName` (the default) simulates each name's default
independently conditional on the systematic factor — a finite-pool copula that
captures name-level lumpiness, which is what a concentrated pool needs.
`LargeHomogeneous` applies the closed-form LHP conditional default probability
uniformly to every name; it is the `N → ∞` limit and a faster approximation that
is only acceptable for genuinely granular pools.

### Correlation (`CorrelationStructure`)

```rust
use finstack_quant_valuations::instruments::fixed_income::structured_credit::CorrelationStructure;

// Deal-type presets.
let clo = CorrelationStructure::clo_standard();
let rmbs = CorrelationStructure::rmbs_standard();
let cmbs = CorrelationStructure::cmbs_standard();
let auto = CorrelationStructure::abs_auto_standard();

// Or build one. The `try_*` forms validate instead of clamping.
let flat = CorrelationStructure::try_flat(0.30, -0.20)?;
let sectored = CorrelationStructure::try_sectored(0.35, 0.15, -0.20)?;
let matrix = CorrelationStructure::try_matrix(correlations, labels)?;
```

Variants: `Flat { asset_correlation, prepay_default_correlation }`,
`Sectored { intra_sector, inter_sector, prepay_default }`,
`Matrix { correlations, labels }` (row-major).
`bump_asset(delta)` produces the bumped structure used by correlation
sensitivities; `validate()` checks the matrix.

### Systematic factor persistence

Monthly systematic draws follow an AR(1)/OU recursion:

```text
Z_1 = ε_1,   Z_m = φ·Z_{m−1} + sqrt(1 − φ²)·ε_m,   φ = exp(−κ/12)
```

Each `Z_m` keeps a stationary `N(0,1)` marginal, so the conditional MDR/SMM
models and the copula barriers `Φ⁻¹(PD)` stay correctly calibrated. Factor
autocorrelation at lag `h` months is `φ^h = exp(−κh/12)`; the correlation
half-life is `12·ln 2 / κ` months (κ = 0.5 gives ≈ 16.6 months).
`κ → ∞` recovers i.i.d. monthly factors; `κ = 0` holds one systematic draw
across the whole horizon.

The transform is linear in the innovations, so antithetic negation of the raw
draws negates the entire evolved path and the variance reduction survives.

## Determinism

Paths are generated from a fixed seed (`ScenarioTreeConfig::seed`, default 42),
so repeated runs and bump-and-reprice sensitivities reuse the same variates
(common random numbers) and finite-difference Greeks carry no Monte Carlo noise.

## Calibration constants

`calibrations.rs` holds the deal-type calibration structs (base CDR, asset
correlation, base CPR, factor loadings, volatilities, refi sensitivity), all
sourced from the embedded assumption registry in
[`data/assumptions/structured_credit_assumptions.v1.json`](../../../../../../data/assumptions/structured_credit_assumptions.v1.json)
rather than hard-coded at the call site.

## Verification

```bash
# Structured-credit tests, including the stochastic simulation suites
cargo nextest run -p finstack-quant-valuations --test instruments structured_credit::

# Whole workspace (never `cargo test` — it runs doctests)
mise run rust-test
```

## References

- Li, D. X. (2000). "On Default Correlation: A Copula Function Approach."
  [`docs/REFERENCES.md#li-2000-gaussian-copula`](../../../../../../../../docs/REFERENCES.md#li-2000-gaussian-copula)
- Duffie, D., & Singleton, K. J. (1999). "Modeling Term Structures of
  Defaultable Bonds."
  [`docs/REFERENCES.md#duffie-singleton-1999`](../../../../../../../../docs/REFERENCES.md#duffie-singleton-1999)
- Richard, S. F., & Roll, R. (1989). "Prepayments on Fixed-Rate
  Mortgage-Backed Securities."
  [`docs/REFERENCES.md#richard-roll-1989`](../../../../../../../../docs/REFERENCES.md#richard-roll-1989)
- Schwartz, E. S., & Torous, W. N. (1989). "Prepayment and the Valuation of
  Mortgage-Backed Securities."
  [`docs/REFERENCES.md#schwartz-torous-1989`](../../../../../../../../docs/REFERENCES.md#schwartz-torous-1989)
- Basel II IRB correlation formulas.
  [`docs/REFERENCES.md#basel-ii-2006`](../../../../../../../../docs/REFERENCES.md#basel-ii-2006)

## See also

- [`../../README.md`](../../README.md) — the structured-credit instrument, deterministic pricing and waterfall
- [`INVARIANTS.md`](../../../../../../../../INVARIANTS.md) — determinism, Decimal/f64 and serde invariants
