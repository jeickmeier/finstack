//! Kyle (1985) linear price impact model.
//!
//! A simple model where price impact is proportional to signed order flow.
//! Useful for quick screening and as a calibration target.
//!
//! # References
//!
//! - Kyle, A.S. (1985). "Continuous Auctions and Insider Trading."
//!   *Econometrica*, 53(6). `docs/REFERENCES.md#kyle1985ContinuousAuctions`

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

use super::impact::{ExecutionTrajectory, ImpactEstimate, MarketImpactModel, TradeParams};

/// Kyle (1985) price impact model.
///
/// A simple linear model where price impact is proportional to
/// signed order flow:
///
/// ```text
/// delta_price = lambda * order_flow
/// ```
///
/// Lambda can be estimated from the Amihud ratio or regressed from
/// trade-and-quote data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KyleLambdaModel {
    /// Price impact per unit of order flow.
    lambda: f64,
}

impl KyleLambdaModel {
    /// Create a new Kyle model with a given lambda.
    ///
    /// # Errors
    ///
    /// Returns `Error::InvalidInput` if `lambda` is negative or non-finite.
    pub fn new(lambda: f64) -> Result<Self> {
        if !lambda.is_finite() || lambda < 0.0 {
            return Err(Error::invalid_input(
                "lambda must be finite and non-negative",
            ));
        }
        Ok(Self { lambda })
    }

    /// Estimate price-space lambda from observed volume/return series and a
    /// reference price.
    ///
    /// ```text
    /// lambda ~= mean(|r_t| / V_t) * reference_price
    /// ```
    ///
    /// Returns `None` when the inputs are empty, mismatched in length, otherwise
    /// invalid, or when `reference_price` is non-positive/non-finite.
    ///
    /// # Arguments
    ///
    /// * `volumes` - Strictly positive volume observations in consistent share
    ///   or contract units.
    /// * `returns` - Simple decimal returns aligned one-for-one with `volumes`.
    /// * `reference_price` - Positive price per share or contract used to
    ///   convert the return-space Amihud ratio into price-space lambda.
    pub fn lambda_from_series(
        volumes: &[f64],
        returns: &[f64],
        reference_price: f64,
    ) -> Option<f64> {
        if volumes.is_empty() || volumes.len() != returns.len() {
            return None;
        }
        let illiq = super::amihud_illiquidity(returns, volumes)?;
        Self::from_amihud(illiq, reference_price)
            .ok()
            .map(|m| m.lambda())
    }

    /// Estimate price-space lambda from an Amihud ratio and reference price.
    ///
    /// ```text
    /// lambda ~= amihud_ratio * reference_price
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `Error::InvalidInput` if inputs are non-finite, negative, or if
    /// `reference_price` is zero.
    ///
    /// # Arguments
    ///
    /// * `amihud_ratio` - Non-negative mean absolute decimal return per unit
    ///   volume.
    /// * `reference_price` - Positive price per share or contract used to
    ///   convert the return-space ratio into price-space lambda.
    pub fn from_amihud(amihud_ratio: f64, reference_price: f64) -> Result<Self> {
        if !amihud_ratio.is_finite() || amihud_ratio < 0.0 {
            return Err(Error::invalid_input(
                "amihud_ratio must be finite and non-negative",
            ));
        }
        if !reference_price.is_finite() || reference_price <= 0.0 {
            return Err(Error::invalid_input(
                "reference_price must be finite and positive",
            ));
        }
        Self::new(amihud_ratio * reference_price)
    }

    /// Get the lambda parameter.
    #[inline]
    pub fn lambda(&self) -> f64 {
        self.lambda
    }
}

