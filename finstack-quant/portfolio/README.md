# finstack-quant-portfolio

Positions, entities, and books; deterministic base-currency valuation and
rollups with explicit FX; and the portfolio-level analytics that sit on top —
attribution, factor risk, margin, liquidity, optimization, scenario P&L, and
historical replay.

## Where it sits

Top of the Rust stack. It depends on `finstack-quant-core`,
`finstack-quant-valuations`, `finstack-quant-attribution`,
`finstack-quant-cashflows`, `finstack-quant-factor-model`,
`finstack-quant-margin`, and `finstack-quant-scenarios`. No other domain crate
depends on it; only the `finstack-quant` umbrella crate and the Python/WASM
binding crates do.

Cargo features: `default = []` and `ts_export` (emits TypeScript declarations
for the materialization contract types via `ts-rs`; see
[`tests/ts_export.rs`](tests/ts_export.rs)). Rayon parallelism is
unconditional — there is no feature flag for it.

## Public surface

Headline types are re-exported at the crate root; everything else is reachable
through its module path. See the rustdoc for detail
(`cargo doc -p finstack-quant-portfolio --open`).

### Container and construction

| Item | Purpose |
|------|---------|
| `Portfolio`, `Portfolio::builder` | Entities, positions, books, base currency, `as_of` |
| `PortfolioBuilder` | Fluent construction; auto-creates the dummy entity when positions use `DUMMY_ENTITY_ID` |
| `Position`, `PositionUnit` | A held instrument (`Arc<dyn Instrument>`) plus signed quantity and its scaling unit |
| `Entity`, `EntityId`, `PositionId` | Ownership and identity types |
| `book::Book`, `book::BookId` | Optional parent/child book hierarchy |
| `types::AttributeValue`, `AttributeTest`, `ComparisonOp` | Position attributes and filters |
| `PortfolioSpec`, `Portfolio::to_spec` / `from_spec` | Portable JSON interchange |
| `materialization` | Strict, versioned bulk-load bundle with a content-addressed instrument cache |

### Valuation and repricing

| Item | Purpose |
|------|---------|
| `valuation::value_portfolio` | Full valuation; returns `PortfolioValuation` |
| `valuation::value_portfolio_at` | Same, at an explicit `as_of` |
| `valuation::revalue_affected` | Selective repricing driven by changed `MarketFactorKey`s |
| `valuation::PortfolioValuationOptions`, `RequestedMetrics` | Strict-risk policy and metric selection |
| `DependencyIndex`, `MarketFactorKey`, `flatten_dependencies` (crate root) | Market-factor → position inverted index |
| `metrics::aggregate_metrics` | FX-converted metric rollup; returns `PortfolioMetrics` |
| `grouping::aggregate_by_book` | Book-tree rollup including descendants |
| `cashflows::aggregate_full_cashflows` | Currency-preserving cashflow ladder (`PortfolioCashflows`) |
| `positions_to_table`, `entities_to_table`, `metrics_to_table`, `aggregated_metrics_to_table` | `core::table::TableEnvelope` exports |

### Analytics

| Module | Entry points |
|--------|--------------|
| `attribution` | `attribute_portfolio_pnl` → `PortfolioAttribution`; re-exports `AttributionMethod`, `PnlAttribution` and friends from `finstack-quant-attribution` |
| `brinson` | `brinson_fachler`, `carino_link`, `carino_link_from_sector_periods` |
| `fi_attribution` | `campisi_attribution`, `campisi_carino_link`, `campisi_carino_link_from_snapshots` |
| `grid_attribution` | `grid_attribution`, `grid_carino_link` |
| `factor_brinson` | `factor_brinson_attribution` |
| `excess_return` | `excess_returns`, `cell_returns_from_curves`, `cell_returns_from_reference` |
| `performance` | `twrr_modified_dietz`, `twrr_linked`, `mwr_xirr`, `mwr_xirr_from_cashflows` |
| `factor_model` | `FactorModel` (`assign_factors` / `compute_sensitivities` / `analyze`), `ParametricDecomposer`, `SimulationDecomposer`, `allocate_weights` |
| `sensitivity` | `DeltaBasedEngine`, `FullRepricingEngine` + `ScenarioGrid`, `FactorSensitivityEngine` |
| `liquidity` | `roll_effective_spread`, `amihud_illiquidity`, `days_to_liquidate`, `classify_tier`, `lvar_bangia_scalar`, `AlmgrenChrissModel`, `KyleLambdaModel` |
| `optimization` | `PortfolioOptimizationProblem`, `DefaultLpOptimizer`, `optimize_from_spec`, `PortfolioOptimizationResult` |
| `margin` (re-exported at root) | `PortfolioMarginAggregator`, `NettingSet`, `PortfolioMarginResult` — see [`src/margin/README.md`](src/margin/README.md) |
| `scenarios` | `apply_and_revalue`, `scenario_pnl`, `scenario_pnl_batch` |
| `replay` | `replay_portfolio` over a `ReplayTimeline` of dated market snapshots |

## Quick start

