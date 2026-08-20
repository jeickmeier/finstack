//! Merton structural credit model with distance-to-default and default probability.
//!
//! Implements the Merton (1974) model and its Black-Cox (1976) first-passage
//! extension for estimating firm default probability from balance-sheet data.
//!
//! # References
//!
//! - Merton, R. C. (1974). "On the Pricing of Corporate Debt: The Risk
//!   Structure of Interest Rates." *Journal of Finance*, 29(2), 449-470. `docs/REFERENCES.md#merton-1974`
//!
//! - Black, F. & Cox, J. C. (1976). "Valuing Corporate Securities: Some
//!   Effects of Bond Indenture Provisions." *Journal of Finance*, 31(2), 351-367. `docs/REFERENCES.md#black-1976`
//!
//! - Merton, R. C. (1976). "Option Pricing When Underlying Stock Returns Are
//!   Discontinuous." *Journal of Financial Economics*, 3(1-2), 125-144.
//!   Poisson mixture behind the jump-diffusion default probability. `docs/REFERENCES.md#merton-1976-jump`
//!
//! - Finger, C. C. et al. (2002). *CreditGrades Technical Document*.
//!   RiskMetrics Group. Uncertain-barrier survival approximation. `docs/REFERENCES.md#finger-2002-creditgrades`
//!
//! - Crosbie, P. & Bohn, J. (2003). *Modeling Default Risk*. Moody's KMV.
//!   Physical-measure distance to default, EDF, and the default point. `docs/REFERENCES.md#crosbie-bohn-2003-kmv`
//!
//! - O'Kane, D. (2008). *Modelling Single-name and Multi-name Credit
//!   Derivatives*. Wiley Finance. CDS premium and protection leg
//!   discretization used by [`MertonModel::cds_par_spread`]. `docs/REFERENCES.md#o-kane-2008`
//!
//! # Spread conventions
//!
//! Three distinct credit spreads are available and they are **not**
//! interchangeable:
//!
//! - [`MertonModel::implied_spread`] — continuously compounded zero-coupon
//!   bond spread with an *exogenous* recovery paid at maturity.
//! - [`MertonModel::debt_spread`] — Merton (1974) *endogenous* debt spread,
//!   where recovery is the firm's own terminal asset value.
//! - [`MertonModel::cds_par_spread`] — ISDA-style CDS par spread built from
//!   the model's survival curve, with a premium leg, accrual on default, and
//!   discounting.
//!
//! # Examples
//!
//! ```
//! use finstack_quant_valuations::models::credit::MertonModel;
//!
//! let model = MertonModel::new(100.0, 0.20, 80.0, 0.05).unwrap();
//! let dd = model.distance_to_default(1.0);
//! let pd = model.default_probability(1.0);
//! let spread = model.implied_spread(5.0, 0.40).unwrap();
//! ```

use finstack_quant_core::dates::DayCount;
use finstack_quant_core::market_data::term_structures::HazardCurve;
use finstack_quant_core::math::norm_cdf;
use finstack_quant_core::math::solver::{BrentSolver, Solver};
use finstack_quant_core::{Error, InputError, Result};

use finstack_quant_core::math::random::{poisson_inverse_cdf, RandomNumberGenerator};

/// Asset dynamics specification for the Merton model.
///
/// Controls the stochastic process assumed for the firm's asset value.
#[derive(
    Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum AssetDynamics {
    /// Standard geometric Brownian motion (lognormal diffusion).
    GeometricBrownian,
    /// Jump-diffusion process (Merton 1976) with Poisson jumps.
    JumpDiffusion {
        /// Poisson jump arrival intensity (jumps per year).
        jump_intensity: f64,
        /// Mean log-jump size.
        jump_mean: f64,
        /// Volatility of log-jump size.
        jump_vol: f64,
    },
    /// CreditGrades model extension with stochastic-barrier survival adjustment.
    ///
    /// `default_probability()` applies the standard approximate CreditGrades
    /// survival function using the log-barrier volatility parameter.
    CreditGrades {
        /// Log-normal barrier volatility `λ`: the standard deviation of the
        /// natural log of the default barrier (Finger et al. 2002,
        /// "CreditGrades Technical Document"). Despite the field name, this
        /// is *not* a generic uncertainty scalar — it is the lognormal
        /// dispersion of the global recovery rate, entering the survival
        /// formula as `a_t² = σ²t + λ²` and the barrier shift `exp(λ²)`.
        barrier_uncertainty: f64,
        /// Mean recovery rate at default.
        mean_recovery: f64,
    },
}

impl AssetDynamics {
    /// Reject parameter values that fall outside the process's domain.
    ///
    /// Every construction path runs this, so a model can never hold a
    /// negative jump intensity, a negative log-barrier volatility, or a
    /// recovery rate outside `[0, 1]`. Without it those values would be
    /// silently sanitized (or produce a wrong drift compensator) far from
    /// the point where they entered the system.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`] naming the offending field when a jump
    /// parameter is non-finite or negative, when `barrier_uncertainty` is
    /// non-finite or negative, or when `mean_recovery` is outside `[0, 1]`.
    fn validate(&self) -> Result<()> {
        match *self {
            Self::GeometricBrownian => Ok(()),
            Self::JumpDiffusion {
                jump_intensity,
                jump_mean,
                jump_vol,
            } => {
                if !(jump_intensity.is_finite() && jump_intensity >= 0.0) {
                    return Err(Error::Validation(format!(
                        "AssetDynamics::JumpDiffusion: jump_intensity must be finite and >= 0, \
                         got {jump_intensity}"
                    )));
                }
                if !jump_mean.is_finite() {
                    return Err(Error::Validation(format!(
                        "AssetDynamics::JumpDiffusion: jump_mean must be finite, got {jump_mean}"
                    )));
                }
                if !(jump_vol.is_finite() && jump_vol >= 0.0) {
                    return Err(Error::Validation(format!(
                        "AssetDynamics::JumpDiffusion: jump_vol must be finite and >= 0, \
                         got {jump_vol}"
                    )));
                }
                Ok(())
            }
            Self::CreditGrades {
                barrier_uncertainty,
                mean_recovery,
            } => {
                if !(barrier_uncertainty.is_finite() && barrier_uncertainty >= 0.0) {
                    return Err(Error::Validation(format!(
                        "AssetDynamics::CreditGrades: barrier_uncertainty (log-barrier vol λ) \
                         must be finite and >= 0, got {barrier_uncertainty}"
                    )));
                }
                if !(0.0..=1.0).contains(&mean_recovery) {
                    return Err(Error::Validation(format!(
                        "AssetDynamics::CreditGrades: mean_recovery must be in [0, 1], \
                         got {mean_recovery}"
                    )));
                }
                Ok(())
            }
        }
    }
}

/// Barrier monitoring type for default determination.
#[derive(
    Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
// Distinct from the barrier-option `finstack_quant_core::types::BarrierType`.
#[schemars(rename = "MertonBarrierType")]
pub enum BarrierType {
    /// Default only assessed at maturity (classic Merton).
    Terminal,
    /// Continuous barrier monitoring (Black-Cox extension).
    FirstPassage {
        /// Growth rate of the default barrier over time.
        barrier_growth_rate: f64,
    },
}

/// Approximate CreditGrades survival probability (Finger et al. 2002).
///
/// `barrier_uncertainty` is the log-barrier volatility `λ` — the standard
/// deviation of the natural log of the default barrier, *not* a generic
/// uncertainty scalar. It enters the time-scaled variance as
/// `a_t² = σ²t + λ²` and shifts the effective leverage by `exp(λ²)`.
///
/// The CreditGrades asset process is **driftless** by construction, so
/// `risk_free_rate` and `payout_rate` play no part here.
///
/// `barrier_uncertainty` is guaranteed non-negative by
/// [`AssetDynamics::validate`], which every construction path runs.
fn credit_grades_default_probability(
    asset_value: f64,
    asset_vol: f64,
    debt_barrier: f64,
    barrier_uncertainty: f64,
    horizon: f64,
) -> f64 {
    if horizon <= 0.0 {
        return 0.0;
    }

    // `lambda` is the log-barrier volatility (lognormal barrier std dev).
    let lambda = barrier_uncertainty;
    let a_t = (asset_vol.mul_add(asset_vol, lambda * lambda / horizon) * horizon).sqrt();
    if a_t <= 0.0 {
        return if asset_value <= debt_barrier {
            1.0
        } else {
            0.0
        };
    }

    let d = (asset_value / debt_barrier) * (lambda * lambda).exp();
    let ln_d = d.ln();
    let survival = norm_cdf(-0.5 * a_t + ln_d / a_t) - d * norm_cdf(-0.5 * a_t - ln_d / a_t);
    (1.0 - survival).clamp(0.0, 1.0)
}

/// Premium payments per year on a standard CDS contract (quarterly, ISDA).
const CDS_PREMIUM_PERIODS_PER_YEAR: f64 = 4.0;

/// Scale factor converting an ACT/365F year fraction into an ACT/360 premium
/// accrual. CDS premium legs accrue ACT/360 while survival and discount times
/// are measured ACT/365F, so the two axes differ by this ratio.
const ACT365F_TO_ACT360: f64 = 365.0 / 360.0;

/// Base date used for the throwaway survival curves built during CDS
/// calibration. The curves are only ever queried by year fraction, so the
/// anchor is immaterial; it exists because `HazardCurveBuilder` rejects the
/// 1970 sentinel to stop callers from accidentally anchoring real curves
/// there.
const CDS_CALIBRATION_ANCHOR: time::Date = time::macros::date!(2000 - 01 - 01);

/// ISDA-style CDS par spread implied by a survival curve.
///
/// Prices both legs on a uniform quarterly premium grid running to
/// `maturity`, using a flat continuously compounded discount rate:
///
/// - **Protection leg**: `(1 - R) * Σ DF(t_mid) * [S(t_{i-1}) - S(t_i)]`,
///   discounting each period's default mass at the period midpoint.
/// - **Premium leg (risky annuity)**: `Σ Δ * DF(t_i) * S(t_i)` plus the
///   standard half-period accrual-on-default term
///   `0.5 * Δ * DF(t_mid) * [S(t_{i-1}) - S(t_i)]`.
///
/// The par spread is the ratio, so the two legs balance at inception. This is
/// the O'Kane (2008) discretization of the ISDA Standard Model on a flat
/// curve; it deliberately does not model IMM roll dates, holiday calendars,
/// or settlement lags, because the structural model it serves is expressed
/// purely in year fractions.
///
/// # Arguments
///
/// * `hazard` - Survival curve to price against; its `recovery_rate()`
///   supplies the loss given default and its `sp(t)` the survival
///   probabilities, both keyed by year fraction from the curve base date
/// * `risk_free_rate` - Flat continuously compounded discount rate as a
///   decimal fraction
/// * `maturity` - Contract maturity in years; must be finite and strictly
///   positive
///
/// # Errors
///
/// Returns [`Error::Validation`] if `maturity` is not finite and positive, or
/// if the risky annuity collapses to zero (survival has already decayed to
/// nothing), which would make the par spread infinite.
fn par_spread_from_survival(
    hazard: &HazardCurve,
    risk_free_rate: f64,
    maturity: f64,
) -> Result<f64> {
    if !(maturity.is_finite() && maturity > 0.0) {
        return Err(Error::Validation(format!(
            "par_spread_from_survival: maturity must be > 0, got {maturity}"
        )));
    }
    let periods = (maturity * CDS_PREMIUM_PERIODS_PER_YEAR).round().max(1.0);
    let dt = maturity / periods;
    let accrual = dt * ACT365F_TO_ACT360;
    let lgd = 1.0 - hazard.recovery_rate();

    let mut protection = 0.0;
    let mut annuity = 0.0;
    let mut prev_survival = 1.0;
    for i in 1..=(periods as usize) {
        let t = i as f64 * dt;
        let survival = hazard.sp(t);
        let default_mass = (prev_survival - survival).max(0.0);
        let df_end = (-risk_free_rate * t).exp();
        let df_mid = (-risk_free_rate * (t - 0.5 * dt)).exp();

        protection += df_mid * default_mass;
        annuity += accrual * df_end * survival;
        annuity += 0.5 * accrual * df_mid * default_mass;

        prev_survival = survival;
    }

    if !(annuity.is_finite() && annuity > 0.0) {
        return Err(Error::Validation(format!(
            "par_spread_from_survival: risky annuity must be > 0, got {annuity}; \
             survival has decayed to zero over the contract life"
        )));
    }
    Ok(lgd * protection / annuity)
}

/// Merton structural credit model.
///
/// Models a firm's equity as a call option on its assets, where default
/// occurs when asset value falls below the debt barrier.
///
/// # Fields
///
/// - `asset_value` (V_0): Current market value of the firm's assets.
/// - `asset_vol` (sigma_V): Annualized volatility of asset returns.
/// - `debt_barrier` (B): Face value of debt / default point.
/// - `risk_free_rate` (r): Continuous risk-free rate.
/// - `payout_rate` (q): Continuous dividend / payout yield on assets.
/// - `barrier_type`: Terminal or first-passage barrier monitoring.
/// - `dynamics`: Asset return dynamics specification.
///
/// # Wire format
///
/// Deserialization is routed through [`MertonModel::new_with_dynamics`] via
/// [`RawMertonModel`], so a model loaded from JSON satisfies exactly the same
/// invariants as one built in Rust. The serialized field set is unchanged.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(try_from = "RawMertonModel")]
pub struct MertonModel {
    /// Current firm asset value `V_0`, in the issuer's reporting currency.
    /// Strictly positive.
    asset_value: f64,
    /// Asset volatility `sigma_V`, annualized and expressed as a decimal
    /// fraction (`0.25` is 25%). Strictly positive.
    asset_vol: f64,
    /// Default barrier `B`, the debt face value the asset value is compared
    /// against. Strictly positive and in the same currency as `asset_value`.
    debt_barrier: f64,
    /// Continuously compounded risk-free rate `r`, as a decimal fraction.
    risk_free_rate: f64,
    /// Continuous payout (dividend) rate on assets, as a decimal fraction.
    /// Reduces the drift of the asset process under the risk-neutral measure.
    payout_rate: f64,
    /// Whether default is tested only at maturity or continuously over the
    /// life of the debt.
    barrier_type: BarrierType,
    /// Stochastic process governing the asset value.
    dynamics: AssetDynamics,
}

/// Unvalidated wire representation of a [`MertonModel`].
///
/// Exists solely so `#[serde(try_from = ...)]` can funnel deserialization
/// through [`MertonModel::new_with_dynamics`]. Field names and types mirror
/// [`MertonModel`] exactly, so the JSON representation is identical.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "MertonModel")]
pub struct RawMertonModel {
    /// Current firm asset value `V_0`, in the issuer's reporting currency.
    /// Strictly positive.
    pub asset_value: f64,
    /// Asset volatility `sigma_V`, annualized and expressed as a decimal
    /// fraction (`0.25` is 25%). Strictly positive.
    pub asset_vol: f64,
    /// Default barrier `B`, the debt face value the asset value is compared
    /// against. Strictly positive and in the same currency as `asset_value`.
    pub debt_barrier: f64,
    /// Continuously compounded risk-free rate `r`, as a decimal fraction.
    pub risk_free_rate: f64,
    /// Continuous payout (dividend) rate on assets, as a decimal fraction.
    /// Reduces the drift of the asset process under the risk-neutral measure.
    pub payout_rate: f64,
    /// Whether default is tested only at maturity or continuously over the
    /// life of the debt.
    pub barrier_type: BarrierType,
    /// Stochastic process governing the asset value.
    pub dynamics: AssetDynamics,
}

impl TryFrom<RawMertonModel> for MertonModel {
    type Error = Error;

    fn try_from(raw: RawMertonModel) -> Result<Self> {
        Self::new_with_dynamics(
            raw.asset_value,
            raw.asset_vol,
            raw.debt_barrier,
            raw.risk_free_rate,
            raw.payout_rate,
            raw.barrier_type,
            raw.dynamics,
        )
    }
}

impl MertonModel {
    /// Create a new Merton model with GBM dynamics and terminal barrier.
    ///
    /// # Arguments
    ///
    /// * `asset_value` - Current asset value V_0 (must be > 0)
    /// * `asset_vol` - Asset volatility sigma_V (must be > 0)
    /// * `debt_barrier` - Debt face value B (must be > 0)
    /// * `risk_free_rate` - Risk-free rate r
    ///
    /// # Errors
    ///
    /// Returns [`InputError::NonPositiveValue`] if `asset_value`, `asset_vol`,
    /// or `debt_barrier` are non-positive.
    pub fn new(
        asset_value: f64,
        asset_vol: f64,
        debt_barrier: f64,
        risk_free_rate: f64,
    ) -> Result<Self> {
        Self::new_with_dynamics(
            asset_value,
            asset_vol,
            debt_barrier,
            risk_free_rate,
            0.0,
            BarrierType::Terminal,
            AssetDynamics::GeometricBrownian,
        )
    }

