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
#![doc(test(attr(allow(clippy::expect_used))))]

//! Multi-period P&L attribution for financial instruments.
//!
//! Attribution explains the change in an instrument's value between two dates
//! by separating carry and market-factor effects.
//!
//! # Methods
//!
//! | Entry point | Use when |
//! |---|---|
//! | [`simple_pnl_bridge`] | Only raw endpoint P&L in an explicit currency is required |
//! | [`attribute_pnl_metrics_based`] | Precomputed first- and optional second-order sensitivities provide a fast approximation |
//! | [`attribute_pnl_parallel`] | Independent factor effects and an interaction residual are required |
//! | [`attribute_pnl_waterfall`] | An ordered full-revaluation decomposition is required |
//! | [`attribute_pnl_taylor`] | A bump-and-reprice first- or second-order decomposition is required |
//!
//! # Conventions
//!
//! - Positive P&L is a gain to the long-position holder.
//! - Decomposition methods use a total-return basis: `total_pnl` includes cash
//!   paid in `[T₀, T₁)`, while `mark_to_market_pnl` preserves the raw endpoint
//!   value change when available. [`simple_pnl_bridge`] returns only that raw
//!   endpoint change.
//! - Repricing methods isolate the date roll before market moves. Waterfall
//!   orders must begin with [`AttributionFactor::Carry`]; metrics-based carry
//!   uses `CarryTotal` or theta over the elapsed days.
//! - Direct decomposition functions report in the instrument's native pricing
//!   currency; [`simple_pnl_bridge`] accepts an explicit target currency.
//!   [`AttributionConfig::target_currency`] can translate aggregate fields:
//!   opening value uses T₀ market/date FX, closing value uses T₁ market/date FX,
//!   factors use T₁ FX, and the opening-position FX move is recorded separately
//!   as `fx_translation_pnl`. Detail maps remain in native currency. The
//!   `fx_pnl` field remains the pricing impact of FX inside the instrument.
//!
//! # Residuals and errors
//!
//! Every result reconciles `total_pnl` to its aggregate factor fields plus the
//! residual. A waterfall residual should be near zero when its order covers all
//! material factors; parallel residuals contain interactions and nonlinearity;
//! metrics-based residuals contain approximation and missing-sensitivity effects;
//! Taylor residuals also contain truncation, uncovered scalars, and soft factor
//! failures.
//!
//! The four decomposition methods reject reversed date ranges; same-day ranges
//! are valid. Waterfall also requires a nonempty, duplicate-free order beginning
//! with carry, and Taylor validates its bump ranges. Repricing, market-data,
//! currency, and FX failures propagate except on documented best-effort factor
//! paths, which record failures in metadata and residual instead.
//!
//! # Example
//!
//! ```rust
//! use finstack_quant_attribution::{attribute_pnl_parallel, ExecutionPolicy};
//! use finstack_quant_core::{
//!     config::FinstackConfig,
//!     currency::Currency,
//!     market_data::{context::MarketContext, scalars::MarketScalar},
//!     money::Money,
//! };
//! use finstack_quant_valuations::instruments::{equity::spot::Equity, Instrument};
//! use std::sync::Arc;
//! use time::macros::date;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let instrument: Arc<dyn Instrument> = Arc::new(
//!     Equity::new("AAPL", "AAPL", Currency::USD)
//!         .with_price_id("AAPL-SPOT")
//!         .with_shares(100.0),
//! );
//! let market_t0 = MarketContext::new().insert_price(
//!     "AAPL-SPOT",
//!     MarketScalar::Price(Money::new(180.0, Currency::USD)),
//! );
//! let market_t1 = MarketContext::new().insert_price(
//!     "AAPL-SPOT",
//!     MarketScalar::Price(Money::new(185.0, Currency::USD)),
//! );
//!
//! let attribution = attribute_pnl_parallel(
//!     &instrument,
//!     &market_t0,
//!     &market_t1,
//!     date!(2025 - 01 - 15),
//!     date!(2025 - 01 - 16),
//!     &FinstackConfig::default(),
//!     ExecutionPolicy::Serial,
//! )?;
//!
//! assert_eq!(attribution.total_pnl, Money::new(500.0, Currency::USD));
//! assert_eq!(
//!     attribution.market_scalars_pnl,
//!     Money::new(500.0, Currency::USD),
//! );
//! # Ok(())
//! # }
//! ```

