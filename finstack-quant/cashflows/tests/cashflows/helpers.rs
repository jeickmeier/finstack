//! Shared test utilities for cashflow tests.
//!
//! # Tolerance Conventions
//!
//! - `RATE_TOLERANCE` (1e-10): For rate/factor comparisons
//! - `FACTOR_TOLERANCE` (1e-12): For high-precision rate conversions
//! - `financial_tolerance(notional)`: For money amounts

/// Tolerance for rate and factor comparisons (e.g., CPR, SMM, and CDR).
pub const RATE_TOLERANCE: f64 = 1e-10;

/// Tolerance for high-precision rate conversion and round-trip comparisons.
pub const FACTOR_TOLERANCE: f64 = 1e-12;

/// Calculate appropriate tolerance for financial amounts based on notional.
pub fn financial_tolerance(notional: f64) -> f64 {
    (notional.abs() * 1e-8).max(0.01)
}
