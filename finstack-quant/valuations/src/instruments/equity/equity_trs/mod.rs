//! Equity total-return swaps for synthetic index or single-stock exposure.
//!
//! # Pricing scope
//!
//! Each reset period exchanges the underlying's price return plus net dividend
//! return against the floating financing rate plus contractual spread.
//! [`TrsSide::ReceiveTotalReturn`](crate::instruments::equity::equity_trs::TrsSide::ReceiveTotalReturn)
//! values the holder as `PV(total return) - PV(financing)`;
//! [`TrsSide::PayTotalReturn`](crate::instruments::equity::equity_trs::TrsSide::PayTotalReturn)
//! uses the opposite sign.
//!
//! Pricing is deterministic from supplied spot, discount, and forward curves,
//! and either a continuous dividend yield or explicit dividends. Constituent-
//! level basket decomposition, stochastic equity dynamics, early termination,
//! and bespoke fees are outside this instrument.

pub(crate) mod metrics;
pub(crate) mod pricer;
mod types;

pub use types::EquityTotalReturnSwap;

pub use crate::instruments::common_impl::parameters::trs_common::{TrsScheduleSpec, TrsSide};