pub(crate) mod credit_cascade;
pub(crate) mod credit_decomposition;
pub(crate) mod credit_factor;
pub(crate) mod execution;
pub(crate) mod factors;
pub(crate) mod helpers;
pub(crate) mod metrics_based;
pub(crate) mod model_params;
pub(crate) mod parallel;
pub(crate) mod return_contribution;
/// JSON Schema generation helpers for attribution contracts.
pub(crate) mod spec;
pub(crate) mod target_currency;
pub(crate) mod taylor;
pub(crate) mod types;
pub(crate) mod waterfall;

// Re-export core types
pub use credit_factor::{
    compute_credit_factor_attribution, credit_factor_model_id, CreditAttributionInput,
    CreditFactorDetailOptions,
};
pub use types::detail::{
    CarryDetail, CorrelationsAttribution, CreditCarryByLevel, CreditCarryDecomposition,
    CreditCurvesAttribution, CreditFactorAttribution, CrossFactorDetail, FxAttribution,
    InflationCurvesAttribution, LevelCarry, LevelPnl, ModelParamsAttribution,
    RatesCurvesAttribution, ScalarsAttribution, SourceLine, VolAttribution,
};
pub use types::result::{
    AttributionFactor, AttributionMeta, AttributionMethod, ExecutionPolicy, PnlAttribution,
};

// Re-export attribution functions
pub use metrics_based::attribute_pnl_metrics_based;
pub use model_params::{
    extract_model_params, measure_conversion_shift, measure_default_shift,
    measure_prepayment_shift, measure_recovery_shift, with_model_params,
};
pub use parallel::attribute_pnl_parallel;
pub use return_contribution::{
    attribute_return_contribution, validate_return_contribution_json,
    BenchmarkRelativeContribution, FactorContribution, GroupContribution, InstrumentContribution,
    ReturnContributionFactor, ReturnContributionPosition, ReturnContributionResult,
    ReturnContributionSpec, ReturnContributionWeighting,
};
pub use spec::{
    default_attribution_metrics, AttributionConfig, AttributionEnvelope, AttributionResult,
    AttributionResultEnvelope, AttributionSchema, AttributionSpec, ATTRIBUTION_SCHEMA,
};
pub use target_currency::translate_to_target_currency;
pub use taylor::{attribute_pnl_taylor, TaylorAttributionConfig};
pub use waterfall::{attribute_pnl_waterfall, default_waterfall_order};
// Market snapshot helpers
pub use factors::{MarketRestoreFlags, MarketSnapshot};
pub use helpers::{compute_pnl, compute_pnl_with_fx};

use finstack_quant_core::currency::Currency;
use finstack_quant_core::dates::Date;
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::money::Money;
use finstack_quant_valuations::instruments::Instrument;
use std::sync::Arc;

/// Hidden support paths for sibling Finstack crates.
#[doc(hidden)]
pub mod __private {
    use super::*;

    /// Attribute one instrument using endpoint values prepared by a portfolio
    /// evaluation engine.
    ///
    /// # Arguments
    ///
    /// * `instrument` - Instrument whose unscaled endpoint values were prepared.
    /// * `market_t0` - Opening market state used by factor repricings.
    /// * `market_t1` - Closing market state used by factor repricings.
    /// * `as_of_t0` - Opening valuation date corresponding to `val_t0`.
    /// * `as_of_t1` - Closing valuation date corresponding to `val_t1`.
    /// * `config` - Finstack configuration used by the selected method.
    /// * `method` - Repricing-based attribution method to execute.
    /// * `execution_policy` - Inner factor scheduling policy.
    /// * `val_t0` - Canonical unscaled instrument value at T0.
    /// * `val_t1` - Canonical unscaled instrument value at T1.
    ///
    /// # Returns
    ///
    /// The same financial decomposition as the standalone method, without
    /// repeating its two ordinary endpoint repricings.
    ///
    /// # Errors
    ///
    /// Returns method-specific validation, market-data, repricing, and
    /// currency errors. Metrics-based attribution is rejected because it
    /// requires complete prepared valuation results.
    #[allow(clippy::too_many_arguments)]
    pub fn attribute_pnl_prepared(
        instrument: &Arc<dyn Instrument>,
        market_t0: &MarketContext,
        market_t1: &MarketContext,
        as_of_t0: Date,
        as_of_t1: Date,
        config: &finstack_quant_core::config::FinstackConfig,
        method: &AttributionMethod,
        execution_policy: ExecutionPolicy,
        val_t0: Money,
        val_t1: Money,
    ) -> finstack_quant_core::Result<PnlAttribution> {
        match method {
            AttributionMethod::Parallel => parallel::attribute_pnl_parallel_prepared(
                instrument,
                market_t0,
                market_t1,
                as_of_t0,
                as_of_t1,
                config,
                execution_policy,
                val_t0,
                val_t1,
            ),
            AttributionMethod::Waterfall(order) => waterfall::attribute_pnl_waterfall_prepared(
                instrument,
                market_t0,
                market_t1,
                as_of_t0,
                as_of_t1,
                config,
                order.clone(),
                val_t0,
                val_t1,
            ),
            AttributionMethod::Taylor(taylor_config) => taylor::attribute_pnl_taylor_prepared(
                instrument,
                market_t0,
                market_t1,
                as_of_t0,
                as_of_t1,
                taylor_config,
                execution_policy,
                val_t0,
                val_t1,
            ),
            AttributionMethod::MetricsBased => Err(finstack_quant_core::Error::Validation(
                "metrics-based attribution requires prepared valuation results, not scalar endpoints"
                    .to_string(),
            )),
        }
    }
}

