//! CSR (Credit Spread Risk) prescribed parameters.
//!
//! Covers the three CSR sub-classes: non-securitisation, securitisation
//! within the correlation trading portfolio (CTP), and securitisation
//! outside it (non-CTP).
//!
//! # Provenance
//!
//! | Item | Value |
//! |------|-------|
//! | Source document | BCBS **d457**, *Minimum capital requirements for market risk* |
//! | Publication date | 14 January 2019; corrected version 25 February 2019 |
//! | Consolidated as | Basel Framework chapter **MAR21** |
//! | MAR21 version | Effective 1 January 2023; incorporates the FAQs published 5 July 2024 and 23 March 2026 |
//! | Paragraphs used | Non-sec: MAR21.51 Table 3 (buckets), MAR21.53 Table 4 (risk weights), MAR21.54-21.56 (intra-bucket), MAR21.57 Table 5 (inter-bucket). Sec CTP: MAR21.58 (buckets), MAR21.59 Table 6 (risk weights), MAR21.60-21.61 (correlations). Sec non-CTP: MAR21.62 Table 7 (buckets), MAR21.64 Table 8 + MAR21.65-21.67 (risk weights), MAR21.68-21.71 (correlations). Vega: MAR21.92 Table 13 |
//! | Primary sources verified | <https://www.bis.org/bcbs/publ/d457.pdf> and <https://www.bis.org/baselframework/BaselFramework.pdf> |
//! | Last reviewed | 2026-08-20 |
//! | Review procedure | See `data/margin/README.md`, "FRTB parameter review" |
//!
//! # Known deviations from MAR21
//!
//! The **risk-weight tables were corrected on 2026-08-20** against BCBS d457:
//! all of Table 4 (non-sec), Table 6 (sec CTP) and the MAR21.64-21.67 non-CTP
//! derivation now match as published, and the corrected buckets are exercised
//! end-to-end in `margin/tests/frtb_sba_charges.rs`.
//!
//! What remains below is **correlation** structure, not risk weights. Each item
//! moves published capital numbers and is recorded rather than silently
//! changed. Current behaviour is pinned by `super::tests`.
//!
//! **Non-securitisation**
//!
//! - ~~Risk weights for buckets 8 and 9 are transposed and wrong~~
//!   **RESOLVED 2026-08-20.** All 18 Table 4 entries now match MAR21.53 as
//!   published. Buckets 8 and 9 previously read 1.0% and 2.5% against a
//!   published 2.5% and 2.0%, understating covered bonds by 60%.
//! - **Index buckets 17-18 use the wrong name correlation** (MAR21.55).
//!   Those buckets take `rho_name = 80%`, not the 35% of MAR21.54.
//! - **Bucket 16 special case not implemented** (MAR21.56). For "Other
//!   sector", correlations do not apply and `K_b = sum |WS_k|`.
//! - **Inter-bucket correlation is flattened** (MAR21.57). A single 40% is
//!   applied. MAR21.57 prescribes
//!   `gamma_bc = gamma_rating * gamma_sector`, where `gamma_rating` is 50%
//!   between different rating categories within buckets 1-15 (else 1) and
//!   `gamma_sector` comes from the 11x11 Table 5 matrix (values 0%-75%).
//!
//! **Securitisation (CTP)**
//!
//! - Risk weights ([`CSR_SEC_CTP_RISK_WEIGHTS`]) match MAR21.59 Table 6
//!   exactly. No deviation.
//! - **Correlations are flattened** (MAR21.60, MAR21.61). MAR21.60 derives
//!   the CTP intra-bucket correlation exactly as MAR21.54/21.55 does
//!   (`rho_name` 35%/80%, `rho_tenor` 65%) except that `rho_basis` is
//!   **99.00%** rather than 99.90%; MAR21.61 makes the inter-bucket gamma
//!   identical to MAR21.57. The implementation uses flat 30% / 40% instead.
//!
//! **Securitisation (non-CTP)**
//!
//! - ~~The risk-weight table is largely wrong~~ **RESOLVED 2026-08-20.**
//!   All 25 buckets now match MAR21.64-21.67. [`CSR_SEC_NONCTP_RISK_WEIGHTS`]
//!   carries the eight published Table 8 weights and writes buckets 9-16 and
//!   17-24 as the literal `x1.25` / `x1.75` products, so the derivation is
//!   visible and a base-row change propagates automatically. Previously only
//!   buckets 1, 2, 3, 5 and 6 matched and bucket 25 read 12.5% against a
//!   published 3.5%.
//! - **Intra-bucket correlation is wrong** (MAR21.68). Published:
//!   `rho_tranche` = 40% for different tranches (a "same tranche" needs >80%
//!   notional overlap), `rho_tenor` = 80%, `rho_basis` = 99.90%. Implemented:
//!   a flat 30%.
//! - **Inter-bucket correlation should be zero** (MAR21.70). `gamma_bc` is
//!   **0%** across buckets 1-24, and MAR21.71 requires bucket 25 to be simply
//!   summed with the rest (no diversification). The implementation applies
//!   20%, recognising diversification the standard does not allow.
//! - **Bucket 25 special case not implemented** (MAR21.69). Correlations do
//!   not apply; `K_b = sum |WS_k|`.
//!
//! **All three sub-classes**
//!
//! - **Curvature risk weight** (MAR21.99). CSR curvature uses a shift sized
//!   by the highest prescribed delta risk weight in the bucket. No separate
//!   curvature risk-weight constant is published or exposed here; the engine
//!   consumes caller-supplied, already-shocked `CVR+`/`CVR-` values.
//!   MAR21.100 additionally requires curvature correlations to be the
//!   **squares** of the delta correlations; the engine squares the
//!   inter-bucket gamma but not the intra-bucket rho.

