use std::sync::Arc;

use finstack_quant_core::market_data::traits::Discounting;
use finstack_quant_core::types::CurveId;
use finstack_quant_core::{Error, Result};

use crate::models::trees::tree_framework::TreeBranching;

use super::black_karasinski::BkTrinomialLattice;
use super::{ShortRateModel, ShortRateTreeConfig};

/// Result of short-rate tree calibration with quality metrics.
///
/// Provides diagnostic information about calibration quality, allowing
/// users to assess whether the tree is suitable for their use case.
#[derive(Debug, Clone, Default)]
pub struct TreeCalibrationResult {
    /// Maximum calibration error in basis points.
    pub max_error_bp: f64,
    /// Step at which maximum error occurred.
    pub max_error_step: usize,
    /// Number of steps where the solver failed and fallback was used.
    pub fallback_count: usize,
    /// Whether calibration completed successfully.
    pub converged: bool,
}

impl TreeCalibrationResult {
    /// Returns true if calibration quality is acceptable (max error < 1bp, no fallbacks).
    #[must_use]
    pub fn is_acceptable(&self) -> bool {
        self.converged && self.max_error_bp < 1.0 && self.fallback_count == 0
    }

    /// Returns true if calibration quality is good (max error < 0.1bp).
    #[must_use]
    pub fn is_good(&self) -> bool {
        self.converged && self.max_error_bp < 0.1 && self.fallback_count == 0
    }
}

/// Short-rate tree for valuing bonds with embedded options
#[derive(Debug, Clone)]
pub struct ShortRateTree {
    pub(super) config: ShortRateTreeConfig,
    /// Calibrated short rates at each node: `rates[step][node]`
    pub(super) rates: Arc<Vec<Vec<f64>>>,
    /// Transition probabilities: `probs[step]` gives (p_up, p_down) for that step
    pub(super) probs: Vec<(f64, f64)>,
    /// Time steps in years
    pub(super) time_steps: Vec<f64>,
    /// Discount curve used for calibration
    pub(super) calibration_curve_id: CurveId,
    /// Calibration quality metrics (populated after calibration).
    pub(super) calibration_quality: Option<TreeCalibrationResult>,
    /// Trinomial Black-Karasinski lattice (set when BDT model has κ ≠ 0).
    pub(super) bk_trinomial: Option<BkTrinomialLattice>,
}

impl ShortRateTree {
    /// Create a new short-rate tree with the given configuration.
    pub fn new(config: ShortRateTreeConfig) -> Self {
        Self {
            config,
            rates: Arc::new(Vec::new()),
            probs: Vec::new(),
            time_steps: Vec::new(),
            calibration_curve_id: CurveId::new(""),
            calibration_quality: None,
            bk_trinomial: None,
        }
    }

    /// Returns the calibration result if calibration has been performed.
    ///
    /// # Returns
    ///
    /// - `Some(TreeCalibrationResult)` with quality metrics if calibrated
    /// - `None` if not yet calibrated
    #[must_use]
    pub fn calibration_result(&self) -> Option<&TreeCalibrationResult> {
        self.calibration_quality.as_ref()
    }

    /// Create a Ho-Lee tree with specified normal (absolute) volatility.
    ///
    /// # Arguments
    ///
    /// * `steps` - Number of tree steps (50-200 typical)
    /// * `normal_vol` - Normal volatility in rate units (e.g., 0.01 = 100 bp/yr)
    ///
    /// # Examples
    ///
    /// ```
    /// use finstack_quant_valuations::models::trees::short_rate_tree::ShortRateTree;
    ///
    /// // Ho-Lee with 100 bp annual volatility
    /// let tree = ShortRateTree::ho_lee(100, 0.01);
    /// ```
    pub fn ho_lee(steps: usize, normal_vol: f64) -> Self {
        Self::new(ShortRateTreeConfig::ho_lee(steps, normal_vol))
    }

    /// Create a Black-Derman-Toy tree with specified lognormal (relative) volatility.
    ///
    /// # Arguments
    ///
    /// * `steps` - Number of tree steps (50-200 typical)
    /// * `lognormal_vol` - Lognormal volatility (e.g., 0.20 = 20%/yr)
    /// * `mean_reversion` - `0.0` for standard binomial BDT; positive values
    ///   calibrate a trinomial Black-Karasinski lattice in x = ln r
    ///
    /// # Examples
    ///
    /// ```
    /// use finstack_quant_valuations::models::trees::short_rate_tree::ShortRateTree;
    ///
    /// // BDT with 20% lognormal volatility
    /// let tree = ShortRateTree::black_derman_toy(100, 0.20, 0.0);
    /// ```
    ///
    /// # Warning
    ///
    /// ⚠️ The volatility parameter is **lognormal** (relative), not normal (absolute).
    /// A value of 0.20 means 20% annual rate volatility, not 20 bp.
    /// Use `finstack_quant_core::math::volatility::convert_atm_volatility` to convert from normal if needed.
    pub fn black_derman_toy(steps: usize, lognormal_vol: f64, mean_reversion: f64) -> Self {
        Self::new(ShortRateTreeConfig::bdt(
            steps,
            lognormal_vol,
            mean_reversion,
        ))
    }