/// Minimal, no-frills P&L bridge: `value(T₁) − value(T₀)`.
///
/// This is the **cheapest** attribution entry point — it prices the
/// instrument once at each date in each market state and returns the
/// scalar total P&L in `target_currency`. FX conversion uses `market_t0` for
/// the T₀ value and `market_t1` for the T₁ value. Use it when you just
/// need the headline number and don't care which
/// factors contributed. For a factor-level decomposition, reach for
/// one of the `attribute_pnl_*` functions listed in the module docs.
///
/// This is intentionally a thin wrapper over direct repricing plus
/// [`compute_pnl_with_fx`]: the function is cheap, it allocates no scratch
/// buffers, and it contains no factor iteration. Benchmark the heavier
/// methodologies against this baseline to quantify the cost of factor
/// attribution.
///
/// # Arguments
///
/// * `instrument` - Instrument to reprice at both valuation dates.
/// * `market_t0` - Opening market state used to calculate the T₀ value and
///   opening FX conversion.
/// * `market_t1` - Closing market state used to calculate the T₁ value and
///   closing FX conversion.
/// * `as_of_t0` - Opening valuation date used for the T₀ repricing.
/// * `as_of_t1` - Closing valuation date used for the T₁ repricing.
/// * `target_currency` - Currency to report P&L in; FX is resolved from the
///   date-specific market contexts.
///
/// # Returns
///
/// The total P&L `v_t1 − v_t0` in `target_currency`.
///
/// # Errors
///
/// Returns an error if either repricing call fails or if the FX
/// conversion cannot be resolved from the provided market contexts.
///
/// # Examples
///
/// ```no_run
/// use finstack_quant_attribution::simple_pnl_bridge;
/// use finstack_quant_valuations::instruments::Instrument;
/// use finstack_quant_core::currency::Currency;
/// use finstack_quant_core::market_data::context::MarketContext;
/// use std::sync::Arc;
/// use time::macros::date;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let instrument: Arc<dyn Instrument> = unimplemented!("obtain the instrument under test");
/// let market_t0 = MarketContext::new();
/// let market_t1 = MarketContext::new();
///
/// let pnl = simple_pnl_bridge(
///     &instrument,
///     &market_t0,
///     &market_t1,
///     date!(2025 - 01 - 15),
///     date!(2025 - 01 - 16),
///     Currency::USD,
/// )?;
/// println!("Daily P&L: {pnl}");
/// # Ok(())
/// # }
/// ```
pub fn simple_pnl_bridge(
    instrument: &Arc<dyn Instrument>,
    market_t0: &MarketContext,
    market_t1: &MarketContext,
    as_of_t0: Date,
    as_of_t1: Date,
    target_currency: Currency,
) -> finstack_quant_core::Result<Money> {
    let v_t0 = helpers::reprice_instrument(instrument, market_t0, as_of_t0)?;
    let v_t1 = helpers::reprice_instrument(instrument, market_t1, as_of_t1)?;
    compute_pnl_with_fx(
        v_t0,
        v_t1,
        target_currency,
        market_t0,
        market_t1,
        as_of_t0,
        as_of_t1,
    )
}
