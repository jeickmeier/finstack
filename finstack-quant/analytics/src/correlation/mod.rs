//! Correlation matrix utilities shared across credit, rates, and portfolio
//! analytics.
//!
//! # Components
//!
//! - [`Error`]: Structured validation diagnostics
//! - [`nearest_correlation_matrix`][]: Higham (2002)
//!   alternating-projection PSD repair
//! - [`validate_correlation_matrix`]: Core's canonical correlation validation
//!   with located [`Error`] variants (`DiagonalNotOne`, `OutOfBounds`, …)

mod error;
mod nearest_correlation;

pub use error::{Error, Result};
pub use nearest_correlation::{nearest_correlation_matrix, NearestCorrelationOpts};

/// Validate a flattened row-major correlation matrix.
///
/// Delegates to core's canonical detailed validator, using its diagonal,
/// symmetry, and `[-1, 1]` thresholds and pivoted Cholesky PSD check. Core's
/// existing [`finstack_quant_core::math::linalg::validate_correlation_matrix`]
/// maps the same detailed diagnostics to its coarse `InputError` API.
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
    finstack_quant_core::math::linalg::validate_correlation_matrix_detailed(matrix, n)
}
