use crate::instruments::common_impl::helpers::get_unitless_scalar_strict;
use finstack_quant_core::market_data::context::MarketContext;
use tracing::warn;

/// Default Heston parameters used when no market scalar is supplied.
///
/// These are conservative, broadly representative SPX-style values. They are
/// the single source of truth for Heston defaults across all equity option
/// pricers (Fourier, PDE, Monte Carlo).
pub mod heston_defaults {
    /// Default mean reversion speed of variance (κ).
    pub const KAPPA: f64 = 2.0;
    /// Default long-run variance level (θ).
    pub const THETA: f64 = 0.04;
    /// Default vol-of-vol (σᵥ).
    pub const SIGMA_V: f64 = 0.3;
    /// Default spot/variance correlation (ρ); negative for equity (leverage effect).
    pub const RHO: f64 = -0.7;
    /// Default initial variance (v₀).
    pub const V0: f64 = 0.04;
}

/// Truncated-tail mass (on the probability scale) above which the Gil-Pelaez
/// integral is considered mis-truncated and a diagnostic is surfaced.
///
/// A well-resolved Heston Fourier integral has a tail far below this; the
/// `[0, 1]` probability clamp would otherwise silently hide truncation error
/// from too small a `u_max` (audit item 4). `1e-4` ≈ 1bp on the probability,
/// which feeds into a price error worth flagging for risk use.
pub(super) const HESTON_TAIL_DIAGNOSTIC_THRESHOLD: f64 = 1e-4;

#[derive(Debug, Clone, Copy)]
/// Heston stochastic volatility model parameters.
///
/// # References
///
/// - Heston, S. L. (1993). "A Closed-Form Solution for Options with Stochastic Volatility
///   with Applications to Bond and Currency Options." *Review of Financial Studies*, 6(2), 327-343.
pub struct HestonParams {
    /// Risk-free interest rate
    pub r: f64,
    /// Continuous dividend yield
    pub q: f64,
    /// Mean reversion speed of variance
    pub kappa: f64,
    /// Long-run variance level
    pub theta: f64,
    /// Volatility of variance (vol-of-vol)
    pub sigma_v: f64,
    /// Correlation between asset price and variance
    pub rho: f64,
    /// Initial variance level
    pub v0: f64,
}

impl HestonParams {
    /// Create new Heston model parameters
    ///
    /// # Arguments
    ///
    /// * `r` - Continuously compounded risk-free rate in decimal annual units
    /// * `q` - Continuous dividend yield in decimal annual units
    /// * `kappa` - Mean-reversion speed of the stochastic volatility or short-rate factor
    /// * `theta` - Long-run mean level of the mean-reverting stochastic factor
    /// * `sigma_v` - Volatility-of-variance parameter for the Heston-style variance process
    /// * `rho` - Instantaneous correlation between Brownian drivers, in `[-1, 1]`
    /// * `v0` - Initial variance level for the stochastic volatility process at time zero
    pub fn new(
        r: f64,
        q: f64,
        kappa: f64,
        theta: f64,
        sigma_v: f64,
        rho: f64,
        v0: f64,
    ) -> finstack_quant_core::Result<Self> {
        if !r.is_finite() {
            return Err(finstack_quant_core::Error::Validation(format!(
                "Heston parameter r (risk-free rate) must be finite, got {r}"
            )));
        }
        if !q.is_finite() {
            return Err(finstack_quant_core::Error::Validation(format!(
                "Heston parameter q (dividend yield) must be finite, got {q}"
            )));
        }
        if kappa <= 0.0 || !kappa.is_finite() {
            return Err(finstack_quant_core::Error::Validation(format!(
                "Heston parameter kappa (mean reversion) must be positive, got {kappa}"
            )));
        }
        if theta <= 0.0 || !theta.is_finite() {
            return Err(finstack_quant_core::Error::Validation(format!(
                "Heston parameter theta (long-run variance) must be positive, got {theta}"
            )));
        }
        if sigma_v <= 0.0 || !sigma_v.is_finite() {
            return Err(finstack_quant_core::Error::Validation(format!(
                "Heston parameter sigma_v (vol-of-vol) must be positive, got {sigma_v}"
            )));
        }
        if rho <= -1.0 || rho >= 1.0 || !rho.is_finite() {
            return Err(finstack_quant_core::Error::Validation(format!(
                "Heston parameter rho (correlation) must be in (-1, 1), got {rho}"
            )));
        }
        if v0 <= 0.0 || !v0.is_finite() {
            return Err(finstack_quant_core::Error::Validation(format!(
                "Heston parameter v0 (initial variance) must be positive, got {v0}"
            )));
        }

        let params = Self {
            r,
            q,
            kappa,
            theta,
            sigma_v,
            rho,
            v0,
        };

        if 2.0 * params.kappa * params.theta <= params.sigma_v * params.sigma_v {
            warn!(
                r = params.r,
                q = params.q,
                kappa = params.kappa,
                theta = params.theta,
                sigma_v = params.sigma_v,
                rho = params.rho,
                v0 = params.v0,
                "Heston Feller condition violated (2κθ ≤ σ²): informational only — the \
                 Fourier pricer never simulates the variance path, so this poses no \
                 zero-variance pricing risk; relevant only for Monte Carlo simulation \
                 of the variance process"
            );
        }

        Ok(params)
    }

