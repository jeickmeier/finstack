//! Credit analysis tools.
//!
//! - [`crate::analysis::credit::covenants`] — covenant forecasting bridge between statements and the covenant engine
//! - [`crate::analysis::credit::credit_context`] — coverage ratios (DSCR, interest coverage, LTV) from statement data

pub(crate) mod covenants;
pub(crate) mod credit_context;

pub use covenants::{
    forecast_breaches, forecast_covenant, forecast_covenants, to_table, StatementsAdapter,
};
pub use credit_context::{compute_credit_context, CreditContextMetrics};
