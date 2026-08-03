//! Comparison and assertion utilities for golden tests.
//!
//! This module provides assertion helpers that produce actionable error
//! messages including case identifiers, metric labels, and provenance.

use crate::golden::types::{Expectation, SuiteMeta, Tolerance};
use crate::Error;

// =============================================================================
// Core assertion functions
// =============================================================================

/// Assert that actual is within tolerance of expected.
///
/// Returns Ok(()) if the assertion passes, Err with details if it fails.
///
/// # Arguments
///
/// * `suite_id` - Suite identifier for error context
/// * `case_id` - Case identifier for error context
/// * `metric` - Metric name for error context
/// * `actual` - Actual computed value
/// * `expected` - Expected value with tolerance
///
/// # Errors
///
/// Returns [`Error::Validation`] instead of panicking when `actual` fails the
/// exact or range expectation. The message includes the suite, case, metric,
/// observed value, and expected tolerance or bounds.
pub fn assert_expected_f64(
    suite_id: &str,
    case_id: &str,
    metric: &str,
    actual: f64,
    expected: &Expectation,
) -> Result<(), Error> {
    if expected.is_satisfied(actual) {
        return Ok(());
    }
    let msg = match expected {
        Expectation::Exact {
            value, tolerance, ..
        } => {
            let tol_str = tolerance.map_or(String::new(), |t| {
                format!(
                    ", tolerance={t:?}, error={:.6e}",
                    t.compute_error(actual, *value)
                )
            });
            format!(
                "[{suite_id}/{case_id}] {metric} failed: actual={actual}, expected={value}{tol_str} - value outside tolerance"
            )
        }
        Expectation::Range { min, max, .. } => {
            format!(
                "[{suite_id}/{case_id}] {metric} failed: actual={actual}, range=[{min:?}, {max:?}] - value outside range"
            )
        }
    };
    Err(Error::Validation(msg))
}

/// Assert with an explicit tolerance (a convenience wrapper).
///
/// # Arguments
///
/// * `suite_id` - Golden-suite identifier included in any assertion diagnostic.
/// * `case_id` - Test-case identifier included in any assertion diagnostic.
/// * `metric` - Name of the measured quantity being compared.
/// * `actual` - Observed floating-point value produced by the system under test.
/// * `expected` - Expected floating-point value before applying `tolerance`.
/// * `tolerance` - Absolute, relative, percentage, or basis-point comparison
///   tolerance.
///
/// # Errors
///
/// Returns the structured mismatch error from [`assert_expected_f64`].
pub fn assert_within_tolerance(
    suite_id: &str,
    case_id: &str,
    metric: &str,
    actual: f64,
    expected: f64,
    tolerance: Tolerance,
) -> Result<(), Error> {
    assert_expected_f64(
        suite_id,
        case_id,
        metric,
        actual,
        &Expectation::Exact {
            value: expected,
            tolerance: Some(tolerance),
            notes: None,
        },
    )
}

/// Assert with absolute tolerance (a convenience wrapper).
///
/// # Arguments
///
/// * `suite_id` - Golden-suite identifier included in any assertion diagnostic.
/// * `case_id` - Test-case identifier included in any assertion diagnostic.
/// * `metric` - Name of the measured quantity being compared.
/// * `actual` - Observed floating-point value produced by the system under test.
/// * `expected` - Expected floating-point value.
/// * `tolerance` - Allowed absolute difference in the same units as `actual`.
///
/// # Errors
///
/// Returns the structured mismatch error from [`assert_within_tolerance`].
pub fn assert_abs(
    suite_id: &str,
    case_id: &str,
    metric: &str,
    actual: f64,
    expected: f64,
    tolerance: f64,
) -> Result<(), Error> {
    assert_within_tolerance(
        suite_id,
        case_id,
        metric,
        actual,
        expected,
        Tolerance::Abs(tolerance),
    )
}

// =============================================================================
// Context-aware assertion builder
// =============================================================================

/// Builder for golden test assertions with suite context.
///
/// This provides a cleaner API when making many assertions with the same context.
///
/// # Example
///
/// ```ignore
/// use finstack_quant_test_utils::golden::GoldenAssert;
///
/// let assert = GoldenAssert::new(&suite.meta, "case_123");
/// assert.abs("price", actual_price, 100.0, 0.01)?;
/// assert.expected("spread", actual_spread, &case.expected.spread)?;
/// ```
pub struct GoldenAssert<'a> {
    suite_id: &'a str,
    case_id: &'a str,
}

impl<'a> GoldenAssert<'a> {
    /// Create a new assertion context.
    pub fn new(meta: &'a SuiteMeta, case_id: &'a str) -> Self {
        Self {
            suite_id: &meta.suite_id,
            case_id,
        }
    }

    /// Assert with absolute tolerance.
    ///
    /// # Errors
    ///
    /// Propagates the structured mismatch error from [`assert_abs`].
    pub fn abs(
        &self,
        metric: &str,
        actual: f64,
        expected: f64,
        tolerance: f64,
    ) -> Result<(), Error> {
        assert_abs(
            self.suite_id,
            self.case_id,
            metric,
            actual,
            expected,
            tolerance,
        )
    }

    /// Assert with an [`Expectation`] fixture entry.
    ///
    /// # Errors
    ///
    /// Propagates the structured mismatch error from [`assert_expected_f64`].
    pub fn expected(&self, metric: &str, actual: f64, expected: &Expectation) -> Result<(), Error> {
        assert_expected_f64(self.suite_id, self.case_id, metric, actual, expected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::golden::types::Expectation;

    #[test]
    fn test_assert_abs_pass() {
        let result = assert_abs("suite", "case", "metric", 1.005, 1.0, 0.01);
        assert!(result.is_ok());
    }

    #[test]
    fn test_assert_abs_fail() {
        let result = assert_abs("suite", "case", "metric", 1.02, 1.0, 0.01);
        assert!(result.is_err(), "Expected assertion to fail");
        if let Err(e) = result {
            let err = e.to_string();
            assert!(err.contains("suite/case"));
            assert!(err.contains("metric"));
        }
    }

    #[test]
    fn test_golden_assert_builder() {
        let meta = SuiteMeta {
            suite_id: "test_suite".to_string(),
            ..Default::default()
        };
        let golden_assert = GoldenAssert::new(&meta, "case_1");

        assert!(golden_assert.abs("value", 1.005, 1.0, 0.01).is_ok());
        assert!(golden_assert.abs("value", 1.02, 1.0, 0.01).is_err());

        let expected = Expectation::Exact {
            value: 1.0,
            tolerance: Some(Tolerance::Abs(0.01)),
            notes: None,
        };
        assert!(golden_assert.expected("value", 1.005, &expected).is_ok());
        assert!(golden_assert.expected("value", 1.02, &expected).is_err());
    }

    #[test]
    fn assert_expected_f64_exact_and_range_branches() {
        let exact = Expectation::Exact {
            value: 1.0,
            tolerance: Some(Tolerance::Abs(0.05)),
            notes: None,
        };
        assert!(assert_expected_f64("s", "c", "m", 1.02, &exact).is_ok());
        assert!(assert_expected_f64("s", "c", "m", 2.0, &exact).is_err());

        let range = Expectation::Range {
            min: Some(0.0),
            max: Some(1.0),
            notes: None,
        };
        assert!(assert_expected_f64("s", "c", "m", 0.5, &range).is_ok());
        assert!(assert_expected_f64("s", "c", "m", 2.0, &range).is_err());
    }
}