    /// Deterministic average variance over `[0, t]`:
    ///
    /// ```text
    /// v̄(t) = θ + (v₀ − θ)·(1 − e^{−κt}) / (κt)
    /// ```
    ///
    /// the time-average of the variance ODE `E[v_s] = θ + (v₀−θ)e^{−κs}`.
    /// This is the correct effective variance for a Black-Scholes proxy of a
    /// maturity-`t` option: using `v₀` ignores the mean reversion toward `θ`
    /// and badly mis-states long-dated fallback prices when `v₀ ≠ θ`.
    ///
    /// Uses a Taylor branch for `κt → 0` where the closed form is `0/0`
    /// (`(1 − e^{−x})/x = 1 − x/2 + x²/6 − …`). For `t ≤ 0` it returns `v₀`.
    #[must_use]
    pub fn deterministic_avg_variance(&self, t: f64) -> f64 {
        if t <= 0.0 {
            return self.v0;
        }
        let kt = self.kappa * t;
        let decay_avg = if kt.abs() < 1e-6 {
            1.0 - 0.5 * kt + kt * kt / 6.0
        } else {
            (1.0 - (-kt).exp()) / kt
        };
        self.theta + (self.v0 - self.theta) * decay_avg
    }

    /// Build `HestonParams` from the market context and error when any of the
    /// five `HESTON_*` scalars (`HESTON_KAPPA`, `HESTON_THETA`,
    /// `HESTON_SIGMA_V`, `HESTON_RHO`, `HESTON_V0`) is missing or carries a
    /// non-unitless type.
    ///
    /// Production pricers use this form so a missing or mistyped market
    /// scalar produces an error instead of falling back to [`heston_defaults`].
    ///
    /// # Errors
    ///
    /// Returns:
    /// - [`InputError::NotFound`](finstack_quant_core::InputError::NotFound) when a scalar is missing.
    /// - [`finstack_quant_core::Error::Validation`] when a scalar exists but is the wrong type.
    /// - The same validation errors as [`Self::new`] (positive κ/θ/σᵥ/v₀, ρ ∈ (−1, 1),
    ///   finite r, q) when the scalars are present but produce an invalid parameter set.
    pub fn from_market_strict(
        market: &MarketContext,
        r: f64,
        q: f64,
    ) -> finstack_quant_core::Result<Self> {
        let kappa = get_unitless_scalar_strict(market, "HESTON_KAPPA", "Heston")?;
        let theta = get_unitless_scalar_strict(market, "HESTON_THETA", "Heston")?;
        let sigma_v = get_unitless_scalar_strict(market, "HESTON_SIGMA_V", "Heston")?;
        let rho = get_unitless_scalar_strict(market, "HESTON_RHO", "Heston")?;
        let v0 = get_unitless_scalar_strict(market, "HESTON_V0", "Heston")?;
        Self::new(r, q, kappa, theta, sigma_v, rho, v0)
    }
}

