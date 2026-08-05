//! XVA (valuation adjustments) types and aggregation formulas.
//!
//! Public surface: [`types`] (configuration, exposure profiles, results),
//! `cva::compute_cva`, `cva::compute_dva`, `cva::compute_fva`,
//! `cva::compute_bilateral_xva`, `mva::compute_mva`,
//! `mva::im_profile_from_simm`, and the netting helpers.
//!
//! # Conventions
//!
//! - Exposure times are year fractions.
//! - Close-out netting follows an ISDA master-agreement view.
//! - Adjustments are positive when they cost the desk and compose as
//!   `total_xva = CVA − DVA + FVA + MVA`. `cva::compute_bilateral_xva`
//!   computes MVA and reduces DVA by posted IM whenever
//!   `types::FundingConfig::im_profile` is set.
//! - CSA terms reduce exposure; MPOR-lagged collateral (gap risk) is applied
//!   whenever `csa.mpor_days > 0`.
//!
//! # Exposure profiles
//!
//! [`crate::xva::types::ExposureProfile`] is a container: callers supply the EPE / ENE grid
//! from their own simulation or valuation stack, and the formulas here turn it
//! into CVA / DVA / FVA / MVA. Defaults are in
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
/// Margin Valuation Adjustment (MVA) on SIMM-based IM profiles.
pub mod mva;
/// Netting and collateral-reduction helpers.
pub mod netting;
/// Shared XVA configuration and result container types.
pub mod types;
