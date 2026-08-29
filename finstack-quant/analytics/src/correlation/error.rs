//! Error types for correlation matrix utilities.
//!
//! This module contains the structured diagnostics returned by
//! [`crate::correlation::validate_correlation_matrix`] and
//! [`crate::correlation::nearest_correlation_matrix`].
//!
//! The errors focus on caller-fixable input problems:
//! - wrong flattened matrix size
//! - non-unit diagonal entries
//! - asymmetric correlation matrices
//! - out-of-bounds correlation values
//! - matrices that are not positive semidefinite
//!
//! Use these variants when surfacing validation failures to configuration or
//! calibration layers.

/// Convenience result type used throughout the correlation crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Error type for correlation matrix validation.
///
/// This enum preserves the first validation failure detected when checking a
/// row-major flattened correlation matrix.
#[derive(Debug, Clone, PartialEq, thiserror::Error, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Error {
    /// Matrix size does not match expected n×n.
    #[error("Invalid matrix size: expected {expected}×{expected}={}, got {actual}", expected * expected)]
    InvalidSize {
        /// Expected number of factors (n for n×n matrix).
        expected: usize,
        /// Actual length of the matrix array.
        actual: usize,
    },
    /// Diagonal element is not 1.
    #[error("Diagonal element [{index},{index}] = {value}, expected 1.0")]
    DiagonalNotOne {
        /// Index of the invalid diagonal element.
        index: usize,
        /// Actual value found on diagonal.
        value: f64,
    },
    /// Matrix is not symmetric.
    #[error("Matrix not symmetric: |ρ[{i},{j}] - ρ[{j},{i}]| = {diff}")]
    NotSymmetric {
        /// Row index.
        i: usize,
        /// Column index.
        j: usize,
        /// Absolute difference `|rho[i,j] - rho[j,i]|`.
        diff: f64,
    },
    /// Matrix is not positive semi-definite (Cholesky failed).
    #[error("Matrix not positive semi-definite: Cholesky failed at row {row}")]
    NotPositiveSemiDefinite {
        /// Row where Cholesky decomposition failed.
        row: usize,
    },
    /// Correlation value out of bounds [-1, 1].
    #[error("Correlation ρ[{i},{j}] = {value} out of bounds [-1, 1]")]
    OutOfBounds {
        /// Row index.
        i: usize,
        /// Column index.
        j: usize,
        /// Out-of-bounds value.
        value: f64,
    },
    /// Iterative algorithm exhausted its iteration budget before converging.
    ///
    /// Returned by
    /// [`crate::correlation::nearest_correlation_matrix`]
    /// when the alternating-projection iteration does not reach the requested
    /// tolerance within `max_iter` iterations. Distinct from
    /// [`Error::NotPositiveSemiDefinite`], which reports a *validation*
    /// failure on a supplied matrix.
    #[error("Did not converge within {max_iter} iterations (tolerance {tol})")]
    DidNotConverge {
        /// Iteration budget that was exhausted.
        max_iter: usize,
        /// Frobenius-norm convergence tolerance that was not reached.
        tol: f64,
    },
}