/// Convert Monte Carlo Heston parameters into closed-form Fourier parameters.
///
/// This is a [`TryFrom`] (not `From`) because the conversion must re-run
/// [`HestonParams::new`] validation: the Monte Carlo `HestonParams` accepts a
/// correlation `ρ ∈ [-1, 1]` (inclusive), whereas the closed-form Fourier
/// pricer requires `ρ ∈ (-1, 1)` (exclusive). A plain `From` impl bypassed all
/// validation and could fabricate a closed-form parameter set the Fourier
/// pricer cannot handle.
impl TryFrom<finstack_quant_monte_carlo::process::heston::HestonParams> for HestonParams {
    type Error = finstack_quant_core::Error;

    fn try_from(
        value: finstack_quant_monte_carlo::process::heston::HestonParams,
    ) -> finstack_quant_core::Result<Self> {
        Self::new(
            value.r,
            value.q,
            value.kappa,
            value.theta,
            value.sigma_v,
            value.rho,
            value.v0,
        )
    }
}

/// Configuration for Heston Fourier integration.
///
/// Provides tuning knobs for the numerical integration.
#[derive(Debug, Clone, Copy)]
pub struct HestonFourierSettings {
    /// Upper limit for Fourier integral (default: 100)
    pub u_max: f64,
    /// Number of panels for composite Gauss-Legendre (default: 100)
    pub panels: usize,
    /// Gauss-Legendre order per panel (default: 16)
    pub gl_order: usize,
    /// Small epsilon to avoid singularity at φ=0 (default: 1e-8)
    pub phi_eps: f64,
}

impl Default for HestonFourierSettings {
    fn default() -> Self {
        Self {
            u_max: 100.0,
            panels: 100,
            gl_order: 16,
            phi_eps: 1e-8,
        }
    }
}

/// Gauss-Legendre orders supported by [`composite_gauss_legendre_grid`].
///
/// A `gl_order` outside this set has no node/weight table, which would make
/// [`HestonStripPricer::new`] return `None` and silently degrade to the slower
/// per-strike path. Callers must pick one of these values.
pub(super) const SUPPORTED_GL_ORDERS: [usize; 4] = [2, 4, 8, 16];

impl HestonFourierSettings {
    /// Construct validated Fourier integration settings.
    ///
    /// # Errors
    ///
    /// Returns a [`finstack_quant_core::Error::Validation`] if `gl_order` is not one
    /// of the supported composite Gauss-Legendre orders ({2, 4, 8, 16}), if
    /// `panels == 0`, or if `u_max` is not a positive finite number. An
    /// unsupported `gl_order` would otherwise cause silent degradation to the
    /// slower per-strike pricing path.
    pub fn new(
        u_max: f64,
        panels: usize,
        gl_order: usize,
        phi_eps: f64,
    ) -> finstack_quant_core::Result<Self> {
        let settings = Self {
            u_max,
            panels,
            gl_order,
            phi_eps,
        };
        settings.validate()?;
        Ok(settings)
    }

    /// Validate that these settings can drive the composite Gauss-Legendre grid.
    ///
    /// # Errors
    ///
    /// Returns a [`finstack_quant_core::Error::Validation`] if `gl_order` is not in
    /// {2, 4, 8, 16}, if `panels == 0`, or if `u_max` is not positive finite.
    pub fn validate(&self) -> finstack_quant_core::Result<()> {
        if !SUPPORTED_GL_ORDERS.contains(&self.gl_order) {
            return Err(finstack_quant_core::Error::Validation(format!(
                "HestonFourierSettings.gl_order must be one of {SUPPORTED_GL_ORDERS:?}, got {}",
                self.gl_order
            )));
        }
        if self.panels == 0 {
            return Err(finstack_quant_core::Error::Validation(
                "HestonFourierSettings.panels must be positive, got 0".to_string(),
            ));
        }
        if !self.u_max.is_finite() || self.u_max <= 0.0 {
            return Err(finstack_quant_core::Error::Validation(format!(
                "HestonFourierSettings.u_max must be a positive finite number, got {}",
                self.u_max
            )));
        }
        Ok(())
    }