    /// Create a new Merton model with full parameterisation.
    ///
    /// # Arguments
    ///
    /// * `asset_value` - Current asset value V_0 (must be > 0)
    /// * `asset_vol` - Asset volatility sigma_V (must be > 0)
    /// * `debt_barrier` - Debt face value B (must be > 0)
    /// * `risk_free_rate` - Risk-free rate r
    /// * `payout_rate` - Dividend / payout yield q
    /// * `barrier_type` - Terminal or first-passage
    /// * `dynamics` - Asset return dynamics
    ///
    /// # `V <= B` (in-default state)
    ///
    /// `asset_value <= debt_barrier` is intentionally accepted: it represents
    /// a firm at or through its default point (distressed names). Pricing
    /// then degenerates consistently — first-passage paths default
    /// immediately, terminal-barrier default probabilities approach 1, and
    /// the CreditGrades survival formula returns PD = 1 in the
    /// zero-variance limit. Callers wanting a strictly solvent firm should
    /// validate `asset_value > debt_barrier` themselves.
    ///
    /// # Dynamics and barrier compatibility
    ///
    /// Not every pairing has a closed-form default probability, so the
    /// unsupported combinations are rejected here rather than silently
    /// falling back to a different process:
    ///
    /// - `JumpDiffusion` requires `BarrierType::Terminal`. First passage of a
    ///   jump-diffusion to a barrier has no elementary closed form.
    /// - `CreditGrades` requires `BarrierType::FirstPassage` with a zero
    ///   growth rate. The CreditGrades survival function *is* a first-passage
    ///   law with a stochastic flat barrier, so any other pairing would
    ///   describe a process the model does not evaluate.
    ///
    /// # Errors
    ///
    /// Returns [`InputError::NonPositiveValue`] if `asset_value`, `asset_vol`,
    /// or `debt_barrier` are non-positive, and [`Error::Validation`] if
    /// `dynamics` carries an out-of-domain parameter (a negative jump
    /// intensity or barrier uncertainty, or a `mean_recovery` outside
    /// `[0, 1]`) or is paired with an incompatible `barrier_type`.
    pub fn new_with_dynamics(
        asset_value: f64,
        asset_vol: f64,
        debt_barrier: f64,
        risk_free_rate: f64,
        payout_rate: f64,
        barrier_type: BarrierType,
        dynamics: AssetDynamics,
    ) -> Result<Self> {
        if !(asset_value.is_finite() && asset_value > 0.0) {
            return Err(InputError::NonPositiveValue.into());
        }
        if !(asset_vol.is_finite() && asset_vol > 0.0) {
            return Err(InputError::NonPositiveValue.into());
        }
        if !(debt_barrier.is_finite() && debt_barrier > 0.0) {
            return Err(InputError::NonPositiveValue.into());
        }
        if !risk_free_rate.is_finite() {
            return Err(Error::Validation(format!(
                "MertonModel: risk_free_rate must be finite, got {risk_free_rate}"
            )));
        }
        if !payout_rate.is_finite() {
            return Err(Error::Validation(format!(
                "MertonModel: payout_rate must be finite, got {payout_rate}"
            )));
        }
        if let BarrierType::FirstPassage {
            barrier_growth_rate,
        } = barrier_type
        {
            if !barrier_growth_rate.is_finite() {
                return Err(Error::Validation(format!(
                    "MertonModel: barrier_growth_rate must be finite, got {barrier_growth_rate}"
                )));
            }
        }
        dynamics.validate()?;
        match (&dynamics, &barrier_type) {
            (AssetDynamics::JumpDiffusion { .. }, BarrierType::FirstPassage { .. }) => {
                return Err(Error::Validation(
                    "MertonModel: JumpDiffusion dynamics require BarrierType::Terminal; \
                     first passage of a jump-diffusion has no closed-form default \
                     probability. Use Monte Carlo for pathwise first-passage default."
                        .to_string(),
                ));
            }
            (AssetDynamics::CreditGrades { .. }, BarrierType::Terminal) => {
                return Err(Error::Validation(
                    "MertonModel: CreditGrades dynamics require \
                     BarrierType::FirstPassage { barrier_growth_rate: 0.0 }; the \
                     CreditGrades survival function is a first-passage law with a \
                     stochastic flat barrier."
                        .to_string(),
                ));
            }
            (
                AssetDynamics::CreditGrades { .. },
                BarrierType::FirstPassage {
                    barrier_growth_rate,
                },
            ) if *barrier_growth_rate != 0.0 => {
                return Err(Error::Validation(format!(
                    "MertonModel: CreditGrades dynamics require a zero barrier_growth_rate \
                     (the CreditGrades barrier is flat in expectation), got \
                     {barrier_growth_rate}"
                )));
            }
            _ => {}
        }
        Ok(Self {
            asset_value,
            asset_vol,
            debt_barrier,
            risk_free_rate,
            payout_rate,
            barrier_type,
            dynamics,
        })
    }

    /// Diffusion log-drift of `ln(V_t)` for a caller-supplied total asset
    /// return.
    ///
    /// `total_return` is `r` under the risk-neutral measure and the expected
    /// physical asset return `mu` under the real-world measure. Under
    /// jump-diffusion the Poisson compensator `-lambda * kappa` is subtracted
    /// so that `E[V_T] = V_0 * exp((total_return - q) * T)` in both measures.
    #[inline]
    fn log_drift(&self, total_return: f64) -> f64 {
        let sigma = self.asset_vol;
        let base = total_return - self.payout_rate - 0.5 * sigma * sigma;
        match self.dynamics {
            AssetDynamics::JumpDiffusion {
                jump_intensity,
                jump_mean,
                jump_vol,
            } => {
                let kappa = (jump_mean + 0.5 * jump_vol * jump_vol).exp() - 1.0;
                base - jump_intensity * kappa
            }
            AssetDynamics::GeometricBrownian | AssetDynamics::CreditGrades { .. } => base,
        }
    }

    /// Poisson-mixture decomposition of `ln(V_T / V_0)` at the terminal date.
    ///
    /// Returns `(weight, mean, variance)` triples such that, conditional on
    /// component `n`, `ln(V_T/V_0) ~ Normal(mean_n, variance_n)`. Geometric
    /// Brownian motion yields the single component
    /// `(1, mu*T, sigma^2*T)`; Merton (1976) jump-diffusion yields the
    /// Poisson mixture `w_n = e^{-lambda T}(lambda T)^n / n!` with
    /// `mean_n = mu*T + n*mu_J` and `variance_n = sigma^2*T + n*sigma_J^2`.
    ///
    /// The series is truncated once the remaining Poisson mass is
    /// negligible, so the weights sum to 1 within `f64` precision.
    fn terminal_log_components(&self, log_drift: f64, horizon: f64) -> Vec<(f64, f64, f64)> {
        let sigma = self.asset_vol;
        let diffusion_var = sigma * sigma * horizon;
        let AssetDynamics::JumpDiffusion {
            jump_intensity,
            jump_mean,
            jump_vol,
        } = self.dynamics
        else {
            return vec![(1.0, log_drift * horizon, diffusion_var)];
        };

        let lambda_t = jump_intensity * horizon;
        // Poisson tail beyond mean + 10 standard deviations carries less than
        // 1e-15 of the mass, which is below the accuracy of `norm_cdf`.
        let n_max = (lambda_t + 10.0 * lambda_t.sqrt()).ceil().max(20.0) as usize;
        let mut components = Vec::with_capacity(n_max + 1);
        let mut weight = (-lambda_t).exp();
        for n in 0..=n_max {
            if n > 0 {
                weight *= lambda_t / n as f64;
            }
            components.push((
                weight,
                log_drift.mul_add(horizon, n as f64 * jump_mean),
                jump_vol.mul_add(jump_vol * n as f64, diffusion_var),
            ));
        }
        components
    }

    /// Terminal-barrier default probability for a given diffusion log-drift.
    fn terminal_pd(&self, log_drift: f64, horizon: f64) -> f64 {
        let log_moneyness = (self.asset_value / self.debt_barrier).ln();
        self.terminal_log_components(log_drift, horizon)
            .into_iter()
            .map(|(weight, mean, variance)| {
                weight * norm_cdf(-(log_moneyness + mean) / variance.sqrt())
            })
            .sum()
    }

    /// Black-Cox first-passage default probability for a given diffusion
    /// log-drift and barrier growth rate.
    fn first_passage_pd(&self, log_drift: f64, barrier_growth_rate: f64, horizon: f64) -> f64 {
        let sigma = self.asset_vol;
        let sigma_sqrt_t = sigma * horizon.sqrt();

        // Reduce to a flat barrier at 0: the distance process
        // X_t = ln(V_t / (B e^{g t})) is BM with drift nu = mu - g started at
        // x0 = ln(V/B). The growing barrier shifts the *drift*, not the
        // starting distance.
        let nu = log_drift - barrier_growth_rate;
        let x0 = (self.asset_value / self.debt_barrier).ln();

        let d_plus = (x0 + nu * horizon) / sigma_sqrt_t;
        let d_minus = (x0 - nu * horizon) / sigma_sqrt_t;

        // Black-Cox reflection term `exp(-2*nu*x0/sigma^2) * N(-d_minus)`.
        // The exponential factor overflows to `+inf` for a large
        // `|exponent|` (e.g. a strongly negative drift, or a low vol with a
        // high rate). When `N(-d_minus)` simultaneously underflows to `0` the
        // naive product is `inf * 0 = NaN`, which would survive the final
        // `clamp(0, 1)`.
        //
        // The Gaussian tail `N(-d_minus)` decays as `exp(-d_minus^2/2)`,
        // which dominates the (at most exponential-in-`d_minus`) power
        // factor, so the term tends to `0` whenever `N(-d_minus)` does.
        // Guard that case, then evaluate the surviving product in log-space
        // so a genuinely large term overflows cleanly to `+inf` (and clamps
        // to `1`) instead of producing a `NaN`.
        let exponent = -2.0 * nu / (sigma * sigma);
        let nd_minus = norm_cdf(-d_minus);
        let reflection_term = if nd_minus <= 0.0 {
            0.0
        } else {
            (exponent * x0 + nd_minus.ln()).exp()
        };

        let pd = norm_cdf(-d_plus) + reflection_term;
        if pd.is_nan() {
            return 0.0;
        }
        pd.clamp(0.0, 1.0)
    }

    /// Default probability for a given diffusion log-drift, dispatching on
    /// the barrier type. Not valid for `CreditGrades` dynamics, which are
    /// driftless and carry their own survival function.
    fn pd_from_log_drift(&self, log_drift: f64, horizon: f64) -> f64 {
        if horizon <= 0.0 {
            return 0.0;
        }
        match self.barrier_type {
            BarrierType::Terminal => self.terminal_pd(log_drift, horizon),
            BarrierType::FirstPassage {
                barrier_growth_rate,
            } => self.first_passage_pd(log_drift, barrier_growth_rate, horizon),
        }
    }

    /// Risk-neutral distance-to-default over the given horizon.
    ///
    /// DD = (ln(V/B) + (r - q - sigma^2/2) * T) / (sigma * sqrt(T))
    ///
    /// A higher DD indicates a lower probability of default.
    /// Returns `f64::INFINITY` when `horizon <= 0` (no time = infinite DD,
    /// yielding PD = N(-∞) = 0).
    ///
    /// # Measure
    ///
    /// This is the **risk-neutral (Q-measure)** `d2`, driven by the risk-free
    /// rate. It is *not* the Moody's KMV distance-to-default, which uses the
    /// firm's expected physical asset return and a default point of
    /// short-term debt plus half of long-term debt. Use
    /// [`Self::distance_to_default_with_drift`] together with
    /// [`Self::kmv_default_point`] for the KMV/EDF quantity.
    ///
    /// # Dynamics
    ///
    /// DD is a pure-diffusion statistic. Under `JumpDiffusion` the drift
    /// includes the Poisson compensator, so `N(-DD)` is the zero-jump term of
    /// the default probability rather than the default probability itself;
    /// call [`Self::default_probability`] for that. Under `CreditGrades` the
    /// survival law is driftless and does not factor through DD at all, so
    /// `N(-DD)` again differs from [`Self::default_probability`].
    ///
    /// # Arguments
    ///
    /// * `horizon` - Time horizon T in years from the valuation date; a
    ///   non-positive horizon returns `+∞`
    #[inline]
    pub fn distance_to_default(&self, horizon: f64) -> f64 {
        if horizon <= 0.0 {
            return f64::INFINITY;
        }
        let sqrt_t = horizon.sqrt();
        ((self.asset_value / self.debt_barrier).ln()
            + self.log_drift(self.risk_free_rate) * horizon)
            / (self.asset_vol * sqrt_t)
    }