impl MarketImpactModel for KyleLambdaModel {
    fn estimate_cost(&self, params: &TradeParams) -> Result<ImpactEstimate> {
        if !params.quantity.is_finite() {
            return Err(Error::invalid_input("quantity must be finite"));
        }
        if !params.horizon_days.is_finite() || params.horizon_days <= 0.0 {
            return Err(Error::invalid_input(
                "horizon_days must be finite and positive",
            ));
        }
        if !params.daily_volatility.is_finite() || params.daily_volatility <= 0.0 {
            return Err(Error::invalid_input(
                "daily_volatility must be finite and positive",
            ));
        }

        let q = params.quantity;

        // Kyle model: total price impact = lambda * |Q|
        // Cost = lambda * Q^2 / 2 (integrated impact over linear execution)
        let total_cost = self.lambda * q * q * 0.5;
        let total_cost_abs = total_cost.abs();

        // Kyle's linear model has a single, fully-permanent impact term —
        // there is no separate transient component to recover from. We
        // therefore report the full cost as `permanent_impact` and zero
        // `temporary_impact` rather than splitting heuristically. Callers who
        // need a permanent/temporary decomposition should use the
        // Almgren-Chriss model, which separates the two by construction.
        let permanent_impact = total_cost_abs;
        let temporary_impact = 0.0;

        let reference_price = params.effective_reference_price();
        let notional = q.abs() * reference_price;
        let cost_bp = if notional > 0.0 {
            total_cost_abs / notional * 10_000.0
        } else {
            0.0
        };

        // Execution risk estimate based on volatility over the horizon
        let execution_risk = params.daily_volatility
            * (params.horizon_days / 3.0).sqrt()
            * q.abs()
            * reference_price;

        Ok(ImpactEstimate {
            permanent_impact,
            temporary_impact,
            total_cost: total_cost_abs,
            cost_bp,
            execution_risk,
        })
    }

    fn optimal_trajectory(
        &self,
        params: &TradeParams,
        num_buckets: usize,
    ) -> Result<ExecutionTrajectory> {
        if num_buckets == 0 {
            return Err(Error::invalid_input("num_buckets must be > 0"));
        }
        if !params.quantity.is_finite() {
            return Err(Error::invalid_input("quantity must be finite"));
        }
        if !params.horizon_days.is_finite() || params.horizon_days <= 0.0 {
            return Err(Error::invalid_input(
                "horizon_days must be finite and positive",
            ));
        }

        let q = params.quantity;
        let t = params.horizon_days;
        let dt = t / num_buckets as f64;

        // For the Kyle linear model, the optimal trajectory is uniform
        // (constant trading rate) when there's no risk aversion on timing,
        // because the impact is purely a function of total quantity.
        let per_bucket = q / num_buckets as f64;

        let mut quantities = Vec::with_capacity(num_buckets);
        let mut remaining = Vec::with_capacity(num_buckets + 1);
        let mut time_points = Vec::with_capacity(num_buckets + 1);

        remaining.push(q);
        time_points.push(0.0);

        for j in 1..=num_buckets {
            quantities.push(per_bucket);
            let rem = if j == num_buckets {
                0.0
            } else {
                q - per_bucket * j as f64
            };
            remaining.push(rem);
            time_points.push(j as f64 * dt);
        }

        // Expected cost under uniform execution
        let expected_cost = self.lambda * q * q * 0.5;
        let expected_cost_abs = expected_cost.abs();

        // Variance of cost: depends on volatility and remaining inventory
        let sigma = params.daily_volatility * params.effective_reference_price();
        let mut cost_variance = 0.0;
        for j in 0..num_buckets {
            cost_variance += sigma * sigma * remaining[j + 1] * remaining[j + 1] * dt;
        }

        Ok(ExecutionTrajectory {
            quantities,
            remaining,
            expected_cost: expected_cost_abs,
            cost_variance,
            time_points,
        })
    }