    /// Create settings adapted to the option's time to maturity.
    ///
    /// Short-dated options require finer integration grids because
    /// the characteristic function oscillates more rapidly.
    ///
    /// | Maturity | u_max | panels | gl_order |
    /// |----------|-------|--------|----------|
    /// | T < 0.05 | 200   | 200    | 16       |
    /// | T < 0.25 | 150   | 150    | 16       |
    /// | T < 1.0  | 100   | 100    | 16       |
    /// | T >= 1.0 | 80    | 80     | 16       |
    ///
    /// The buckets are tuned for a typical initial variance v0 ≈ 0.04
    /// (20% vol). For low-variance regimes prefer
    /// [`HestonFourierSettings::for_maturity_with_variance`], which widens
    /// `u_max` when `v0` is small.
    ///
    /// # Arguments
    ///
    /// * `time` - Time supplied by the caller for this operation
    #[must_use]
    pub fn for_maturity(time: f64) -> Self {
        Self::for_maturity_with_variance(time, HESTON_REFERENCE_V0)
    }

    /// Create settings adapted to both maturity and initial variance.
    ///
    /// The Heston characteristic function decays on a `u`-scale proportional
    /// to `1/√(v0·T)`. The [`HestonFourierSettings::for_maturity`] buckets
    /// already widen the grid for small `T` assuming v0 ≈ 0.04 (20% vol);
    /// when `v0` itself is small the integrand tail extends past the bucket
    /// `u_max` and the truncated Gil-Pelaez integral loses mass. This variant
    /// scales `u_max` (and `panels`, to preserve node density) by
    /// `√(v0_ref / v0)`, capped to keep the grid bounded; the existing tail
    /// diagnostic remains as a safety net.
    ///
    /// # Arguments
    ///
    /// * `time` - Time supplied by the caller for this operation
    /// * `v0` - Initial variance level for the stochastic volatility process at time zero
    #[must_use]
    pub fn for_maturity_with_variance(time: f64, v0: f64) -> Self {
        let mut settings = if time < 0.05 {
            Self {
                u_max: 200.0,
                panels: 200,
                gl_order: 16,
                phi_eps: 1e-8,
            }
        } else if time < 0.25 {
            Self {
                u_max: 150.0,
                panels: 150,
                gl_order: 16,
                phi_eps: 1e-8,
            }
        } else if time < 1.0 {
            Self::default()
        } else {
            Self {
                u_max: 80.0,
                panels: 80,
                gl_order: 16,
                phi_eps: 1e-8,
            }
        };

        if v0.is_finite() && v0 > 0.0 && v0 < HESTON_REFERENCE_V0 {
            let scale = (HESTON_REFERENCE_V0 / v0)
                .sqrt()
                .min(HESTON_UMAX_MAX_VARIANCE_SCALE);
            settings.u_max *= scale;
            // Keep the per-unit-u node density unchanged so the wider grid
            // does not get coarser.
            settings.panels = ((settings.panels as f64) * scale).ceil() as usize;
        }
        settings
    }
}

/// Reference initial variance the [`HestonFourierSettings::for_maturity`]
/// buckets are tuned for (20% vol).
const HESTON_REFERENCE_V0: f64 = 0.04;

/// Cap on the variance-driven `u_max` scale factor in
/// [`HestonFourierSettings::for_maturity_with_variance`]. A factor of 8
/// covers initial variances down to `0.04 / 64 = 6.25e-4` (2.5% vol) at full
/// fidelity; below that the tail diagnostic still flags any residual
/// mis-truncation.
const HESTON_UMAX_MAX_VARIANCE_SCALE: f64 = 8.0;
