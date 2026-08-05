//! Documents where `analytics::correlation::validate_correlation_matrix` and
//! `core::math::linalg::validate_correlation_matrix` disagree.
//!
//! Both validate "is this a correlation matrix", but with different
//! strictness, and the difference runs in *both* directions:
//!
//! | check              | analytics        | core             |
//! |--------------------|------------------|------------------|
//! | diagonal ≈ 1       | 1e-10            | 1e-6             |
//! | symmetry           | 1e-10            | 1e-6             |
//! | entries in [-1, 1] | ±1 ± 1e-10 (lax) | strict [-1, 1]   |
//!
//! So a matrix with 1e-8 of diagonal noise passes core and fails analytics,
//! while a matrix with an entry at 1 + 1e-12 passes analytics and fails core.
//! Both are reachable from Python (`core.math.linalg.validate_correlation_matrix`
//! and `valuations.correlation.validate_correlation_matrix`), so a caller can
//! get opposite answers depending on which entry point they picked.
//!
//! Reconciling them is a semantics decision (which strictness is correct),
//! not a refactor, so this test pins the current divergence rather than
//! hiding it. If the two are unified, both assertions below will fail and
//! this file should be deleted.

use finstack_quant_analytics::correlation::validate_correlation_matrix as analytics_validate;
use finstack_quant_core::math::linalg::validate_correlation_matrix as core_validate;

#[test]
fn diagonal_noise_is_accepted_by_core_and_rejected_by_analytics() {
    // Diagonal off by 1e-8: inside core's 1e-6, outside analytics' 1e-10.
    let m = vec![1.0 + 1e-8, 0.5, 0.5, 1.0];
    assert!(
        core_validate(&m, 2).is_ok(),
        "core tolerates 1e-8 diagonal noise"
    );
    assert!(
        analytics_validate(&m, 2).is_err(),
        "analytics rejects 1e-8 diagonal noise (1e-10 tolerance)"
    );
}

#[test]
fn marginally_out_of_range_entry_is_accepted_by_analytics_and_rejected_by_core() {
    // Off-diagonal at 1 + 1e-12: inside analytics' tolerance band, outside
    // core's strict [-1, 1] bound. Symmetric and PSD-repairable otherwise.
    let over = 1.0 + 1e-12;
    let m = vec![1.0, over, over, 1.0];
    assert!(
        core_validate(&m, 2).is_err(),
        "core enforces a strict [-1, 1] bound"
    );
    assert!(
        analytics_validate(&m, 2).is_ok(),
        "analytics allows ±1 ± 1e-10 rounding slack"
    );
}
