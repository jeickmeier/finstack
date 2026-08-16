# Equity Index Future

Cash-settled exchange-traded futures on equity indices (ES, MES, NQ, FESX,
FDAX, FTSE 100, Nikkei 225), valued mark-to-market against a quoted price or at
cost-of-carry fair value.

## Public surface

Import path:
`finstack_quant_valuations::instruments::equity::equity_index_future`
(`EquityIndexFuture` is also re-exported at
`finstack_quant_valuations::instruments`).

| Item | Purpose |
|------|---------|
| `EquityIndexFuture` | The instrument. `builder()`, `example()`, `sp500_emini(..)`, `nasdaq100_emini(..)`. |
| `EquityFutureSpecs` | Contract specs (multiplier, tick size, tick value, settlement method), loaded from the contract-spec registry. |

Useful methods: `validate()`, `position_sign()`, `num_contracts(price)`,
`delta()`, `fair_forward(market, as_of)`, `npv_raw(market, as_of)`.

## Module layout

```
equity_index_future/
├── mod.rs      # re-exports + module overview and contract table
├── types.rs    # EquityIndexFuture, EquityFutureSpecs, factories, Instrument impl
├── pricer.rs   # compute_pv / compute_pv_raw, price_quoted, price_fair_value, fair_forward
└── metrics/
    ├── delta.rs    # futures-price delta
    └── pricing.rs  # FuturesPrice, Basis
```

Registered with `register_generic!` under `InstrumentType::EquityIndexFuture`
in [`src/pricer/equity.rs`](../../../pricer/equity.rs), so pricing runs through
`Instrument::base_value`.

## Contract specifications

`EquityFutureSpecs` constructors read the embedded registry at
[`data/contract_specs/contract_specs.v1.json`](../../../../data/contract_specs/contract_specs.v1.json)
(source and effective date recorded per contract):

| Constructor | Registry id | Exchange | Multiplier | Tick size | Tick value |
|-------------|-------------|----------|-----------|-----------|------------|
| `sp500_emini()` | `cme.es` | CME | 50 | 0.25 | 12.50 |
| `sp500_micro_emini()` | `cme.mes` | CME | 5 | 0.25 | 1.25 |
| `nasdaq100_emini()` | `cme.nq` | CME | 20 | 0.25 | 5.00 |
| `euro_stoxx_50()` | `eurex.fesx` | Eurex | 10 | 1.0 | 10.00 |
| `dax()` | `eurex.fdax` | Eurex | 25 | 0.5 | 12.50 |
| `ftse_100()` | `ice.ftse_100` | ICE | 10 | 0.5 | 5.00 |
| `nikkei_225()` | `ose.nikkei_225` | OSE | 500 | 5.0 | 2500.00 |

Multiplier and tick value are in the contract's own currency (USD, EUR, GBP,
JPY respectively).

## Position sizing

The instrument is defined by a **notional** in settlement currency, not by a
contract count. The contract count is derived:

```text
contracts = notional / (entry_price × multiplier)
```

`entry_price` is therefore required for PV and delta —
`require_entry_price()` returns a validation error when it is unset, because an
unfilled order has no defined exposure.

## Pricing

`pricer::compute_pv_raw` selects, in order:

1. `as_of > expiry` → PV is zero.
2. `as_of > last_trading_date` → use `settlement_price` (required; a missing or
   non-positive settlement fixing is an error). The settlement fixing is the
   final mark, not a live quote.
3. `quoted_price` present → mark to market.
4. Otherwise → cost-of-carry fair value.

Both live branches use the same expression:

```text
PV = (price − entry_price) × multiplier × contracts × position_sign
```

with `position_sign = +1` for `Position::Long`, `−1` for `Short`.

### Fair value

```text
F = S₀ · exp((r − q) · T)
```

- `T` is Act/365F from `as_of` to `expiry`, floored at zero.
- `r` is the **date-based** zero rate over `[as_of, expiry]`, derived from the
  relative discount factor rather than `curve.zero(t)`, which avoids the axis
  bias when the curve base date differs from `as_of` or the day counts differ.
- `q` is the continuous dividend yield from `div_yield_id` (a unitless
  `MarketScalar`; a `Price` scalar is rejected). Absent, `q = 0`.

When `discrete_dividends` is non-empty the fair forward switches to PV-spot
adjustment instead, and continuous `q` is ignored to avoid double counting:

```text
F = (S₀ − Σ PV(dividends in (as_of, expiry])) / DF(as_of → expiry)
```

## Construction