    /// Physical-measure (Moody's KMV) distance-to-default over the given
    /// horizon.
    ///
    /// DD = (ln(V/B) + (mu - q - sigma^2/2) * T) / (sigma * sqrt(T))
    ///
    /// This is the KMV/EDF construction: the risk-free rate is replaced by
    /// the firm's expected physical asset return, so the result measures how
    /// many standard deviations of one-year asset return separate the firm
    /// from its default point under the real-world measure. Pair it with
    /// [`Self::kmv_default_point`] to reproduce the KMV default point
    /// convention when building the model.
    ///
    /// # Arguments
    ///
    /// * `asset_drift` - Expected **physical** total return on the firm's
    ///   assets `mu`, continuously compounded and expressed as a decimal
    ///   fraction (`0.09` is 9% per annum). This replaces the risk-free rate
    ///   in the drift; the model's `payout_rate` is still subtracted.
    /// * `horizon` - Time horizon T in years from the valuation date; a
    ///   non-positive horizon returns `+∞`
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`] if `asset_drift` is not finite, or if
    /// the model uses `CreditGrades` dynamics, whose survival function is
    /// driftless by construction and therefore has no physical-measure
    /// counterpart.
    pub fn distance_to_default_with_drift(&self, asset_drift: f64, horizon: f64) -> Result<f64> {
        self.check_drift_supported(asset_drift, "distance_to_default_with_drift")?;
        if horizon <= 0.0 {
            return Ok(f64::INFINITY);
        }
        let sqrt_t = horizon.sqrt();
        Ok(
            ((self.asset_value / self.debt_barrier).ln() + self.log_drift(asset_drift) * horizon)
                / (self.asset_vol * sqrt_t),
        )
    }

    /// Reject drift-parameterised queries the configured dynamics cannot
    /// answer.
    fn check_drift_supported(&self, asset_drift: f64, method: &str) -> Result<()> {
        if !asset_drift.is_finite() {
            return Err(Error::Validation(format!(
                "{method}: asset_drift must be finite, got {asset_drift}"
            )));
        }
        if matches!(self.dynamics, AssetDynamics::CreditGrades { .. }) {
            return Err(Error::Validation(format!(
                "{method}: CreditGrades dynamics are driftless by construction \
                 (Finger et al. 2002), so no physical-measure distance-to-default \
                 or EDF is defined. Use GeometricBrownian dynamics for KMV/EDF work."
            )));
        }
        Ok(())
    }

    /// Risk-neutral default probability over the given horizon.
    ///
    /// # Measure
    ///
    /// This is the **risk-neutral (Q-measure)** default probability: the
    /// drift is the risk-free `r − q − σ²/2`, not the firm's real-world
    /// asset drift. It is the right quantity for pricing and credit-spread
    /// work ([`Self::implied_spread`], [`Self::cds_par_spread`],
    /// [`Self::to_hazard_curve`]), but it materially **overstates** the
    /// physical/real-world PD (EDF) whenever the market price of asset risk
    /// is positive — often by several times for a healthy firm. For
    /// expected-loss, capital, or rating analytics use
    /// [`Self::default_probability_with_drift`], which substitutes the
    /// physical asset drift.
    ///
    /// - **Terminal barrier, `GeometricBrownian`**: PD = N(-DD) (Merton 1974).
    /// - **Terminal barrier, `JumpDiffusion`**: the Merton (1976) Poisson
    ///   mixture. Conditional on `n` jumps the terminal log-asset value is
    ///   Gaussian with mean shifted by `n * mu_J` and variance inflated by
    ///   `n * sigma_J^2`, so
    ///
    ///   PD = Σ_n e^{-λT}(λT)^n/n! · N(-(ln(V/B) + (r-q-λκ-σ²/2)T + n·mu_J) / sqrt(σ²T + n·sigma_J²))
    ///
    ///   with `κ = exp(mu_J + sigma_J²/2) - 1`. `JumpDiffusion` is rejected
    ///   with a first-passage barrier at construction time.
    /// - **First-passage barrier**: Black-Cox (1976) closed-form with
    ///   exponentially growing barrier `B(t) = B * exp(g * t)`. The distance
    ///   process `X_t = ln(V_t / B(t))` is Brownian motion with drift
    ///   `nu = mu - g` (where `mu = r - q - sigma^2/2`) started at
    ///   `x0 = ln(V/B)`, and the first-passage probability to 0 is
    ///
    ///   PD = N(-(x0 + nu*T) / (sigma*sqrt(T)))
    ///      + exp(-2*nu*x0 / sigma^2) * N((-x0 + nu*T) / (sigma*sqrt(T)))
    ///
    ///   The first-passage result is clamped to `[0, 1]` to absorb
    ///   floating-point overshoot when the drift is strongly negative.
    /// - **`CreditGrades`**: the Finger et al. (2002) approximate survival
    ///   function, which is driftless and therefore ignores `r` and `q`.
    ///
    /// # Arguments
    ///
    /// * `horizon` - Time horizon T in years from the valuation date; a
    ///   non-positive horizon returns 0
    pub fn default_probability(&self, horizon: f64) -> f64 {
        if let AssetDynamics::CreditGrades {
            barrier_uncertainty,
            ..
        } = self.dynamics
        {
            return credit_grades_default_probability(
                self.asset_value,
                self.asset_vol,
                self.debt_barrier,
                barrier_uncertainty,
                horizon,
            );
        }
        self.pd_from_log_drift(self.log_drift(self.risk_free_rate), horizon)
    }

    /// Physical-measure default probability (theoretical EDF) over the given
    /// horizon.
    ///
    /// Identical dispatch to [`Self::default_probability`], with the firm's
    /// expected physical asset return substituted for the risk-free rate.
    /// This is the "theoretical EDF" of the Moody's KMV framework: the
    /// real-world probability that the asset value ends below (or, with a
    /// first-passage barrier, ever touches) the default point. Moody's
    /// published EDF applies a further proprietary empirical mapping from
    /// distance-to-default to observed default frequency, which is not
    /// reproduced here.
    ///
    /// # Arguments
    ///
    /// * `asset_drift` - Expected **physical** total return on the firm's
    ///   assets `mu`, continuously compounded and expressed as a decimal
    ///   fraction (`0.09` is 9% per annum). Replaces the risk-free rate in
    ///   the drift; the model's `payout_rate` is still subtracted, and under
    ///   `JumpDiffusion` the Poisson compensator still applies.
    /// * `horizon` - Time horizon T in years from the valuation date; a
    ///   non-positive horizon returns 0
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`] if `asset_drift` is not finite, or if
    /// the model uses `CreditGrades` dynamics, whose survival function is
    /// driftless by construction.
    pub fn default_probability_with_drift(&self, asset_drift: f64, horizon: f64) -> Result<f64> {
        self.check_drift_supported(asset_drift, "default_probability_with_drift")?;
        Ok(self.pd_from_log_drift(self.log_drift(asset_drift), horizon))
    }

    /// Moody's KMV default point: short-term debt plus half of long-term
    /// debt.
    ///
    /// The KMV framework does not use total liabilities as the default
    /// barrier. Empirically firms default when asset value falls to roughly
    /// current liabilities plus half of long-term liabilities, because
    /// long-dated debt does not have to be repaid immediately. Feed the
    /// result in as `debt_barrier` when building a model for KMV/EDF work.
    ///
    /// # Arguments
    ///
    /// * `short_term_debt` - Book value of debt and other liabilities due
    ///   within one year, in the issuer's reporting currency. Must be finite
    ///   and non-negative.
    /// * `long_term_debt` - Book value of debt maturing beyond one year, in
    ///   the same currency. Must be finite and non-negative; exactly half of
    ///   it enters the default point.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`] if either input is non-finite or
    /// negative, or if the resulting default point is zero (a firm with no
    /// liabilities has no default point).
    pub fn kmv_default_point(short_term_debt: f64, long_term_debt: f64) -> Result<f64> {
        if !(short_term_debt.is_finite() && short_term_debt >= 0.0) {
            return Err(Error::Validation(format!(
                "kmv_default_point: short_term_debt must be finite and >= 0, \
                 got {short_term_debt}"
            )));
        }
        if !(long_term_debt.is_finite() && long_term_debt >= 0.0) {
            return Err(Error::Validation(format!(
                "kmv_default_point: long_term_debt must be finite and >= 0, \
                 got {long_term_debt}"
            )));
        }
        let default_point = 0.5f64.mul_add(long_term_debt, short_term_debt);
        if default_point <= 0.0 {
            return Err(Error::Validation(
                "kmv_default_point: short_term_debt + 0.5 * long_term_debt must be > 0; \
                 a firm with no liabilities has no default point"
                    .to_string(),
            ));
        }
        Ok(default_point)
    }

    /// Zero-coupon bond credit spread with an exogenous recovery rate.
    ///
    /// s = -ln(1 - PD * (1 - R)) / T
    ///
    /// # Convention
    ///
    /// This is the **continuously compounded zero-coupon spread** of a risky
    /// discount bond whose recovery `R` is a fixed fraction of face value
    /// **paid at maturity**: `price = e^{-rT}(1 - PD·LGD)`, hence the formula
    /// above. It is deliberately not:
    ///
    /// - Merton's *endogenous* debt spread, where recovery is the firm's own
    ///   terminal asset value — see [`Self::debt_spread`], which can differ
    ///   by a factor of two or more for the same model;
    /// - a CDS par spread, which has a premium leg, accrual on default, and
    ///   discounting of the protection payment at the default time — see
    ///   [`Self::cds_par_spread`]. The two agree only to first order in PD
    ///   and diverge by roughly 7% at a 30% cumulative default probability.
    ///
    /// With a first-passage barrier the underlying PD refers to a default
    /// that can occur at any time before `T`, while this formula still
    /// assumes recovery is paid at `T`; that understates the present value of
    /// recovery. Use [`Self::cds_par_spread`] when default timing matters.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`] if `horizon <= 0` (the spread is
    /// undefined at zero horizon) or `recovery` is outside `[0, 1]` (an
    /// out-of-range recovery can make `1 - PD·LGD` non-positive, yielding
    /// NaN).
    ///
    /// # Arguments
    ///
    /// * `horizon` - Bond maturity T in years from the valuation date; must
    ///   be finite and strictly positive
    /// * `recovery` - Recovery rate as a decimal fraction of face value
    ///   (`0.40` is the senior-unsecured market convention), assumed paid at
    ///   maturity; must lie in `[0, 1]`
    #[inline]
    pub fn implied_spread(&self, horizon: f64, recovery: f64) -> Result<f64> {
        if !(horizon.is_finite() && horizon > 0.0) {
            return Err(Error::Validation(format!(
                "implied_spread: horizon must be > 0, got {horizon}"
            )));
        }
        if !(0.0..=1.0).contains(&recovery) {
            return Err(Error::Validation(format!(
                "implied_spread: recovery must be in [0, 1], got {recovery}"
            )));
        }
        let pd = self.default_probability(horizon);
        let lgd = 1.0 - recovery;
        Ok(-(1.0 - pd * lgd).ln() / horizon)
    }

    /// Merton (1974) endogenous credit spread on the firm's zero-coupon debt.
    ///
    /// s = -ln(D / (B * e^{-rT})) / T
    ///
    /// where `D` is the model value of the firm's debt claim. Recovery is
    /// **endogenous**: debt holders receive `min(V_T, B)`, so the recovery
    /// rate is the firm's own terminal asset value rather than an assumed
    /// constant. Because equity and debt exhaust the firm,
    /// `D = V·e^{-qT} - E` where `E` is the equity call value, which is what
    /// this method evaluates (Poisson-mixed under `JumpDiffusion`).
    ///
    /// This is the "risk structure of interest rates" of Merton (1974) and is
    /// the model-consistent spread. It is typically well below
    /// [`Self::implied_spread`] with a 40% exogenous recovery, because a firm
    /// that defaults in the Merton model usually retains substantial asset
    /// value.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`] if `horizon <= 0`, if the barrier type
    /// is not `Terminal` (the Black-Cox and CreditGrades debt claims pay
    /// recovery at the first-passage time and are not valued here; use
    /// [`Self::cds_par_spread`]), or if the implied debt value is
    /// non-positive, which makes the log undefined.
    ///
    /// # Arguments
    ///
    /// * `horizon` - Maturity T of the firm's debt in years from the
    ///   valuation date; must be finite and strictly positive
    pub fn debt_spread(&self, horizon: f64) -> Result<f64> {
        if !(horizon.is_finite() && horizon > 0.0) {
            return Err(Error::Validation(format!(
                "debt_spread: horizon must be > 0, got {horizon}"
            )));
        }
        if !matches!(self.barrier_type, BarrierType::Terminal) {
            return Err(Error::Validation(
                "debt_spread: the endogenous Merton (1974) debt spread is defined for \
                 BarrierType::Terminal only; a first-passage claim pays recovery at the \
                 hitting time. Use cds_par_spread or implied_spread instead."
                    .to_string(),
            ));
        }
        let equity = self.terminal_equity_value(horizon);
        let debt_value = self.asset_value * (-self.payout_rate * horizon).exp() - equity;
        let risk_free_value = self.debt_barrier * (-self.risk_free_rate * horizon).exp();
        if !(debt_value.is_finite() && debt_value > 0.0) {
            return Err(Error::Validation(format!(
                "debt_spread: implied debt value must be > 0, got {debt_value}"
            )));
        }
        Ok(-(debt_value / risk_free_value).ln() / horizon)
    }

    /// Value of the equity claim `E[max(V_T - B, 0)]` discounted at `r`,
    /// under the terminal-barrier Poisson-mixture terminal distribution.
    ///
    /// Reduces to Black-Scholes for `GeometricBrownian` and to the Merton
    /// (1976) option series for `JumpDiffusion`.
    fn terminal_equity_value(&self, horizon: f64) -> f64 {
        let log_moneyness = (self.asset_value / self.debt_barrier).ln();
        let discount = (-self.risk_free_rate * horizon).exp();
        self.terminal_log_components(self.log_drift(self.risk_free_rate), horizon)
            .into_iter()
            .map(|(weight, mean, variance)| {
                let std_dev = variance.sqrt();
                let d2 = (log_moneyness + mean) / std_dev;
                let d1 = d2 + std_dev;
                let forward = self.asset_value * (mean + 0.5 * variance).exp();
                weight
                    * discount
                    * forward.mul_add(norm_cdf(d1), -(self.debt_barrier * norm_cdf(d2)))
            })
            .sum()
    }

    /// ISDA-style CDS par spread implied by the model's survival curve.
    ///
    /// The model's risk-neutral survival probabilities are exported to a
    /// [`HazardCurve`] on the quarterly premium grid — the same object
    /// [`Self::to_hazard_curve`] hands to downstream pricers — and both CDS
    /// legs are priced against it on a quarterly premium grid: a protection
    /// leg discounting each period's default mass at the period midpoint, and
    /// a risky annuity carrying the standard half-period accrual on default.
    /// Because the curve is built through the standard
    /// bootstrap, a model whose survival curve cannot produce a usable hazard
    /// curve fails here rather than silently returning a spread no pricer
    /// could reproduce.
    ///
    /// Prefer this over [`Self::implied_spread`] whenever the target is a
    /// quoted CDS level: the zero-coupon formula omits the premium leg,
    /// accrual on default, and discounting, and understates the par spread by
    /// roughly 7% at a 30% cumulative default probability.
    ///
    /// # Arguments
    ///
    /// * `maturity` - CDS maturity in years from the valuation date; must be
    ///   finite and strictly positive
    /// * `recovery` - Recovery rate as a decimal fraction of notional
    ///   (`0.40` is the senior-unsecured market convention); must lie in
    ///   `[0, 1]`, and for `CreditGrades` dynamics must equal the model's own
    ///   `mean_recovery`
    ///
    /// # Returns
    ///
    /// Par spread as a decimal fraction per annum (multiply by 10,000 for
    /// basis points).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`] if `maturity` is not positive, if
    /// `recovery` is outside `[0, 1]` or contradicts the model's
    /// `mean_recovery`, if the implied survival curve is non-monotonic, or if
    /// the risky annuity collapses to zero. Propagates
    /// [`HazardCurve`] builder errors, including the hazard-rate ceiling for
    /// a firm whose implied default probability is effectively 1.
    pub fn cds_par_spread(&self, maturity: f64, recovery: f64) -> Result<f64> {
        if !(maturity.is_finite() && maturity > 0.0) {
            return Err(Error::Validation(format!(
                "cds_par_spread: maturity must be > 0, got {maturity}"
            )));
        }
        let periods = (maturity * CDS_PREMIUM_PERIODS_PER_YEAR).round().max(1.0);
        let tenors: Vec<f64> = (1..=(periods as usize))
            .map(|i| i as f64 * maturity / periods)
            .collect();
        let hazard = self.to_hazard_curve(
            "MERTON-CDS-PAR",
            CDS_CALIBRATION_ANCHOR,
            &tenors,
            recovery,
            DayCount::Act365F,
        )?;
        par_spread_from_survival(&hazard, self.risk_free_rate, maturity)
    }

    // Calibration methods

    /// Minimum equity value **as a fraction of firm value** for which the
    /// equity-vol relation `sigma_E = N(d1) * exp(-qT) * sigma_V * V / E` is
    /// numerically well posed. Below this, `E` is treated as effectively zero
    /// (the firm is economically in default) and the division is rejected.
    ///
    /// The threshold is relative rather than absolute so that it behaves the
    /// same for a firm reporting in units and one reporting in billions.
    const MIN_EQUITY_FRACTION: f64 = 1.0e-10;

    /// Minimum `N(d1)` for which the KMV / equity-vol inversion is well posed.
    /// Below this, the equity is deep-out-of-the-money on the firm's assets
    /// and the inversion is numerically unstable.
    const MIN_ND1: f64 = 1.0e-12;

    /// Compute implied equity value and equity volatility from the structural
    /// model, rejecting numerically degenerate (near-default) configurations.
    ///
    /// Uses the Black-Scholes call option formula where equity is a call on
    /// the firm's assets with strike equal to the debt barrier, accounting
    /// for continuous payout rate q (Hull, 9th ed., Chapter 17):
    ///
    /// - d1 = (ln(V/B) + (r - q + sigma^2/2) * T) / (sigma * sqrt(T))
    /// - d2 = d1 - sigma * sqrt(T)
    /// - E = V * exp(-q*T) * N(d1) - B * exp(-r*T) * N(d2)
    /// - sigma_E = N(d1) * exp(-q*T) * sigma_V * V / E
    ///
    /// For a deeply distressed firm `E -> 0+`, so `sigma_E` would diverge to
    /// `+inf`. This method rejects such inputs up front with a descriptive
    /// error instead of returning `inf`/`NaN`.
    ///
    /// # Dynamics
    ///
    /// Diffusion-only. `JumpDiffusion` is rejected because the returned
    /// `sigma_E` is the delta-scaled *diffusive* equity volatility, which is
    /// not the observable equity volatility once jumps contribute variance;
    /// pairing them would silently corrupt a KMV inversion.
    ///
    /// # Arguments
    ///
    /// * `horizon` - Time horizon T in years from the valuation date over
    ///   which equity is treated as a call on the firm's assets; must be
    ///   finite and strictly positive
    ///
    /// # Returns
    ///
    /// A tuple `(equity_value, equity_vol)`, the first in the same currency
    /// as `asset_value` and the second an annualized decimal fraction.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`] if `horizon` is not positive or the
    /// model uses `JumpDiffusion` dynamics, and [`InputError::Invalid`] if
    /// the implied equity value or `N(d1)` is below the well-posed floor (the
    /// firm is economically in default).
    pub fn try_implied_equity(&self, horizon: f64) -> Result<(f64, f64)> {
        if !(horizon.is_finite() && horizon > 0.0) {
            return Err(Error::Validation(format!(
                "try_implied_equity: horizon must be > 0, got {horizon}"
            )));
        }
        if matches!(self.dynamics, AssetDynamics::JumpDiffusion { .. }) {
            return Err(Error::Validation(
                "try_implied_equity: the Black-Scholes equity-vol inversion is \
                 diffusion-only; under JumpDiffusion the delta-scaled volatility is \
                 the diffusive component alone and would misstate observed equity \
                 volatility. Use GeometricBrownian dynamics for equity calibration."
                    .to_string(),
            ));
        }
        let v = self.asset_value;
        let sigma = self.asset_vol;
        let b = self.debt_barrier;
        let r = self.risk_free_rate;
        let q = self.payout_rate;
        let sqrt_t = horizon.sqrt();

        let d1 = ((v / b).ln() + (r - q + 0.5 * sigma * sigma) * horizon) / (sigma * sqrt_t);
        let d2 = d1 - sigma * sqrt_t;

        let nd1 = norm_cdf(d1);
        let nd2 = norm_cdf(d2);

        let exp_neg_qt = (-q * horizon).exp();
        let equity = v * exp_neg_qt * nd1 - b * (-r * horizon).exp() * nd2;

        if !equity.is_finite() || equity <= Self::MIN_EQUITY_FRACTION * v || nd1 <= Self::MIN_ND1 {
            return Err(InputError::Invalid.into());
        }

        let equity_vol = nd1 * exp_neg_qt * sigma * v / equity;
        Ok((equity, equity_vol))
    }

    /// KMV calibration: recover asset value and asset volatility from observed
    /// equity value and equity volatility.
    ///
    /// Solves the 2x2 nonlinear system iteratively (fixed-point iteration),
    /// including the continuous payout rate q:
    ///
    /// - E = V * exp(-q*T) * N(d1) - B * exp(-r*T) * N(d2)
    /// - sigma_E * E = N(d1) * exp(-q*T) * sigma_V * V
    ///
    /// Convergence is typically fast (10-20 iterations).
    ///
    /// # Arguments
    ///
    /// * `equity_value` - Observed market capitalization E, strictly positive
    ///   and in the same currency as `total_debt`
    /// * `equity_vol` - Observed annualized equity volatility `sigma_E` as a
    ///   decimal fraction (`0.35` is 35%); must be non-negative
    /// * `total_debt` - Face value of debt B acting as the option strike, in
    ///   the same currency as `equity_value` and strictly positive. Use
    ///   [`Self::kmv_default_point`] for the Moody's KMV default-point
    ///   convention
    /// * `risk_free_rate` - Continuously compounded risk-free rate r as a
    ///   decimal fraction
    /// * `payout_rate` - Continuous dividend / payout yield q on assets as a
    ///   decimal fraction
    /// * `maturity` - Debt horizon T in years, conventionally 1.0 in KMV
    ///   practice; must be strictly positive
    ///
    /// # Errors
    ///
    /// Returns [`InputError::NonPositiveValue`] or [`InputError::NegativeValue`]
    /// for out-of-domain inputs, [`InputError::Invalid`] when equity is a
    /// negligible fraction of firm value or `N(d1)` collapses to zero
    /// (deep-out-of-the-money, economically in default), and
    /// [`InputError::SolverConvergenceFailed`] if the fixed point does not
    /// settle within 100 iterations.
    pub fn from_equity(
        equity_value: f64,
        equity_vol: f64,
        total_debt: f64,
        risk_free_rate: f64,
        payout_rate: f64,
        maturity: f64,
    ) -> Result<Self> {
        if equity_value <= 0.0 || total_debt <= 0.0 || maturity <= 0.0 {
            return Err(InputError::NonPositiveValue.into());
        }
        if equity_vol < 0.0 {
            return Err(InputError::NegativeValue.into());
        }
        // A near-zero equity value makes the KMV volatility inversion
        // `sigma_V = sigma_E * E / (N(d1) * exp(-qT) * V)` ill-conditioned and
        // can drive intermediate iterates to inf/NaN, silently defeating the
        // convergence test. Reject it up front with a descriptive error. The
        // test is relative to the initial firm-value estimate so it scales
        // with the reporting units.
        if equity_value <= Self::MIN_EQUITY_FRACTION * (equity_value + total_debt) {
            return Err(InputError::Invalid.into());
        }

        let e = equity_value;
        let sigma_e = equity_vol;
        let b = total_debt;
        let r = risk_free_rate;
        let q = payout_rate;
        let t = maturity;
        let sqrt_t = t.sqrt();
        let exp_neg_qt = (-q * t).exp();

        // Initial guesses
        let mut v = e + b;
        let mut sigma_v = sigma_e * e / v;

        let max_iter = 100;
        let tol = 1e-8;

        for _ in 0..max_iter {
            let v_prev = v;
            let sigma_v_prev = sigma_v;

            let d1 = ((v / b).ln() + (r - q + 0.5 * sigma_v * sigma_v) * t) / (sigma_v * sqrt_t);
            let d2 = d1 - sigma_v * sqrt_t;

            let nd1 = norm_cdf(d1);
            let nd2 = norm_cdf(d2);

            // Deep-OTM: N(d1) -> 0 makes the V / sigma_V updates blow up to
            // inf/NaN, which makes the relative-change test silently never
            // fire. Reject explicitly rather than burning all iterations.
            if nd1 <= Self::MIN_ND1 {
                return Err(InputError::Invalid.into());
            }

            // Update V from the call pricing equation: E = V*exp(-qT)*N(d1) - B*exp(-rT)*N(d2)
            v = (e + b * (-r * t).exp() * nd2) / (exp_neg_qt * nd1);
            // Update sigma_V from the volatility relation
            sigma_v = sigma_e * e / (nd1 * exp_neg_qt * v);

            // Both unknowns must settle. Testing V alone can exit while
            // sigma_V is still moving, because V is far less sensitive to
            // sigma_V than the reverse near the fixed point.
            let v_converged = ((v - v_prev) / v_prev).abs() < tol;
            let sigma_converged = ((sigma_v - sigma_v_prev) / sigma_v_prev).abs() < tol;
            if v_converged && sigma_converged {
                return Self::new_with_dynamics(
                    v,
                    sigma_v,
                    b,
                    r,
                    q,
                    BarrierType::Terminal,
                    AssetDynamics::GeometricBrownian,
                );
            }
        }

        Err(InputError::SolverConvergenceFailed {
            iterations: max_iter,
            residual: {
                let d1 =
                    ((v / b).ln() + (r - q + 0.5 * sigma_v * sigma_v) * t) / (sigma_v * sqrt_t);
                let nd1 = norm_cdf(d1);
                let nd2 = norm_cdf(d1 - sigma_v * sqrt_t);
                (v * exp_neg_qt * nd1 - b * (-r * t).exp() * nd2 - e).abs()
            },
            last_x: v,
            reason: "KMV fixed-point iteration did not converge".to_string(),
        }
        .into())
    }

    /// Lowest asset volatility considered when calibrating to a CDS spread.
    const CDS_CALIBRATION_MIN_VOL: f64 = 0.01;

    /// Highest asset volatility considered when calibrating to a CDS spread.
    const CDS_CALIBRATION_MAX_VOL: f64 = 2.0;

    /// Number of scan points used to locate sign changes of the CDS
    /// calibration objective before handing an interval to Brent.
    const CDS_CALIBRATION_SCAN_POINTS: usize = 128;

    /// CDS spread calibration: find the asset volatility that reproduces a
    /// quoted CDS par spread.
    ///
    /// The objective is the model's [`cds_par_spread`](Self::cds_par_spread),
    /// i.e. an ISDA-style par spread built from the model's survival curve
    /// with a premium leg, accrual on default, and discounting — not the
    /// zero-coupon approximation of [`implied_spread`](Self::implied_spread),
    /// which understates a quoted spread by several percent at distressed
    /// levels. The calibrated model has `BarrierType::Terminal` with
    /// `AssetDynamics::GeometricBrownian` and the supplied `payout_rate`.
    ///
    /// # Multiple solutions
    ///
    /// The par spread is **not** monotonic in asset volatility. For a firm
    /// whose risk-neutral forward asset value is below the barrier, raising
    /// volatility first *lowers* the default probability (more upside to
    /// recover) before raising it, so a quoted spread can be consistent with
    /// two different volatilities. Rather than returning whichever root a
    /// bracketing solver happens to land on, the objective is scanned across
    /// `[0.01, 2.0]`; a unique sign change is refined with Brent's method,
    /// and zero or several sign changes produce a descriptive error naming
    /// the attainable spread range or the competing intervals.
    ///
    /// To use first-passage barriers after calibration, construct a new
    /// model via [`new_with_dynamics`](Self::new_with_dynamics) using the
    /// calibrated `asset_vol`.
    ///
    /// # Arguments
    ///
    /// * `cds_spread_bp` - Quoted CDS par spread in basis points (`150.0` is
    ///   150 bp); must be finite and strictly positive
    /// * `recovery` - Recovery rate as a decimal fraction of notional
    ///   (`0.40` is the senior-unsecured market convention); must lie in
    ///   `[0, 1]` and be strictly below 1, since a full-recovery contract has
    ///   a zero spread at every volatility
    /// * `total_debt` - Face value of debt B acting as the default barrier,
    ///   in the same currency as `asset_value` and strictly positive
    /// * `risk_free_rate` - Continuously compounded discount rate r as a
    ///   decimal fraction, used for both legs
    /// * `maturity` - CDS maturity T in years (`5.0` for the benchmark
    ///   point); must be strictly positive
    /// * `asset_value` - Assumed initial firm asset value V, held fixed
    ///   during the solve; must be strictly positive
    /// * `payout_rate` - Continuous dividend / payout yield q on assets as a
    ///   decimal fraction (pass `0.0` for a firm with no asset payout)
    ///
    /// # Errors
    ///
    /// Returns [`InputError::NonPositiveValue`] for non-positive
    /// `total_debt`, `maturity`, or `asset_value`, and [`Error::Validation`]
    /// if `recovery` or `cds_spread_bp` is out of range, if no volatility in
    /// `[0.01, 2.0]` reproduces the quote, or if the quote is consistent with
    /// more than one volatility. Propagates solver failures from Brent's
    /// method.
    pub fn from_cds_spread(
        cds_spread_bp: f64,
        recovery: f64,
        total_debt: f64,
        risk_free_rate: f64,
        maturity: f64,
        asset_value: f64,
        payout_rate: f64,
    ) -> Result<Self> {
        if total_debt <= 0.0 || maturity <= 0.0 || asset_value <= 0.0 {
            return Err(InputError::NonPositiveValue.into());
        }
        if !(0.0..1.0).contains(&recovery) {
            return Err(Error::Validation(format!(
                "from_cds_spread: recovery must be in [0, 1), got {recovery}; a \
                 full-recovery contract has a zero spread at every volatility"
            )));
        }
        if !(cds_spread_bp.is_finite() && cds_spread_bp > 0.0) {
            return Err(Error::Validation(format!(
                "from_cds_spread: cds_spread_bp must be finite and > 0, got {cds_spread_bp}"
            )));
        }

        let target_spread = cds_spread_bp / 10_000.0;
        let objective = |sigma: f64| -> Result<f64> {
            let trial = Self::new_with_dynamics(
                asset_value,
                sigma,
                total_debt,
                risk_free_rate,
                payout_rate,
                BarrierType::Terminal,
                AssetDynamics::GeometricBrownian,
            )?;
            Ok(trial.cds_par_spread(maturity, recovery)? - target_spread)
        };

        let sigma_v = Self::solve_unique_root(&objective, cds_spread_bp, maturity)?;

        Self::new_with_dynamics(
            asset_value,
            sigma_v,
            total_debt,
            risk_free_rate,
            payout_rate,
            BarrierType::Terminal,
            AssetDynamics::GeometricBrownian,
        )
    }

    /// Locate the single volatility in `[CDS_CALIBRATION_MIN_VOL,
    /// CDS_CALIBRATION_MAX_VOL]` where `objective` changes sign, refusing to
    /// guess when the root is not unique.
    ///
    /// Some volatilities have no par spread at all: at the low end of the
    /// range a terminal-barrier firm's default probability is a vanishing
    /// number that *falls* with horizon (the drift outruns the diffusion), so
    /// it is not a survival curve and the hazard bootstrap rejects it. Those
    /// points are skipped rather than aborting the scan — they carry no
    /// credit risk and so cannot match a positive quote — and the count is
    /// reported if the scan ends up finding nothing.
    fn solve_unique_root(
        objective: &dyn Fn(f64) -> Result<f64>,
        cds_spread_bp: f64,
        maturity: f64,
    ) -> Result<f64> {
        let lo = Self::CDS_CALIBRATION_MIN_VOL;
        let hi = Self::CDS_CALIBRATION_MAX_VOL;
        let steps = Self::CDS_CALIBRATION_SCAN_POINTS;
        let step = (hi - lo) / steps as f64;

        let mut brackets: Vec<(f64, f64)> = Vec::new();
        let mut previous: Option<(f64, f64)> = None;
        let mut span: Option<(f64, f64)> = None;
        let mut skipped = 0usize;
        for i in 0..=steps {
            let sigma = lo + i as f64 * step;
            let Ok(value) = objective(sigma) else {
                skipped += 1;
                previous = None;
                continue;
            };
            span = Some(match span {
                Some((min, max)) => (min.min(value), max.max(value)),
                None => (value, value),
            });
            if value == 0.0 {
                return Ok(sigma);
            }
            if let Some((prev_sigma, prev_value)) = previous {
                if (prev_value < 0.0) != (value < 0.0) {
                    brackets.push((prev_sigma, sigma));
                }
            }
            previous = Some((sigma, value));
        }

        let Some((min_spread, max_spread)) = span else {
            return Err(Error::Validation(format!(
                "from_cds_spread: no asset volatility in [{lo}, {hi}] produces a \
                 usable survival curve at {maturity}y, so a {cds_spread_bp} bp quote \
                 cannot be matched; check the assumed asset value, debt barrier, and \
                 maturity."
            )));
        };

        match brackets.len() {
            1 => {
                let (bracket_lo, bracket_hi) = brackets[0];
                BrentSolver::new()
                    .tolerance(1e-10)
                    .bracket_bounds(bracket_lo, bracket_hi)
                    .solve(
                        |sigma| objective(sigma).unwrap_or(f64::NAN),
                        0.5 * (bracket_lo + bracket_hi),
                    )
            }
            0 => Err(Error::Validation(format!(
                "from_cds_spread: no asset volatility in [{lo}, {hi}] reproduces a \
                 {cds_spread_bp} bp spread at {maturity}y ({skipped} of {} scanned \
                 volatilities carry no credit risk at all). Attainable model spreads \
                 span {:.2} to {:.2} bp over that range; check the assumed asset \
                 value, debt barrier, and recovery.",
                steps + 1,
                (min_spread + cds_spread_bp / 10_000.0) * 10_000.0,
                (max_spread + cds_spread_bp / 10_000.0) * 10_000.0,
            ))),
            n => Err(Error::Validation(format!(
                "from_cds_spread: a {cds_spread_bp} bp spread at {maturity}y is \
                 consistent with {n} distinct asset volatilities (the par spread is \
                 non-monotonic in volatility for a firm below its barrier). \
                 Candidate brackets: {brackets:?}. Pin down the volatility from \
                 equity data instead, via from_equity."
            ))),
        }
    }

    /// Calibrate the debt barrier to match a target cumulative default
    /// probability over the given maturity.
    ///
    /// Uses terminal-barrier (classic Merton) `PD = N(-DD)` and Brent's
    /// method to find the barrier B such that `default_probability(maturity)
    /// == target_pd`. PD is strictly increasing in the barrier, so the root
    /// is unique.
    ///
    /// # Measure
    ///
    /// `target_pd` is interpreted under the **risk-neutral** measure, since
    /// the drift is `risk_free_rate - payout_rate - sigma^2/2`. To calibrate
    /// to a physical PD (a rating-implied or EDF-style default rate), pass
    /// the firm's expected physical asset return as `risk_free_rate`; the
    /// resulting barrier then reproduces that PD through
    /// [`default_probability_with_drift`](Self::default_probability_with_drift).
    ///
    /// # Arguments
    ///
    /// * `asset_value` - Current firm asset value V in the issuer's
    ///   reporting currency; must be strictly positive
    /// * `asset_vol` - Annualized asset volatility `sigma_V` as a decimal
    ///   fraction; must be strictly positive, since a zero-volatility firm
    ///   has a degenerate step-function PD that cannot hit an interior target
    /// * `risk_free_rate` - Continuously compounded risk-free rate r as a
    ///   decimal fraction
    /// * `payout_rate` - Continuous dividend / payout yield q on assets as a
    ///   decimal fraction. Omitting it (passing `0.0`) shifts the calibrated
    ///   barrier whenever the model is later evaluated with a non-zero payout
    /// * `target_pd` - Target cumulative default probability over `maturity`
    ///   as a decimal fraction (`0.01` is 1%); must lie in `(0, 1)`
    /// * `maturity` - Time horizon T in years; must be strictly positive
    ///
    /// # Errors
    ///
    /// Returns [`InputError::NonPositiveValue`] if `asset_value`,
    /// `asset_vol`, or `maturity` is non-positive, [`InputError::Invalid`] if
    /// `target_pd` is outside `(0, 1)`, and a solver error if no barrier in
    /// `[0.001 V, 0.999 V]` attains the target.
    pub fn from_target_pd(
        asset_value: f64,
        asset_vol: f64,
        risk_free_rate: f64,
        payout_rate: f64,
        target_pd: f64,
        maturity: f64,
    ) -> Result<Self> {
        if asset_value <= 0.0 || asset_vol <= 0.0 || maturity <= 0.0 {
            return Err(InputError::NonPositiveValue.into());
        }
        if !(0.0..1.0).contains(&target_pd) || target_pd <= 0.0 {
            return Err(InputError::Invalid.into());
        }

        let solver = BrentSolver::new()
            .tolerance(1e-10)
            .bracket_bounds(0.001 * asset_value, 0.999 * asset_value);

        let barrier = solver.solve(
            |b| {
                let sigma = asset_vol;
                let mu = risk_free_rate - payout_rate - 0.5 * sigma * sigma;
                let sqrt_t = maturity.sqrt();
                let dd = ((asset_value / b).ln() + mu * maturity) / (sigma * sqrt_t);
                norm_cdf(-dd) - target_pd
            },
            0.5 * asset_value,
        )?;

        Self::new_with_dynamics(
            asset_value,
            asset_vol,
            barrier,
            risk_free_rate,
            payout_rate,
            BarrierType::Terminal,
            AssetDynamics::GeometricBrownian,
        )
    }

    /// CreditGrades model construction from equity observables (simplified).
    ///
    /// Derives asset value and asset volatility from equity data and
    /// constructs a model with `CreditGrades` dynamics and `FirstPassage`
    /// barrier. This is a simplified version of the CreditGrades model
    /// (Finger et al. 2002) that uses:
    ///
    /// - Asset value: `V_0 = E + D * R_mean`
    /// - Asset volatility: `sigma_V = sigma_E * E / V_0`
    /// - Barrier: `B = D * R_mean` (deterministic)
    ///
    /// The `barrier_uncertainty` parameter is the log-barrier volatility `λ`
    /// (the lognormal standard deviation of the default barrier) and feeds the
    /// `CreditGrades` survival function via `a_t² = σ²t + λ²`.
    ///
    /// # Drift
    ///
    /// The CreditGrades asset process is driftless by construction, so the
    /// resulting model's default probabilities do **not** respond to
    /// `risk_free_rate`; the rate is retained only so the model can be used
    /// for discounting elsewhere. The payout rate is fixed at zero for the
    /// same reason.
    ///
    /// # Arguments
    ///
    /// * `equity_value` - Observed market capitalization E, strictly positive
    ///   and in the same currency as `total_debt`
    /// * `equity_vol` - Observed annualized equity volatility `sigma_E` as a
    ///   decimal fraction; must be non-negative
    /// * `total_debt` - Face value of debt, strictly positive; the barrier is
    ///   `total_debt * mean_recovery`
    /// * `risk_free_rate` - Continuously compounded risk-free rate r as a
    ///   decimal fraction. Stored on the model but not used by the
    ///   CreditGrades survival function
    /// * `barrier_uncertainty` - Log-barrier volatility `λ` (lognormal std dev
    ///   of the default barrier; Finger et al. 2002), a non-negative decimal.
    ///   `0.30` is the value calibrated in the original paper
    /// * `mean_recovery` - Mean recovery rate on debt at default as a decimal
    ///   fraction in `(0, 1]`; sets both the barrier level and the asset
    ///   value `V_0 = E + D * mean_recovery`
    ///
    /// # Errors
    ///
    /// Returns [`InputError::NonPositiveValue`] or [`InputError::NegativeValue`]
    /// for out-of-domain equity and debt inputs, and [`Error::Validation`] if
    /// `mean_recovery` is outside `[0, 1]` or `barrier_uncertainty` is
    /// negative or non-finite.
    pub fn credit_grades(
        equity_value: f64,
        equity_vol: f64,
        total_debt: f64,
        risk_free_rate: f64,
        barrier_uncertainty: f64,
        mean_recovery: f64,
    ) -> Result<Self> {
        if equity_value <= 0.0 || total_debt <= 0.0 {
            return Err(InputError::NonPositiveValue.into());
        }
        if equity_vol < 0.0 {
            return Err(InputError::NegativeValue.into());
        }
        if !(0.0..=1.0).contains(&mean_recovery) {
            return Err(Error::Validation(format!(
                "credit_grades: mean_recovery must be in [0, 1], got {mean_recovery}"
            )));
        }
        if !(barrier_uncertainty.is_finite() && barrier_uncertainty >= 0.0) {
            return Err(Error::Validation(format!(
                "credit_grades: barrier_uncertainty (log-barrier vol λ) must be \
                 finite and >= 0, got {barrier_uncertainty}"
            )));
        }

        // Asset value = equity + debt * mean_recovery
        let v0 = equity_value + total_debt * mean_recovery;
        // Asset vol from leverage relation
        let sigma_v = equity_vol * equity_value / v0;
        // Barrier = debt * mean_recovery
        let barrier = total_debt * mean_recovery;

        Self::new_with_dynamics(
            v0,
            sigma_v,
            barrier,
            risk_free_rate,
            0.0,
            BarrierType::FirstPassage {
                barrier_growth_rate: 0.0,
            },
            AssetDynamics::CreditGrades {
                barrier_uncertainty,
                mean_recovery,
            },
        )
    }

    /// Current asset value V_0.
    #[inline]
    pub fn asset_value(&self) -> f64 {
        self.asset_value
    }

    /// Asset volatility sigma_V.
    #[inline]
    pub fn asset_vol(&self) -> f64 {
        self.asset_vol
    }

    /// Debt barrier B.
    #[inline]
    pub fn debt_barrier(&self) -> f64 {
        self.debt_barrier
    }

    /// Risk-free rate r.
    #[inline]
    pub fn risk_free_rate(&self) -> f64 {
        self.risk_free_rate
    }

    /// Payout rate q (dividend yield).
    #[inline]
    pub fn payout_rate(&self) -> f64 {
        self.payout_rate
    }

    /// Barrier monitoring type.
    #[inline]
    pub fn barrier_type(&self) -> &BarrierType {
        &self.barrier_type
    }

    /// Asset dynamics specification.
    #[inline]
    pub fn dynamics(&self) -> &AssetDynamics {
        &self.dynamics
    }

    // Hazard curve generation

    /// Generate a [`HazardCurve`] compatible with existing pricing engines.
    ///
    /// Converts structural model default probabilities to piecewise-constant
    /// hazard rates at the specified tenor grid.
    ///
    /// # Measure
    ///
    /// The curve carries **risk-neutral** hazard rates, because it is built
    /// from [`default_probability`](Self::default_probability). That is what
    /// pricing engines want; it is not a physical default-intensity curve.
    ///
    /// # Algorithm
    ///
    /// 1. Compute survival probability S(t) = 1 - PD(t) at each tenor.
    /// 2. Back out piecewise-constant hazard rates between consecutive tenors:
    ///    - λ_0 = -ln(S(t_0)) / t_0
    ///    - λ_i = -ln(S(t_i) / S(t_{i-1})) / (t_i - t_{i-1}) for i >= 1
    /// 3. Build via `HazardCurve::builder`.
    ///
    /// # Arguments
    ///
    /// * `id` - Curve identifier assigned to the resulting [`HazardCurve`],
    ///   used as the lookup key in a market context
    /// * `base_date` - Valuation date the curve's year fractions are measured
    ///   from
    /// * `tenors` - Tenor grid in years from `base_date`. Must be non-empty
    ///   and strictly positive, and must be distinct; it need not be sorted,
    ///   as it is sorted internally
    /// * `recovery` - Recovery rate stored on the curve as a decimal fraction
    ///   of notional (`0.40` is the senior-unsecured market convention). Must
    ///   lie in `[0, 1]`; for `CreditGrades` dynamics it must equal the
    ///   model's own `mean_recovery`, since that value already determines the
    ///   barrier and a different recovery here would price the same default
    ///   event two ways
    /// * `day_count` - Day-count convention the curve uses to turn dates into
    ///   year fractions. Pass the convention of the discount curve the hazard
    ///   curve will be paired with; [`DayCount::Act365F`] matches the
    ///   year-fraction axis this model's horizons are expressed on
    ///
    /// # Errors
    ///
    /// Returns [`InputError::TooFewPoints`] if `tenors` is empty,
    /// [`InputError::NonPositiveValue`] if any tenor is non-positive, and
    /// [`Error::Validation`] if `recovery` is out of range or contradicts the
    /// model's `mean_recovery`, if tenors are not strictly increasing, if the
    /// implied survival curve is non-monotonic, or if survival reaches zero
    /// at some tenor (no finite hazard rate exists there). Propagates
    /// `HazardCurve` builder errors, including the hazard-rate ceiling.
    pub fn to_hazard_curve(
        &self,
        id: &str,
        base_date: time::Date,
        tenors: &[f64],
        recovery: f64,
        day_count: DayCount,
    ) -> Result<HazardCurve> {
        if tenors.is_empty() {
            return Err(InputError::TooFewPoints.into());
        }
        if !(0.0..=1.0).contains(&recovery) {
            return Err(Error::Validation(format!(
                "to_hazard_curve: recovery must be in [0, 1], got {recovery}"
            )));
        }
        // The CreditGrades barrier is `debt * mean_recovery`, so the model
        // already embeds a recovery assumption. Letting the exported curve
        // carry a different one would price the same default event under two
        // inconsistent loss assumptions.
        if let AssetDynamics::CreditGrades { mean_recovery, .. } = self.dynamics {
            if (recovery - mean_recovery).abs() > 1e-12 {
                return Err(Error::Validation(format!(
                    "to_hazard_curve: recovery {recovery} contradicts the model's \
                     CreditGrades mean_recovery {mean_recovery}; the barrier is derived \
                     from mean_recovery, so the exported curve must use the same value"
                )));
            }
        }

        // Sort tenors and validate positivity
        let mut sorted_tenors: Vec<f64> = tenors.to_vec();
        sorted_tenors.sort_by(|a, b| a.total_cmp(b));

        if sorted_tenors[0] <= 0.0 {
            return Err(InputError::NonPositiveValue.into());
        }

        // Survival of exactly zero has no finite hazard rate. Clamping it to a
        // tiny epsilon would bury a total-loss model behind an arbitrary
        // 34,000% hazard rate, so report it instead.
        let survivals: Vec<f64> = sorted_tenors
            .iter()
            .map(|&t| {
                let survival = 1.0 - self.default_probability(t);
                if survival <= 0.0 {
                    return Err(Error::Validation(format!(
                        "Merton hazard bootstrap: survival is zero at {t:.6}y (default \
                         probability is numerically 1), so no finite hazard rate exists. \
                         Shorten the tenor grid or reduce leverage/volatility."
                    )));
                }
                Ok(survival.min(1.0))
            })
            .collect::<Result<Vec<f64>>>()?;

        let mut knots: Vec<(f64, f64)> = Vec::with_capacity(sorted_tenors.len());

        // First point: λ_0 = -ln(S(t_0)) / t_0
        let lambda_0 = -survivals[0].ln() / sorted_tenors[0];
        knots.push((sorted_tenors[0], lambda_0));

        // Subsequent points: λ_i = -ln(S(t_{i+1}) / S(t_i)) / (t_{i+1} - t_i)
        for i in 1..sorted_tenors.len() {
            if survivals[i] > survivals[i - 1] {
                return Err(Error::Validation(format!(
                    "Merton hazard bootstrap produced non-monotonic survival: \
                     S({:.6}y)={:.12} > S({:.6}y)={:.12}",
                    sorted_tenors[i],
                    survivals[i],
                    sorted_tenors[i - 1],
                    survivals[i - 1]
                )));
            }
            let dt = sorted_tenors[i] - sorted_tenors[i - 1];
            // Duplicate/non-increasing tenors give dt == 0; the equal survivals
            // pass the monotonic check above, so guard here to avoid emitting a
            // NaN hazard knot (-ln(1)/0 = 0/0) into the curve.
            if dt <= 0.0 {
                return Err(Error::Validation(format!(
                    "Merton hazard bootstrap requires strictly increasing tenors; \
                     got duplicate or non-increasing tenor {:.6}y",
                    sorted_tenors[i]
                )));
            }
            let lambda_i = -(survivals[i] / survivals[i - 1]).ln() / dt;
            knots.push((sorted_tenors[i], lambda_i));
        }

        HazardCurve::builder(id)
            .base_date(base_date)
            .day_count(day_count)
            .knots(knots)
            .recovery_rate(recovery)
            .build()
    }
}