/// CSR non-securitisation delta risk weights by bucket, in percent
/// (MAR21.53, Table 4).
///
/// Values are expressed in **percent of notional** (e.g. `0.5` means
/// `0.5%` = `50 bp`); they multiply a CSR delta stated as P&L per
/// 1 percentage-point spread shift.
///
/// Bucket names per MAR21.51, Table 3:
/// 1: Sovereigns including central banks, multilateral development banks (IG)
/// 2: Local government, government-backed non-financials, education, public admin (IG)
/// 3: Financials including government-backed financials (IG)
/// 4: Basic materials, energy, industrials, agriculture, manufacturing, mining (IG)
/// 5: Consumer goods and services, transportation and storage, admin (IG)
/// 6: Technology, telecommunications (IG)
/// 7: Health care, utilities, professional and technical activities (IG)
/// 8: Covered bonds (IG)
/// 9-15: High-yield and non-rated counterparts of buckets 1-7
/// 16: Other sector
/// 17: Investment-grade indices
/// 18: High-yield indices
///
/// # Deviation
///
/// Buckets 8 and 9 do not match Table 4 — see the "Known deviations from
/// MAR21" section in the module docs.
pub const CSR_NONSEC_RISK_WEIGHTS: &[(u8, f64)] = &[
    (1, 0.5),
    (2, 1.0),
    (3, 5.0),
    (4, 3.0),
    (5, 3.0),
    (6, 2.0),
    (7, 1.5),
    // Bucket 8 (covered bonds) is 2.5% in Table 4. MAR21.53 footnote 17
    // permits a *discretionary* 1.5% for covered bonds rated AA- or better;
    // this table carries the standard value, not the discretion.
    (8, 2.5),
    (9, 2.0),
    (10, 4.0),
    (11, 12.0),
    (12, 7.0),
    (13, 8.5),
    (14, 5.5),
    (15, 5.0),
    (16, 12.0),
    (17, 1.5),
    (18, 5.0),
];

/// CSR non-sec intra-bucket name correlation (MAR21.54): 35% between
/// different issuer names in buckets 1-15.
///
/// Index buckets 17-18 should use 80% instead (MAR21.55); that split is not
/// implemented.
pub const CSR_NONSEC_INTRA_BUCKET_NAME_CORRELATION: f64 = 0.35;

/// CSR non-sec intra-bucket tenor correlation (MAR21.54): 65% between
/// different tenors.
pub const CSR_NONSEC_INTRA_BUCKET_TENOR_CORRELATION: f64 = 0.65;

