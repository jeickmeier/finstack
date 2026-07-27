//! WASM bindings for `finstack-quant-analytics`.
//!
//! The primary entry point exposed to JS is the [`JsPerformance`] class
//! (exported to JS as `Performance`). Every analytic — returns/risk
//! metrics, periodic returns, benchmark comparisons, basic factor models —
//! is reachable as a `Performance` method. The one free function,
//! [`constrained_least_squares`] (exported as `constrainedLeastSquares`),
//! is a standalone numeric regression building block for factor-Brinson
//! attribution and does not depend on `Performance`.

mod performance;
mod regression;
mod support;

pub use performance::JsPerformance;
pub use regression::constrained_least_squares;
