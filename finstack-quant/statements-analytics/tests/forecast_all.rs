//! Forecast analytics tests.
//!
//! Note: Cargo only discovers integration tests that are direct children of `tests/`.
//! This file wires in the nested forecast test modules so they run.

#[path = "forecast/forecast_backtesting_tests.rs"]
mod forecast_backtesting_tests;
