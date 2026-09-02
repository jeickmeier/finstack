//! Custom assertion helpers for risk tests.
//!
//! These helpers provide better error messages and consistent tolerance handling.

/// Assert that a value is positive.
#[track_caller]
pub fn assert_positive(value: f64, name: &str) {
    assert!(
        value > 0.0,
        "assertion failed: {name} should be positive, got {value}"
    );
}

/// Assert that a value is negative.
#[track_caller]
pub fn assert_negative(value: f64, name: &str) {
    assert!(
        value < 0.0,
        "assertion failed: {name} should be negative, got {value}"
    );
}

/// Assert that a value is non-negative.
#[track_caller]
pub fn assert_non_negative(value: f64, name: &str) {
    assert!(
        value >= 0.0,
        "assertion failed: {name} should be non-negative, got {value}"
    );
}

/// Assert that a value is finite (not NaN or infinity).
#[track_caller]
pub fn assert_finite(value: f64, name: &str) {
    assert!(
        value.is_finite(),
        "assertion failed: {name} should be finite, got {value}"
    );
}