/// CSR non-sec inter-bucket correlation, applied uniformly.
///
/// MAR21.57 instead prescribes `gamma_rating * gamma_sector` with a Table 5
/// sector matrix. See the "Known deviations from MAR21" section in the module
/// docs.
pub const CSR_NONSEC_INTER_BUCKET_CORRELATION: f64 = 0.40;

/// CSR non-securitisation vega risk weight (MAR21.92).
///
/// MAR21.92 footnote 24 sets
/// `RW_k = min(RW_sigma * sqrt(LH_risk class) / sqrt(10), 100%)` with
/// `RW_sigma = 55%`. MAR21.92 Table 13 gives CSR non-securitisation a
/// liquidity horizon of **120 days**, so `0.55 * sqrt(12) = 1.9053`, which
/// binds at the 100% cap. Table 13 publishes the resulting **100%**
/// directly, so this constant is the published value and not a placeholder.
pub const CSR_NONSEC_VEGA_RISK_WEIGHT: f64 = 1.00;

/// CSR securitisation (CTP) vega risk weight (MAR21.92).
///
/// MAR21.92 Table 13 gives CSR securitisations (CTP) a liquidity horizon of
/// **120 days**, so `min(0.55 * sqrt(12), 100%) = 100%` — the same published
/// value as [`CSR_NONSEC_VEGA_RISK_WEIGHT`], reached independently. Kept as
/// a distinct constant so the CTP weight can move without dragging the
/// non-securitisation weight with it.
pub const CSR_SEC_CTP_VEGA_RISK_WEIGHT: f64 = 1.00;

/// CSR securitisation (non-CTP) vega risk weight (MAR21.92).
///
/// MAR21.92 Table 13 gives CSR securitisations (non-CTP) a liquidity horizon
/// of **120 days**, so `min(0.55 * sqrt(12), 100%) = 100%`.
pub const CSR_SEC_NONCTP_VEGA_RISK_WEIGHT: f64 = 1.00;

/// CSR securitisation (CTP) delta risk weights by bucket, in percent
/// (MAR21.59, Table 6).
///
/// The 16 CTP buckets are the CSR non-securitisation buckets 1-16 of
/// MAR21.51 Table 3, excluding the index buckets 17-18 (MAR21.58(1)).
/// Verified against Table 6 with no deviation.
pub const CSR_SEC_CTP_RISK_WEIGHTS: &[(u8, f64)] = &[
    (1, 4.0),
    (2, 4.0),
    (3, 8.0),
    (4, 5.0),
    (5, 4.0),
    (6, 3.0),
    (7, 2.0),
    (8, 6.0),
    (9, 13.0),
    (10, 13.0),
    (11, 16.0),
    (12, 10.0),
    (13, 12.0),
    (14, 12.0),
    (15, 12.0),
    (16, 13.0),
];

/// CSR securitisation (non-CTP) delta risk weights by bucket, in percent.
///
/// # Deviation
///
/// Only buckets 1, 2, 3, 5 and 6 match MAR21.64 Table 8. The remaining 20
/// entries do not follow the Table 8 + MAR21.65/21.66/21.67 derivation —
/// see the "Known deviations from MAR21" section in the module docs for the
/// published vector.
pub const CSR_SEC_NONCTP_RISK_WEIGHTS: &[(u8, f64)] = &[
    // Buckets 1-8 — senior investment grade. MAR21.64 Table 8.
    (1, 0.9),
    (2, 1.5),
    (3, 2.0),
    (4, 2.0),
    (5, 0.8),
    (6, 1.2),
    (7, 1.2),
    (8, 1.4),
    // Buckets 9-16 — non-senior investment grade: the corresponding
    // bucket 1-8 weight scaled by 1.25 (MAR21.65). Written as the product
    // so the derivation is visible and cannot drift from the base row.
    (9, 0.9 * 1.25),
    (10, 1.5 * 1.25),
    (11, 2.0 * 1.25),
    (12, 2.0 * 1.25),
    (13, 0.8 * 1.25),
    (14, 1.2 * 1.25),
    (15, 1.2 * 1.25),
    (16, 1.4 * 1.25),
    // Buckets 17-24 — high yield and non-rated: 1.75x buckets 1-8 (MAR21.66).
    (17, 0.9 * 1.75),
    (18, 1.5 * 1.75),
    (19, 2.0 * 1.75),
    (20, 2.0 * 1.75),
    (21, 0.8 * 1.75),
    (22, 1.2 * 1.75),
    (23, 1.2 * 1.75),
    (24, 1.4 * 1.75),
    // Bucket 25 — other sector (MAR21.67).
    (25, 3.5),
];