```rust
use finstack_quant_core::config::FinstackConfig;
use finstack_quant_core::currency::Currency;
use finstack_quant_core::dates::DayCount;
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::money::Money;
use finstack_quant_portfolio::position::{Position, PositionUnit};
use finstack_quant_portfolio::types::Entity;
use finstack_quant_portfolio::valuation::{value_portfolio, PortfolioValuationOptions};
use finstack_quant_portfolio::Portfolio;
use finstack_quant_valuations::instruments::rates::deposit::Deposit;
use std::sync::Arc;
use time::macros::date;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let as_of = date!(2024 - 01 - 01);
    // Populate the context with the curves your instruments declare.
    let market = MarketContext::new();
    let config = FinstackConfig::default();

    let deposit = Deposit::builder()
        .id("DEP_1M".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start_date(as_of)
        .maturity(date!(2024 - 02 - 01))
        .day_count(DayCount::Act360)
        .discount_curve_id("USD".into())
        .build()?;

    let position = Position::new(
        "POS_001",
        "ACME_FUND",
        "DEP_1M",
        Arc::new(deposit),
        1.0,
        PositionUnit::Units,
    )?
    .with_text_attribute("asset_class", "cash");

    let portfolio = Portfolio::builder("MY_FUND")
        .base_currency(Currency::USD)
        .as_of(as_of)
        .entity(Entity::new("ACME_FUND"))
        .position(position)
        .build()?;

    let valuation = value_portfolio(
        &portfolio,
        &market,
        &config,
        &PortfolioValuationOptions::default(),
    )?;
    println!("Portfolio total: {}", valuation.total_base_currency);
    Ok(())
}
```

A larger end-to-end program lives in
[`examples/portfolio_optimization.rs`](examples/portfolio_optimization.rs).

## Conventions that bite

- **Base currency.** `Portfolio::base_currency` is the reporting currency for
  totals and every portfolio-level analytic. Position values are kept in both
  native and base currency; summable risk metrics are FX-converted before
  aggregation. `aggregate_metrics` rejects a `base_currency` or `as_of` that
  disagrees with the valuation it was handed.
- **FX rate source.** `aggregate_metrics` prefers the market context's FX
  matrix spot for `(native → base, as_of)`; the PV-implied ratio
  `value_base / value_native` is only a fallback for when the matrix or the
  pair is missing, and it is rejected when `|value_native| <= 1e-6` (an
  `FxConversionFailed` error rather than a distorted rate). The market spot is
  preferred because `value_base` was already rounded to currency decimals, and
  that quantization noise would scale every summable risk metric.
- **`quantity` is not always a share count.** `PositionUnit` decides:
  - `Units` — scale by share or contract count.
  - `Notional(Option<Currency>)` — scale by notional; the per-unit PV must be
    computed against a notional of 1. The currency is a validation tag only and
    does not change the scale factor.
  - `FaceValue` — scale by held face amount.
  - `Percentage` — percentage points (`50.0` → `0.50` internally).
- **Selective repricing.** `revalue_affected` consults the `DependencyIndex`.
  Positions whose dependencies could not be resolved are repriced
  unconditionally. A changed FX quote additionally forces a base-currency
  refresh for every reused position, because a cross may be triangulated
  through that quote. Mutating positions directly requires
  `Portfolio::rebuild_index` before the next selective call.
- **Cashflow FX.** `PortfolioCashflows` is currency-preserving. The
  base-currency projection (`collapse_to_base_by_date_kind`) uses
  spot-equivalent FX at every date, which is *not* forward FX; derive forward
  rates from discount curves when NPV-grade accuracy matters.
- **Serialization.** `Portfolio` intentionally has no direct
  `Serialize`/`Deserialize` — positions hold `Arc<dyn Instrument>`. Use
  `to_spec` / `from_spec`. `to_spec` records `instrument_spec: None` for any
  instrument that does not implement `to_instrument_json()`, and `from_spec`
  then returns an error for that position; there is no external-registry hook.
- **Determinism.** Position pricing runs on Rayon above
  `POSITION_PARALLEL_MIN_POSITIONS = 64` positions. The selective path has its
  own gate (`SELECTIVE_PARALLEL_MIN_REPRICES = 64`) and additionally requires
  the work set to be at least a quarter of the book, so a small dirty set in a
  large portfolio stays serial; a forced base-currency refresh counts the whole
  book as work rather than just the dirty set. Parallel results are collected
  by index and folded serially with Neumaier summation, so totals, first-error
  selection, and ordering are identical to the serial path. All evaluation
  state and caches are request-local; see
  [INVARIANTS.md §2.6](../../INVARIANTS.md) for the full portfolio-evaluation
  contract.

## Bindings

- **Python:** `finstack_quant.portfolio` — `Portfolio`, `value_portfolio`,
  `aggregate_metrics`, `attribute_portfolio_pnl`, `replay_portfolio`,
  `optimize_portfolio`, the attribution/factor-model/optimization result
  classes, and `finstack_quant.portfolio.schema`.
- **WASM:** the `portfolio` namespace from `finstack-quant-wasm/index.js`
  (`exports/portfolio.js`).

Both are thin adapters over this crate; no portfolio logic lives in them.

## Schemas

This crate owns two checked-in JSON Schemas in
[`schemas/portfolio/1/`](schemas/portfolio/1):
`portfolio_materialization.schema.json` and
`portfolio_optimization_result.schema.json`, listed in
[`schemas/index.json`](schemas/index.json). Regenerate and verify with:

```bash
cargo run -p finstack-quant-portfolio --bin gen_materialization_schemas -- --write
mise run rust-check-schemas
```

## Verification

```bash
mise run rust-test                                        # whole workspace, cargo-nextest
cargo nextest run -p finstack-quant-portfolio             # this crate only
cargo nextest run -p finstack-quant-portfolio --test selective_repricing
cargo run -p finstack-quant-portfolio --example portfolio_optimization
mise run rust-lint
```

Do not invoke `cargo test` directly in this workspace — it pulls in doc tests.
Benchmarks are documented in [`benches/README.md`](benches/README.md).

## References

Quantitative references (Brinson-Fachler, Carino, Campisi, Bangia LVaR,
Almgren-Chriss, Kyle, Tasche Euler allocation, ISDA SIMM):
[`docs/REFERENCES.md`](../../docs/REFERENCES.md).
