//! Re-calibration helpers for curve and surface bumping.
//!
//! Supports "what-if" risk analysis: apply a `BumpRequest` (parallel or
//! per-tenor) to a calibrated market object and re-run the matching
//! calibration step to produce a new curve/surface. Used by the scenarios
//! engine and per-instrument risk metrics (CS01, key-rate duration, vega).
//!
//! # Entry points by asset class
//!
//! | Asset class       | Functions                                                   |
//! |-------------------|-------------------------------------------------------------|
//! | Discount rates    | `bump_discount_curve`, `bump_discount_curve_from_rate_calibration`, `bump_discount_curve_with_config`, `bump_discount_curve_synthetic` |
//! | Forward rates     | `bump_forward_curve_from_rate_calibration`                    |
//! | Credit hazard     | `bump_hazard_spreads`, `bump_hazard_shift`                  |
//! | Inflation         | `bump_inflation_rates`                                      |
//! | Volatility        | `bump_vol_surface` (uses `VolBumpRequest`)                  |
//!
//! # Convention
//!
//! Rate/spread bumps are specified in basis points. Vol bumps use
//! `VolBumpRequest` because the semantics differ (absolute vol points vs
//! relative shifts) — see that type for details.
//!
//! # Calibration policy
//!
//! Stored rate-calibration recipes retain the quote set, method, curve role,
//! day count, and OIS convention needed by the
//! `*_from_rate_calibration` entry points. `bump_discount_curve_with_config`
//! additionally accepts the runtime solver and validation policy. Synthetic
//! bump helpers operate directly on curve knots and do not recalibrate.
//!
//! ## Induced-error bound
//!
//! For recalibrated bumps, both the base curve and the bumped curve reprice
//! every input quote to within their respective calibration tolerances, so residual leakage into a
//! sensitivity is bounded by roughly the **sum of the two repricing tolerances
//! divided by the bump size**. With the default `1e-10` tolerance and a 1bp
//! bump this is on the order of `2e-10 / 1e-4 ≈ 2e-6` of the PV unit —
//! negligible versus the bump itself.

mod currency;
pub(crate) mod hazard;
pub(crate) mod inflation;
pub(crate) mod rates;
pub(crate) mod vol;

pub use hazard::{
    bump_hazard_shift, bump_hazard_spreads, bump_hazard_spreads_cached, HazardRecalibrationCache,
};
pub use inflation::{
    bump_inflation_rates, infer_currency_from_curve_id, observation_lag_from_curve,
};
pub use rates::{
    bump_discount_curve, bump_discount_curve_from_rate_calibration, bump_discount_curve_synthetic,
    bump_discount_curve_with_config, bump_forward_curve_from_rate_calibration,
    infer_currency_from_discount_curve_id,
};
pub use vol::{bump_vol_surface, VolBumpRequest};

/// Request for a curve bump operation.
///
/// Defines the type and magnitude of a shift to be applied to market quotes
/// before re-calibration.
#[derive(Debug, Clone, PartialEq)]
pub enum BumpRequest {
    /// Parallel shift in basis points (additive to rates/spreads).
    ///
    /// # Example
    /// ```rust
    /// # use finstack_quant_valuations::calibration::bumps::BumpRequest;
    /// let bp_10 = BumpRequest::Parallel(10.0);
    /// ```
    Parallel(f64),
    /// Node-specific shifts in basis points.
    ///
    /// Vector of `(Tenor in Years, Shift in BP)`. Used for key-rate durations
    /// and bucketed risk.
    Tenors(Vec<(f64, f64)>),
}
