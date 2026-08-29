//! Error types for correlation matrix utilities.

/// Convenience result type used throughout the correlation crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Detailed error type for correlation matrix operations.
pub use finstack_quant_core::math::linalg::CorrelationError as Error;
