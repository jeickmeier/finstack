//! Netting and collateral logic for XVA exposure calculations.
//!
//! Implements close-out netting under ISDA master agreements and
//! CSA collateral reduction for counterparty credit exposure.
//!
//! # Close-Out Netting
//!
//! Under a valid ISDA master agreement, upon default all transactions
//! are terminated and a single net amount is determined:
//!
//! ```text
//! Net exposure = max(Σᵢ Vᵢ, 0)
//! ```
//!
//! This is significantly less than the sum of individual positive exposures:
//! ```text
//! Gross exposure = Σᵢ max(Vᵢ, 0) ≥ Net exposure
//! ```
//!
//! # References
//!
//! - ISDA (2002). "2002 ISDA Master Agreement." Section 6 (Close-Out Netting).
//! - Gregory, J. (2020). *The xVA Challenge*, Chapter 6.
//! - BCBS 279 (2014). SA-CCR: "The standardised approach for measuring
//!   counterparty credit risk exposures."

use super::types::CsaTerms;
use finstack_quant_core::math::neumaier_sum;

/// Apply close-out netting to a set of instrument mark-to-market values.
///
/// Under a valid ISDA master agreement, the exposure is computed on the
/// net portfolio value rather than summing individual positive exposures.
///
/// # Arguments
///
/// * `instrument_values` - Individual instrument MtM values (positive or negative)
///
/// # Returns
///
/// Net positive exposure: `max(Σᵢ Vᵢ, 0)`.
///
/// # Examples
///
/// ```
/// use finstack_quant_margin::xva::netting::apply_netting;
///
/// // Two offsetting trades: net exposure is reduced
/// let values = [100.0, -80.0];
/// assert!((apply_netting(&values) - 20.0).abs() < 1e-12);
///
/// // All negative: no exposure
/// let values = [-50.0, -30.0];
/// assert!((apply_netting(&values)).abs() < 1e-12);
/// ```
#[inline]
pub fn apply_netting(instrument_values: &[f64]) -> f64 {
    let net = neumaier_sum(instrument_values.iter().copied());
    net.max(0.0)
}

/// Apply CSA collateral terms to reduce gross exposure.
///
/// Models the collateral mechanics of a Credit Support Annex:
///
/// ```text
/// unsecured_exposure = min(exposure, threshold + MTA)
/// net_exposure = max(unsecured_exposure - IA, 0)
/// ```
///
/// The independent amount (IA) is additional collateral posted by the
/// counterparty that further reduces credit exposure beyond the
/// variation margin collateral call.
///
/// # Arguments
///
/// * `gross_exposure` - Portfolio exposure before collateral (non-negative)
/// * `csa` - CSA terms governing collateral exchange
///
/// # Returns
///
/// Net exposure after collateral, always non-negative.
///
/// # Examples
///
/// ```
/// use finstack_quant_margin::xva::netting::apply_collateral;
/// use finstack_quant_margin::xva::types::CsaTerms;
///
/// let csa = CsaTerms {
///     threshold: 10.0,
///     mta: 1.0,
///     mpor_days: 10,
///     independent_amount: 0.0,
/// };
///
/// // Exposure below threshold: no collateral called
/// assert!((apply_collateral(8.0, &csa) - 8.0).abs() < 1e-12);
///
/// // Exposure above threshold + MTA: the residual exposure is threshold + MTA
/// assert!((apply_collateral(20.0, &csa) - 11.0).abs() < 1e-12);
/// ```
#[inline]
pub fn apply_collateral(gross_exposure: f64, csa: &CsaTerms) -> f64 {
    apply_collateral_with_independent_amount(gross_exposure, csa, csa.independent_amount)
}

#[inline]
fn apply_collateral_with_independent_amount(
    gross_exposure: f64,
    csa: &CsaTerms,
    independent_amount: f64,
) -> f64 {
    let unsecured_exposure = if gross_exposure > csa.threshold + csa.mta {
        csa.threshold + csa.mta
    } else {
        gross_exposure
    };
    (unsecured_exposure - independent_amount).max(0.0)
}

/// Collateral held under this CSA against a given exposure level.
///
/// Uses the same cap semantics as [`apply_collateral`]: the counterparty posts
/// variation margin only above `threshold + MTA`, so
/// `C(E) = max(E − (threshold + MTA), 0)` and the unsecured residual is
/// `E − C(E) = min(E, threshold + MTA)`.
#[inline]
fn collateral_held(exposure: f64, csa: &CsaTerms) -> f64 {
    (exposure - (csa.threshold + csa.mta)).max(0.0)
}

