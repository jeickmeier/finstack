//! Commodity risk prescribed parameters.
//!
//! # Provenance
//!
//! | Item | Value |
//! |------|-------|
//! | Source document | BCBS **d457**, *Minimum capital requirements for market risk* |
//! | Publication date | 14 January 2019; corrected version 25 February 2019 |
//! | Consolidated as | Basel Framework chapter **MAR21** |
//! | MAR21 version | Effective 1 January 2023; incorporates the FAQs published 5 July 2024 and 23 March 2026 |
//! | Paragraphs used | MAR21.81 (buckets), MAR21.82 Table 11 (delta risk weights), MAR21.83 Table 12 (intra-bucket correlation), MAR21.85 (inter-bucket correlation), MAR21.92 Table 13 (vega risk weight) |
//! | Primary sources verified | <https://www.bis.org/bcbs/publ/d457.pdf> and <https://www.bis.org/baselframework/BaselFramework.pdf> |
//! | Last reviewed | 2026-08-20 |
//! | Review procedure | See `data/margin/README.md`, "FRTB parameter review" |
//!
//! # Known deviations from MAR21
//!
//! Recorded rather than silently corrected: each one moves published capital
//! numbers and needs explicit sign-off. Pinned by the tests in
//! `super::tests` and by `margin/tests/frtb_sba_charges.rs`.
//!
//! - **Intra-bucket correlation is flattened** (MAR21.83).
//!   [`COMMODITY_INTRA_BUCKET_CORRELATION`] applies a single 55% to every
//!   bucket. MAR21.83 specifies
//!   `rho_kl = rho_cty * rho_tenor * rho_basis`, where `rho_cty` is the
//!   **per-bucket** Table 12 vector
//!   `[55, 95, 40, 80, 60, 65, 55, 45, 15, 40, 15]%`, `rho_tenor` is 99.00%
//!   for different tenors, and `rho_basis` is 99.90% for different delivery
//!   locations. 55% is the published value for buckets 1 and 7 only; all
//!   other buckets are wrong, in both directions.
//! - **Inter-bucket correlation is missing the bucket-11 carve-out**
//!   (MAR21.85). The 20% in [`COMMODITY_INTER_BUCKET_CORRELATION`] is the
//!   published MAR21.85(1) value for bucket pairs drawn from buckets 1-10,
//!   but MAR21.85(2) sets gamma to **0%** whenever either bucket is 11
//!   ("Other commodity"). Applying 20% there overstates diversification
//!   benefit and understates capital.
//! - **Curvature risk weight** (MAR21.99). Commodity curvature uses a
//!   *relative* shock sized by the highest prescribed delta risk weight in
//!   the curvature bucket. No separate curvature risk-weight constant is
//!   published or exposed here; the engine consumes caller-supplied,
//!   already-shocked `CVR+`/`CVR-` values.

/// Commodity delta risk weights by bucket, in percent (MAR21.82, Table 11).
///
/// Bucket names per MAR21.81:
/// 1: Energy - solid combustibles
/// 2: Energy - liquid combustibles
/// 3: Energy - electricity and carbon trading
/// 4: Freight
/// 5: Metals - non-precious
/// 6: Gaseous combustibles
/// 7: Precious metals (including gold)
/// 8: Grains and oilseed
/// 9: Livestock and dairy
/// 10: Softs and other agriculturals
/// 11: Other commodity
pub const COMMODITY_RISK_WEIGHTS: &[(u8, f64)] = &[
    (1, 30.0),
    (2, 35.0),
    (3, 60.0),
    (4, 80.0),
    (5, 40.0),
    (6, 45.0),
    (7, 20.0),
    (8, 35.0),
    (9, 25.0),
    (10, 35.0),
    (11, 50.0),
];

/// Commodity intra-bucket correlation applied to every bucket.
///
/// 55% is the MAR21.83 Table 12 `rho_cty` entry for buckets 1 and 7 only.
/// See the "Known deviations from MAR21" section in the module docs.
pub const COMMODITY_INTRA_BUCKET_CORRELATION: f64 = 0.55;

/// Commodity inter-bucket correlation (MAR21.85(1)).
///
/// Published as 20% for bucket pairs drawn from buckets 1-10. MAR21.85(2)
/// sets gamma to 0% when either bucket is 11; that carve-out is not applied.
/// See the "Known deviations from MAR21" section in the module docs.
pub const COMMODITY_INTER_BUCKET_CORRELATION: f64 = 0.20;

/// Commodity vega risk weight after liquidity-horizon scaling (MAR21.92).
///
/// MAR21.92 footnote 24 sets
/// `RW_k = min(RW_sigma * sqrt(LH_risk class) / sqrt(10), 100%)` with
/// `RW_sigma = 55%`. MAR21.92 Table 13 gives commodity a liquidity horizon
/// of **120 days**, so `0.55 * sqrt(12) = 1.9053`, which binds at the 100%
/// cap. Table 13 publishes the resulting **100%** directly, so this constant
/// is the published value and not a placeholder.
pub const COMMODITY_VEGA_RISK_WEIGHT: f64 = 1.00;

/// Look up a commodity delta risk weight by bucket (MAR21.82, Table 11).
///
/// # Arguments
///
/// * `bucket` - FRTB commodity risk bucket number; unmapped buckets use the
///   fallback risk weight of 20.0. The fallback is a library convention, not
///   a Basel-published value, and is pinned by `super::tests`.
#[must_use]
pub fn commodity_risk_weight(bucket: u8) -> f64 {
    COMMODITY_RISK_WEIGHTS
        .iter()
        .find(|(b, _)| *b == bucket)
        .map(|(_, w)| *w)
        .unwrap_or(20.0) // Default for unmapped buckets
}
