use crate::trees::tree_framework::TreeBranching;
use finstack_quant_core::Result;

/// Default normal (absolute) volatility for Ho-Lee model.
///
/// 100 basis points per year, typical for developed market government bonds
/// in a normal rate environment (2-5% rates).
pub const DEFAULT_NORMAL_VOL: f64 = 0.01; // 100 bp/yr

/// Default lognormal (relative) volatility for Black-Derman-Toy model.
///
/// 20% annualized, typical for developed market government bonds.
/// This corresponds to ~100 bp normal vol at a 5% rate level.
pub const DEFAULT_LOGNORMAL_VOL: f64 = 0.20; // 20%

/// Default maximum initial-curve repricing error for calibrated trees, in basis points.
pub const DEFAULT_CURVE_FIT_TOLERANCE_BP: f64 = 0.1;

// Short-Rate Model Types

/// Compounding convention for per-node discount factors in the short-rate tree.
///
/// | Convention | Formula | Use Case |
/// |------------|---------|----------|
/// | `Continuous` | `exp(-r * dt)` | Default; matches continuous short-rate dynamics |
/// | `Simple` | `1 / (1 + r * dt)` | Money-market / Bloomberg BDT convention |
/// | `SemiAnnual` | `(1 + r/2)^(-2 * dt)` | US bond market convention |
/// | `Quarterly` | `(1 + r/4)^(-4 * dt)` | Quarterly compounding |
/// | `Monthly` | `(1 + r/12)^(-12 * dt)` | Monthly compounding |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TreeCompounding {
    /// Continuous compounding: `df = exp(-r * dt)`.
    #[default]
    Continuous,
    /// Simple (money-market) compounding: `df = 1 / (1 + r * dt)`.
    Simple,
    /// Semi-annual compounding: `df = (1 + r/2)^(-2 * dt)`.
    SemiAnnual,
    /// Quarterly compounding: `df = (1 + r/4)^(-4 * dt)`.
    Quarterly,
    /// Monthly compounding: `df = (1 + r/12)^(-12 * dt)`.
    Monthly,
}

impl TreeCompounding {
    /// Compute the per-step discount factor for a given rate and time step.
    ///
    /// Returns a positive discount factor. For pathological inputs (e.g.,
    /// deeply negative rates with simple compounding where `1 + r*dt <= 0`),
    /// the base is clamped to a small positive value to avoid negative or
    /// NaN discount factors.
    ///
    /// # Arguments
    ///
    /// * `rate` - Rate supplied by the caller for this operation
    /// * `dt` - Dt supplied by the caller for this operation
    #[inline]
    pub fn df(self, rate: f64, dt: f64) -> f64 {
        const FLOOR: f64 = 1e-15;
        match self {
            Self::Continuous => (-rate * dt).exp(),
            Self::Simple => {
                let denom = 1.0 + rate * dt;
                1.0 / denom.max(FLOOR)
            }
            Self::SemiAnnual => {
                let base = (1.0 + rate / 2.0).max(FLOOR);
                base.powf(-2.0 * dt)
            }
            Self::Quarterly => {
                let base = (1.0 + rate / 4.0).max(FLOOR);
                base.powf(-4.0 * dt)
            }
            Self::Monthly => {
                let base = (1.0 + rate / 12.0).max(FLOOR);
                base.powf(-12.0 * dt)
            }
        }
    }

