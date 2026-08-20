//! FX risk prescribed parameters.
//!
//! # Provenance
//!
//! | Item | Value |
//! |------|-------|
//! | Source document | BCBS **d457**, *Minimum capital requirements for market risk* |
//! | Publication date | 14 January 2019; corrected version 25 February 2019 |
//! | Consolidated as | Basel Framework chapter **MAR21** |
//! | MAR21 version | Effective 1 January 2023; incorporates the FAQs published 5 July 2024 and 23 March 2026 |
//! | Paragraphs used | MAR21.87 (delta risk weight), MAR21.89 (inter-bucket correlation), MAR21.92 Table 13 (vega risk weight) |
//! | Primary sources verified | <https://www.bis.org/bcbs/publ/d457.pdf> and <https://www.bis.org/baselframework/BaselFramework.pdf> |
//! | Last reviewed | 2026-08-20 |
//! | Review procedure | See `data/margin/README.md`, "FRTB parameter review" |
//!
//! # Known deviations from MAR21
//!
//! - **Specified-currency-pair relief not implemented** (MAR21.88). The 15%
//!   delta risk weight may be divided by `sqrt(2)` for the currency pairs
//!   listed in MAR21.88 footnote 22 and their first-order crosses. Only the
//!   unrelieved weight is exposed here, which overstates capital for those
//!   pairs.
//! - **FX curvature scalar not implemented** (MAR21.98). For options that do
//!   not reference the bank's reporting (or base) currency, `CVR+` and `CVR-`
//!   may be divided by a scalar of **1.5**. The engine consumes
//!   caller-supplied `CVR` values, so a caller wishing to take that relief
//!   must apply the scalar before calling.
//! - **Curvature risk weight** (MAR21.98). For FX the curvature shock is a
//!   relative shift equal to the delta risk weight; no separate curvature
//!   risk-weight constant is published or exposed here.

/// FX delta risk weight, uniform across all currency pairs (MAR21.87).
///
/// MAR21.87: "A unique relative risk weight equal to 15% applies to all the
/// FX sensitivities." Expressed in percent (`15.0` = 15%).
pub const FX_DELTA_RISK_WEIGHT: f64 = 15.0;

/// FX vega risk weight after liquidity-horizon scaling (MAR21.92).
///
/// MAR21.92 footnote 24 sets
/// `RW_k = min(RW_sigma * sqrt(LH_risk class) / sqrt(10), 100%)` with
/// `RW_sigma = 55%`. MAR21.92 Table 13 gives FX a liquidity horizon of
/// **40 days**, so `0.55 * sqrt(4) = 1.10`, which binds at the 100% cap.
/// Table 13 publishes the resulting **100%** directly, so this constant is
/// the published value and not a placeholder.
pub const FX_VEGA_RISK_WEIGHT: f64 = 1.00;

/// FX inter-bucket (cross-pair) correlation (MAR21.89): a uniform 60%.
pub const FX_INTER_PAIR_CORRELATION: f64 = 0.60;