// Monte Carlo path simulation (feature-gated)

/// Results from Monte Carlo path simulation.
#[derive(Debug, Clone)]
pub struct SimulatedPaths {
    /// Time grid from 0 to T.
    pub times: Vec<f64>,
    /// Asset values in row-major order: `path_idx * (num_steps + 1) + time_idx`.
    pub asset_values: Vec<f64>,
    /// Number of paths simulated.
    pub num_paths: usize,
    /// Number of time steps.
    pub num_steps: usize,
}

impl SimulatedPaths {
    /// Number of stored values per path, including the initial value.
    #[must_use]
    pub fn values_per_path(&self) -> usize {
        self.num_steps + 1
    }

    /// Return one asset value by path and time-grid index.
    #[must_use]
    pub fn get(&self, path_idx: usize, time_idx: usize) -> Option<f64> {
        if path_idx >= self.num_paths || time_idx > self.num_steps {
            return None;
        }
        self.asset_values
            .get(path_idx * self.values_per_path() + time_idx)
            .copied()
    }

    /// Return the contiguous row for one path.
    #[must_use]
    pub fn path(&self, path_idx: usize) -> Option<&[f64]> {
        if path_idx >= self.num_paths {
            return None;
        }
        let start = path_idx * self.values_per_path();
        let end = start + self.values_per_path();
        self.asset_values.get(start..end)
    }

