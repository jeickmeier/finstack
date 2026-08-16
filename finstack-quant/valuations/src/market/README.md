# Market

Market-quote schemas, convention registries, and quote-to-instrument
construction. This module is the front door for calibration input: it defines
what a quote looks like on the wire, resolves the market conventions a quote
implies, and turns the pair into a concrete priceable instrument.

It deliberately owns representation and construction only. Solvers and
bootstrapping live in [`../calibration/`](../calibration/README.md); pricing
models live in `../models/`.

## Layout

| Path | Visibility | Contents |
|------|-----------|----------|
| `quotes/` | public | Quote schemas per asset class plus `QuoteId` / `Pillar` |
| `conventions/` | public | Convention definitions, typed IDs, and the global registry |
| `build/` | crate-private | `BuildCtx`, quote-to-instrument builders, `PreparedQuote` |
| `credit_option_vol.rs` | public | CDX/iTraxx option-surface lookup converted to additive hazard volatility |

The builder entry points that escape `build/` are re-exported at the module root:
`BuildCtx`, `build_rate_instrument`, `build_cds_instrument`,
`build_cds_tranche_instrument` (with `CDSTrancheBuildOverrides`), and
`build_xccy_instrument`.

## Quotes

`quotes::market_quote::MarketQuote` is the unified enum, serialized with an
internal `class` tag and `deny_unknown_fields`:

| Variant | Module | Covers |
|---------|--------|--------|
| `Bond` | `quotes::bond` | Bond price/yield/spread quotes |
| `Rates` | `quotes::rates` | `RateQuote::{Deposit, Fra, Futures, Swap}` |
| `Cds` | `quotes::cds` | Par spread and upfront CDS quotes |
| `CDSTranche` | `quotes::cds_tranche` | Tranche spread / upfront quotes (`cds_tranche` tag) |
| `Fx` | `quotes::fx` | FX spot and forward quotes |
| `Inflation` | `quotes::inflation` | Zero-coupon and YoY inflation swap quotes |
| `Vol` | `quotes::vol` | Option, swaption, and cap/floor implied-vol quotes |
| `Xccy` | `quotes::xccy` | Cross-currency basis swap quotes |

`quotes::ids::QuoteId` is the stable identifier; `quotes::ids::Pillar` is either
`Pillar::Tenor(Tenor)` or a fixed date. Quotes expose bump helpers (for example
`RateQuote::bump_rate_decimal`) used by the sensitivity paths.

Note that not every variant is bootstrapped. `Fx` and `Bond` exist for
documentation and persistence; the calibration engine consumes FX matrices,
equity spots, and bond prices as snapshot `market_data` entries instead. See the
two-track description in [`../calibration/mod.rs`](../calibration/mod.rs) rustdoc.

## Conventions

`conventions::ConventionRegistry::try_global()` returns a lazily built,
process-wide singleton loaded from JSON embedded at compile time from
[`../../data/conventions/`](../../data/conventions/):

| Registry | Data file | Key type |
|----------|-----------|----------|
| Rate index | `rate_index_conventions.json` | `finstack_quant_core::types::IndexId` |
| CDS | `cds_conventions.json` | `conventions::ids::CdsConventionKey` (currency + `CdsDocClause`) |
| Swaption | `swaption_conventions.json` | `conventions::ids::SwaptionConventionId` |
| Inflation swap | `inflation_swap_conventions.json` | `conventions::ids::InflationSwapConventionId` |
| IR future | `ir_future_conventions.json` | `conventions::ids::IrFutureContractId` |
| Cross-currency | `xccy_conventions.json` | `conventions::ids::XccyConventionId` |

Lookups are strict. `require_rate_index` and its siblings return
`InputError::NotFound` when an id is absent; builders do not silently fall back
to a currency-derived default. `conventions::ids` also defines identifier
newtypes for convention families that are referenced on quotes but not yet
registry-backed (`OptionConventionId`, `CapFloorConventionId`, `FxConventionId`,
`BondConventionId`, `FxOptionConventionId`).