/// Apply CSA collateral with an explicit margin-period-of-risk (MPOR) lag.
///
/// During the close-out period after a counterparty default, collateral stops
/// flowing while the portfolio keeps moving: the collateral actually held at
/// time `t` was called against the exposure observed at `t − MPOR`. This
/// function models that gap risk:
///
/// ```text
/// C(t)   = max(E(t − δ) − (threshold + MTA), 0)      (collateral held)
/// net(t) = max(E(t) − C(t) − IA, 0)                  (residual exposure)
/// ```
///
/// With `exposure_at_lag == exposure_now` (i.e. `δ = 0`) this reduces exactly
/// to [`apply_collateral`].
///
/// # Arguments
///
/// * `exposure_now` - Positive portfolio exposure at time `t` (non-negative)
/// * `exposure_at_lag` - Positive portfolio exposure at `t − MPOR` (non-negative)
/// * `csa` - CSA terms; `csa.independent_amount` further reduces the residual
///
/// # Returns
///
/// Net exposure after MPOR-lagged collateral, always non-negative.
///
/// # References
///
/// - Andersen, L., Pykhtin, M., & Sokol, A. (2017). "Rethinking the margin
///   period of risk." *Journal of Credit Risk*, 13(1), 1-45.
/// - Gregory XVA Challenge: `docs/REFERENCES.md#gregory-xva-challenge`
///
/// # Examples
///
/// ```
/// use finstack_quant_margin::xva::netting::apply_collateral_mpor;
/// use finstack_quant_margin::xva::types::CsaTerms;
///
/// let csa = CsaTerms { threshold: 0.0, mta: 0.0, mpor_days: 10, independent_amount: 0.0 };
/// // Exposure grew from 90 to 100 over the MPOR window: 10 is uncollateralized.
/// assert!((apply_collateral_mpor(100.0, 90.0, &csa) - 10.0).abs() < 1e-12);
/// ```
#[inline]
pub fn apply_collateral_mpor(exposure_now: f64, exposure_at_lag: f64, csa: &CsaTerms) -> f64 {
    (exposure_now - collateral_held(exposure_at_lag, csa) - csa.independent_amount).max(0.0)
}