    /// Invert [`df`](Self::df): the per-step rate under this convention that
    /// reproduces the given discount factor over `dt`.
    ///
    /// Returns `rate` such that `self.df(rate, dt) = df`. For `dt ≈ 0` or a
    /// non-positive `df` the continuous-equivalent fallback is used.
    #[inline]
    pub fn rate_from_df(self, df: f64, dt: f64) -> f64 {
        if dt.abs() < f64::EPSILON || df <= 0.0 {
            tracing::warn!(
                "TreeCompounding::rate_from_df: degenerate input df={df:.6e}, dt={dt}, \
                 convention={self:?}; returning 0"
            );
            return 0.0;
        }
        match self {
            Self::Continuous => -df.ln() / dt,
            Self::Simple => (1.0 / df - 1.0) / dt,
            Self::SemiAnnual => 2.0 * (df.powf(-1.0 / (2.0 * dt)) - 1.0),
            Self::Quarterly => 4.0 * (df.powf(-1.0 / (4.0 * dt)) - 1.0),
            Self::Monthly => 12.0 * (df.powf(-1.0 / (12.0 * dt)) - 1.0),
        }
    }

    /// Convert a rate under this convention to the equivalent continuous rate.
    ///
    /// Returns `r_cont` such that `exp(-r_cont * dt) = self.df(rate, dt)`.
    #[inline]
    pub fn to_continuous(self, rate: f64, dt: f64) -> f64 {
        if dt.abs() < f64::EPSILON {
            return rate;
        }
        let d = self.df(rate, dt);
        if d > 0.0 {
            -d.ln() / dt
        } else {
            tracing::warn!(
                "TreeCompounding::to_continuous: non-positive DF {d:.6e} for rate={rate}, \
                 dt={dt}, convention={self:?}; falling back to raw rate"
            );
            rate
        }
    }
}

/// Short-rate tree model types.
///
/// Each model has distinct volatility conventions and mathematical properties:
///
/// | Model | Vol Type | Negative Rates | Mean Reversion | Use Case |
/// |-------|----------|----------------|----------------|----------|
/// | Ho-Lee | Normal | ✅ Yes | ❌ No | Low/negative rate environments |
/// | BDT/BK | Lognormal | ❌ No | ✅ Yes (κ ≠ 0 → trinomial BK lattice) | Traditional positive rate environments |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortRateModel {
    /// Ho-Lee model: Gaussian/normal short rates.
    ///
    /// ## Rate Dynamics
    /// ```text
    /// dr = θ(t)dt + σdW
    /// ```
    /// where:
    /// - `θ(t)` is calibrated to match the discount curve
    /// - `σ` is the **normal volatility** (absolute, in rate units like 0.01 = 100 bp)
    ///
    /// ## Properties
    /// - ✅ Handles negative rates naturally
    /// - ❌ No mean reversion (rates can drift arbitrarily)
    /// - Analytically tractable
    ///
    /// ## Typical Volatility Range
    /// - Low rates (<2%): 50-80 bp (0.005-0.008)
    /// - Normal rates (2-5%): 80-120 bp (0.008-0.012)
    /// - High rates (>5%): 100-150 bp (0.010-0.015)
    /// - Crisis: 150-300 bp (0.015-0.030)
    HoLee,

    /// Black-Derman-Toy / Black-Karasinski model: Lognormal short rates.
    ///
    /// ## Rate Dynamics
    /// ```text
    /// d(ln r) = [θ(t) - κ ln r] dt + σ dW
    /// ```
    /// where:
    /// - `θ(t)` is calibrated to match the discount curve
    /// - `σ` is the **lognormal volatility** (relative, like 0.20 = 20%)
    /// - `κ` is the mean reversion speed (0 recovers standard BDT)
    ///
    /// ## Properties
    /// - ❌ Cannot handle negative rates (rates stay positive)
    /// - When `κ = 0`: standard BDT with constant lognormal volatility on a
    ///   binomial lattice
    /// - When `κ > 0`: Black-Karasinski on a trinomial lattice in x = ln r
    ///   (Hull-White geometry with edge branch switching); terminal log-rate
    ///   dispersion tightens toward `σ√((1-e^{-2κT})/(2κ))`
    /// - Lognormal distribution matches cap/floor market conventions
    ///
    /// ## Typical Volatility Range
    /// - Low vol environment: 10-15% (0.10-0.15)
    /// - Normal market: 15-25% (0.15-0.25)
    /// - High vol/stress: 25-40% (0.25-0.40)
    ///
    /// ## Important
    /// ⚠️ The default 1% volatility in older code is **far too low** for BDT.
    /// Use [`DEFAULT_LOGNORMAL_VOL`] (20%) or calibrate to swaption market.
    BlackDermanToy,
}

