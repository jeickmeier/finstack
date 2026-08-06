//! Pins that `analytics::correlation::validate_correlation_matrix` and
//! `core::math::linalg::validate_correlation_matrix` agree.
//!
//! The two used to disagree in *both* directions: analytics applied a 1e-10
//! diagonal/symmetry tolerance against core's 1e-6, while core enforced a
//! strict `[-1, 1]` bound against analytics' 1e-10 slack. A caller could get
//! opposite answers from `core.math.linalg.validate_correlation_matrix` and
//! `valuations.correlation.validate_correlation_matrix` on the same matrix.
//!
//! They now share their thresholds — analytics defines its constants in terms
//! of core's — so this file guards that agreement instead of documenting the
//! gap. The two keep separate error *types* on purpose: analytics returns
//! located variants (`DiagonalNotOne { index, value }`, `OutOfBounds { i, j,
//! value }`, `NotSymmetric { i, j, diff }`) that core's coarser
//! `InputError::Invalid` cannot express.

use finstack_quant_analytics::correlation::validate_correlation_matrix as analytics_validate;
use finstack_quant_core::math::linalg::validate_correlation_matrix as core_validate;

/// Cases spanning both former divergence directions plus the boundaries.
fn agreement_cases() -> Vec<(&'static str, Vec<f64>, usize, bool)> {
    vec![
        ("clean 2x2", vec![1.0, 0.5, 0.5, 1.0], 2, true),
        // Formerly: passed core (1e-6), failed analytics (1e-10).
        (
            "diagonal noise 1e-8",
            vec![1.0 + 1e-8, 0.5, 0.5, 1.0],
            2,
            false,
        ),
        // Formerly: passed analytics (1e-10 slack), failed core (strict bound).
        (
            "off-diagonal at 1 + 1e-10",
            vec![1.0, 1.0 + 1e-10, 1.0 + 1e-10, 1.0],
            2,
            false,
        ),
        // Genuinely out of range.
        ("off-diagonal at 1.5", vec![1.0, 1.5, 1.5, 1.0], 2, false),
        // Asymmetry well above noise scale.
        (
            "asymmetric by 1e-8",
            vec![1.0, 0.5, 0.5 + 1e-8, 1.0],
            2,
            false,
        ),
        // Asymmetry at rounding scale is tolerated.
        (
            "asymmetric by 1e-16",
            vec![1.0, 0.5, 0.5 + 1e-16, 1.0],
            2,
            true,
        ),
    ]
}

#[test]
fn analytics_and_core_validators_agree() {
    for (label, matrix, n, expect_ok) in agreement_cases() {
        let core_ok = core_validate(&matrix, n).is_ok();
        let analytics_ok = analytics_validate(&matrix, n).is_ok();
        assert_eq!(
            core_ok, analytics_ok,
            "validators disagree on '{label}': core_ok={core_ok}, analytics_ok={analytics_ok}"
        );
        assert_eq!(
            core_ok, expect_ok,
            "'{label}' should validate as {expect_ok}, got {core_ok}"
        );
    }
}

#[test]
fn diagonal_noise_above_tolerance_is_rejected_by_both() {
    // 1e-8 sits above the shared 1e-10 diagonal tolerance; it signals a
    // mis-normalized covariance matrix rather than rounding.
    let m = vec![1.0 + 1e-8, 0.5, 0.5, 1.0];
    assert!(core_validate(&m, 2).is_err());
    assert!(analytics_validate(&m, 2).is_err());
}

#[test]
fn rounding_scale_overshoot_is_accepted_by_both() {
    // A perfectly collinear pair (duplicated series, or genuinely identical
    // instruments) computes to a few ulp above 1. Rejecting it would reject a
    // valid input, so both validators allow rounding-scale slack.
    let over = 1.0 + f64::EPSILON;
    let m = vec![1.0, over, over, 1.0];
    assert!(core_validate(&m, 2).is_ok());
    assert!(analytics_validate(&m, 2).is_ok());
}