/// MPOR-lagged variation-margin reduction without the counterparty-posted
/// independent amount (the ENE/DVA mirror of [`apply_collateral_mpor`]).
///
/// Wired into the deterministic XVA engine's ENE/DVA path in
/// [`super::exposure::compute_exposure_profile`].
#[inline]
pub(crate) fn apply_variation_margin_mpor(
    exposure_now: f64,
    exposure_at_lag: f64,
    csa: &CsaTerms,
) -> f64 {
    (exposure_now - collateral_held(exposure_at_lag, csa)).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Netting tests ──────────────────────────────────────────────

    #[test]
    fn netting_reduces_exposure() {
        // Offsetting trades should reduce net exposure
        let gross: f64 = [100.0_f64, -80.0].iter().filter(|v| **v > 0.0).sum::<f64>();
        let net = apply_netting(&[100.0, -80.0]);
        assert!(
            net < gross,
            "Netting should reduce exposure: net={net}, gross={gross}"
        );
        assert!((net - 20.0).abs() < 1e-12);
    }

    #[test]
    fn netting_all_positive() {
        // All positive values: net equals sum
        let values = [10.0, 20.0, 30.0];
        assert!((apply_netting(&values) - 60.0).abs() < 1e-12);
    }

    #[test]
    fn netting_all_negative_gives_zero() {
        // All negative: no exposure
        let values = [-10.0, -20.0, -30.0];
        assert!(apply_netting(&values).abs() < 1e-12);
    }

    #[test]
    fn netting_empty_gives_zero() {
        assert!(apply_netting(&[]).abs() < 1e-12);
    }

    #[test]
    fn netting_single_positive() {
        assert!((apply_netting(&[42.0]) - 42.0).abs() < 1e-12);
    }

    #[test]
    fn netting_single_negative_gives_zero() {
        assert!(apply_netting(&[-42.0]).abs() < 1e-12);
    }

    #[test]
    fn netting_mixed_magnitude_cancellation_preserves_small_residual() {
        let values = [1e16_f64, 1.0, -1e16];
        assert!((apply_netting(&values) - 1.0).abs() < 1e-10);
    }

    // ── Collateral tests ───────────────────────────────────────────

    fn make_csa(threshold: f64, mta: f64, ia: f64) -> CsaTerms {
        CsaTerms {
            threshold,
            mta,
            mpor_days: 10,
            independent_amount: ia,
        }
    }

    #[test]
    fn collateral_below_threshold_unchanged() {
        let csa = make_csa(10.0, 1.0, 0.0);
        assert!((apply_collateral(8.0, &csa) - 8.0).abs() < 1e-12);
    }

    #[test]
    fn collateral_between_threshold_and_mta() {
        // Over threshold by 0.5, but below MTA (1.0) → no collateral called
        let csa = make_csa(10.0, 1.0, 0.0);
        assert!((apply_collateral(10.5, &csa) - 10.5).abs() < 1e-12);
    }

    #[test]
    fn collateral_above_threshold_plus_mta() {
        // Exposure = 20, threshold = 10, MTA = 1
        // residual unsecured exposure = threshold + MTA = 11
        let csa = make_csa(10.0, 1.0, 0.0);
        assert!((apply_collateral(20.0, &csa) - 11.0).abs() < 1e-12);
    }

    #[test]
    fn collateral_with_independent_amount() {
        // IA reduces the net exposure (additional collateral posted by counterparty)
        let csa = make_csa(10.0, 1.0, 5.0);
        // Residual exposure = threshold + MTA = 11; net = max(11 - 5, 0) = 6.
        assert!((apply_collateral(20.0, &csa) - 6.0).abs() < 1e-12);
    }

    #[test]
    fn counterparty_independent_amount_does_not_reduce_negative_exposure() {
        // The bank-posted (ENE/DVA) side cannot also benefit from a
        // counterparty-posted independent amount — verified here via the
        // zero-lag MPOR variant, which is exactly the non-MPOR variation
        // margin reduction (see `mpor_zero_lag_reduces_to_apply_collateral`).
        let csa = make_csa(10.0, 1.0, 5.0);
        assert!((apply_collateral(20.0, &csa) - 6.0).abs() < 1e-12);
        assert!((apply_variation_margin_mpor(20.0, 20.0, &csa) - 11.0).abs() < 1e-12);
    }

    #[test]
    fn collateral_zero_threshold() {
        // Zero threshold CSA (bilateral VM): MTA remains as unsecured residual.
        let csa = make_csa(0.0, 0.5, 0.0);
        assert!((apply_collateral(100.0, &csa) - 0.5).abs() < 1e-12);
    }

    #[test]
    fn collateral_zero_exposure() {
        let csa = make_csa(10.0, 1.0, 0.0);
        assert!(apply_collateral(0.0, &csa).abs() < 1e-12);
    }

    #[test]
    fn collateral_never_negative() {
        // Even with large IA on zero exposure, result is floored at zero
        let csa = make_csa(0.0, 0.0, 100.0);
        let result = apply_collateral(0.0, &csa);
        assert!(
            result.abs() < 1e-12,
            "Collateralized exposure should be zero when IA exceeds exposure, got {result}"
        );
    }

    #[test]
    fn collateral_ia_reduces_to_zero() {
        // Large IA should reduce exposure to zero (floored)
        let csa = make_csa(0.0, 0.0, 1000.0);
        let result = apply_collateral(50.0, &csa);
        assert!(
            result.abs() < 1e-12,
            "Large IA should reduce exposure to zero, got {result}"
        );
    }

    // ── MPOR-lagged collateral tests ───────────────────────────────

    #[test]
    fn mpor_zero_lag_reduces_to_apply_collateral() {
        // With exposure_at_lag == exposure_now the MPOR variant must agree
        // exactly with apply_collateral for every regime of the piecewise
        // formula (below threshold, in the MTA band, above threshold+MTA,
        // and with a counterparty IA).
        let exposures = [0.0, 5.0, 8.0, 10.5, 11.0, 20.0, 1_000.0];
        let csas = [
            make_csa(10.0, 1.0, 0.0),
            make_csa(10.0, 1.0, 5.0),
            make_csa(0.0, 0.5, 0.0),
            make_csa(0.0, 0.0, 100.0),
        ];
        for csa in &csas {
            for &e in &exposures {
                let mpor = apply_collateral_mpor(e, e, csa);
                let classic = apply_collateral(e, csa);
                assert!(
                    (mpor - classic).abs() < 1e-12,
                    "zero-lag MPOR ({mpor}) must equal apply_collateral ({classic}) for e={e}"
                );
            }
        }
    }

    #[test]
    fn mpor_gap_risk_leaves_exposure_growth_uncollateralized() {
        // Zero threshold/MTA CSA: collateral held = lagged exposure, so the
        // residual is exactly the exposure growth over the MPOR window.
        let csa = make_csa(0.0, 0.0, 0.0);
        // Exposure grew from 90 to 100 over the close-out period.
        assert!((apply_collateral_mpor(100.0, 90.0, &csa) - 10.0).abs() < 1e-12);
        // Exposure fell: collateral over-covers, floored at zero.
        assert!(apply_collateral_mpor(80.0, 90.0, &csa).abs() < 1e-12);
    }

    #[test]
    fn mpor_respects_threshold_and_ia() {
        // threshold=10, mta=1, ia=5: collateral held on lagged exposure 50 is
        // max(50 − 11, 0) = 39; net = max(60 − 39 − 5, 0) = 16.
        let csa = make_csa(10.0, 1.0, 5.0);
        assert!((apply_collateral_mpor(60.0, 50.0, &csa) - 16.0).abs() < 1e-12);
        // ENE mirror ignores the counterparty IA: max(60 − 39, 0) = 21.
        assert!((apply_variation_margin_mpor(60.0, 50.0, &csa) - 21.0).abs() < 1e-12);
    }

    #[test]
    fn mpor_never_negative() {
        let csa = make_csa(0.0, 0.0, 1_000.0);
        assert!(apply_collateral_mpor(50.0, 500.0, &csa).abs() < 1e-12);
    }
}