/// Configuration for short-rate tree construction.
///
/// # Volatility Convention
///
/// ⚠️ **Critical**: The `volatility` field has different interpretations depending on the model:
///
/// | Model | Volatility Type | Example |
/// |-------|-----------------|---------|
/// | [`ShortRateModel::HoLee`] | Normal (absolute) | 0.01 = 100 bp/yr |
/// | [`ShortRateModel::BlackDermanToy`] | Lognormal (relative) | 0.20 = 20%/yr |
///
/// Use the helper constructors ([`ShortRateTreeConfig::ho_lee`], [`ShortRateTreeConfig::bdt`])
/// or `finstack_quant_core::math::volatility::convert_atm_volatility` to avoid convention errors.
///
/// # Examples
///
/// ```
/// use finstack_quant_models::trees::short_rate_tree::{
///     ShortRateTreeConfig, ShortRateModel, DEFAULT_NORMAL_VOL, DEFAULT_LOGNORMAL_VOL,
/// };
///
/// // Ho-Lee with 100 bp normal vol (recommended for negative rate environments)
/// let ho_lee = ShortRateTreeConfig::ho_lee(100, 0.01);
/// assert_eq!(ho_lee.model, ShortRateModel::HoLee);
///
/// // BDT with 20% lognormal vol (recommended for positive rate environments)
/// let bdt = ShortRateTreeConfig::bdt(100, 0.20, 0.03);
/// assert_eq!(bdt.model, ShortRateModel::BlackDermanToy);
///
/// // Use defaults with model-appropriate volatility
/// let ho_lee_default = ShortRateTreeConfig::default_ho_lee(100);
/// assert_eq!(ho_lee_default.volatility, DEFAULT_NORMAL_VOL);
///
/// let bdt_default = ShortRateTreeConfig::default_bdt(100);
/// assert_eq!(bdt_default.volatility, DEFAULT_LOGNORMAL_VOL);
/// ```
#[derive(Debug, Clone)]
pub struct ShortRateTreeConfig {
    /// Number of time steps in the tree.
    ///
    /// More steps improve accuracy but increase computation time O(n²).
    /// Typical values: 50 (fast), 100 (standard), 200+ (high precision).
    pub steps: usize,

    /// Tree model type determining rate dynamics and volatility interpretation.
    pub model: ShortRateModel,

    /// Interest rate volatility (annualized).
    ///
    /// ⚠️ **Interpretation depends on model**:
    /// - **Ho-Lee**: Normal volatility in rate units (0.01 = 100 bp/yr)
    /// - **BDT**: Lognormal volatility as proportion (0.20 = 20%/yr)
    ///
    /// See [`ShortRateModel`] for typical ranges per model type.
    pub volatility: f64,

    /// Mean reversion parameter.
    ///
    /// Controls how quickly rates revert to the long-term mean.
    /// - Typical values: 0.01-0.10 (1-10% per year)
    /// - Higher values = faster reversion, less rate dispersion
    /// - Ho-Lee: not supported (breaks lattice recombination); use
    ///   `HullWhiteTree` for mean-reverting normal models
    /// - BDT/Black-Karasinski: κ = 0 calibrates standard binomial BDT;
    ///   κ > 0 calibrates a trinomial Black-Karasinski lattice in x = ln r
    pub mean_reversion: Option<f64>,

    /// Tree branching type (binomial or trinomial).
    ///
    /// - **Binomial**: Standard two-branch tree (up/down)
    /// - **Trinomial**: Three-branch tree (up/mid/down) for models with
    ///   trinomial calibration support
    ///
    /// Default: Binomial. Use trinomial only with a matching calibrated lattice.
    pub branching: TreeBranching,