    /// Create a Ho-Lee tree with default normal volatility (100 bp).
    pub fn default_ho_lee(steps: usize) -> Self {
        Self::new(ShortRateTreeConfig::default_ho_lee(steps))
    }

    /// Create a BDT tree with default lognormal volatility (20%).
    pub fn default_bdt(steps: usize) -> Self {
        Self::new(ShortRateTreeConfig::default_bdt(steps))
    }

    /// Calibrate the tree to match a given discount curve.
    ///
    /// The `curve_id` is stored so that
    /// [`calculate_greeks`](crate::models::trees::TreeModel::calculate_greeks) can
    /// look up the curve from the `MarketContext` when recalibrating bumped trees
    /// for vega and theta.
    pub fn calibrate(
        &mut self,
        curve_id: &CurveId,
        discount_curve: &dyn Discounting,
        time_to_maturity: f64,
    ) -> Result<()> {
        self.calibration_curve_id = curve_id.clone();

        let dt = time_to_maturity / self.config.steps as f64;
        self.time_steps = (0..=self.config.steps).map(|i| i as f64 * dt).collect();

        let mut rates = vec![Vec::new(); self.config.steps + 1];
        self.probs = vec![(0.5, 0.5); self.config.steps]; // Default to equal probabilities
        self.bk_trinomial = None;

        match self.config.model {
            ShortRateModel::HoLee => self.calibrate_ho_lee(&mut rates, discount_curve, dt)?,
            ShortRateModel::BlackDermanToy => {
                let kappa = self.config.mean_reversion.unwrap_or(0.0);
                if kappa < 0.0 {
                    return Err(Error::Validation(format!(
                        "Black-Karasinski mean reversion must be non-negative, got {kappa}"
                    )));
                }
                if kappa.abs() < 1e-12 {
                    // κ = 0: standard binomial BDT calibration.
                    self.calibrate_bdt(&mut rates, discount_curve, dt)?;
                } else {
                    // κ ≠ 0: genuine trinomial Black-Karasinski lattice in
                    // x = ln r .
                    self.calibrate_bk_trinomial(&mut rates, discount_curve, dt, kappa)?;
                }
            }
        }

        self.rates = Arc::new(rates);

        Ok(())
    }

    /// Get the short rate at a specific node.
    ///
    /// # Node Ordering
    ///
    /// The ordering convention differs by model:
    ///
    /// | Model | Node 0 | Node N |
    /// |-------|--------|--------|
    /// | Ho-Lee | **lowest** rate | **highest** rate |
    /// | BDT (κ = 0, binomial) | **highest** rate (`α·u^(n-1)`) | **lowest** rate (`α·u^(-(n-1))`) |
    /// | BK (κ ≠ 0, trinomial) | **lowest** rate (j = −j_max) | **highest** rate (j = +j_max) |
    pub fn rate_at_node(&self, step: usize, node: usize) -> Result<f64> {
        if step >= self.rates.len() || node >= self.rates[step].len() {
            return Err(Error::internal(format!(
                "short-rate tree node out of bounds: step={step}, node={node}"
            )));
        }
        Ok(self.rates[step][node])
    }

    /// Get transition probabilities at a step
    pub fn probabilities(&self, step: usize) -> Result<(f64, f64)> {
        if step >= self.probs.len() {
            return Err(Error::internal(format!(
                "short-rate tree probability row out of bounds: step={step}"
            )));
        }
        Ok(self.probs[step])
    }

    /// Get time at step
    pub fn time_at_step(&self, step: usize) -> Result<f64> {
        if step >= self.time_steps.len() {
            return Err(Error::internal(format!(
                "short-rate tree time step out of bounds: step={step}"
            )));
        }
        Ok(self.time_steps[step])
    }

    fn expected_nodes_at_step(branching: TreeBranching, step: usize) -> usize {
        match branching {
            TreeBranching::Binomial => step + 1,
            TreeBranching::Trinomial => 2 * step + 1,
        }
    }

    pub(super) fn validate_lattice_geometry(&self) -> Result<()> {
        if self.rates.len() != self.config.steps + 1 {
            return Err(Error::internal(format!(
                "short-rate tree lattice geometry mismatch: expected {} rate rows, got {}",
                self.config.steps + 1,
                self.rates.len()
            )));
        }

        // Black-Karasinski trinomial lattice: width grows 2·step+1 until the
        // j_max cap, then stays at 2·j_max+1.
        if let Some(lattice) = &self.bk_trinomial {
            for (step, rates_at_step) in self.rates.iter().enumerate() {
                let expected = 2 * step.min(lattice.j_max) + 1;
                if rates_at_step.len() != expected {
                    return Err(Error::internal(format!(
                        "Black-Karasinski lattice geometry mismatch: step {} expected {} \
                         nodes, got {}",
                        step,
                        expected,
                        rates_at_step.len()
                    )));
                }
            }
            return Ok(());
        }

        for (step, rates_at_step) in self.rates.iter().enumerate() {
            let expected = Self::expected_nodes_at_step(self.config.branching, step);
            if rates_at_step.len() != expected {
                return Err(Error::internal(format!(
                    "short-rate tree lattice geometry mismatch for {:?}: step {} expected {} nodes, got {}",
                    self.config.branching,
                    step,
                    expected,
                    rates_at_step.len()
                )));
            }
        }

        Ok(())
    }
}
