//! Correlation matrix utilities shared across credit, rates, and portfolio
//! analytics.
//!
//! These helpers were originally in `finstack-quant-valuations::correlation` but were
//! relocated so that downstream modules (e.g. `finstack-quant-models::factor`) can
//! consume them without taking a dependency on `finstack-quant-valuations`.
//!
//! # Components
//!
//! - [`Error`]: Structured validation diagnostics
//! - [`nearest_correlation_matrix`][]: Higham (2002)
//!   alternating-projection PSD repair
//! - [`validate_correlation_matrix`]: Same accept/reject thresholds as
//!   [`finstack_quant_core::math::linalg::validate_correlation_matrix`], with
//!   located [`Error`] variants (`DiagonalNotOne`, `OutOfBounds`, …) that
//!   core's coarser `InputError` cannot express

mod error;
mod nearest_correlation;

pub use error::{Error, Result};
pub use nearest_correlation::{nearest_correlation_matrix, NearestCorrelationOpts};

/// Tolerance used by [`validate_correlation_matrix`] to classify diagonal and
/// symmetry violations.
///
/// Kept identical to [`finstack_quant_core::math::linalg::DIAGONAL_TOLERANCE`]
/// so this validator and core's agree; see that constant for the reasoning.
pub(crate) const CORRELATION_TOLERANCE: f64 = finstack_quant_core::math::linalg::DIAGONAL_TOLERANCE;

/// Slack on the `[-1, 1]` bound, shared with core's validator.
pub(crate) const CORRELATION_BOUND_SLACK: f64 =
    finstack_quant_core::math::linalg::CORRELATION_BOUND_SLACK;

/// Validate a flattened row-major correlation matrix.
///
/// Uses the same diagonal, symmetry, and `[-1, 1]` thresholds as
/// [`finstack_quant_core::math::linalg::validate_correlation_matrix`]
/// ([`CORRELATION_TOLERANCE`], [`CORRELATION_BOUND_SLACK`]) and the same
/// pivoted Cholesky PSD check. This function reports the first failure as a
/// located [`Error`] variant; core only returns a coarse `InputError`.
///
/// The two validators are kept in agreement by
/// `tests/correlation_validator_agreement.rs`. Do not change a threshold
/// here without updating core (or the reverse).
///
/// Checks performed:
/// - Correct size (`matrix.len() == n * n`)
/// - Unit diagonal (within `1e-10`)
/// - Symmetry (within `1e-10`)
/// - All values within `[-1, 1]` (within `1e-10`)
/// - Positive semi-definiteness (via Cholesky)
///
/// # Arguments
///
/// * `matrix` - Correlation coefficients in row-major `n × n` order; each
///   entry at row `i`, column `j` is stored at `i * n + j`.
/// * `n` - Number of variables represented by each matrix dimension; `0`
///   accepts an empty matrix.
///
/// # Errors
///
/// Returns the first [`Error`] variant detected.
///
/// # Examples
///
/// ```
/// use finstack_quant_analytics::correlation::validate_correlation_matrix;
///
/// let corr = vec![1.0, 0.5, 0.5, 1.0];
/// assert!(validate_correlation_matrix(&corr, 2).is_ok());
/// ```
pub fn validate_correlation_matrix(matrix: &[f64], n: usize) -> Result<()> {
    if matrix.len() != n * n {
        return Err(Error::InvalidSize {
            expected: n,
            actual: matrix.len(),
        });
    }
    if n == 0 {
        return Ok(());
    }

    for i in 0..n {
        let v = matrix[i * n + i];
        if (v - 1.0).abs() > CORRELATION_TOLERANCE {
            return Err(Error::DiagonalNotOne { index: i, value: v });
        }
    }

    for i in 0..n {
        for j in 0..n {
            let v = matrix[i * n + j];
            if !(-1.0 - CORRELATION_BOUND_SLACK..=1.0 + CORRELATION_BOUND_SLACK).contains(&v) {
                return Err(Error::OutOfBounds { i, j, value: v });
            }
            if i < j {
                let diff = (matrix[i * n + j] - matrix[j * n + i]).abs();
                if diff > CORRELATION_TOLERANCE {
                    return Err(Error::NotSymmetric { i, j, diff });
                }
            }
        }
    }

    if let Err(err) = finstack_quant_core::math::linalg::cholesky_correlation(matrix, n) {
        let row = match err {
            finstack_quant_core::math::linalg::CholeskyError::NotPositiveDefinite {
                row, ..
            } => row,
            _ => 0,
        };
        return Err(Error::NotPositiveSemiDefinite { row });
    }

    Ok(())
}