    /// Per-node discount factor convention.
    ///
    /// Controls whether calibration and pricing use continuous `exp(-r*dt)` or
    /// simple `1/(1+r*dt)` compounding. Bloomberg's lognormal OAS model uses
    /// simple compounding; the default is continuous compounding.
    pub compounding: TreeCompounding,

    /// Maximum permitted initial-curve repricing error, in basis points.
    ///
    /// BDT calibration fails rather than returning a tree when this tolerance
    /// is exceeded. The default is 0.1 bp.
    pub curve_fit_tolerance_bp: f64,
}

impl Default for ShortRateTreeConfig {
    /// Default configuration using Ho-Lee model with appropriate normal volatility.
    ///
    /// For BDT model, use [`ShortRateTreeConfig::default_bdt`] instead.
    fn default() -> Self {
        Self::default_ho_lee(100)
    }
}

impl ShortRateTreeConfig {
    /// Create a Ho-Lee configuration with specified normal volatility.
    ///
    /// Uses binomial branching by default. For trinomial branching,
    /// use [`with_trinomial`](Self::with_trinomial) after construction.
    ///
    /// # Arguments
    ///
    /// * `steps` - Number of tree steps (50-200 typical)
    /// * `normal_vol` - Normal volatility in rate units (e.g., 0.01 = 100 bp/yr)
    ///
    /// # Examples
    ///
    /// ```
    /// use finstack_quant_models::trees::short_rate_tree::ShortRateTreeConfig;
    ///
    /// // 100 steps, 80 bp normal vol
    /// let config = ShortRateTreeConfig::ho_lee(100, 0.008);
    /// ```
    pub fn ho_lee(steps: usize, normal_vol: f64) -> Self {
        Self {
            steps,
            model: ShortRateModel::HoLee,
            volatility: normal_vol,
            mean_reversion: None,
            branching: TreeBranching::Binomial,
            compounding: TreeCompounding::default(),
            curve_fit_tolerance_bp: DEFAULT_CURVE_FIT_TOLERANCE_BP,
        }
    }

    /// Create a Black-Derman-Toy / Black-Karasinski configuration.
    ///
    /// Uses binomial branching with state-price recursion calibration.
    ///
    /// # Arguments
    ///
    /// * `steps` - Number of tree steps (50-200 typical)
    /// * `lognormal_vol` - Lognormal volatility (e.g., 0.20 = 20%/yr)
    /// * `mean_reversion` - Mean reversion speed; `0.0` calibrates standard
    ///   binomial BDT, any positive value calibrates a trinomial
    ///   Black-Karasinski lattice in x = ln r
    ///
    /// # Examples
    ///
    /// ```
    /// use finstack_quant_models::trees::short_rate_tree::ShortRateTreeConfig;
    ///
    /// // 100 steps, 20% lognormal vol
    /// let config = ShortRateTreeConfig::bdt(100, 0.20, 0.0);
    /// ```
    pub fn bdt(steps: usize, lognormal_vol: f64, mean_reversion: f64) -> Self {
        Self {
            steps,
            model: ShortRateModel::BlackDermanToy,
            volatility: lognormal_vol,
            mean_reversion: Some(mean_reversion),
            branching: TreeBranching::Binomial,
            compounding: TreeCompounding::default(),
            curve_fit_tolerance_bp: DEFAULT_CURVE_FIT_TOLERANCE_BP,
        }
    }

    /// Set the per-node compounding convention.
    #[must_use]
    pub fn with_compounding(mut self, compounding: TreeCompounding) -> Self {
        self.compounding = compounding;
        self
    }