Public convention structs: `RateIndexConventions` (with `RateIndexKind`),
`CdsConventions`, `SwaptionConventions`, `InflationSwapConventions`,
`IrFutureConventions`, `XccyConventions`.

## Data flow

1. Load conventions: `ConventionRegistry::try_global()?`.
2. Deserialize quotes into `MarketQuote` (or a per-asset-class quote type).
3. Build a `BuildCtx` with the valuation date, standard notional, and curve-role
   map, then call the matching `build_*_instrument`.
4. The calibration layer wraps the result in a crate-private `PreparedQuote`
   (quote + instrument + pillar date + precomputed pillar time) for the solvers.

`BuildCtx::new` takes `finstack_quant_core::HashMap` (an `FxHashMap` alias), not
`std::collections::HashMap`. Curve-role keys are builder-specific:

| Builder | Roles read | Missing-role behavior |
|---------|-----------|-----------------------|
| Rates | `"discount"`, `"forward"` | error |
| CDS, CDS tranche | `"discount"`, `"credit"` | error |
| XCCY | `"domestic_discount"`, `"foreign_discount"`, `"domestic_forward"`, `"foreign_forward"` | falls back to convention- or currency-derived curve ids |

So the rates, CDS, and CDS tranche builders fail closed, while the XCCY builder
does not require any role on the context.

## Example: build a deposit instrument from a quote

```rust
use finstack_quant_core::dates::Date;
use finstack_quant_core::types::IndexId;
use finstack_quant_core::HashMap;
use finstack_quant_valuations::market::conventions::ConventionRegistry;
use finstack_quant_valuations::market::quotes::ids::{Pillar, QuoteId};
use finstack_quant_valuations::market::quotes::rates::RateQuote;
use finstack_quant_valuations::market::{build_rate_instrument, BuildCtx};

fn example() -> finstack_quant_core::Result<()> {
    let _registry = ConventionRegistry::try_global()?;

    let as_of = Date::from_calendar_date(2024, time::Month::January, 2)
        .map_err(|e| finstack_quant_core::Error::Validation(e.to_string()))?;
    let ctx = BuildCtx::new(as_of, 1_000_000.0, HashMap::default());

    let quote = RateQuote::Deposit {
        id: QuoteId::new("USD-SOFR-DEP-1M"),
        index: IndexId::new("USD-SOFR-1M"),
        pillar: Pillar::Tenor("1M".parse()?),
        rate: 0.0525, // decimal annual rate
    };

    let _instrument = build_rate_instrument(&quote, &ctx)?;
    Ok(())
}
```

## Conventions that bite

- Quote rates and spreads are **decimals**, not basis points, unless the field
  name says `_bp`.
- Vol quotes carry an explicit quote type; a surface built from mixed quote
  conventions fails rather than blending. `credit_option_vol` queries index
  option surfaces at the **native displayed coordinate** — a decimal spread for
  CDX IG / iTraxx, a clean price in percentage points for CDX HY — and surface
  values are lognormal forward-spread model vols in every case, including on a
  price-quoted strike axis.
- `credit_option_vol`'s spread-vol to hazard-vol mapping is a first-order local
  conversion, not a calibration. It will not exactly reprice the index option it
  came from, and any issuer/index beta is a caller decision.
- All loaders and builders return `finstack_quant_core::Error` variants with
  explicit context, so bad market data surfaces before calibration starts.

## Extending

- New quote type: add a module under `quotes/`, a `MarketQuote` variant, and a
  stable `QuoteId` strategy.
- New convention family: add the struct to `conventions/defs.rs`, a typed id to
  `conventions/ids.rs`, a loader under `conventions/loaders/`, the JSON file
  under `data/conventions/`, and a field plus accessor on `ConventionRegistry`.
- New builder: add it under `build/` and re-export from `mod.rs`.

## TypeScript export

With the `ts_export` feature, quote and calibration schema types derive `ts-rs`
`TS` for client-side interchange and validation.

## Verification

```bash
cargo nextest run -p finstack-quant-valuations --test market
mise run rust-lint
```
