//! Bond quote engine for mapping between price, yields, and spreads.
//!
//! This module provides a small, opinionated API that takes **one**
//! quote input (price, yield, or spread) and produces a consistent
//! set of derived bond quotes using the existing pricing and metric
//! infrastructure.
//!
//! All spread-style quantities exposed here use **decimal units**:
//! `0.01` corresponds to **100 basis points**.

mod annuity;
mod compute;
mod spread_price;
mod types;
mod yield_price;

pub use annuity::{
    asset_swap_forward_components, fixed_leg_annuity, par_rate_and_annuity_from_discount,
    par_rate_and_annuity_from_forward, periods_per_year,
};
pub use compute::compute_quotes;
pub use spread_price::{price_from_dm, price_from_oas, price_from_z_spread};
pub use types::{BondQuoteInput, BondQuoteSet, YieldCompounding};
pub use yield_price::{
    df_from_yield, price_from_ytm, price_from_ytm_compounded, price_from_ytm_compounded_params,
    price_from_ytw,
};

pub(crate) use annuity::asset_swap_projection_rate;
pub(crate) use compute::{clear_price_driving_overrides, price_from_quote_overrides};
pub(crate) use yield_price::{
    enumerate_exit_paths, outstanding_principal_at_date, price_from_japanese_simple_yield,
    solve_ytw_from_flows,
};

/// One candidate early-exit for yield-to-worst enumeration.
///
/// Represents a single admissible exercise date and the corresponding clean
/// redemption price expressed as a percentage of par (e.g. `103.0` for 103%).
#[derive(Debug, Clone, Copy)]
pub(crate) struct ExitCandidate {
    /// The admissible exercise date (call or put date, aligned to a flow date
    /// where possible, and clipped to `[as_of, bond.maturity]`).
    pub(crate) date: finstack_quant_core::dates::Date,
    /// Clean redemption price as percent of par (e.g. `103.0` for 103%).
    pub(crate) price_pct_of_par: f64,
}

#[cfg(test)]
mod tests;
