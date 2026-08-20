//! Equity risk prescribed parameters.
//!
//! # Provenance
//!
//! | Item | Value |
//! |------|-------|
//! | Source document | BCBS **d457**, *Minimum capital requirements for market risk* |
//! | Publication date | 14 January 2019; corrected version 25 February 2019 |
//! | Consolidated as | Basel Framework chapter **MAR21** |
//! | MAR21 version | Effective 1 January 2023; incorporates the FAQs published 5 July 2024 and 23 March 2026 |
//! | Paragraphs used | MAR21.72 Table 9 (buckets), MAR21.77 Table 10 (delta risk weights), MAR21.78 (intra-bucket correlation), MAR21.80 (inter-bucket correlation), MAR21.92 Table 13 (vega risk weight) |
//! | Primary sources verified | <https://www.bis.org/bcbs/publ/d457.pdf> and <https://www.bis.org/baselframework/BaselFramework.pdf> |
//! | Last reviewed | 2026-08-20 |
//! | Review procedure | See `data/margin/README.md`, "FRTB parameter review" |
//!
//! # Known deviations from MAR21
//!
//! Recorded rather than silently corrected: each one moves published capital
//! numbers and needs explicit sign-off. Pinned by the tests in
//! `super::tests`.
//!
//! - **Vega risk weight is rounded, and is not bucket-dependent**
//!   (MAR21.92 Table 13). See [`EQUITY_VEGA_RISK_WEIGHT`].
//! - **Equity repo-rate risk weights are missing** (MAR21.77, Table 10).
//!   Table 10 has two columns: a spot risk weight and a repo-rate risk
//!   weight equal to `spot / 100` (e.g. bucket 1: 55% spot, 0.55% repo).
//!   Only the spot column is implemented, so MAR21.22 equity repo-rate
//!   sensitivities have no weight of their own.
//! - **Intra-bucket correlation is flattened** (MAR21.78). A single 15% is
//!   applied to every bucket. MAR21.78 prescribes 15% for buckets 1-4
//!   (large cap, emerging market), **25%** for buckets 5-8 (large cap,
//!   advanced), **7.5%** for bucket 9, **12.5%** for bucket 10 and **80%**
//!   for the index buckets 12-13, plus a 99.90% rule for spot-versus-repo
//!   pairs on the same issuer name. MAR21.79 also disapplies correlations
//!   entirely for bucket 11 (`K_b = sum |WS_k|`).
//! - **Inter-bucket correlation is flattened** (MAR21.80). A single 15% is
//!   applied. MAR21.80 prescribes 15% only when both buckets are in 1-10;
//!   **0%** if either bucket is 11; **75%** between buckets 12 and 13; and
//!   **45%** otherwise.
//! - **Curvature risk weight** (MAR21.98). For equity the curvature shock is
//!   a relative shift equal to the delta risk weight; no separate curvature
//!   risk-weight constant is published or exposed here.

/// Equity **spot** delta risk weights by bucket, in percent
/// (MAR21.77, Table 10, spot column).
///
/// Bucket names per MAR21.72 Table 9:
/// 1-4: Large cap, emerging market economy
/// 5-8: Large cap, advanced economy
/// 9: Small cap, emerging market economy
/// 10: Small cap, advanced economy
/// 11: Other sector
/// 12: Large-cap advanced-economy equity indices
/// 13: Other equity indices
pub const EQUITY_RISK_WEIGHTS: &[(u8, f64)] = &[
    (1, 55.0),
    (2, 60.0),
    (3, 45.0),
    (4, 55.0),
    (5, 30.0),
    (6, 35.0),
    (7, 40.0),
    (8, 50.0),
    (9, 70.0),
    (10, 50.0),
    (11, 70.0),
    (12, 15.0),
    (13, 25.0),
];

/// Equity intra-bucket correlation applied to every bucket.
///
/// 15% is the MAR21.78(2)(a) value for buckets 1-4 only. See the "Known
/// deviations from MAR21" section in the module docs.
pub const EQUITY_INTRA_BUCKET_CORRELATION: f64 = 0.15;

/// Equity inter-bucket correlation applied to every bucket pair.
///
/// 15% is the MAR21.80(1) value for pairs drawn from buckets 1-10 only.
/// See the "Known deviations from MAR21" section in the module docs.
pub const EQUITY_INTER_BUCKET_CORRELATION: f64 = 0.15;

/// Equity vega risk weight after liquidity-horizon scaling (MAR21.92).
///
/// MAR21.92 footnote 24 sets
/// `RW_k = min(RW_sigma * sqrt(LH_risk class) / sqrt(10), 100%)` with
/// `RW_sigma = 55%`. MAR21.92 Table 13 splits equity in two:
///
/// | Table 13 row | LH | Published RW |
/// |--------------|----|--------------|
/// | Equity (large cap and indices) | 20 days | **77.78%** (`0.55 * sqrt(2)`) |
/// | Equity (small cap and other sector) | 60 days | **100%** (`0.55 * sqrt(6)` capped) |
///
/// The FAQ under MAR21.92 maps the 20-day row to buckets **1-8 and 12-13**
/// and the 60-day row to buckets **9, 10 and 11**.
///
/// # Deviations
///
/// This constant is `0.78`, applied to every equity bucket. That differs
/// from MAR21 in two ways:
///
/// 1. **Rounding.** The published figure is 77.78%, not 78%; `0.78`
///    overstates large-cap and index equity vega capital by about 0.29%
///    relative.
/// 2. **Missing bucket split.** Buckets 9, 10 and 11 should carry a **100%**
///    vega risk weight; applying 77.78% understates their vega capital by
///    about 22%.
///
/// Neither has been changed here, because both move published capital
/// numbers. The current value is pinned by
/// `equity_vega_risk_weight_pins_known_deviation_from_mar21_92` in
/// `super::tests`.
pub const EQUITY_VEGA_RISK_WEIGHT: f64 = 0.78;

use std::sync::LazyLock;

use finstack_quant_core::HashMap;

static EQUITY_RW_BY_BUCKET: LazyLock<HashMap<u8, f64>> =
    LazyLock::new(|| EQUITY_RISK_WEIGHTS.iter().copied().collect());

/// Look up an equity spot delta risk weight by bucket (MAR21.77, Table 10).
///
/// # Arguments
///
/// * `bucket` - FRTB equity risk bucket number; unmapped buckets use the
///   fallback risk weight of 55.0. The fallback is a library convention, not
///   a Basel-published value, and is pinned by `super::tests`.
#[must_use]
pub fn equity_risk_weight(bucket: u8) -> f64 {
    EQUITY_RW_BY_BUCKET.get(&bucket).copied().unwrap_or(55.0)
}
