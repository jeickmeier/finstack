//! Comprehensive unit tests for common instruments module.
//!
//! Organized by:
//! - parameters: Parameter types and conventions
//! - test_helpers: Shared test utilities and fixtures
//! - parity: Tolerance-based comparison against documented reference values.

#[macro_use]
pub mod parity;
pub mod parameters;
pub mod pricer;
pub mod test_discountable;
pub mod test_helpers;
pub mod test_rates_credit_tree;
