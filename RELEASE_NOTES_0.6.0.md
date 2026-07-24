# Finstack Quant 0.6.0

**Release Date**: 2026-07-23
**Bump type**: minor (pre-1.0; includes intentional breaking wire/API changes)
**Status**: Ready to commit and tag after `mise run all-ci` / user approval

## Executive Summary

0.6.0 hardens statements, structured credit, portfolio analytics, and schedule
conventions; removes the lenient Heston market-parameter helper; and restores
dual-license / changelog / security governance files required for publish.

## Who Should Upgrade

- Anyone pricing or risking **structured credit**, **portfolio scenarios**, or
  **statement waterfalls**.
- Consumers of **Heston** closed-form parameters (migration required).
- Callers that serialize **config** or **structured-credit** JSON with unknown
  fields (deserialization is now strict).

## Breaking Changes

### 1. `HestonParams::from_market` removed

Use `HestonParams::from_market_strict` and supply all five unitless market
scalars: `HESTON_KAPPA`, `HESTON_THETA`, `HESTON_SIGMA_V`, `HESTON_RHO`,
`HESTON_V0`.

```rust
let params = HestonParams::from_market_strict(&market, r, q)?;
```

### 2. Strict unknown-field rejection on config and structured-credit JSON

`FinstackConfig` / rounding / currency-scale / tolerances objects, and nested
structured-credit payloads, reject unknown keys. Remove stray keys or namespace
extensions under `{crate}.{domain}.v{N}`.

### 3. Structured-credit spread metrics need a deal-level quote

`ZSpread` / `Cs01` / `BucketedCs01` / `SpreadDuration` fail unless
`metric_pricing_overrides.quoted_price_pct` is set (percent of original balance).

### 4. `CapFloor` compounded-RFR public fields

Exhaustive Rust struct literals must include `spread` and `overnight_coupon`.
Serde and builders still default them for legacy payloads.

### 5. Systematic-factor semantics when `kappa = 0`

Zero mean reversion holds one systematic draw for the full horizon. Recalibrate
scenarios that treated `kappa = 0` as monthly independence.

## Highlights

- Batched portfolio `scenario_pnl_batch` with shared base valuation.
- Statements: interest income vs expense split, finite waterfall inputs,
  bounded formula parsing (DoS), duplicate-ID rejection.
- Structured credit: Python tranche analytics, cash conservation, OAS path
  coupons, senior fees, Richard-Roll burnout/refi, Monte Carlo default engine.
- IMM / CDS-IMM schedule rolls; term-loan step-up coupons.
- Hull-White cap/floor vega and calibration aligned to contractual schedules.
- Dual license: MIT OR Apache-2.0.

## Deprecated (kept until 1.0.0)

- `Estimate::with_num_skipped` — engines reject non-finite payoffs; field retained
  for serde compatibility only.

## Migration Checklist

1. Replace `HestonParams::from_market` with `from_market_strict`.
2. Strip unknown keys from config and structured-credit JSON.
3. Set `quoted_price_pct` before requesting structured-credit spread metrics.
4. Update exhaustive `CapFloor` Rust literals for new RFR fields.
5. Recalibrate `kappa = 0` structured-credit scenarios if independence was assumed.

## Known Limitations

- Amortizing-bond MOIC/XIRR-to-worst still uses initial notional as redemption
  basis (documented TODO for a follow-up).
- Umbrella-crate `cargo semver-checks` re-export surface is thin; treat CHANGELOG
  behavioral breaks as authoritative for pre-1.0 consumers.

See [`CHANGELOG.md`](CHANGELOG.md) for the full itemized list.
