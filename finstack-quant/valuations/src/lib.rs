#![forbid(unsafe_code)]
#![warn(clippy::float_cmp)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::indexing_slicing,
        clippy::float_cmp,
    )
)]
// Allow expect() in doc tests (they are test code)
#![doc(test(attr(allow(clippy::expect_used))))]

//! Financial-instrument pricing, risk, calibration, and cashflow analysis.
//!
//! Instruments implement [`crate::instruments::Instrument`], consume market data from
//! [`finstack_quant_core::market_data::MarketContext`], and dispatch through registered pricing
//! models. Pricing returns a currency-tagged present value plus requested metrics in
//! [`crate::results::ValuationResult`].
//!
//! # Documentation Conventions
//!
//! - Prefer typed rates and spreads where the API accepts them, and always state the
//!   representation and units of financial inputs.
//! - Treat [`crate::results::ValuationResult::value`] as monetary present value; interpret other
//!   measures using the units, sign, and bump contracts on [`crate::metrics::MetricId`].
//! - Document day-count, calendar, compounding, settlement, quote, and curve-role conventions at
//!   the public API that owns the behavior.
//! - Cite named models and market conventions with canonical entries.
//!
//! # Modules
//!
//! ## Pricing Workflow
//!
//! - [`crate::instruments`]: instrument definitions and the common pricing contract.
//! - [`crate::market`]: market quotes and conventions.
//! - [`crate::pricer`]: pricing dispatch and registry infrastructure.
//! - [`crate::metrics`]: risk metric identifiers, calculators, and registries.
//! - [`crate::results`]: valuation result envelopes and metadata.
//!
//! ## Models and Calibration
//!
//! - [`crate::calibration`]: curve and surface calibration.
//! - [`crate::models`]: pricing models and numerical methods.
//! - [`crate::correlation`]: credit correlation models.
//!
//! ## Supporting APIs
//!
//! - [`crate::constants`]: shared numerical constants.
//! - [`crate::schema`]: JSON Schema generation for API contracts.
//! - [`crate::error`]: valuation error types; [`crate::Error`] and [`crate::Result`] are also
//!   available at the crate root.
//! - [`crate::prelude`]: convenient re-exports for common pricing and risk workflows.
//!
//! # Example
//!
//! [`crate::instruments::Instrument::price_with_metrics`] is the canonical pricing entry point.
//!
//! ```rust
//! use finstack_quant_core::{
//!     currency::Currency,
//!     dates::create_date,
//!     market_data::MarketContext,
//! };
//! use finstack_quant_valuations::{
//!     instruments::{Equity, Instrument, PricingOptions},
//!     metrics::MetricId,
//! };
//! use time::Month;
//!
//! # fn main() -> finstack_quant_core::Result<()> {
//! let equity = Equity::new("AAPL", "AAPL", Currency::USD)
//!     .with_shares(50.0)
//!     .with_price(200.0);
//! let result = equity.price_with_metrics(
//!     &MarketContext::new(),
//!     create_date(2026, Month::January, 2)?,
//!     &[MetricId::EquityPricePerShare],
//!     PricingOptions::default(),
//! )?;
//! assert_eq!(result.value.amount(), 10_000.0);
//! assert_eq!(result.metric(MetricId::EquityPricePerShare), Some(200.0));
//! # Ok(())
//! # }
//! ```
//!
//! # Feature Flag
//!
//! `ts_export` enables TypeScript schema type generation.

extern crate self as finstack_quant_valuations;

/// Curve and surface calibration tooling.
pub mod calibration;
/// Cashflow schedule generation and builders.
pub(crate) use finstack_quant_cashflows as cashflow;
/// Shared numerical constants and basis point helpers.
pub mod constants;
pub(crate) mod contract_specs;
/// Copula, factor, and recovery models for credit correlation.
///
/// Provides reusable correlation infrastructure used across credit instruments
/// (CDS tranche pricing, structured credit engines, portfolio credit risk).
pub mod correlation;
/// Error types for pricing and valuation workflows.
pub mod error;
/// Market quotes and conventions
pub mod market;
/// Pricing models and numerical methods.
pub mod models;
/// Convenient re-exports for pricing and risk calculations.
pub mod prelude;
/// Pricing dispatch and registry infrastructure.
pub mod pricer;
/// Valuation result envelopes and metadata.
pub mod results;
/// JSON Schema generation for API contracts.
pub mod schema;
pub(crate) mod serde_defaults;

/// Hidden support paths for exported macros.
#[doc(hidden)]
pub mod __private {
    /// Cashflow crate path used by exported macros without exposing `cashflow`.
    pub use finstack_quant_cashflows;
}

#[macro_use]
/// Financial instrument definitions and builders.
pub mod instruments;
/// Risk metric calculators and registries.
pub mod metrics;

pub use error::{Error, Result};