```rust
use finstack_quant_valuations::instruments::equity::equity_index_future::{
    EquityFutureSpecs, EquityIndexFuture,
};
use finstack_quant_valuations::instruments::{Attributes, Position};
use finstack_quant_core::currency::Currency;
use finstack_quant_core::money::Money;
use finstack_quant_core::types::{CurveId, InstrumentId};
use time::macros::date;

// Builder: field names are underlying_ticker / notional / expiry.
let es = EquityIndexFuture::builder()
    .id(InstrumentId::new("ESH5"))
    .underlying_ticker("SPX".to_string())
    .notional(Money::new(2_250_000.0, Currency::USD))
    .expiry(date!(2025 - 03 - 21))
    .last_trading_date(date!(2025 - 03 - 20))
    .entry_price_opt(Some(4500.0))
    .quoted_price_opt(Some(4550.0))
    .position(Position::Long)
    .contract_specs(EquityFutureSpecs::sp500_emini())
    .discount_curve_id(CurveId::new("USD-OIS"))
    .spot_id("SPX-SPOT".into())
    .attributes(Attributes::new())
    .build()?;

// Convenience constructor with SPX defaults.
let es2 = EquityIndexFuture::sp500_emini(
    "ESH5",
    Money::new(2_250_000.0, Currency::USD),
    date!(2025 - 03 - 21),
    date!(2025 - 03 - 20),
    Some(4500.0),
    Position::Long,
    "USD-OIS",
)?;
```

Pricing against a market context:

```rust
use finstack_quant_valuations::instruments::Instrument;
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::market_data::scalars::MarketScalar;
use finstack_quant_core::market_data::term_structures::DiscountCurve;
use time::macros::date;

let base_date = date!(2025 - 01 - 01);
let market = MarketContext::new()
    .insert(
        DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots(vec![(0.0, 1.0), (1.0, 0.95)])
            .build()?,
    )
    .insert_price("SPX-SPOT", MarketScalar::Unitless(4500.0));

let future = EquityIndexFuture::example()?;
let pv = future.value(&market, base_date)?;
```

## Market data requirements

| Path | Required |
|------|----------|
| Mark-to-market (`quoted_price` set) | Discount curve (for DV01 only) |
| Post-last-trading-date | `settlement_price` on the instrument |
| Fair value | Discount curve, spot index level via `spot_id` |
| Fair value with continuous yield | plus `div_yield_id` (unitless scalar) |
| Fair value with discrete dividends | plus `discrete_dividends` on the instrument |

## Metrics

Registered for `InstrumentType::EquityIndexFuture` in `metrics/mod.rs`:

| `MetricId` | Meaning |
|-----------|---------|
| `Delta` | **Futures-price** delta `∂PV/∂F = multiplier × contracts × position_sign`. Convert to spot/equity delta by multiplying by the carry factor `exp((r−q)T)`. Requires `entry_price`. |
| `FuturesPrice` | Quoted price when present, otherwise the cost-of-carry fair forward |
| `Basis` | Futures price minus spot index level |
| `Dv01` | Parallel discount-curve DV01 |
| `BucketedDv01` | Triangular key-rate DV01 |

`Theta` is registered universally by `metrics::standard_registry()`.

## Bindings

Reachable from Python and WASM through the JSON envelope
(`InstrumentJson::EquityIndexFuture` inside `finstack_quant.instrument/1`):

- **Python**: `finstack_quant.valuations.instruments.price_instrument(...)`.
- **WASM**: `valuations.instruments.priceInstrument`.

There is no typed `EquityIndexFuture` class in either binding.

## Verification

```bash
# Equity index future construction and pricing tests
cargo nextest run -p finstack-quant-valuations --test instruments equity_index_future::

# Whole workspace (never `cargo test` — it runs doctests)
mise run rust-test

# Lints
mise run rust-lint
```

Tests live in
[`tests/instruments/equity_index_future/`](../../../../tests/instruments/equity_index_future/).

## References

- Hull, J. C. (2018). *Options, Futures, and Other Derivatives*, ch. 5:
  Determination of Forward and Futures Prices —
  [`docs/REFERENCES.md#hull-options-futures`](../../../../../../docs/REFERENCES.md#hull-options-futures)
- Contract specifications and their sources are recorded per contract in
  [`data/contract_specs/contract_specs.v1.json`](../../../../data/contract_specs/contract_specs.v1.json)
  (CME, Eurex, ICE, OSE).

## See also

- [`../../README.md`](../../README.md) — instrument module map and how to add one
- [`../vol_index_future/`](../vol_index_future/) — the volatility-index sibling
- [`INVARIANTS.md`](../../../../../../INVARIANTS.md) — Decimal/f64, determinism and serde invariants
