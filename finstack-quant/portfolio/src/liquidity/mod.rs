//! Liquidity risk metrics, spread estimation, and portfolio scoring.
//!
//! This module provides market microstructure liquidity modeling for traded
//! positions. It is orthogonal to the balance-sheet liquidity ratios in
//! `finstack-quant-statements-analytics` and focuses on:
//!
//! - **Spread estimation**: Roll (1984) effective spread and Amihud (2002)
//!   illiquidity ratio from return/volume data.
//! - **Liquidity-adjusted VaR (LVaR)**: Bangia et al. (1999) framework
//!   combining exogenous spread costs, endogenous position-size effects,
//!   and time-to-liquidation horizon adjustments.
//! - **Market impact models**: Almgren-Chriss (2001) optimal execution with
//!   permanent/temporary impact decomposition, and Kyle (1985) linear lambda.
//! - **Portfolio liquidity scoring**: Position-level days-to-liquidate, tier
//!   classification, and aggregate portfolio liquidity reports.
//!
//! # Architecture
//!
//! The module is structured in layers:
//!
//! 1. **Types** (`types`): `LiquidityProfile`, `LiquidityTier`, `LiquidityConfig`
//! 2. **Estimators** (`estimators`): Pure functions on `&[f64]` slices
//! 3. **LVaR** (`lvar`): Composes with existing VaR numbers
//! 4. **Impact** (`impact`, `almgren_chriss`, `kyle`): Trade execution cost models
//!
//! # Usage
//!
//! Estimate the effective spread from a return series, then charge that spread
//! against an existing VaR number:
//!
//! ```
//! use finstack_quant_portfolio::liquidity::{lvar_bangia_scalar, roll_effective_spread};
//!
//! # fn main() -> finstack_quant_core::Result<()> {
//! let returns = [0.004, -0.005, 0.006, -0.004, 0.005, -0.006];
//! let spread = roll_effective_spread(&returns).expect("at least two returns");
//! assert!(spread > 0.0);
//!
//! // VaR uses the loss-sign convention, so it is non-positive.
//! let result = lvar_bangia_scalar(-100_000.0, spread, 0.25 * spread, 0.99, 1_000_000.0)?;
//! assert!(result.spread_cost >= 0.0);
//! assert!(result.lvar <= result.var);
//! # Ok(())
//! # }
//! ```

mod almgren_chriss;
mod estimators;
mod impact;
mod kyle;
mod lvar;
mod types;

// Re-export core types
pub use types::{
    classify_tier, days_to_liquidate, LiquidityConfig, LiquidityProfile, LiquidityTier,
    SpreadVolatilityKind, TierAllocation,
};

// Re-export estimators
pub use estimators::{amihud_illiquidity, roll_effective_spread};

// Re-export LVaR
pub use lvar::{lvar_bangia_scalar, LvarBangiaScalar};

// Re-export impact models
pub use almgren_chriss::AlmgrenChrissModel;
pub use impact::{ExecutionTrajectory, ImpactEstimate, MarketImpactModel, TradeParams};
pub use kyle::KyleLambdaModel;

/// Build and evaluate a uniform Almgren-Chriss market-impact estimate.
///
/// # Arguments
///
/// * `position_size` - Signed quantity to execute; its sign determines trade
///   direction while its magnitude determines participation.
/// * `avg_daily_volume` - Positive average daily tradable volume in the same
///   quantity units as `position_size`.
/// * `volatility` - Positive daily volatility as a decimal fraction.
/// * `execution_horizon_days` - Intended execution horizon in trading days.
/// * `permanent_impact_coef` - Permanent-impact coefficient for the model.
/// * `temporary_impact_coef` - Temporary-impact coefficient for the model.
/// * `reference_price` - Optional positive mid price; `None` uses unit price
///   and reports scale-free impact.
///
/// # Errors
///
/// Returns an error when the liquidity inputs are not finite and positive, or
/// when the underlying model/profile validation fails.
#[allow(clippy::too_many_arguments)]
pub fn almgren_chriss_uniform_impact(
    position_size: f64,
    avg_daily_volume: f64,
    volatility: f64,
    execution_horizon_days: f64,
    permanent_impact_coef: f64,
    temporary_impact_coef: f64,
    reference_price: Option<f64>,
) -> crate::error::Result<ImpactEstimate> {
    if !avg_daily_volume.is_finite() || avg_daily_volume <= 0.0 {
        return Err(crate::Error::validation(
            "avg_daily_volume must be finite and positive",
        ));
    }
    if !volatility.is_finite() || volatility <= 0.0 {
        return Err(crate::Error::validation(
            "volatility must be finite and positive",
        ));
    }
    if let Some(price) = reference_price {
        if !price.is_finite() || price <= 0.0 {
            return Err(crate::Error::validation(
                "reference_price must be finite and positive",
            ));
        }
    }

    let model = AlmgrenChrissModel::new(permanent_impact_coef, temporary_impact_coef, 0.5)?;
    let mid = reference_price.unwrap_or(1.0);
    let profile = LiquidityProfile::new(
        "AC_CALIBRATION",
        mid,
        mid * 0.999,
        mid * 1.001,
        avg_daily_volume,
        1.0,
        0.0,
    )?;
    let params = TradeParams {
        quantity: position_size,
        horizon_days: execution_horizon_days,
        daily_volatility: volatility,
        profile,
        risk_aversion: None,
        reference_price,
    };
    model.estimate_cost(&params)
}