    fn model_name(&self) -> &str {
        "Kyle-Lambda"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::liquidity::types::LiquidityProfile;

    fn test_profile() -> std::result::Result<LiquidityProfile, Box<dyn std::error::Error>> {
        Ok(LiquidityProfile::new(
            "TEST",
            100.0,
            99.5,
            100.5,
            1_000_000.0,
            500.0,
            0.001,
        )?)
    }

    fn test_params(quantity: f64) -> std::result::Result<TradeParams, Box<dyn std::error::Error>> {
        Ok(TradeParams {
            quantity,
            horizon_days: 5.0,
            daily_volatility: 0.02,
            profile: test_profile()?,
            risk_aversion: None,
            reference_price: None,
        })
    }

    #[test]
    fn construction_valid() {
        assert!(KyleLambdaModel::new(0.001).is_ok());
        assert!(KyleLambdaModel::new(0.0).is_ok()); // zero lambda is valid
    }

    #[test]
    fn construction_rejects_negative() {
        assert!(KyleLambdaModel::new(-0.001).is_err());
    }

    #[test]
    fn construction_rejects_nan() {
        assert!(KyleLambdaModel::new(f64::NAN).is_err());
    }

    #[test]
    fn from_amihud_calibrates_price_space_lambda(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let model = KyleLambdaModel::from_amihud(1e-8, 100.0)?;
        assert!((model.lambda() - 1e-6).abs() < 1e-12);

        let params = test_params(10_000.0)?;
        let est = model.estimate_cost(&params)?;
        assert!((est.total_cost - 50.0).abs() < 1e-10);
        assert!((est.cost_bp - 0.5).abs() < 1e-10);
        Ok(())
    }

    #[test]
    fn lambda_from_series_calibrates_price_space_lambda() {
        let lambda = KyleLambdaModel::lambda_from_series(&[100.0, 200.0], &[0.01, -0.02], 50.0)
            .expect("valid price-space inputs");
        assert!((lambda - 0.005).abs() < 1e-15);
        assert_eq!(
            KyleLambdaModel::lambda_from_series(&[100.0], &[0.01], 0.0),
            None
        );
    }

    #[test]
    fn estimate_cost_nonnegative() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let model = KyleLambdaModel::new(0.001)?;
        let params = test_params(10_000.0)?;
        let est = model.estimate_cost(&params)?;

        assert!(est.total_cost >= 0.0);
        assert!(est.cost_bp >= 0.0);
        let expected_execution_risk = params.daily_volatility * params.horizon_days.sqrt()
            / 3.0_f64.sqrt()
            * params.quantity.abs()
            * params.effective_reference_price();
        assert!((est.execution_risk - expected_execution_risk).abs() < 1e-9);
        Ok(())
    }

    #[test]
    fn estimate_cost_matches_price_space_lambda(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        // Price impact for a trade of size Q is lambda * Q.
        // For Q=1000 shares at lambda=0.001:
        // price_impact = 0.001 * 1000 = 1.0
        // total_cost = 0.001 * 1000^2 / 2 = 500.0
        let model = KyleLambdaModel::new(0.001)?;
        let params = test_params(1_000.0)?;
        let est = model.estimate_cost(&params)?;
        assert!(
            (est.total_cost - 500.0).abs() < 1e-6,
            "expected 500.0, got {}",
            est.total_cost
        );
        Ok(())
    }

    #[test]
    fn trajectory_uniform_execution() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let model = KyleLambdaModel::new(0.001)?;
        let params = test_params(10_000.0)?;
        let traj = model.optimal_trajectory(&params, 5)?;

        assert_eq!(traj.quantities.len(), 5);
        let expected = 10_000.0 / 5.0;
        for q in &traj.quantities {
            assert!(
                (q - expected).abs() < 1e-6,
                "expected uniform {expected}, got {q}"
            );
        }

        let total: f64 = traj.quantities.iter().sum();
        assert!((total - 10_000.0).abs() < 1e-6);
        Ok(())
    }

    #[test]
    fn trajectory_remaining_monotone() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let model = KyleLambdaModel::new(0.001)?;
        let params = test_params(10_000.0)?;
        let traj = model.optimal_trajectory(&params, 10)?;

        for i in 1..traj.remaining.len() {
            assert!(
                traj.remaining[i] <= traj.remaining[i - 1] + 1e-10,
                "remaining should be monotonically decreasing"
            );
        }
        Ok(())
    }

    #[test]
    fn trajectory_rejects_zero_buckets() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let model = KyleLambdaModel::new(0.001)?;
        assert!(model.optimal_trajectory(&test_params(1000.0)?, 0).is_err());
        Ok(())
    }
}