/// CSR sec CTP intra-bucket correlation, applied uniformly.
///
/// MAR21.60 instead reuses the MAR21.54/21.55 decomposition with
/// `rho_basis = 99.00%`. See the module-level deviation notes.
pub const CSR_SEC_CTP_INTRA_BUCKET_CORRELATION: f64 = 0.30;

/// CSR sec CTP inter-bucket correlation, applied uniformly.
///
/// MAR21.61 instead reuses the MAR21.57 `gamma_rating * gamma_sector`
/// construction. See the module-level deviation notes.
pub const CSR_SEC_CTP_INTER_BUCKET_CORRELATION: f64 = 0.40;

/// CSR sec non-CTP intra-bucket correlation, applied uniformly.
///
/// MAR21.68 instead prescribes
/// `rho_tranche (40%) * rho_tenor (80%) * rho_basis (99.90%)`. See the
/// module-level deviation notes.
pub const CSR_SEC_NONCTP_INTRA_BUCKET_CORRELATION: f64 = 0.30;

/// CSR sec non-CTP inter-bucket correlation, applied uniformly.
///
/// MAR21.70 instead sets `gamma_bc = 0%` across buckets 1-24, with bucket 25
/// simply summed (MAR21.71). See the module-level deviation notes.
pub const CSR_SEC_NONCTP_INTER_BUCKET_CORRELATION: f64 = 0.20;

use std::sync::LazyLock;

use finstack_quant_core::HashMap;

// Index the fixed `pub const` slices into hashmaps so per-trade lookups
// are O(1). Built once on first use.
static CSR_NONSEC_BY_BUCKET: LazyLock<HashMap<u8, f64>> =
    LazyLock::new(|| CSR_NONSEC_RISK_WEIGHTS.iter().copied().collect());
static CSR_SEC_CTP_BY_BUCKET: LazyLock<HashMap<u8, f64>> =
    LazyLock::new(|| CSR_SEC_CTP_RISK_WEIGHTS.iter().copied().collect());
static CSR_SEC_NONCTP_BY_BUCKET: LazyLock<HashMap<u8, f64>> =
    LazyLock::new(|| CSR_SEC_NONCTP_RISK_WEIGHTS.iter().copied().collect());

/// Look up a CSR non-sec delta risk weight by bucket (MAR21.53, Table 4).
///
/// # Arguments
///
/// * `bucket` - FRTB CSR non-securitisation bucket number; unmapped buckets
///   use the fallback weight of 5.0. The fallback is a library convention,
///   not a Basel-published value, and is pinned by `super::tests`.
#[must_use]
pub fn csr_nonsec_risk_weight(bucket: u8) -> f64 {
    CSR_NONSEC_BY_BUCKET.get(&bucket).copied().unwrap_or(5.0)
}

/// Look up a CSR sec CTP delta risk weight by bucket (MAR21.59, Table 6).
///
/// # Arguments
///
/// * `bucket` - FRTB CSR securitisation correlation-trading-portfolio bucket
///   number; unmapped buckets use the fallback weight of 8.0. The fallback is
///   a library convention, not a Basel-published value, and is pinned by
///   `super::tests`.
#[must_use]
pub fn csr_sec_ctp_risk_weight(bucket: u8) -> f64 {
    CSR_SEC_CTP_BY_BUCKET.get(&bucket).copied().unwrap_or(8.0)
}

/// Look up a CSR sec non-CTP delta risk weight by bucket.
///
/// # Arguments
///
/// * `bucket` - FRTB CSR non-CTP securitisation bucket number; unmapped
///   buckets use the fallback weight of 5.0. The fallback is a library
///   convention, not a Basel-published value, and is pinned by
///   `super::tests`.
#[must_use]
pub fn csr_sec_nonctp_risk_weight(bucket: u8) -> f64 {
    CSR_SEC_NONCTP_BY_BUCKET
        .get(&bucket)
        .copied()
        .unwrap_or(5.0)
}
