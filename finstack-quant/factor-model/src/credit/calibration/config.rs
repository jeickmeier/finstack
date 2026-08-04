use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::credit::hierarchy::{CreditHierarchySpec, IssuerBetaPolicy};

/// Whether the calibrator works in price-difference (return) or raw-level space.
///
/// `Returns` (the default) matches the spec's reference math: `r_i(t) =
/// S_i(t) - S_i(t-1)` and the generic factor is differenced the same way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PanelSpace {
    /// Difference consecutive observations into a return panel before peeling.
    #[default]
    Returns,
    /// Use the raw level panel as-is.
    Levels,
}

/// Volatility model selector for the per-factor variance forecast.
///
/// `Sample` is the plain (unbiased) sample variance. `Ewma` is the RiskMetrics
/// finite-window exponentially weighted variance estimator (Longerstaey &
/// Spencer, 1996, §5.2): both are fully supported by the calibrator.
///
/// The two differ in centering: `Sample` demeans the series before squaring
/// (the usual `Var(x) = E[(x − x̄)²]`), while `Ewma` does not — it recurses
/// directly on squared observations (`σ²_t = λσ²_{t−1} + (1−λ)r²_{t−1}`),
/// matching the RiskMetrics convention of treating financial return series as
/// zero-mean. That convention only holds for a *return* panel; combining
/// `Ewma` with a raw levels panel
/// ([`PanelSpace::Levels`]) is rejected by
/// `validate_calibration_config` because the squared-level mean-square is
/// dominated by the level itself, not its dispersion.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum VolModelChoice {
    /// Plain sample variance (unbiased, Bessel-corrected); demeans before
    /// squaring.
    Sample,
    /// RiskMetrics exponentially weighted moving-average variance.
    ///
    /// Uncentered (zero-mean) squared returns, no demeaning step — see the
    /// enum-level docs above. Implemented by `ewma_variance`. `lambda` must
    /// be in the open interval `(0, 1)`; validated by
    /// `validate_calibration_config`.
    ///
    /// # References
    ///
    /// - Longerstaey, J., & Spencer, M. (1996). *RiskMetrics — Technical
    ///   Document* (4th ed.). J.P. Morgan/Reuters. §5.2.
    Ewma {
        /// Smoothing parameter λ ∈ (0, 1) (RiskMetrics daily default 0.94).
        #[schemars(extend("exclusiveMinimum" = 0.0, "exclusiveMaximum" = 1.0))]
        lambda: f64,
    },
}

/// Strategy for assembling the factor covariance matrix.
///
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CovarianceStrategy {
    /// Diagonal Σ = diag(σ²) under identity correlation.
    Diagonal,
    /// Sample correlation (PSD-repaired if needed) plus diagonal ridge:
    /// Σ = D·ρ·D + α·I. Requires `alpha >= 0`. See design spec §4.1.
    Ridge {
        /// Ridge regularisation parameter; must be `>= 0`.
        #[schemars(range(min = 0.0))]
        alpha: f64,
    },
    /// Full sample covariance with PSD repair via nearest-correlation projection:
    /// Σ = D·ρ_repaired·D. See design spec §4.1.
    FullSampleRepaired,
    /// Ledoit-Wolf (2004) identity-target shrinkage over complete-case
    /// observations: `Σ = annualization_factor · (δ*·μ·I + (1 − δ*)·S)` with
    /// the analytic optimal intensity `δ*`, and `ρ` derived from `Σ`.
    ///
    /// Only dates where **every** factor is observed enter the estimate;
    /// calibration fails with a validation error when fewer than 2 such dates
    /// exist (use [`CovarianceStrategy::Ridge`] or
    /// [`CovarianceStrategy::FullSampleRepaired`] for very sparse panels).
    ///
    /// The resulting `config.covariance` is authoritative for point-in-time
    /// risk but diverges from the vol-forecast rebuild `D·ρ·D` on both the
    /// diagonal and off-diagonal, because `vol_state` variances come from a
    /// different estimator (the configured
    /// [`VolModelChoice`]) over a different observation set (per-factor
    /// all-available rows, not the complete-case rows used here). See
    /// [`CreditFactorModel::static_correlation`][crate::credit::hierarchy::CreditFactorModel::static_correlation]
    /// for the full explanation.
    ///
    /// Reference: Ledoit, O., & Wolf, M. (2004). "A well-conditioned estimator
    /// for large-dimensional covariance matrices." *Journal of Multivariate
    /// Analysis*, 88(2), 365–411.
    LedoitWolf,
}

/// OLS β shrinkage rule.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum BetaShrinkage {
    /// No shrinkage; use the OLS estimate directly.
    None,
    /// Convex shrinkage toward 1.0: `β ← (1 - α) · β_fit + α · 1.0`.
    TowardOne {
        /// Shrinkage weight in `[0, 1]`.
        #[schemars(range(min = 0.0, max = 1.0))]
        alpha: f64,
    },
}

/// Per-level minimum-bucket-size thresholds used to gate fold-up of sparse
/// hierarchy buckets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BucketSizeThresholds {
    /// Threshold per hierarchy level. Levels beyond `per_level.len()` use the
    /// default of 5.
    pub per_level: Vec<usize>,
}

impl BucketSizeThresholds {
    pub(super) fn threshold_for_level(&self, k: usize) -> usize {
        self.per_level.get(k).copied().unwrap_or(5)
    }

    /// Default thresholds for `n` hierarchy levels (5 per level).
    #[must_use]
    pub fn default_for_levels(n: usize) -> Self {
        Self {
            per_level: vec![5; n],
        }
    }
}

/// Configuration for the calibrator.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreditCalibrationConfig {
    /// Issuer-beta classification policy.
    pub policy: IssuerBetaPolicy,
    /// Hierarchy specification (broadest → narrowest).
    pub hierarchy: CreditHierarchySpec,
    /// Per-level minimum-bucket-size thresholds.
    pub min_bucket_size_per_level: BucketSizeThresholds,
    /// Vol-model choice for the per-factor variance forecast (sample or EWMA).
    pub vol_model: VolModelChoice,
    /// Covariance assembly strategy.
    pub covariance_strategy: CovarianceStrategy,
    /// Optional shrinkage applied to OLS β estimates.
    pub beta_shrinkage: BetaShrinkage,
    /// Whether to differentiate the panel before peeling.
    pub use_returns_or_levels: PanelSpace,
    /// Annualization factor for sample variance (default 12.0 ≈ monthly data).
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub annualization_factor: f64,
}

impl Default for CreditCalibrationConfig {
    fn default() -> Self {
        Self {
            policy: IssuerBetaPolicy::GloballyOff,
            hierarchy: CreditHierarchySpec { levels: vec![] },
            min_bucket_size_per_level: BucketSizeThresholds { per_level: vec![] },
            vol_model: VolModelChoice::Sample,
            covariance_strategy: CovarianceStrategy::Diagonal,
            beta_shrinkage: BetaShrinkage::None,
            use_returns_or_levels: PanelSpace::Returns,
            annualization_factor: 12.0,
        }
    }
}
