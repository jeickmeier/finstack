//! Lightweight panicking float assertions shared across workspace test trees.
//!
//! Thin wrappers over [`crate::golden::Tolerance`] that panic with a
//! descriptive message on failure, for tests that want a one-line assertion
//! rather than [`crate::golden::GoldenAssert`]'s suite/case provenance.
//!
//! # Examples
//!
//! ```
//! use finstack_quant_test_utils::assert::{approx_eq, in_range, relative_eq};
//!
//! approx_eq(1.00005, 1.0, 1e-3, "price");
//! relative_eq(100.4, 100.0, 0.01, "yield"); // 0.4% relative error, 1% tolerance
//! in_range(0.5, 0.0, 1.0, "probability");
//! ```

use crate::golden::Tolerance;

/// Panic unless `actual` is within an absolute `tolerance` of `expected`.
///
/// # Arguments
///
/// * `actual` - Observed value produced by the code under test.
/// * `expected` - Reference value `actual` is compared against.
/// * `tolerance` - Maximum allowed `|actual - expected|`, in the same units
///   as `actual` and `expected`.
/// * `label` - Short description of the compared quantity, included in the
///   panic message.
///
/// # Panics
///
/// Panics when `|actual - expected| > tolerance`.
#[track_caller]
pub fn approx_eq(actual: f64, expected: f64, tolerance: f64, label: &str) {
    let tol = Tolerance::Abs(tolerance);
    assert!(
        tol.is_within(actual, expected),
        "{label}: actual={actual}, expected={expected}, tolerance={tolerance}, \
         error={:.6e}",
        tol.compute_error(actual, expected)
    );
}

/// Panic unless `actual` is within a relative `tolerance` (as a fraction) of
/// `expected`.
///
/// # Arguments
///
/// * `actual` - Observed value produced by the code under test.
/// * `expected` - Reference value `actual` is compared against. When its
///   magnitude is below `1e-15`, this falls back to comparing `|actual|`
///   against `tolerance` directly (avoids a divide-by-zero).
/// * `tolerance` - Maximum allowed `|actual - expected| / |expected|`,
///   expressed as a fraction (e.g. `0.01` for 1%).
/// * `label` - Short description of the compared quantity, included in the
///   panic message.
///
/// # Panics
///
/// Panics when the relative error exceeds `tolerance`.
#[track_caller]
pub fn relative_eq(actual: f64, expected: f64, tolerance: f64, label: &str) {
    let tol = Tolerance::Rel(tolerance);
    assert!(
        tol.is_within(actual, expected),
        "{label}: actual={actual}, expected={expected}, relative tolerance={tolerance}, \
         error={:.6e}",
        tol.compute_error(actual, expected)
    );
}

/// Panic unless `min <= actual <= max`.
///
/// # Arguments
///
/// * `actual` - Observed value produced by the code under test.
/// * `min` - Inclusive lower bound.
/// * `max` - Inclusive upper bound.
/// * `label` - Short description of the compared quantity, included in the
///   panic message.
///
/// # Panics
///
/// Panics when `actual` falls outside `[min, max]`.
#[track_caller]
pub fn in_range(actual: f64, min: f64, max: f64, label: &str) {
    assert!(
        actual >= min && actual <= max,
        "{label}: actual={actual} outside range [{min}, {max}]"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approx_eq_accepts_within_tolerance() {
        approx_eq(1.005, 1.0, 0.01, "value");
    }

    #[test]
    #[should_panic(expected = "value")]
    fn approx_eq_rejects_outside_tolerance() {
        approx_eq(1.02, 1.0, 0.01, "value");
    }

    #[test]
    fn relative_eq_accepts_within_tolerance() {
        relative_eq(100.5, 100.0, 0.01, "yield");
    }

    #[test]
    #[should_panic(expected = "yield")]
    fn relative_eq_rejects_outside_tolerance() {
        relative_eq(102.0, 100.0, 0.01, "yield");
    }

    #[test]
    fn in_range_accepts_within_bounds() {
        in_range(0.5, 0.0, 1.0, "probability");
    }

    #[test]
    #[should_panic(expected = "probability")]
    fn in_range_rejects_outside_bounds() {
        in_range(1.5, 0.0, 1.0, "probability");
    }
}