    /// Iterate over path rows.
    pub fn iter_paths(&self) -> impl Iterator<Item = &[f64]> {
        self.asset_values.chunks_exact(self.values_per_path())
    }

    /// Materialize nested path storage for callers that need the old shape.
    #[must_use]
    pub fn to_nested(&self) -> Vec<Vec<f64>> {
        self.iter_paths().map(<[f64]>::to_vec).collect()
    }
}

struct StepJumpData {
    base_count: usize,
    anti_count: usize,
    jump_normals: Vec<f64>,
}

impl MertonModel {
    /// Simulate asset value paths using Monte Carlo.
    ///
    /// Supports GBM and jump-diffusion dynamics. Optionally uses antithetic
    /// variates to reduce variance.
    ///
    /// `CreditGrades` dynamics simulate the *asset value* as plain GBM: the
    /// CreditGrades stochastic barrier only enters the analytic
    /// default-probability formulas, not the simulated paths. Callers needing
    /// pathwise CreditGrades default times must layer the barrier on top.
    ///
    /// # Discrete monitoring
    ///
    /// This returns the raw asset grid; it applies no barrier and no
    /// Brownian-bridge correction. A caller that infers first-passage default
    /// by testing `V_t <= B` at grid points alone will **understate** default,
    /// because a path can dip below the barrier and recover between two
    /// steps. The bias grows with step size and shrinks as `O(sqrt(dt))`. To
    /// recover the continuous-monitoring law, apply the Brownian-bridge
    /// crossing probability
    /// `exp(-2 * ln(V_i/B) * ln(V_{i+1}/B) / (sigma^2 * dt))` to each
    /// surviving step, as the PIK-toggle Monte Carlo bond engine does. The
    /// analytic [`default_probability`](Self::default_probability) already
    /// uses the continuous-monitoring Black-Cox law, so a naive grid test
    /// will not reproduce it.
    ///
    /// # Arguments
    ///
    /// * `num_paths` - Number of independent paths to simulate. With
    ///   `antithetic` set, each path is paired with its sign-flipped twin
    /// * `num_steps` - Number of equally spaced time steps per path; must be
    ///   at least 1, and each path stores `num_steps + 1` values
    /// * `horizon` - Time horizon T in years spanned by the grid; must be
    ///   finite and strictly positive
    /// * `rng` - Random number generator supplying the standard normal (and,
    ///   under `JumpDiffusion`, uniform) draws; determines reproducibility
    /// * `antithetic` - When true, generate each odd-indexed path from the
    ///   negated normals of its predecessor for variance reduction
    ///
    /// # Returns
    ///
    /// [`SimulatedPaths`] containing the time grid and all simulated asset paths.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`] if `num_steps == 0` (the time grid would
    /// be degenerate with `dt = inf`) or `horizon` is not finite and
    /// positive.
    pub fn simulate_paths(
        &self,
        num_paths: usize,
        num_steps: usize,
        horizon: f64,
        rng: &mut dyn RandomNumberGenerator,
        antithetic: bool,
    ) -> Result<SimulatedPaths> {
        if num_steps == 0 {
            return Err(Error::Validation(
                "simulate_paths: num_steps must be >= 1".into(),
            ));
        }
        if !(horizon.is_finite() && horizon > 0.0) {
            return Err(Error::Validation(format!(
                "simulate_paths: horizon must be > 0, got {horizon}"
            )));
        }
        let dt = horizon / num_steps as f64;
        let sqrt_dt = dt.sqrt();

        // Build time grid: t = 0, dt, 2*dt, ..., T
        let times: Vec<f64> = (0..=num_steps).map(|i| i as f64 * dt).collect();

        let v0 = self.asset_value;
        let sigma = self.asset_vol;
        let r = self.risk_free_rate;
        let q = self.payout_rate;

        // Determine drift and whether we have jumps
        let (drift_per_step, jump_params) = match &self.dynamics {
            AssetDynamics::GeometricBrownian | AssetDynamics::CreditGrades { .. } => {
                let drift = (r - q - 0.5 * sigma * sigma) * dt;
                (drift, None)
            }
            AssetDynamics::JumpDiffusion {
                jump_intensity,
                jump_mean,
                jump_vol,
            } => {
                // kappa = E[e^J] - 1 where J ~ N(mu_J, sigma_J^2)
                let kappa = (jump_mean + 0.5 * jump_vol * jump_vol).exp() - 1.0;
                // Compensated drift to keep E[V(T)] = V0 * e^{(r-q)T}
                let drift = (r - q - jump_intensity * kappa - 0.5 * sigma * sigma) * dt;
                (drift, Some((*jump_intensity, *jump_mean, *jump_vol)))
            }
        };

        let diffusion = sigma * sqrt_dt;

        // Determine how many base paths to generate
        let (n_base, gen_antithetic) = if antithetic {
            // For num_paths requested: generate ceil(num_paths/2) base paths
            // and their mirrors. Total = 2 * n_base.
            // If num_paths is odd, we generate one extra base path without mirror
            // to hit exactly num_paths.
            let n_base = num_paths.div_ceil(2);
            (n_base, true)
        } else {
            (num_paths, false)
        };

        let values_per_path = num_steps + 1;
        let mut all_paths: Vec<f64> = Vec::with_capacity(num_paths * values_per_path);
        let mut normals = vec![0.0; num_steps];

        for _ in 0..n_base {
            // Generate normals for this base path
            normals.iter_mut().for_each(|z| *z = rng.normal(0.0, 1.0));

            // Generate jump data if needed
            let jump_data: Option<Vec<StepJumpData>> = jump_params.map(|(lambda, _, _)| {
                let lambda_dt = lambda * dt;
                (0..num_steps)
                    .map(|_| {
                        let u = rng.uniform();
                        let base_count = poisson_inverse_cdf(lambda_dt, u);
                        let anti_count =
                            poisson_inverse_cdf(lambda_dt, (1.0 - u).min(1.0 - f64::EPSILON));
                        let max_count = base_count.max(anti_count);
                        let jump_normals: Vec<f64> =
                            (0..max_count).map(|_| rng.normal(0.0, 1.0)).collect();
                        StepJumpData {
                            base_count,
                            anti_count,
                            jump_normals,
                        }
                    })
                    .collect()
            });

            all_paths.push(v0);
            let mut v = v0;

            for step in 0..num_steps {
                let z = normals[step];
                v *= (drift_per_step + diffusion * z).exp();

                // Apply jumps if present
                if let (Some(ref jd), Some((_, mu_j, sigma_j))) = (&jump_data, jump_params) {
                    let jump_step = &jd[step];
                    for &jz in jump_step.jump_normals.iter().take(jump_step.base_count) {
                        // Jump multiplier e^J with J ~ N(mu_J, sigma_J^2): mean
                        // E[e^J] = exp(mu_J + sigma_J^2/2), matching the kappa
                        // compensator above. No Ito correction belongs on the
                        // jump itself .
                        v *= (mu_j + sigma_j * jz).exp();
                    }
                }

                all_paths.push(v);
            }

            // Build the antithetic (mirror) path if requested
            if gen_antithetic && all_paths.len() / values_per_path < num_paths {
                all_paths.push(v0);
                let mut v_anti = v0;

                for step in 0..num_steps {
                    let z = -normals[step]; // Negated normal
                    v_anti *= (drift_per_step + diffusion * z).exp();

                    if let (Some(ref jd), Some((_, mu_j, sigma_j))) = (&jump_data, jump_params) {
                        let jump_step = &jd[step];
                        for &jz in jump_step.jump_normals.iter().take(jump_step.anti_count) {
                            let jz = -jz;
                            // Same e^J law as the base path .
                            v_anti *= (mu_j + sigma_j * jz).exp();
                        }
                    }

                    all_paths.push(v_anti);
                }
            }
        }

        // Trim to exact num_paths in case antithetic generated one extra
        all_paths.truncate(num_paths * values_per_path);

        Ok(SimulatedPaths {
            times,
            asset_values: all_paths,
            num_paths,
            num_steps,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dd_textbook_values() {
        let m = MertonModel::new(100.0, 0.20, 80.0, 0.05).unwrap();
        let dd = m.distance_to_default(1.0);
        // DD = (ln(100/80) + (0.05 - 0 - 0.02)*1) / (0.2*1) = (0.22314 + 0.03) / 0.2 = 1.2657
        assert!((dd - 1.2657).abs() < 0.01, "DD={dd}");
    }

    #[test]
    fn pd_textbook_values() {
        let m = MertonModel::new(100.0, 0.20, 80.0, 0.05).unwrap();
        let pd = m.default_probability(1.0);
        // PD = N(-1.2657) ~ 0.1028
        assert!((pd - 0.1028).abs() < 0.01, "PD={pd}");
    }

    #[test]
    fn zero_vol_means_no_default_when_solvent() {
        let m = MertonModel::new(100.0, 1e-10, 80.0, 0.05).unwrap();
        let pd = m.default_probability(1.0);
        assert!(pd < 1e-6, "Zero vol, solvent -> PD~0, got {pd}");
    }

    #[test]
    fn pd_increases_with_vol() {
        let m_low = MertonModel::new(100.0, 0.10, 80.0, 0.05).unwrap();
        let m_high = MertonModel::new(100.0, 0.40, 80.0, 0.05).unwrap();
        assert!(m_high.default_probability(1.0) > m_low.default_probability(1.0));
    }

    #[test]
    fn pd_increases_with_leverage() {
        let m_low = MertonModel::new(100.0, 0.20, 60.0, 0.05).unwrap();
        let m_high = MertonModel::new(100.0, 0.20, 95.0, 0.05).unwrap();
        assert!(m_high.default_probability(1.0) > m_low.default_probability(1.0));
    }

    #[test]
    fn first_passage_pd_higher_than_terminal() {
        let m_term = MertonModel::new(100.0, 0.20, 80.0, 0.05).unwrap();
        let m_fp = MertonModel::new_with_dynamics(
            100.0,
            0.20,
            80.0,
            0.05,
            0.0,
            BarrierType::FirstPassage {
                barrier_growth_rate: 0.05,
            },
            AssetDynamics::GeometricBrownian,
        )
        .unwrap();
        assert!(
            m_fp.default_probability(5.0) > m_term.default_probability(5.0),
            "First-passage PD should be higher than terminal PD"
        );
    }

    #[test]
    fn implied_spread_positive_for_risky_firm() {
        let m = MertonModel::new(100.0, 0.25, 80.0, 0.04).unwrap();
        let spread = m.implied_spread(5.0, 0.40).expect("spread");
        assert!(spread > 0.0, "Spread should be positive");
        assert!(spread < 0.20, "Spread should be reasonable, got {spread}");
    }

    #[test]
    fn new_rejects_invalid_inputs() {
        assert!(MertonModel::new(0.0, 0.20, 80.0, 0.05).is_err());
        assert!(MertonModel::new(-1.0, 0.20, 80.0, 0.05).is_err());
        assert!(MertonModel::new(100.0, -0.20, 80.0, 0.05).is_err());
        assert!(MertonModel::new(100.0, 0.20, 0.0, 0.05).is_err());
    }

    #[test]
    fn implied_spread_rejects_invalid_inputs() {
        let m = MertonModel::new(100.0, 0.25, 80.0, 0.04).unwrap();
        assert!(m.implied_spread(0.0, 0.40).is_err(), "horizon = 0");
        assert!(m.implied_spread(-1.0, 0.40).is_err(), "horizon < 0");
        assert!(m.implied_spread(5.0, -0.1).is_err(), "recovery < 0");
        assert!(m.implied_spread(5.0, 1.1).is_err(), "recovery > 1");
    }

    #[test]
    fn from_cds_spread_rejects_out_of_range_recovery() {
        assert!(MertonModel::from_cds_spread(150.0, -0.1, 80.0, 0.04, 5.0, 100.0, 0.0).is_err());
        assert!(MertonModel::from_cds_spread(150.0, 1.5, 80.0, 0.04, 5.0, 100.0, 0.0).is_err());
    }

    #[test]
    fn simulate_paths_rejects_degenerate_grid() {
        let m = MertonModel::new(100.0, 0.25, 80.0, 0.04).unwrap();
        let mut rng = finstack_quant_core::math::random::Pcg64Rng::new(42);
        assert!(m.simulate_paths(10, 0, 5.0, &mut rng, false).is_err());
        assert!(m.simulate_paths(10, 60, 0.0, &mut rng, false).is_err());
    }

    #[test]
    fn implied_equity_from_known_asset() {
        let m = MertonModel::new(100.0, 0.20, 80.0, 0.05).expect("valid");
        let (equity, equity_vol) = m.try_implied_equity(1.0).expect("healthy firm");
        // E should be V*N(d1) - B*e^(-rT)*N(d2)
        assert!(equity > 0.0, "Equity should be positive, got {equity}");
        assert!(
            equity_vol > 0.0,
            "Equity vol should be positive, got {equity_vol}"
        );
        // With V=100, B=80, sigma=0.20, r=0.05, T=1:
        // d1 = (ln(1.25) + (0.05 + 0.02)*1) / 0.2 = (0.2231 + 0.07) / 0.2 = 1.4657
        // d2 = 1.4657 - 0.2 = 1.2657
        // E = 100*N(1.4657) - 80*e^(-0.05)*N(1.2657) ~ 100*0.9286 - 76.10*0.8972 ~ 24.59
        assert!((equity - 24.59).abs() < 1.0, "Equity={equity}");
    }

    #[test]
    fn from_equity_recovers_known_values() {
        let m_known = MertonModel::new(100.0, 0.20, 80.0, 0.05).expect("valid");
        let (equity, equity_vol) = m_known.try_implied_equity(1.0).expect("healthy firm");
        let m_calibrated = MertonModel::from_equity(equity, equity_vol, 80.0, 0.05, 0.0, 1.0)
            .expect("calibration");
        assert!(
            (m_calibrated.asset_value() - 100.0).abs() < 0.5,
            "Asset value should recover: got {}",
            m_calibrated.asset_value()
        );
        assert!(
            (m_calibrated.asset_vol() - 0.20).abs() < 0.01,
            "Asset vol should recover: got {}",
            m_calibrated.asset_vol()
        );
    }

    #[test]
    fn from_cds_spread_roundtrips() {
        let m = MertonModel::new(100.0, 0.25, 80.0, 0.04).expect("valid");
        let spread_bp = m.cds_par_spread(5.0, 0.40).expect("spread") * 10_000.0;
        let m2 = MertonModel::from_cds_spread(spread_bp, 0.40, 80.0, 0.04, 5.0, 100.0, 0.0)
            .expect("cds cal");
        assert!(
            (m2.asset_vol() - 0.25).abs() < 1e-6,
            "Asset vol should recover: got {}",
            m2.asset_vol()
        );
    }

    #[test]
    fn from_cds_spread_roundtrips_with_payout_rate() {
        // Calibrate a firm with a real asset payout q > 0. The spread is
        // produced by a model carrying that payout; from_cds_spread must
        // thread the same q into its calibration drift so the recovered
        // sigma_V matches. Dropping q biases sigma_V.
        let q = 0.04;
        let m_known = MertonModel::new_with_dynamics(
            100.0,
            0.25,
            80.0,
            0.04,
            q,
            BarrierType::Terminal,
            AssetDynamics::GeometricBrownian,
        )
        .expect("valid");
        let spread_bp = m_known.cds_par_spread(5.0, 0.40).expect("spread") * 10_000.0;

        let m_cal = MertonModel::from_cds_spread(spread_bp, 0.40, 80.0, 0.04, 5.0, 100.0, q)
            .expect("cds cal");

        assert!(
            (m_cal.asset_vol() - 0.25).abs() < 1e-6,
            "Asset vol should recover with q={q}: got {}",
            m_cal.asset_vol()
        );
        assert!(
            (m_cal.payout_rate() - q).abs() < 1e-12,
            "Payout rate should be preserved: got {}",
            m_cal.payout_rate()
        );
    }

    #[test]
    fn cds_par_spread_exceeds_zero_coupon_spread() {
        // The zero-coupon formula ignores the premium leg, accrual on
        // default, and discounting of the protection payment. For a name
        // with a material cumulative PD the true par spread is visibly
        // higher, which is why calibration must not use implied_spread.
        let m = MertonModel::new(100.0, 0.25, 80.0, 0.04).expect("valid");
        let zero_coupon = m.implied_spread(5.0, 0.40).expect("zc");
        let par = m.cds_par_spread(5.0, 0.40).expect("par");
        assert!(
            par > zero_coupon * 1.02,
            "par spread {par} should exceed zero-coupon spread {zero_coupon} by >2%"
        );
        assert!(
            par < zero_coupon * 1.30,
            "par spread {par} should stay within 30% of zero-coupon spread {zero_coupon}"
        );
    }

    #[test]
    fn cds_par_spread_increases_with_leverage() {
        let low = MertonModel::new(100.0, 0.25, 60.0, 0.04).expect("valid");
        let high = MertonModel::new(100.0, 0.25, 90.0, 0.04).expect("valid");
        assert!(
            high.cds_par_spread(5.0, 0.40).expect("high")
                > low.cds_par_spread(5.0, 0.40).expect("low")
        );
    }

    #[test]
    fn cds_par_spread_scales_with_loss_given_default() {
        // Halving LGD roughly halves the par spread: the protection leg is
        // linear in LGD and the annuity does not depend on it.
        let m = MertonModel::new(100.0, 0.25, 80.0, 0.04).expect("valid");
        let lgd_60 = m.cds_par_spread(5.0, 0.40).expect("R=40%");
        let lgd_30 = m.cds_par_spread(5.0, 0.70).expect("R=70%");
        let ratio = lgd_30 / lgd_60;
        assert!(
            (ratio - 0.5).abs() < 1e-12,
            "spread should scale linearly in LGD, got ratio {ratio}"
        );
    }

    #[test]
    fn from_cds_spread_scans_past_volatilities_with_no_survival_curve() {
        // At the bottom of the search range a firm at 1% asset vol and a 5%
        // drift has a default probability that shrinks with horizon, so the
        // hazard bootstrap refuses it. Those points carry no credit risk and
        // must be skipped, not treated as a calibration failure.
        let unusable = (0..=MertonModel::CDS_CALIBRATION_SCAN_POINTS).any(|i| {
            let step = (MertonModel::CDS_CALIBRATION_MAX_VOL
                - MertonModel::CDS_CALIBRATION_MIN_VOL)
                / MertonModel::CDS_CALIBRATION_SCAN_POINTS as f64;
            let sigma = MertonModel::CDS_CALIBRATION_MIN_VOL + i as f64 * step;
            MertonModel::new(100.0, sigma, 80.0, 0.05)
                .expect("valid")
                .cds_par_spread(5.0, 0.40)
                .is_err()
        });
        assert!(
            unusable,
            "the low-vol end of the scan should contain volatilities with no usable \
             survival curve, otherwise this test no longer exercises the skip path"
        );

        let calibrated = MertonModel::from_cds_spread(200.0, 0.40, 80.0, 0.05, 5.0, 100.0, 0.0)
            .expect("cds cal");
        assert!(
            (calibrated.cds_par_spread(5.0, 0.40).expect("spread") - 0.02).abs() < 1e-8,
            "calibrated model should reprice the 200 bp quote, got {}",
            calibrated.cds_par_spread(5.0, 0.40).expect("spread")
        );
    }

    #[test]
    fn from_cds_spread_rejects_unattainable_quote() {
        // A 10,000 bp quote on a modestly levered firm is out of reach for
        // any volatility in the search range.
        let err = MertonModel::from_cds_spread(10_000.0, 0.40, 40.0, 0.04, 5.0, 100.0, 0.0)
            .expect_err("unattainable");
        let message = err.to_string();
        assert!(
            message.contains("no asset volatility"),
            "expected an attainability error, got: {message}"
        );
    }

    #[test]
    fn to_hazard_curve_survival_matches_pd() {
        let m = MertonModel::new(100.0, 0.25, 80.0, 0.04).expect("valid");
        let base = time::Date::from_calendar_date(2026, time::Month::March, 1).expect("valid date");
        let hc = m
            .to_hazard_curve(
                "TEST",
                base,
                &[1.0, 3.0, 5.0, 7.0, 10.0],
                0.40,
                DayCount::Act365F,
            )
            .expect("hc");
        // Survival at 5Y should match 1 - PD(5)
        let sp5 = hc.sp(5.0);
        let pd5 = m.default_probability(5.0);
        assert!(
            (sp5 - (1.0 - pd5)).abs() < 0.02,
            "sp5={sp5}, 1-pd5={}",
            1.0 - pd5
        );
    }

    #[test]
    fn to_hazard_curve_hazard_rates_positive() {
        let m = MertonModel::new(100.0, 0.30, 80.0, 0.04).expect("valid");
        let base = time::Date::from_calendar_date(2026, time::Month::March, 1).expect("valid date");
        let hc = m
            .to_hazard_curve("TEST2", base, &[1.0, 3.0, 5.0], 0.40, DayCount::Act365F)
            .expect("hc");
        // All hazard rates should be positive for a risky firm
        for t in [0.5, 1.0, 2.0, 3.0, 4.0, 5.0] {
            let hr = hc.hazard_rate(t);
            assert!(
                hr > 0.0,
                "Hazard rate at t={t} should be positive, got {hr}"
            );
        }
    }

    #[test]
    fn to_hazard_curve_riskier_firm_higher_hazard() {
        let base = time::Date::from_calendar_date(2026, time::Month::March, 1).expect("valid date");
        let m_safe = MertonModel::new(100.0, 0.15, 50.0, 0.04).expect("valid");
        let m_risky = MertonModel::new(100.0, 0.30, 85.0, 0.04).expect("valid");
        let hc_safe = m_safe
            .to_hazard_curve("SAFE", base, &[1.0, 5.0, 10.0], 0.40, DayCount::Act365F)
            .expect("hc");
        let hc_risky = m_risky
            .to_hazard_curve("RISKY", base, &[1.0, 5.0, 10.0], 0.40, DayCount::Act365F)
            .expect("hc");
        assert!(
            hc_risky.hazard_rate(3.0) > hc_safe.hazard_rate(3.0),
            "Riskier firm should have higher hazard rate"
        );
    }

    #[test]
    fn to_hazard_curve_rejects_non_monotonic_survival() {
        // V < B with positive drift can make terminal PD fall with horizon,
        // which implies increasing survival and a negative hazard segment.
        let m = MertonModel::new(98.0, 0.30, 100.0, 0.10).expect("valid");
        let base = time::Date::from_calendar_date(2026, time::Month::March, 1).expect("valid date");

        let survival_1y = 1.0 - m.default_probability(1.0);
        let survival_5y = 1.0 - m.default_probability(5.0);
        assert!(
            survival_5y > survival_1y,
            "fixture must have increasing survival: 1Y={survival_1y}, 5Y={survival_5y}"
        );

        assert!(m
            .to_hazard_curve("BAD", base, &[1.0, 5.0], 0.40, DayCount::Act365F)
            .is_err());
    }

    #[test]
    fn to_hazard_curve_rejects_duplicate_tenors() {
        // Duplicate tenors (after sorting) give dt == 0 with equal survivals,
        // which previously slipped past the monotonic-survival check and emitted
        // a NaN hazard knot (-ln(1)/0). It must now be rejected.
        let m = MertonModel::new(120.0, 0.25, 100.0, 0.05).expect("valid");
        let base = time::Date::from_calendar_date(2026, time::Month::March, 1).expect("valid date");
        let result = m.to_hazard_curve("DUP", base, &[1.0, 5.0, 5.0], 0.40, DayCount::Act365F);
        assert!(
            result.is_err(),
            "duplicate tenors must be rejected, got {result:?}"
        );
    }

    #[test]
    fn credit_grades_produces_valid_model() {
        let m = MertonModel::credit_grades(25.0, 0.50, 80.0, 0.04, 0.30, 0.40).expect("cg");
        assert!(m.asset_value() > 0.0);
        assert!(m.asset_vol() > 0.0);
        assert!(matches!(m.dynamics(), AssetDynamics::CreditGrades { .. }));
        assert!(matches!(m.barrier_type(), BarrierType::FirstPassage { .. }));
        let pd = m.default_probability(5.0);
        assert!(pd > 0.0 && pd < 1.0, "PD should be in (0,1), got {pd}");
    }

    // Non-zero payout rate tests

    #[test]
    fn implied_equity_with_payout_rate() {
        // With q > 0, equity should be lower than the q=0 case because
        // the asset leaks value via dividends.
        let m_no_q = MertonModel::new(100.0, 0.20, 80.0, 0.05).expect("valid");
        let m_with_q = MertonModel::new_with_dynamics(
            100.0,
            0.20,
            80.0,
            0.05,
            0.03,
            BarrierType::Terminal,
            AssetDynamics::GeometricBrownian,
        )
        .expect("valid");

        let (eq_no_q, _) = m_no_q.try_implied_equity(1.0).expect("healthy firm");
        let (eq_with_q, _) = m_with_q.try_implied_equity(1.0).expect("healthy firm");

        assert!(
            eq_with_q < eq_no_q,
            "Equity with payout should be lower: q=0 -> {eq_no_q}, q=0.03 -> {eq_with_q}"
        );
    }

    #[test]
    fn from_equity_roundtrips_with_payout_rate() {
        let m_known = MertonModel::new_with_dynamics(
            100.0,
            0.20,
            80.0,
            0.05,
            0.02,
            BarrierType::Terminal,
            AssetDynamics::GeometricBrownian,
        )
        .expect("valid");
        let (equity, equity_vol) = m_known.try_implied_equity(1.0).expect("healthy firm");

        let m_cal = MertonModel::from_equity(equity, equity_vol, 80.0, 0.05, 0.02, 1.0)
            .expect("calibration");

        assert!(
            (m_cal.asset_value() - 100.0).abs() < 0.5,
            "Asset value should recover with q=0.02: got {}",
            m_cal.asset_value()
        );
        assert!(
            (m_cal.asset_vol() - 0.20).abs() < 0.01,
            "Asset vol should recover with q=0.02: got {}",
            m_cal.asset_vol()
        );
        assert!(
            (m_cal.payout_rate() - 0.02).abs() < 1e-10,
            "Payout rate should be preserved: got {}",
            m_cal.payout_rate()
        );
    }

    // Near-zero equity guards (W-10)

    #[test]
    fn implied_equity_rejects_near_zero_equity() {
        // A deeply distressed firm: V far below B with low vol drives the
        // call-option equity value to ~0, so the equity-vol division would
        // otherwise blow up to inf/NaN.
        let m = MertonModel::new(1.0, 0.05, 1.0e9, 0.05).expect("valid");
        let res = m.try_implied_equity(1.0);
        assert!(
            res.is_err(),
            "near-zero implied equity should be rejected, got {res:?}"
        );
    }

    #[test]
    fn implied_equity_ok_for_healthy_firm() {
        let m = MertonModel::new(100.0, 0.20, 80.0, 0.05).expect("valid");
        let (equity, equity_vol) = m.try_implied_equity(1.0).expect("healthy firm");
        assert!(equity.is_finite() && equity > 0.0);
        assert!(equity_vol.is_finite() && equity_vol > 0.0);
    }

    #[test]
    fn from_equity_rejects_near_zero_equity() {
        // A near-zero equity input must be rejected up front with a
        // descriptive `Invalid` error, not silently churned through the
        // fixed-point loop until `SolverConvergenceFailed` (which would
        // burn all 100 iterations and report a misleading reason).
        let res = MertonModel::from_equity(1.0e-12, 0.30, 80.0, 0.05, 0.0, 1.0);
        let err = res.expect_err("near-zero equity input should be rejected");
        let msg = err.to_string();
        assert!(
            !msg.contains("did not converge"),
            "should be an up-front input rejection, not a convergence failure: {msg}"
        );
    }

    // Non-convergence test

    #[test]
    fn from_equity_rejects_invalid_inputs() {
        assert!(
            MertonModel::from_equity(0.0, 0.30, 80.0, 0.05, 0.0, 1.0).is_err(),
            "Zero equity should be rejected"
        );
        assert!(
            MertonModel::from_equity(25.0, -0.30, 80.0, 0.05, 0.0, 1.0).is_err(),
            "Negative vol should be rejected"
        );
        assert!(
            MertonModel::from_equity(25.0, 0.30, 0.0, 0.05, 0.0, 1.0).is_err(),
            "Zero debt should be rejected"
        );
        assert!(
            MertonModel::from_equity(25.0, 0.30, 80.0, 0.05, 0.0, 0.0).is_err(),
            "Zero maturity should be rejected"
        );
    }

    // Extreme parameter edge cases

    #[test]
    fn high_vol_pd_approaches_one() {
        // With very high vol and high leverage, PD should approach 1.
        let m = MertonModel::new(100.0, 2.0, 99.0, 0.01).expect("valid");
        let pd = m.default_probability(10.0);
        assert!(
            pd > 0.5,
            "High vol + high leverage PD should be > 0.5, got {pd}"
        );
    }

    #[test]
    fn very_low_leverage_pd_near_zero() {
        // V >> B should give near-zero PD.
        let m = MertonModel::new(1000.0, 0.20, 10.0, 0.05).expect("valid");
        let pd = m.default_probability(1.0);
        assert!(pd < 1e-6, "Very low leverage PD should be ~0, got {pd}");
    }

    #[test]
    fn first_passage_pd_bounded() {
        let m = MertonModel::new_with_dynamics(
            100.0,
            0.25,
            80.0,
            0.05,
            0.0,
            BarrierType::FirstPassage {
                barrier_growth_rate: 0.02,
            },
            AssetDynamics::GeometricBrownian,
        )
        .expect("valid");

        for &t in &[0.5, 1.0, 5.0, 10.0, 30.0] {
            let pd = m.default_probability(t);
            assert!((0.0..=1.0).contains(&pd), "PD({t}) = {pd} out of [0, 1]");
        }
    }

    #[test]
    fn first_passage_pd_finite_under_overflowing_power_term() {
        // P0-2: the Black-Cox first-passage term `(V/H)^(-2*mu/sigma^2) * N(d-)`
        // can overflow. With a large positive risk-neutral drift (low vol,
        // very high rate) the exponent `-2*mu/sigma^2` is a large negative
        // number; for a firm trading below its barrier `(V/H)^exponent`
        // overflows to `+inf` while `N(d-)` underflows to `0`, so the naive
        // product is `inf * 0 = NaN` which survives `clamp(0, 1)`.
        let m = MertonModel::new_with_dynamics(
            50.0,
            0.05,
            100.0,
            3.0,
            0.0,
            BarrierType::FirstPassage {
                barrier_growth_rate: 0.0,
            },
            AssetDynamics::GeometricBrownian,
        )
        .expect("valid");
        let pd = m.default_probability(1.0);
        assert!(
            pd.is_finite(),
            "first-passage PD must be finite (not NaN from inf*0), got {pd}"
        );
        assert!(
            (0.0..=1.0).contains(&pd),
            "first-passage PD must lie in [0, 1], got {pd}"
        );
    }

    #[test]
    fn first_passage_pd_finite_under_large_negative_drift() {
        // The mirror case: a strongly negative drift (very high vol) drives
        // a large positive exponent; for a firm above its barrier the power
        // term overflows. PD must still be a finite, clamped value.
        let m = MertonModel::new_with_dynamics(
            1.0e6,
            0.05,
            1.0,
            0.0,
            5.0,
            BarrierType::FirstPassage {
                barrier_growth_rate: 0.0,
            },
            AssetDynamics::GeometricBrownian,
        )
        .expect("valid");
        for &t in &[0.5, 1.0, 5.0, 30.0] {
            let pd = m.default_probability(t);
            assert!(
                pd.is_finite() && (0.0..=1.0).contains(&pd),
                "first-passage PD({t}) must be finite in [0, 1], got {pd}"
            );
        }
    }

    #[test]
    fn first_passage_higher_barrier_growth_higher_pd() {
        // Higher barrier growth rate should produce higher PD at any given horizon,
        // since the default barrier rises faster.
        let m_low = MertonModel::new_with_dynamics(
            100.0,
            0.25,
            80.0,
            0.05,
            0.0,
            BarrierType::FirstPassage {
                barrier_growth_rate: 0.0,
            },
            AssetDynamics::GeometricBrownian,
        )
        .expect("valid");
        let m_high = MertonModel::new_with_dynamics(
            100.0,
            0.25,
            80.0,
            0.05,
            0.0,
            BarrierType::FirstPassage {
                barrier_growth_rate: 0.05,
            },
            AssetDynamics::GeometricBrownian,
        )
        .expect("valid");

        let pd_low = m_low.default_probability(5.0);
        let pd_high = m_high.default_probability(5.0);
        assert!(
            pd_high > pd_low,
            "Higher barrier growth should increase PD: g=0 -> {pd_low}, g=0.05 -> {pd_high}"
        );
    }

    /// Black-Cox (1976) growing-barrier regression .
    ///
    /// With V=120, B=80, sigma=0.25, r=0.06, q=0.01, g=0.03, T=5:
    ///   mu = 0.06 - 0.01 - 0.03125 = 0.01875, nu = mu - g = -0.01125,
    ///   x0 = ln(1.5) = 0.4054651,
    ///   d_plus  = (x0 + nu*T)/(sigma*sqrt(T)) =  0.624694,
    ///   d_minus = (x0 - nu*T)/(sigma*sqrt(T)) =  0.825941,
    ///   PD = N(-0.624694) + e^{0.1459674} * N(-0.825941) = 0.502635.
    #[test]
    fn first_passage_growing_barrier_matches_black_cox_1976() {
        let m = MertonModel::new_with_dynamics(
            120.0,
            0.25,
            80.0,
            0.06,
            0.01,
            BarrierType::FirstPassage {
                barrier_growth_rate: 0.03,
            },
            AssetDynamics::GeometricBrownian,
        )
        .expect("valid");

        let pd = m.default_probability(5.0);
        let expected = 0.502635;
        assert!(
            (pd - expected).abs() < 1e-3,
            "growing-barrier Black-Cox PD should be ~{expected}, got {pd}"
        );
    }

    /// A growing barrier B*e^{g t} only shifts the drift of the distance
    /// process: PD(V, B, sigma, r, q, g) == PD(V, B, sigma, r - g, q, 0)
    /// exactly. The pre-M4 formula violated this invariance for g != 0.
    #[test]
    fn first_passage_growing_barrier_equals_drift_shifted_flat_barrier() {
        let (v, sigma, b, r, q, g) = (120.0, 0.25, 80.0, 0.06, 0.01, 0.03);
        let m_growing = MertonModel::new_with_dynamics(
            v,
            sigma,
            b,
            r,
            q,
            BarrierType::FirstPassage {
                barrier_growth_rate: g,
            },
            AssetDynamics::GeometricBrownian,
        )
        .expect("valid");
        let m_shifted = MertonModel::new_with_dynamics(
            v,
            sigma,
            b,
            r - g,
            q,
            BarrierType::FirstPassage {
                barrier_growth_rate: 0.0,
            },
            AssetDynamics::GeometricBrownian,
        )
        .expect("valid");

        for horizon in [0.5, 1.0, 3.0, 5.0, 10.0] {
            let pd_g = m_growing.default_probability(horizon);
            let pd_s = m_shifted.default_probability(horizon);
            assert!(
                (pd_g - pd_s).abs() < 1e-12,
                "growing-barrier PD must equal drift-shifted flat-barrier PD at T={horizon}: \
                 {pd_g} vs {pd_s}"
            );
        }
    }

    #[test]
    fn implied_spread_monotonic_in_leverage() {
        let low_lev = MertonModel::new(100.0, 0.25, 40.0, 0.04).expect("valid");
        let mid_lev = MertonModel::new(100.0, 0.25, 70.0, 0.04).expect("valid");
        let high_lev = MertonModel::new(100.0, 0.25, 95.0, 0.04).expect("valid");

        let s_low = low_lev.implied_spread(5.0, 0.40).expect("spread");
        let s_mid = mid_lev.implied_spread(5.0, 0.40).expect("spread");
        let s_high = high_lev.implied_spread(5.0, 0.40).expect("spread");

        assert!(
            s_low < s_mid && s_mid < s_high,
            "Spread should increase with leverage: {s_low} < {s_mid} < {s_high}"
        );
    }

    // from_target_pd calibration tests

    #[test]
    fn from_target_pd_roundtrips() {
        let target_pd = 0.05; // 5% cumulative PD over 5 years
        let m = MertonModel::from_target_pd(200.0, 0.25, 0.04, 0.0, target_pd, 5.0).expect("cal");
        let actual_pd = m.default_probability(5.0);
        assert!(
            (actual_pd - target_pd).abs() < 1e-6,
            "PD should match target: got {actual_pd}, want {target_pd}"
        );
    }

    #[test]
    fn from_target_pd_higher_pd_higher_barrier() {
        let m_low = MertonModel::from_target_pd(200.0, 0.25, 0.04, 0.0, 0.01, 5.0).expect("low");
        let m_high = MertonModel::from_target_pd(200.0, 0.25, 0.04, 0.0, 0.10, 5.0).expect("high");
        assert!(
            m_high.debt_barrier() > m_low.debt_barrier(),
            "Higher PD target should need higher barrier: low={}, high={}",
            m_low.debt_barrier(),
            m_high.debt_barrier()
        );
    }

    #[test]
    fn from_target_pd_realistic_credit_grades() {
        // BB: annual PD ~20bp → 5Y cumulative ~1.0%
        let bb_pd = 1.0 - (-0.0020_f64 * 5.0).exp();
        let m_bb = MertonModel::from_target_pd(200.0, 0.20, 0.045, 0.0, bb_pd, 5.0).expect("BB");
        assert!(
            (m_bb.default_probability(5.0) - bb_pd).abs() < 1e-6,
            "BB PD mismatch"
        );

        // B: annual PD ~200bp → 5Y cumulative ~9.5%
        let b_pd = 1.0 - (-0.0200_f64 * 5.0).exp();
        let m_b = MertonModel::from_target_pd(140.0, 0.30, 0.045, 0.0, b_pd, 5.0).expect("B");
        assert!(
            (m_b.default_probability(5.0) - b_pd).abs() < 1e-6,
            "B PD mismatch"
        );

        // CCC: annual PD ~400bp → 5Y cumulative ~18.1%
        let ccc_pd = 1.0 - (-0.0400_f64 * 5.0).exp();
        let m_ccc = MertonModel::from_target_pd(115.0, 0.40, 0.045, 0.0, ccc_pd, 5.0).expect("CCC");
        assert!(
            (m_ccc.default_probability(5.0) - ccc_pd).abs() < 1e-6,
            "CCC PD mismatch"
        );

        // All calibrated barriers should be below asset value
        assert!(m_bb.debt_barrier() < 200.0);
        assert!(m_b.debt_barrier() < 140.0);
        assert!(m_ccc.debt_barrier() < 115.0);
    }

    #[test]
    fn from_target_pd_rejects_invalid_inputs() {
        assert!(MertonModel::from_target_pd(0.0, 0.25, 0.04, 0.0, 0.05, 5.0).is_err());
        assert!(MertonModel::from_target_pd(200.0, 0.25, 0.04, 0.0, 1.0, 5.0).is_err());
        assert!(MertonModel::from_target_pd(200.0, 0.25, 0.04, 0.0, -0.01, 5.0).is_err());
        // Zero asset volatility gives a degenerate step-function PD.
        assert!(MertonModel::from_target_pd(200.0, 0.0, 0.04, 0.0, 0.05, 5.0).is_err());
        // A zero target PD is unattainable for any interior barrier.
        assert!(MertonModel::from_target_pd(200.0, 0.25, 0.04, 0.0, 0.0, 5.0).is_err());
    }

    // CreditGrades cross-checks

    #[test]
    fn credit_grades_asset_value_matches_formula() {
        // V_0 = E + D * R_mean
        let e = 25.0;
        let d = 80.0;
        let r_mean = 0.40;
        let m = MertonModel::credit_grades(e, 0.50, d, 0.04, 0.30, r_mean).expect("cg");
        let expected_v = e + d * r_mean;
        assert!(
            (m.asset_value() - expected_v).abs() < 1e-10,
            "V = E + D*R_mean = {expected_v}, got {}",
            m.asset_value()
        );
    }

    #[test]
    fn credit_grades_barrier_matches_formula() {
        // Barrier = D * R_mean
        let d = 80.0;
        let r_mean = 0.40;
        let m = MertonModel::credit_grades(25.0, 0.50, d, 0.04, 0.30, r_mean).expect("cg");
        let expected_barrier = d * r_mean;
        assert!(
            (m.debt_barrier() - expected_barrier).abs() < 1e-10,
            "Barrier = D*R_mean = {expected_barrier}, got {}",
            m.debt_barrier()
        );
    }

    #[test]
    fn credit_grades_asset_vol_matches_formula() {
        // sigma_V = sigma_E * E / V_0
        let e = 25.0;
        let sigma_e = 0.50;
        let d = 80.0;
        let r_mean = 0.40;
        let m = MertonModel::credit_grades(e, sigma_e, d, 0.04, 0.30, r_mean).expect("cg");
        let v0 = e + d * r_mean;
        let expected_sigma_v = sigma_e * e / v0;
        assert!(
            (m.asset_vol() - expected_sigma_v).abs() < 1e-10,
            "sigma_V = sigma_E * E / V_0 = {expected_sigma_v}, got {}",
            m.asset_vol()
        );
    }

    #[test]
    fn credit_grades_higher_equity_vol_higher_pd() {
        let m_low = MertonModel::credit_grades(25.0, 0.30, 80.0, 0.04, 0.30, 0.40).expect("cg");
        let m_high = MertonModel::credit_grades(25.0, 0.70, 80.0, 0.04, 0.30, 0.40).expect("cg");
        assert!(
            m_high.default_probability(5.0) > m_low.default_probability(5.0),
            "Higher equity vol should increase CG PD"
        );
    }

    /// `try_implied_equity` must reject a non-positive horizon explicitly
    /// (matching `implied_spread` and `simulate_paths`) rather than letting
    /// `horizon = 0` silently return intrinsic value with a meaningless
    /// "equity vol".
    #[test]
    fn try_implied_equity_rejects_non_positive_horizon() {
        let m = MertonModel::new(100.0, 0.20, 80.0, 0.05).expect("valid");
        assert!(m.try_implied_equity(0.0).is_err(), "horizon=0 must error");
        assert!(
            m.try_implied_equity(-1.0).is_err(),
            "negative horizon must error"
        );
        assert!(
            m.try_implied_equity(f64::NAN).is_err(),
            "NaN horizon must error"
        );
    }

    /// The CreditGrades constructor must reject out-of-domain inputs instead
    /// of silently building a nonsensical model: `mean_recovery > 1` yields a
    /// barrier above face debt, and a negative `barrier_uncertainty` was
    /// previously clamped to 0 at compute time, silently discarding it.
    #[test]
    fn credit_grades_rejects_out_of_range_recovery_and_negative_lambda() {
        assert!(
            MertonModel::credit_grades(25.0, 0.50, 80.0, 0.04, 0.30, 1.5).is_err(),
            "mean_recovery > 1 must be rejected"
        );
        assert!(
            MertonModel::credit_grades(25.0, 0.50, 80.0, 0.04, 0.30, -0.1).is_err(),
            "negative mean_recovery must be rejected"
        );
        assert!(
            MertonModel::credit_grades(25.0, 0.50, 80.0, 0.04, -0.30, 0.40).is_err(),
            "negative barrier_uncertainty must be rejected"
        );
    }

    /// Golden pin for the CreditGrades survival formula (Finger et al. 2002).
    ///
    /// Every other CreditGrades test asserts only structural properties
    /// (monotonicity, sensitivity), so a subtle regression — e.g. dropping
    /// the `exp(λ²)` leverage factor or the `λ²` term in `a_t²` — would pass
    /// them all. Reference values computed with an independent Python
    /// implementation of the Technical Document survival function
    /// `P = Φ(−A/2 + ln d/A) − d·Φ(−A/2 − ln d/A)` with `A² = σ_V²t + λ²`,
    /// `d = (V₀/B)·e^{λ²}`, and the constructor mapping `V₀ = E + D·R̄`,
    /// `σ_V = σ_E·E/V₀`, `B = D·R̄`:
    ///   E=25, σ_E=0.50, D=80, λ=0.30, R̄=0.40
    ///   → PD(1y) = 0.10002066946020438, PD(5y) = 0.3349985804601995.
    #[test]
    fn credit_grades_pd_matches_independent_finger_reference() {
        let m = MertonModel::credit_grades(25.0, 0.50, 80.0, 0.04, 0.30, 0.40).expect("cg");
        let pd_1y = m.default_probability(1.0);
        let pd_5y = m.default_probability(5.0);
        // Tolerance 1e-9: the reference uses Python's erf-based normal CDF,
        // which differs from this crate's norm_cdf by O(1e-11) in the tails.
        // A structural regression (e.g. losing exp(λ²)) moves PD by O(1e-2).
        assert!(
            (pd_1y - 0.100_020_669_460_204_38).abs() < 1e-9,
            "CreditGrades 1y PD {pd_1y:.15} != independent reference 0.100020669460204"
        );
        assert!(
            (pd_5y - 0.334_998_580_460_199_5).abs() < 1e-9,
            "CreditGrades 5y PD {pd_5y:.15} != independent reference 0.334998580460200"
        );
    }

    #[test]
    fn credit_grades_barrier_uncertainty_affects_pd() {
        let low_lambda =
            MertonModel::credit_grades(25.0, 0.50, 80.0, 0.04, 0.10, 0.40).expect("cg");
        let high_lambda =
            MertonModel::credit_grades(25.0, 0.50, 80.0, 0.04, 0.60, 0.40).expect("cg");
        assert_ne!(
            low_lambda.default_probability(5.0),
            high_lambda.default_probability(5.0),
            "CreditGrades barrier uncertainty must feed the survival function"
        );
    }

    // Monte Carlo path simulation tests

    #[test]
    fn simulate_paths_deterministic_with_seed() {
        use finstack_quant_core::math::random::Pcg64Rng;
        let m = MertonModel::new(100.0, 0.20, 80.0, 0.05).expect("valid");
        let mut rng1 = Pcg64Rng::new(42);
        let mut rng2 = Pcg64Rng::new(42);
        let paths1 = m
            .simulate_paths(10, 60, 5.0, &mut rng1, false)
            .expect("paths");
        let paths2 = m
            .simulate_paths(10, 60, 5.0, &mut rng2, false)
            .expect("paths");
        assert_eq!(
            paths1.path(0),
            paths2.path(0),
            "Same seed should give same paths"
        );
    }

    #[test]
    fn simulate_paths_gbm_mean_converges() {
        use finstack_quant_core::math::random::Pcg64Rng;
        let m = MertonModel::new(100.0, 0.20, 80.0, 0.05).expect("valid");
        let mut rng = Pcg64Rng::new(42);
        let paths = m
            .simulate_paths(50_000, 60, 5.0, &mut rng, true)
            .expect("paths");
        let mean_terminal: f64 = paths
            .iter_paths()
            .map(|p| *p.last().expect("non-empty"))
            .sum::<f64>()
            / paths.num_paths as f64;
        let expected = 100.0 * (0.05_f64 * 5.0).exp();
        let rel_error = (mean_terminal - expected).abs() / expected;
        assert!(
            rel_error < 0.02,
            "Mean terminal should converge to E[V(T)] = V\u{2080}\u{00d7}e^(rT) = {expected}, got {mean_terminal}, rel_err={rel_error}"
        );
    }

    /// Jump-diffusion martingale check : the compensated
    /// drift uses kappa = E[e^J] - 1 = exp(mu_J + sigma_J^2/2) - 1, so the
    /// simulated jump multiplier must be e^J with J ~ N(mu_J, sigma_J^2)
    /// (mean exp(mu_J + sigma_J^2/2)). With that pairing,
    /// E[V(T)] = V0 * e^{(r-q)T} holds exactly.
    #[test]
    fn simulate_paths_jump_diffusion_mean_converges() {
        use finstack_quant_core::math::random::Pcg64Rng;
        let m = MertonModel::new_with_dynamics(
            100.0,
            0.20,
            80.0,
            0.05,
            0.0,
            BarrierType::Terminal,
            AssetDynamics::JumpDiffusion {
                jump_intensity: 0.5,
                jump_mean: -0.05,
                jump_vol: 0.10,
            },
        )
        .expect("valid");
        let mut rng = Pcg64Rng::new(42);
        let paths = m
            .simulate_paths(50_000, 60, 5.0, &mut rng, true)
            .expect("paths");
        let mean_terminal: f64 = paths
            .iter_paths()
            .map(|p| *p.last().expect("non-empty"))
            .sum::<f64>()
            / paths.num_paths as f64;
        let expected = 100.0 * (0.05_f64 * 5.0).exp();
        let rel_error = (mean_terminal - expected).abs() / expected;
        assert!(
            rel_error < 0.02,
            "JD mean terminal should converge to E[V(T)] = V0*e^(rT) = {expected}, \
             got {mean_terminal}, rel_err={rel_error}"
        );
    }

    #[test]
    fn simulate_paths_correct_dimensions() {
        use finstack_quant_core::math::random::Pcg64Rng;
        let m = MertonModel::new(100.0, 0.20, 80.0, 0.05).expect("valid");
        let mut rng = Pcg64Rng::new(42);
        let paths = m
            .simulate_paths(100, 60, 5.0, &mut rng, false)
            .expect("paths");
        assert_eq!(paths.num_paths, 100);
        assert_eq!(paths.num_steps, 60);
        assert_eq!(paths.times.len(), 61); // includes t=0
        assert_eq!(paths.asset_values.len(), 100 * 61);
        assert_eq!(paths.values_per_path(), 61);
        assert!(
            (paths.times[0] - 0.0).abs() < 1e-10,
            "First time should be 0"
        );
        assert!(
            (paths.times[60] - 5.0).abs() < 1e-10,
            "Last time should be horizon"
        );
        assert!(
            (paths.get(0, 0).expect("path value") - 100.0).abs() < 1e-10,
            "Should start at V\u{2080}"
        );
    }

    #[test]
    fn jump_diffusion_produces_different_paths() {
        use finstack_quant_core::math::random::Pcg64Rng;
        let m_gbm = MertonModel::new(100.0, 0.20, 80.0, 0.05).expect("valid");
        let m_jd = MertonModel::new_with_dynamics(
            100.0,
            0.20,
            80.0,
            0.05,
            0.0,
            BarrierType::Terminal,
            AssetDynamics::JumpDiffusion {
                jump_intensity: 0.5,
                jump_mean: -0.05,
                jump_vol: 0.10,
            },
        )
        .expect("valid");
        let mut rng1 = Pcg64Rng::new(42);
        let mut rng2 = Pcg64Rng::new(42);
        let paths_gbm = m_gbm
            .simulate_paths(100, 60, 5.0, &mut rng1, false)
            .expect("paths");
        let paths_jd = m_jd
            .simulate_paths(100, 60, 5.0, &mut rng2, false)
            .expect("paths");
        // JD paths should differ from GBM (different drift compensation + jumps)
        let gbm_terminal: f64 = paths_gbm
            .iter_paths()
            .map(|p| *p.last().expect("non-empty"))
            .sum::<f64>();
        let jd_terminal: f64 = paths_jd
            .iter_paths()
            .map(|p| *p.last().expect("non-empty"))
            .sum::<f64>();
        assert!(
            (gbm_terminal - jd_terminal).abs() > 1.0,
            "JD should produce different terminal values"
        );
    }

    /// Regression test for the Black-Cox first-passage reflection-term sign.
    ///
    /// When the risk-neutral log-drift mu = r - q - 0.5*sigma^2 is exactly 0,
    /// d_plus == d_minus == d = ln(V/H) / (sigma*sqrt(T)), so the correct
    /// Black-Cox formula collapses to:
    ///
    ///   PD = N(-d) + (V/H)^0 * N(-d) = 2*N(-d)
    ///
    /// With the original buggy code (N(+d_minus) instead of N(-d_minus)) the
    /// reflection term becomes N(d) = 1 - N(-d), yielding PD = 1.0 — certain
    /// default for a healthy, well-capitalised firm.
    #[test]
    fn first_passage_zero_drift_reflection_sign() {
        // Parameters chosen so mu = r - q - 0.5*sigma^2 = 0 exactly.
        // r = 0.5 * sigma^2, q = 0.0, sigma = 0.2  =>  r = 0.02.
        let sigma: f64 = 0.2;
        let r = 0.5 * sigma * sigma; // = 0.02
        let t = 3.0_f64;
        let v = 120.0_f64;
        let b = 100.0_f64;

        let model = MertonModel::new_with_dynamics(
            v,
            sigma,
            b,
            r,
            0.0, // payout_rate = 0
            BarrierType::FirstPassage {
                barrier_growth_rate: 0.0,
            },
            AssetDynamics::GeometricBrownian,
        )
        .expect("valid model");

        let pd = model.default_probability(t);

        // When mu = 0, H = B (no barrier growth), d = ln(V/B) / (sigma*sqrt(T)).
        let d = (v / b).ln() / (sigma * t.sqrt());
        let expected = 2.0 * norm_cdf(-d);

        assert!(
            (pd - expected).abs() < 1e-10,
            "Black-Cox zero-drift PD should be 2*N(-d) = {expected:.10}, got {pd:.10}"
        );
        assert!(
            pd < 1.0,
            "Healthy firm (V > B) with zero drift must have PD < 1.0, got {pd}"
        );
    }

    #[test]
    fn first_passage_nonzero_drift_reflection_sign() {
        // Parameters chosen so mu = r - q - 0.5*sigma^2 != 0.
        // r = 0.05, q = 0.01, sigma = 0.20  =>  mu = 0.05 - 0.01 - 0.02 = 0.02 (positive drift).
        let sigma: f64 = 0.20;
        let r = 0.05_f64;
        let q = 0.01_f64;
        let t = 3.0_f64;
        let v = 120.0_f64;
        let b = 100.0_f64;

        let model = MertonModel::new_with_dynamics(
            v,
            sigma,
            b,
            r,
            q,
            BarrierType::FirstPassage {
                barrier_growth_rate: 0.0,
            },
            AssetDynamics::GeometricBrownian,
        )
        .expect("valid model");

        let pd = model.default_probability(t);

        // Closed-form reference (mu != 0, barrier_growth_rate = 0 => H = B).
        let mu = r - q - 0.5 * sigma * sigma; // = 0.02
        let sqrt_t = t.sqrt();
        let sigma_sqrt_t = sigma * sqrt_t;
        let log_v_h = (v / b).ln();
        let d_plus = (log_v_h + mu * t) / sigma_sqrt_t;
        let d_minus = (log_v_h - mu * t) / sigma_sqrt_t;
        let exponent = -2.0 * mu / (sigma * sigma);
        let expected = norm_cdf(-d_plus) + (v / b).powf(exponent) * norm_cdf(-d_minus);

        assert!(
            (pd - expected).abs() < 1e-10,
            "Black-Cox non-zero-drift PD should be {expected:.10}, got {pd:.10}"
        );
        assert!(
            pd < 1.0,
            "Healthy firm (V > B) with positive drift must have PD < 1.0, got {pd}"
        );
    }

    // Jump-diffusion default probability

    fn jump_diffusion_model(jump_mean: f64) -> MertonModel {
        MertonModel::new_with_dynamics(
            100.0,
            0.20,
            80.0,
            0.05,
            0.0,
            BarrierType::Terminal,
            AssetDynamics::JumpDiffusion {
                jump_intensity: 0.5,
                jump_mean,
                jump_vol: 0.15,
            },
        )
        .expect("valid")
    }

    #[test]
    fn jump_diffusion_pd_matches_monte_carlo() {
        use finstack_quant_core::math::random::Pcg64Rng;
        let m = jump_diffusion_model(-0.30);
        let analytic = m.default_probability(2.0);

        let mut rng = Pcg64Rng::new(7);
        let paths = m
            .simulate_paths(200_000, 200, 2.0, &mut rng, true)
            .expect("paths");
        let defaults = paths
            .iter_paths()
            .filter(|p| *p.last().expect("non-empty") < m.debt_barrier())
            .count();
        let empirical = defaults as f64 / paths.num_paths as f64;

        assert!(
            (analytic - empirical).abs() < 0.005,
            "Merton-1976 mixture PD {analytic} should match simulated terminal PD {empirical}"
        );
    }

    #[test]
    fn jump_diffusion_pd_exceeds_diffusion_only_pd() {
        // Downward jumps add left-tail mass that a pure diffusion of the same
        // sigma cannot produce.
        let jd = jump_diffusion_model(-0.30);
        let gbm = MertonModel::new(100.0, 0.20, 80.0, 0.05).expect("valid");
        assert!(
            jd.default_probability(2.0) > gbm.default_probability(2.0) * 1.5,
            "jump PD {} should materially exceed GBM PD {}",
            jd.default_probability(2.0),
            gbm.default_probability(2.0)
        );
    }

    #[test]
    fn jump_diffusion_pd_collapses_to_gbm_without_jumps() {
        let jd = MertonModel::new_with_dynamics(
            100.0,
            0.20,
            80.0,
            0.05,
            0.0,
            BarrierType::Terminal,
            AssetDynamics::JumpDiffusion {
                jump_intensity: 0.0,
                jump_mean: -0.30,
                jump_vol: 0.15,
            },
        )
        .expect("valid");
        let gbm = MertonModel::new(100.0, 0.20, 80.0, 0.05).expect("valid");
        assert!((jd.default_probability(3.0) - gbm.default_probability(3.0)).abs() < 1e-12);
    }

    #[test]
    fn jump_diffusion_rejects_first_passage_barrier() {
        let result = MertonModel::new_with_dynamics(
            100.0,
            0.20,
            80.0,
            0.05,
            0.0,
            BarrierType::FirstPassage {
                barrier_growth_rate: 0.0,
            },
            AssetDynamics::JumpDiffusion {
                jump_intensity: 0.5,
                jump_mean: -0.30,
                jump_vol: 0.15,
            },
        );
        assert!(result.is_err(), "JD + first passage has no closed form");
    }

    #[test]
    fn try_implied_equity_rejects_jump_diffusion() {
        assert!(jump_diffusion_model(-0.30).try_implied_equity(1.0).is_err());
    }

    // Construction validation

    #[test]
    fn new_with_dynamics_rejects_out_of_domain_dynamics() {
        let build = |dynamics| {
            MertonModel::new_with_dynamics(
                100.0,
                0.20,
                80.0,
                0.05,
                0.0,
                BarrierType::Terminal,
                dynamics,
            )
        };
        assert!(build(AssetDynamics::JumpDiffusion {
            jump_intensity: -0.5,
            jump_mean: 0.0,
            jump_vol: 0.1,
        })
        .is_err());
        assert!(build(AssetDynamics::JumpDiffusion {
            jump_intensity: 0.5,
            jump_mean: 0.0,
            jump_vol: -0.1,
        })
        .is_err());
        assert!(build(AssetDynamics::JumpDiffusion {
            jump_intensity: f64::NAN,
            jump_mean: 0.0,
            jump_vol: 0.1,
        })
        .is_err());
    }

    #[test]
    fn credit_grades_requires_flat_first_passage_barrier() {
        let build = |barrier_type| {
            MertonModel::new_with_dynamics(
                100.0,
                0.20,
                80.0,
                0.05,
                0.0,
                barrier_type,
                AssetDynamics::CreditGrades {
                    barrier_uncertainty: 0.30,
                    mean_recovery: 0.40,
                },
            )
        };
        assert!(build(BarrierType::Terminal).is_err());
        assert!(build(BarrierType::FirstPassage {
            barrier_growth_rate: 0.02
        })
        .is_err());
        assert!(build(BarrierType::FirstPassage {
            barrier_growth_rate: 0.0
        })
        .is_ok());
        // A negative log-barrier volatility is rejected rather than clamped.
        assert!(MertonModel::new_with_dynamics(
            100.0,
            0.20,
            80.0,
            0.05,
            0.0,
            BarrierType::FirstPassage {
                barrier_growth_rate: 0.0
            },
            AssetDynamics::CreditGrades {
                barrier_uncertainty: -0.30,
                mean_recovery: 0.40,
            },
        )
        .is_err());
    }

    #[test]
    fn deserialization_enforces_constructor_invariants() {
        let valid = MertonModel::new(100.0, 0.25, 80.0, 0.04).expect("valid");
        let json = serde_json::to_string(&valid).expect("serialize");
        let round_tripped: MertonModel = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(round_tripped, valid);

        // Negative volatility must not survive deserialization.
        let bad_vol = json.replace("\"asset_vol\":0.25", "\"asset_vol\":-0.25");
        assert!(serde_json::from_str::<MertonModel>(&bad_vol).is_err());

        // Neither must an unsupported dynamics/barrier pairing.
        let jd_first_passage = r#"{
            "asset_value": 100.0,
            "asset_vol": 0.2,
            "debt_barrier": 80.0,
            "risk_free_rate": 0.05,
            "payout_rate": 0.0,
            "barrier_type": {"first_passage": {"barrier_growth_rate": 0.0}},
            "dynamics": {"jump_diffusion": {"jump_intensity": 0.5, "jump_mean": -0.3, "jump_vol": 0.15}}
        }"#;
        assert!(serde_json::from_str::<MertonModel>(jd_first_passage).is_err());

        // Nor an out-of-domain jump intensity.
        let negative_intensity = r#"{
            "asset_value": 100.0,
            "asset_vol": 0.2,
            "debt_barrier": 80.0,
            "risk_free_rate": 0.05,
            "payout_rate": 0.0,
            "barrier_type": "terminal",
            "dynamics": {"jump_diffusion": {"jump_intensity": -0.5, "jump_mean": -0.3, "jump_vol": 0.15}}
        }"#;
        assert!(serde_json::from_str::<MertonModel>(negative_intensity).is_err());
    }

    // Physical measure (Moody's KMV / EDF)

    #[test]
    fn physical_dd_matches_risk_neutral_dd_at_the_risk_free_rate() {
        let m = MertonModel::new(100.0, 0.25, 80.0, 0.04).expect("valid");
        let physical = m
            .distance_to_default_with_drift(0.04, 3.0)
            .expect("supported");
        assert!((physical - m.distance_to_default(3.0)).abs() < 1e-12);
    }

    #[test]
    fn physical_pd_below_risk_neutral_pd_for_a_positive_risk_premium() {
        let m = MertonModel::new(100.0, 0.25, 80.0, 0.04).expect("valid");
        let edf = m
            .default_probability_with_drift(0.11, 5.0)
            .expect("supported");
        assert!(
            edf < m.default_probability(5.0),
            "EDF {edf} must be below the risk-neutral PD {}",
            m.default_probability(5.0)
        );
        assert!(edf > 0.0);
    }

    #[test]
    fn physical_measure_rejects_credit_grades() {
        let m = MertonModel::credit_grades(25.0, 0.50, 80.0, 0.04, 0.30, 0.40).expect("cg");
        assert!(m.distance_to_default_with_drift(0.09, 1.0).is_err());
        assert!(m.default_probability_with_drift(0.09, 1.0).is_err());
    }

    #[test]
    fn kmv_default_point_is_short_term_plus_half_long_term_debt() {
        let dp = MertonModel::kmv_default_point(40.0, 120.0).expect("valid");
        assert!((dp - 100.0).abs() < 1e-12);
        assert!(MertonModel::kmv_default_point(-1.0, 10.0).is_err());
        assert!(MertonModel::kmv_default_point(0.0, 0.0).is_err());
    }

    #[test]
    fn kmv_edf_workflow_produces_a_plausible_one_year_default_rate() {
        // Textbook KMV setup: equity inversion for (V, sigma_V), the
        // short-term-plus-half-long-term default point, then a physical drift.
        let default_point = MertonModel::kmv_default_point(30.0, 60.0).expect("valid");
        let m = MertonModel::from_equity(120.0, 0.40, default_point, 0.04, 0.0, 1.0)
            .expect("kmv inversion");
        let dd = m
            .distance_to_default_with_drift(0.09, 1.0)
            .expect("supported");
        let edf = m
            .default_probability_with_drift(0.09, 1.0)
            .expect("supported");
        assert!(dd > 0.0, "a solvent firm should have positive DD, got {dd}");
        assert!((edf - norm_cdf(-dd)).abs() < 1e-12);
    }

    // Endogenous debt spread

    #[test]
    fn debt_spread_matches_black_scholes_put_valuation() {
        // D = B e^{-rT} - Put(V, B), so the endogenous spread must agree with
        // an independently computed Black-Scholes put.
        let (v, sigma, b, r, t) = (100.0_f64, 0.25_f64, 80.0_f64, 0.04_f64, 5.0_f64);
        let m = MertonModel::new(v, sigma, b, r).expect("valid");

        let sqrt_t = t.sqrt();
        let d1 = ((v / b).ln() + (r + 0.5 * sigma * sigma) * t) / (sigma * sqrt_t);
        let d2 = d1 - sigma * sqrt_t;
        let put = b * (-r * t).exp() * norm_cdf(-d2) - v * norm_cdf(-d1);
        let risk_free_value = b * (-r * t).exp();
        let expected = -((risk_free_value - put) / risk_free_value).ln() / t;

        assert!((m.debt_spread(t).expect("spread") - expected).abs() < 1e-12);
    }

    #[test]
    fn debt_spread_below_exogenous_forty_percent_recovery_spread() {
        // Endogenous recovery in the Merton model is the firm's own terminal
        // asset value, which for a moderately levered firm is far above 40%.
        let m = MertonModel::new(100.0, 0.25, 80.0, 0.04).expect("valid");
        assert!(m.debt_spread(5.0).expect("endogenous") < m.implied_spread(5.0, 0.40).expect("zc"));
    }

    #[test]
    fn debt_spread_rejects_first_passage_barrier() {
        let m = MertonModel::new_with_dynamics(
            100.0,
            0.25,
            80.0,
            0.04,
            0.0,
            BarrierType::FirstPassage {
                barrier_growth_rate: 0.0,
            },
            AssetDynamics::GeometricBrownian,
        )
        .expect("valid");
        assert!(m.debt_spread(5.0).is_err());
    }

    // Hazard curve export

    #[test]
    fn to_hazard_curve_rejects_recovery_inconsistent_with_credit_grades() {
        let m = MertonModel::credit_grades(25.0, 0.50, 80.0, 0.04, 0.30, 0.40).expect("cg");
        let base = time::Date::from_calendar_date(2026, time::Month::March, 1).expect("valid date");
        assert!(m
            .to_hazard_curve("CG", base, &[1.0, 5.0], 0.60, DayCount::Act365F)
            .is_err());
        assert!(m
            .to_hazard_curve("CG", base, &[1.0, 5.0], 0.40, DayCount::Act365F)
            .is_ok());
    }

    #[test]
    fn to_hazard_curve_honours_the_requested_day_count() {
        let m = MertonModel::new(100.0, 0.25, 80.0, 0.04).expect("valid");
        let base = time::Date::from_calendar_date(2026, time::Month::March, 1).expect("valid date");
        let hc = m
            .to_hazard_curve("DC", base, &[1.0, 5.0], 0.40, DayCount::Act360)
            .expect("hc");
        assert_eq!(hc.day_count(), DayCount::Act360);
    }

    #[test]
    fn credit_grades_survival_ignores_the_risk_free_rate() {
        // The CreditGrades process is driftless, so the rate must not move PD.
        let low = MertonModel::credit_grades(25.0, 0.50, 80.0, 0.00, 0.30, 0.40).expect("cg");
        let high = MertonModel::credit_grades(25.0, 0.50, 80.0, 0.20, 0.30, 0.40).expect("cg");
        assert!((low.default_probability(5.0) - high.default_probability(5.0)).abs() < 1e-15);
    }
}