    /// Set the maximum permitted initial-curve repricing error.
    ///
    /// # Arguments
    ///
    /// * `tolerance_bp` - Positive finite error tolerance in basis points.
    pub fn with_curve_fit_tolerance_bp(mut self, tolerance_bp: f64) -> Result<Self> {
        if !tolerance_bp.is_finite() || tolerance_bp <= 0.0 {
            return Err(finstack_quant_core::Error::Validation(format!(
                "short-rate tree curve-fit tolerance must be finite and positive, got {tolerance_bp}"
            )));
        }
        self.curve_fit_tolerance_bp = tolerance_bp;
        Ok(self)
    }

    /// Create Ho-Lee configuration with default normal volatility (100 bp).
    ///
    /// Suitable for developed market government bonds in normal rate environments.
    pub fn default_ho_lee(steps: usize) -> Self {
        Self::ho_lee(steps, DEFAULT_NORMAL_VOL)
    }

    /// Create BDT configuration with default lognormal volatility (20%).
    ///
    /// Suitable for developed market government bonds with positive rates.
    /// Uses the non-mean-reverting (κ = 0) binomial BDT calibration.
    ///
    /// # Arguments
    ///
    /// * `steps` - Steps supplied by the caller for this operation
    pub fn default_bdt(steps: usize) -> Self {
        Self::bdt(steps, DEFAULT_LOGNORMAL_VOL, 0.0)
    }

    /// Set trinomial branching.
    ///
    /// The selected model must calibrate a matching `2 * step + 1` lattice.
    #[must_use]
    pub fn with_trinomial(mut self) -> Self {
        self.branching = TreeBranching::Trinomial;
        self
    }

    /// Set binomial branching (standard two-branch tree).
    #[must_use]
    pub fn with_binomial(mut self) -> Self {
        self.branching = TreeBranching::Binomial;
        self
    }

    /// Create configuration from normal volatility, automatically selecting
    /// the appropriate model based on rate environment.
    ///
    /// # Arguments
    ///
    /// * `steps` - Number of tree steps
    /// * `normal_vol` - Normal volatility in rate units (e.g., 0.01 = 100 bp)
    /// * `rate_level` - Current/reference rate level for model selection
    ///
    /// # Model Selection
    ///
    /// - If `rate_level < 0.01` (1%): Uses Ho-Lee (handles negative rates)
    /// - Otherwise: Uses BDT with converted lognormal vol
    ///
    /// # Examples
    ///
    /// ```
    /// use finstack_quant_models::trees::short_rate_tree::{
    ///     ShortRateTreeConfig, ShortRateModel,
    /// };
    ///
    /// // Low rate environment → Ho-Lee
    /// let config = ShortRateTreeConfig::from_normal_vol(100, 0.008, 0.005)?;
    /// assert_eq!(config.model, ShortRateModel::HoLee);
    ///
    /// // Normal rate environment → BDT with converted vol
    /// let config = ShortRateTreeConfig::from_normal_vol(100, 0.01, 0.05)?;
    /// assert_eq!(config.model, ShortRateModel::BlackDermanToy);
    /// // Vol should be approximately 20% (price-matching conversion)
    /// assert!(config.volatility > 0.15 && config.volatility < 0.25);
    /// # Ok::<(), finstack_quant_core::Error>(())
    /// ```
    pub fn from_normal_vol(steps: usize, normal_vol: f64, rate_level: f64) -> Result<Self> {
        if rate_level < 0.01 {
            // Low/negative rate environment: use Ho-Lee
            Ok(Self::ho_lee(steps, normal_vol))
        } else {
            // Positive rate environment: use BDT with converted vol
            let lognormal_vol = finstack_quant_core::math::volatility::convert_atm_volatility(
                normal_vol,
                finstack_quant_core::math::volatility::VolatilityConvention::Normal,
                finstack_quant_core::math::volatility::VolatilityConvention::Lognormal,
                rate_level,
                1.0,
            )?;
            Ok(Self::bdt(steps, lognormal_vol, 0.0))
        }
    }
}
