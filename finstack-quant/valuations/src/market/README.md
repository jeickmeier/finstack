# Market

Valuation-owned market conventions and option-volatility lookup policy.

Quote DTOs, quote-to-instrument construction, date resolution, calibration, and
recalibration live in `finstack-quant-calibration`. This module deliberately
contains no quote ingestion or calibration engine.

## Layout

| Path | Visibility | Contents |
|------|------------|----------|
| `conventions/` | public | Convention definitions, typed IDs, and the global registry |
| `credit_option_vol.rs` | public | CDX/iTraxx option-surface lookup converted to additive hazard volatility |

## Conventions

`conventions::ConventionRegistry::try_global()` returns a lazily built,
process-wide singleton loaded from JSON embedded at compile time from
`../../data/conventions/`.

| Registry | Data file | Key type |
|----------|-----------|----------|
| Rate index | `rate_index_conventions.json` | `finstack_quant_core::types::IndexId` |
| CDS | `cds_conventions.json` | `conventions::ids::CdsConventionKey` |
| Swaption | `swaption_conventions.json` | `conventions::ids::SwaptionConventionId` |
| Inflation swap | `inflation_swap_conventions.json` | `conventions::ids::InflationSwapConventionId` |
| IR future | `ir_future_conventions.json` | `conventions::ids::IrFutureContractId` |
| Cross-currency | `xccy_conventions.json` | `conventions::ids::XccyConventionId` |

Lookups are strict. `require_rate_index` and its siblings return
`InputError::NotFound` when an ID is absent. Calibration builders depend on
these valuation-owned conventions and do not silently fall back to unrelated
currency defaults.

`conventions::ids` also defines typed identifiers referenced by instruments
and calibration quotes, including `OptionConventionId`,
`CapFloorConventionId`, `FxConventionId`, `BondConventionId`, and
`FxOptionConventionId`.

## Credit option volatility

`credit_option_vol` queries index-option surfaces at the native displayed
coordinate: decimal spread for CDX IG and iTraxx, clean price in percentage
points for CDX HY. Surface values are lognormal forward-spread model
volatilities.

Its spread-volatility to hazard-volatility mapping is a first-order local
conversion, not calibration. It does not exactly reprice the source index
option, and issuer/index beta remains a caller decision.

## Dependency boundary

The permanent direction is:

```text
finstack-quant-calibration -> finstack-quant-valuations -> core/models/cashflows
```

Valuations exposes the object-safe recalibration contract in
`crate::recalibration`; calibration implements it. Pricing receives that
service through `PricingOptions` and never imports the calibration crate.
