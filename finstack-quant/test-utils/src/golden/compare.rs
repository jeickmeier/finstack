//! Comparison and assertion utilities for golden tests.
//!
//! This module provides assertion helpers that produce actionable error
//! messages including case identifiers, metric labels, and provenance.

use crate::golden::types::{Expectation, SuiteMeta, Tolerance};
use crate::Error;

/// Assertion context for golden test comparisons.
///
/// Retains suite and case identifiers across comparisons so every mismatch has
/// actionable provenance.
///
/// # Example
///
/// ```
/// use finstack_quant_test_utils::golden::{GoldenAssert, SuiteMeta};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let meta: SuiteMeta =
///     serde_json::from_str(r#"{"suite_id": "black_scholes", "schema_version": 1}"#)?;
/// let check = GoldenAssert::new(&meta, "case_123");
///
/// check.abs("price", 10.4506, 10.4500, 0.01)?;
/// assert!(check.abs("price", 10.4506, 10.4500, 1e-9).is_err());
/// # Ok(())
/// # }
/// ```
pub struct GoldenAssert<'a> {
    suite_id: &'a str,
    case_id: &'a str,
}

impl<'a> GoldenAssert<'a> {
    /// Create a new assertion context.
    ///
    /// # Arguments
    ///
    /// * `meta` - Suite metadata whose identifier is included in mismatch
    ///   diagnostics.
    /// * `case_id` - Case identifier included in mismatch diagnostics.
    pub fn new(meta: &'a SuiteMeta, case_id: &'a str) -> Self {
        Self {
            suite_id: &meta.suite_id,
            case_id,
        }
    }

    /// Assert with absolute tolerance.
    ///
    /// # Arguments
    ///
    /// * `metric` - Name of the measured quantity included in diagnostics.
    /// * `actual` - Observed floating-point value produced by the test.
    /// * `expected` - Expected floating-point value.
    /// * `tolerance` - Allowed absolute difference in the same units as
    ///   `actual` and `expected`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`] when `actual` differs from `expected` by
    /// more than `tolerance`.
    pub fn abs(
        &self,
        metric: &str,
        actual: f64,
        expected: f64,
        tolerance: f64,
    ) -> Result<(), Error> {
        self.expected(
            metric,
            actual,
            &Expectation::Exact {
                value: expected,
                tolerance: Some(Tolerance::Abs(tolerance)),
                notes: None,
            },
        )
    }

    /// Assert with an [`Expectation`] fixture entry.
    ///
    /// # Arguments
    ///
    /// * `metric` - Name of the measured quantity included in diagnostics.
    /// * `actual` - Observed floating-point value produced by the test.
    /// * `expected` - Exact-value or range expectation loaded from the golden
    ///   fixture.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`] when `actual` does not satisfy `expected`.
    pub fn expected(&self, metric: &str, actual: f64, expected: &Expectation) -> Result<(), Error> {
        if expected.is_satisfied(actual) {
            return Ok(());
        }
        let suite_id = self.suite_id;
        let case_id = self.case_id;
        let message = match expected {
            Expectation::Exact {
                value, tolerance, ..
            } => {
                let tolerance = tolerance.map_or(String::new(), |tolerance| {
                    format!(
                        ", tolerance={tolerance:?}, error={:.6e}",
                        tolerance.compute_error(actual, *value)
                    )
                });
                format!(
                    "[{suite_id}/{case_id}] {metric} failed: actual={actual}, expected={value}{tolerance} - value outside tolerance"
                )
            }
            Expectation::Range { min, max, .. } => {
                format!(
                    "[{suite_id}/{case_id}] {metric} failed: actual={actual}, range=[{min:?}, {max:?}] - value outside range"
                )
            }
        };
        Err(Error::Validation(message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::golden::types::Expectation;

    #[test]
    fn golden_assert_context_checks_values_and_diagnostics() {
        let meta = SuiteMeta {
            suite_id: "test_suite".to_string(),
            ..Default::default()
        };
        let golden_assert = GoldenAssert::new(&meta, "case_1");

        assert!(golden_assert.abs("value", 1.005, 1.0, 0.01).is_ok());
        let mismatch = golden_assert.abs("value", 1.02, 1.0, 0.01);
        assert!(mismatch.is_err());
        if let Err(error) = mismatch {
            let message = error.to_string();
            assert!(message.contains("test_suite/case_1"));
            assert!(message.contains("value"));
        }

        let expected = Expectation::Exact {
            value: 1.0,
            tolerance: Some(Tolerance::Abs(0.01)),
            notes: None,
        };
        assert!(golden_assert.expected("value", 1.005, &expected).is_ok());
        assert!(golden_assert.expected("value", 1.02, &expected).is_err());
    }

    #[test]
    fn expected_checks_exact_and_range_branches() {
        let meta = SuiteMeta {
            suite_id: "s".to_string(),
            ..Default::default()
        };
        let golden_assert = GoldenAssert::new(&meta, "c");
        let exact = Expectation::Exact {
            value: 1.0,
            tolerance: Some(Tolerance::Abs(0.05)),
            notes: None,
        };
        assert!(golden_assert.expected("m", 1.02, &exact).is_ok());
        assert!(golden_assert.expected("m", 2.0, &exact).is_err());

        let range = Expectation::Range {
            min: Some(0.0),
            max: Some(1.0),
            notes: None,
        };
        assert!(golden_assert.expected("m", 0.5, &range).is_ok());
        assert!(golden_assert.expected("m", 2.0, &range).is_err());
    }
}
