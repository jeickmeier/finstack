//! XVA (valuation adjustments) types and deterministic exposure engines.
//!
//! Public surface: [`types`] (configuration, exposure profiles, results),
//! `exposure::compute_exposure_profile`, `cva::compute_cva`,
//! `cva::compute_dva`, `cva::compute_fva`, `cva::compute_bilateral_xva`,
//! `mva::compute_mva`, `mva::im_profile_from_simm`, and the deterministic
//! netting helpers.
//!
//! # Conventions
//!
//! - Exposure times are year fractions.
//! - Deterministic exposure assumes constant curves on the roll-forward grid.
//! - Close-out netting follows an ISDA master-agreement view.
//! - Adjustments are positive when they cost the desk and compose as
//!   `bilateral_cva = CVA − DVA + FVA` for legacy compatibility and
//!   `total_xva = bilateral_cva + MVA` (all-in). `cva::compute_bilateral_xva`
//!   computes MVA and reduces bilateral DVA by posted IM whenever
//!   `types::FundingConfig::im_profile` is set.
//! - CSA terms reduce exposure; MPOR-lagged collateral (gap risk) is applied
//!   whenever `csa.mpor_days > 0`, in both the deterministic and stochastic
//!   engines.
//!
//! # Stochastic exposure
//!
//! Pathwise exposure with quantile-based PFE uses `finstack-quant-monte-carlo`:
//! `exposure::compute_stochastic_exposure_profile` (generic valuation callback)
//! and `exposure::compute_stochastic_exposure_with_valuer` (path-consistent
//! portfolio repricing via `traits::PathValuer`, with per-path MPOR-lagged CSA
//! collateral and optional per-path IM via `mva::PathImModel`). Defaults are in
//! `data/margin/xva_defaults.v1.json`.
//!
//! # References
//!
//! - Gregory XVA Challenge: `docs/REFERENCES.md#gregory-xva-challenge`
//! - Green XVA: `docs/REFERENCES.md#green-xva`
//! - ISDA 2002 Master Agreement: `docs/REFERENCES.md#isda-2002-master-agreement`
//! - BCBS 279 SA-CCR: `docs/REFERENCES.md#bcbs-279-saccr`

/// CVA, DVA, FVA, MVA aggregation, and bilateral-XVA integration formulas.
pub mod cva;
/// Deterministic and stochastic exposure engines.
pub mod exposure;
/// Margin Valuation Adjustment (MVA) on SIMM-based IM profiles.
pub mod mva;
/// Netting and collateral-reduction helpers.
pub mod netting;
/// Minimal trait surface for XVA-compatible instruments.
pub mod traits;
/// Shared XVA configuration and result container types.
pub mod types;
