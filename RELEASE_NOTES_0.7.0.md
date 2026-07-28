# Finstack Quant 0.7.0

**Release Date**: 2026-07-27
**Bump type**: minor (pre-1.0; includes intentional breaking API/wire changes)
**Status**: Ready to tag after final CI approval

## Executive Summary

0.7.0 adds a cross-language fixed-income attribution suite, Arrow table
interoperability, path-consistent exposure and MVA analytics, and additional
credit-risk models. It also removes public variants that parsed but were never
implemented and makes MPOR collateral treatment economically consistent.

## Who Should Upgrade

- Portfolio users who need Campisi, credit-excess, hierarchical grid, or
  factor-Brinson attribution in Rust, Python, or WASM.
- Python users moving statements or portfolio tables into PyArrow or Polars.
- XVA users who need MPOR gap risk, path-consistent exposure, or MVA.
- Credit-model users who need CECL WARM, downturn LGD, rating-downgrade staging,
  or EWMA/Ledoit-Wolf factor calibration.

## Breaking Changes

### 1. Unimplemented CECL vintage methodology removed

Persisted configurations using `"methodology": "Vintage"` no longer parse.
Use `PdLgdEad` for the component method or `Warm` for the weighted-average
remaining-maturity practical expedient.

### 2. Unimplemented GARCH factor-volatility choices removed

`VolModelChoice::Garch` and `VolModelChoice::Egarch` no longer deserialize.
Use the sample estimator or the implemented EWMA estimator:

```rust
VolModelChoice::Ewma { lambda: 0.94 }
```

### 3. Campisi `SpreadChangeMode` removed

Pass the realized absolute decimal spread move directly. The former DTS-relative
representation produced the same ex-post return effect and did not belong in
the attribution API:

```rust
FiPositionSnapshot {
    delta_spread: 0.0020, // +20 bp
    // ...
}
```

### 4. MPOR collateral now includes gap risk

When a CSA has `mpor_days > 0`, collateral at time `t` is based on the
interpolated portfolio value at `t - mpor_days / 365`. Recalibrate exposure and
XVA limits that relied on instantaneous collateral for non-constant profiles.

## Highlights

- Campisi fixed-income attribution with Carino multi-period linking.
- Duration-matched credit excess returns from reference instruments or curves.
- Hierarchical duration-cell × sector grid attribution.
- Equality-constrained factor-Brinson attribution.
- Bond and structured-credit `spread_duration`.
- Python Arrow C-stream export and Rust Arrow IPC round trips.
- Deterministic MVA from SIMM decay profiles and path-consistent stochastic
  exposure hooks.
- CECL WARM, downturn LGD, rating-downgrade staging, EWMA volatility, and
  Ledoit-Wolf covariance shrinkage.

## Bug Fixes and Hardening

- Campisi and grid attribution reject near-zero net bucket weights instead of
  producing numerically explosive rates.
- Attribution regression guards are scale-invariant and reject non-finite or
  rank-deficient inputs.
- Python, WASM, stubs, declaration files, and facade exports cover the new
  attribution and MVA APIs.
- Notebook and benchmark fixtures were updated for strict calendar, fixing,
  spread-quote, hazard-calibration, and forecast-parameter requirements.

## Verification

- Numerical parity, golden, and calibration filter: 888 tests passed.
- Python example notebooks: passing after fixture and compatibility updates.
- Rust benchmark targets execute without runtime fixture failures.
- Full workspace `all-test` and `all-ci` gates pass.
- Rust first-crate, Python wheel/sdist, and WASM web/node/npm package dry-runs
  pass.
- Cargo, Python, and npm supply-chain checks report no blocking advisories.

See [`CHANGELOG.md`](CHANGELOG.md) for the complete itemized changes.
