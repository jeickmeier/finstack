use std::collections::BTreeMap;

use finstack_quant_core::dates::Date;
use finstack_quant_core::types::IssuerId;

use super::config::{
    BetaShrinkage, BucketSizeThresholds, CovarianceStrategy, CreditCalibrationConfig, PanelSpace,
    VolModelChoice,
};
use super::inputs::HistoryPanel;
use super::statistics::{
    ewma_variance, factor_variances, ledoit_wolf_cov_and_corr, sample_correlation_flat,
};
use super::validation::validate_calibration_config;
use crate::credit::hierarchy::{CreditHierarchySpec, IssuerBetaPolicy};
use crate::FactorId;

/// Audit item #5: on a sparse panel the pairwise-overlap mean differs from
/// each factor's marginal mean. Demeaning the covariance with the marginal
/// mean (the previous behavior) is not a Pearson correlation; a perfectly
/// co-moving pair on the overlap window must come back as ρ = +1.
#[test]
fn sample_correlation_uses_pairwise_overlap_mean_on_sparse_panel() {
    // Factor A: observed on dates 0..4. Factor B: observed on dates 2..4.
    // On the overlap (dates 2,3) the two move identically, so the true
    // pairwise correlation is exactly +1. A's marginal mean (over 4 obs)
    // is far from its overlap mean (over 2 obs) — the bug would produce a
    // biased value, possibly even > 1 before the clamp.
    let a = FactorId::new("credit::generic");
    let b = FactorId::new("credit::bucket::A");
    let mut returns: BTreeMap<FactorId, Vec<Option<f64>>> = BTreeMap::new();
    returns.insert(
        a.clone(),
        vec![Some(-5.0), Some(-4.0), Some(1.0), Some(2.0)],
    );
    returns.insert(b.clone(), vec![None, None, Some(1.0), Some(2.0)]);

    let order = vec![a, b];
    let rho = sample_correlation_flat(&order, &returns);

    // 2x2 flat row-major: [aa, ab, ba, bb].
    assert_eq!(rho.len(), 4);
    assert!((rho[0] - 1.0).abs() < 1e-12, "diagonal must be 1.0");
    assert!((rho[3] - 1.0).abs() < 1e-12, "diagonal must be 1.0");
    assert!(
        (rho[1] - 1.0).abs() < 1e-9,
        "perfectly co-moving overlap must give correlation +1, got {}",
        rho[1]
    );
    assert!(
        (rho[1] - rho[2]).abs() < 1e-15,
        "correlation matrix must be symmetric"
    );
}

/// A perfectly anti-correlated overlap must give ρ = −1 regardless of the
/// (different) marginal means.
#[test]
fn sample_correlation_handles_anti_correlated_overlap() {
    let a = FactorId::new("credit::generic");
    let b = FactorId::new("credit::bucket::A");
    let mut returns: BTreeMap<FactorId, Vec<Option<f64>>> = BTreeMap::new();
    returns.insert(a.clone(), vec![Some(10.0), Some(1.0), Some(2.0), Some(3.0)]);
    returns.insert(b.clone(), vec![None, Some(-1.0), Some(-2.0), Some(-3.0)]);

    let order = vec![a, b];
    let rho = sample_correlation_flat(&order, &returns);
    assert!(
        (rho[1] - (-1.0)).abs() < 1e-9,
        "perfectly anti-correlated overlap must give -1, got {}",
        rho[1]
    );
}

/// Audit item #12: `factor_variances` must use the unbiased (`n − 1`)
/// sample-variance estimator, not population (`÷ n`).
#[test]
fn factor_variances_use_unbiased_n_minus_one_estimator() {
    // Series {0, 2}: mean 1, Σ(dev²) = 1 + 1 = 2.
    //   population variance = 2/2 = 1.0
    //   unbiased  variance  = 2/1 = 2.0
    let fid = FactorId::new("credit::generic");
    let mut returns: BTreeMap<FactorId, Vec<Option<f64>>> = BTreeMap::new();
    returns.insert(fid.clone(), vec![Some(0.0), Some(2.0)]);

    let out = factor_variances(&returns, VolModelChoice::Sample, 1.0);
    let var = *out.get(&fid).expect("variance present");
    assert!(
        (var - 2.0).abs() < 1e-12,
        "expected unbiased variance 2.0 (n-1), got {var}"
    );
}

/// RiskMetrics finite-window EWMA (Longerstaey & Spencer 1996, §5.2),
/// zero-mean convention, normalized weights w_t ∝ λ^{T−1−t}.
///
/// λ = 0.5, returns (oldest → newest) [2, 1]:
///   raw weights (1−λ)·λ^{T−1−t} = [0.25, 0.5], sum = 1 − λ² = 0.75
///   normalized  = [1/3, 2/3]
///   σ² = (1/3)·4 + (2/3)·1 = 2.0
#[test]
fn ewma_variance_matches_hand_worked_recursion() {
    let series = vec![Some(2.0), Some(1.0)];
    let var = ewma_variance(&series, 0.5, 1.0);
    assert!((var - 2.0).abs() < 1e-12, "expected 2.0, got {var}");
}

/// Sparse entries are skipped; annualization multiplies the per-period
/// variance. Same data as above with a gap and ×12 → 24.0.
#[test]
fn ewma_variance_skips_missing_observations_and_annualizes() {
    let series = vec![Some(2.0), None, Some(1.0)];
    let var = ewma_variance(&series, 0.5, 12.0);
    assert!((var - 24.0).abs() < 1e-12, "expected 24.0, got {var}");
}

/// λ = 0.5, returns [1, 1, 3]:
///   raw weights = [0.125, 0.25, 0.5], sum = 1 − 0.5³ = 0.875
///   σ² = (0.125·1 + 0.25·1 + 0.5·9)/0.875 = 4.875/0.875 = 39/7
/// Recency weighting must overweight the large final move relative to the
/// equally-weighted mean of squares (11/3).
#[test]
fn ewma_variance_weights_recent_observations() {
    let series = vec![Some(1.0), Some(1.0), Some(3.0)];
    let var = ewma_variance(&series, 0.5, 1.0);
    assert!((var - 39.0 / 7.0).abs() < 1e-12, "expected 39/7, got {var}");
}

/// Fewer than 2 valid observations → 0.0 (same fallback as
/// `factor_variances`).
#[test]
fn ewma_variance_insufficient_history_is_zero() {
    assert_eq!(ewma_variance(&[Some(5.0)], 0.94, 12.0), 0.0);
    assert_eq!(ewma_variance(&[None, None], 0.94, 12.0), 0.0);
}

/// Golden Ledoit-Wolf adapter test — same panel as the hand-worked example
/// in `core::math::linalg::ledoit_wolf_shrinkage`:
///   per-period Σ* = [[2.5, −1/12], [−1/12, 2.5]], δ* = 17/18.
/// With annualization_factor = 12 the stored covariance is
///   Σ_ann = [[30, −1], [−1, 30]],
/// and the (scale-invariant) correlation is −(1/12)/2.5 = −1/30.
#[test]
fn ledoit_wolf_adapter_matches_hand_worked_example() {
    let a = FactorId::new("credit::generic");
    let b = FactorId::new("credit::bucket::IG");
    let mut returns: BTreeMap<FactorId, Vec<Option<f64>>> = BTreeMap::new();
    returns.insert(
        a.clone(),
        vec![Some(1.0), Some(-1.0), Some(2.0), Some(-2.0)],
    );
    returns.insert(
        b.clone(),
        vec![Some(1.0), Some(-1.0), Some(-2.0), Some(2.0)],
    );
    let order = vec![a, b];

    let (corr, cov) = ledoit_wolf_cov_and_corr(&order, &returns, 12.0).expect("dense panel");

    // Row-major flat covariance [aa, ab, ba, bb].
    assert!((cov[0] - 30.0).abs() < 1e-12, "cov[0] = {}", cov[0]);
    assert!((cov[3] - 30.0).abs() < 1e-12, "cov[3] = {}", cov[3]);
    assert!((cov[1] - (-1.0)).abs() < 1e-12, "cov[1] = {}", cov[1]);
    assert!((cov[2] - (-1.0)).abs() < 1e-12, "cov[2] = {}", cov[2]);

    assert!((corr[0][0] - 1.0).abs() < 1e-15);
    assert!((corr[1][1] - 1.0).abs() < 1e-15);
    assert!(
        (corr[0][1] - (-1.0 / 30.0)).abs() < 1e-12,
        "rho = {}",
        corr[0][1]
    );
    assert!((corr[1][0] - (-1.0 / 30.0)).abs() < 1e-12);
}

/// Complete-case construction: with no date where both factors are
/// observed, Ledoit-Wolf must fail with a clean validation error rather
/// than silently imputing.
#[test]
fn ledoit_wolf_adapter_rejects_sparse_overlap() {
    let a = FactorId::new("credit::generic");
    let b = FactorId::new("credit::bucket::IG");
    let mut returns: BTreeMap<FactorId, Vec<Option<f64>>> = BTreeMap::new();
    returns.insert(a.clone(), vec![Some(1.0), Some(-1.0), None, None]);
    returns.insert(b.clone(), vec![None, None, Some(1.0), Some(-1.0)]);
    let err =
        ledoit_wolf_cov_and_corr(&[a, b], &returns, 12.0).expect_err("no complete observations");
    assert!(
        err.to_string().contains("complete observation"),
        "unexpected error: {err}"
    );
}

#[test]
fn credit_calibration_config_default_values() {
    let config = CreditCalibrationConfig::default();
    assert_eq!(config.policy, IssuerBetaPolicy::GloballyOff);
    assert!(config.hierarchy.levels.is_empty());
    assert_eq!(config.vol_model, VolModelChoice::Sample);
    assert_eq!(config.covariance_strategy, CovarianceStrategy::Diagonal);
    assert_eq!(config.beta_shrinkage, BetaShrinkage::None);
    assert_eq!(config.use_returns_or_levels, PanelSpace::Returns);
    assert_eq!(config.annualization_factor, 12.0);
}

#[test]
fn panel_space_serde_roundtrip() {
    for variant in [PanelSpace::Returns, PanelSpace::Levels] {
        let json = serde_json::to_string(&variant).unwrap();
        let back: PanelSpace = serde_json::from_str(&json).unwrap();
        assert_eq!(variant, back);
    }
}

#[test]
fn panel_space_default_is_returns() {
    assert_eq!(PanelSpace::default(), PanelSpace::Returns);
}

#[test]
fn vol_model_choice_serde_roundtrip() {
    for variant in [
        VolModelChoice::Sample,
        VolModelChoice::Ewma { lambda: 0.94 },
    ] {
        let json = serde_json::to_string(&variant).unwrap();
        let back: VolModelChoice = serde_json::from_str(&json).unwrap();
        assert_eq!(variant, back);
    }
    assert_eq!(
        serde_json::to_string(&VolModelChoice::Ewma { lambda: 0.94 }).unwrap(),
        r#"{"ewma":{"lambda":0.94}}"#
    );
}

#[test]
fn ewma_lambda_must_be_in_open_unit_interval() {
    for bad in [0.0, 1.0, -0.5, 1.5, f64::NAN] {
        let config = CreditCalibrationConfig {
            vol_model: VolModelChoice::Ewma { lambda: bad },
            ..CreditCalibrationConfig::default()
        };
        assert!(
            validate_calibration_config(&config).is_err(),
            "lambda = {bad} must be rejected"
        );
    }
    let good = CreditCalibrationConfig {
        vol_model: VolModelChoice::Ewma { lambda: 0.94 },
        ..CreditCalibrationConfig::default()
    };
    assert!(validate_calibration_config(&good).is_ok());
}

#[test]
fn ewma_vol_model_rejected_for_levels_panel() {
    let config = CreditCalibrationConfig {
        vol_model: VolModelChoice::Ewma { lambda: 0.94 },
        use_returns_or_levels: PanelSpace::Levels,
        ..CreditCalibrationConfig::default()
    };
    let err = validate_calibration_config(&config)
        .expect_err("Ewma + Levels must be rejected as fail-closed");
    let msg = err.to_string();
    assert!(
        msg.contains("Ewma") && msg.contains("Levels"),
        "error message must name both offending settings, got: {msg}"
    );

    // Sample vol model remains accepted for a levels panel (it demeans).
    let sample_config = CreditCalibrationConfig {
        vol_model: VolModelChoice::Sample,
        use_returns_or_levels: PanelSpace::Levels,
        ..CreditCalibrationConfig::default()
    };
    assert!(validate_calibration_config(&sample_config).is_ok());

    // Ewma remains accepted for a returns panel.
    let returns_config = CreditCalibrationConfig {
        vol_model: VolModelChoice::Ewma { lambda: 0.94 },
        use_returns_or_levels: PanelSpace::Returns,
        ..CreditCalibrationConfig::default()
    };
    assert!(validate_calibration_config(&returns_config).is_ok());
}

#[test]
fn garch_and_egarch_json_are_rejected() {
    // Removed in v0.6: an inert config surface is worse than a smaller enum.
    assert!(
        serde_json::from_str::<VolModelChoice>("\"garch\"").is_err(),
        "\"garch\" must no longer deserialize"
    );
    assert!(
        serde_json::from_str::<VolModelChoice>("\"egarch\"").is_err(),
        "\"egarch\" must no longer deserialize"
    );
}

#[test]
fn covariance_strategy_serde_roundtrip() {
    for variant in [
        CovarianceStrategy::Diagonal,
        CovarianceStrategy::Ridge { alpha: 0.05 },
        CovarianceStrategy::FullSampleRepaired,
        CovarianceStrategy::LedoitWolf,
    ] {
        let json = serde_json::to_string(&variant).unwrap();
        let back: CovarianceStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(variant, back);
    }
    assert_eq!(
        serde_json::to_string(&CovarianceStrategy::LedoitWolf).unwrap(),
        r#""ledoit_wolf""#
    );
}

#[test]
fn beta_shrinkage_serde_roundtrip() {
    for variant in [BetaShrinkage::None, BetaShrinkage::TowardOne { alpha: 0.1 }] {
        let json = serde_json::to_string(&variant).unwrap();
        let back: BetaShrinkage = serde_json::from_str(&json).unwrap();
        assert_eq!(variant, back);
    }
}

#[test]
fn credit_calibration_config_serde_roundtrip() {
    let config = CreditCalibrationConfig {
        policy: IssuerBetaPolicy::GloballyOff,
        hierarchy: CreditHierarchySpec {
            levels: vec![crate::credit::hierarchy::HierarchyDimension::Rating],
        },
        min_bucket_size_per_level: BucketSizeThresholds { per_level: vec![5] },
        vol_model: VolModelChoice::Sample,
        covariance_strategy: CovarianceStrategy::Diagonal,
        beta_shrinkage: BetaShrinkage::None,
        use_returns_or_levels: PanelSpace::Returns,
        annualization_factor: 12.0,
    };
    let json = serde_json::to_string(&config).unwrap();
    let back: CreditCalibrationConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config.policy, back.policy);
    assert_eq!(config.vol_model, back.vol_model);
    assert_eq!(config.covariance_strategy, back.covariance_strategy);
    assert_eq!(config.beta_shrinkage, back.beta_shrinkage);
    assert_eq!(config.use_returns_or_levels, back.use_returns_or_levels);
    assert_eq!(config.annualization_factor, back.annualization_factor);
}

#[test]
fn bucket_size_thresholds_default_for_levels() {
    let thresholds = BucketSizeThresholds::default_for_levels(3);
    assert_eq!(thresholds.per_level, vec![5, 5, 5]);
    assert_eq!(thresholds.threshold_for_level(0), 5);
    assert_eq!(thresholds.threshold_for_level(2), 5);
    assert_eq!(thresholds.threshold_for_level(99), 5);
}

#[test]
fn bucket_size_thresholds_custom_values() {
    let thresholds = BucketSizeThresholds {
        per_level: vec![3, 7],
    };
    assert_eq!(thresholds.threshold_for_level(0), 3);
    assert_eq!(thresholds.threshold_for_level(1), 7);
    assert_eq!(thresholds.threshold_for_level(2), 5);
}

#[test]
fn history_panel_serde_roundtrip() {
    let mut spreads = BTreeMap::new();
    spreads.insert(IssuerId::new("A"), vec![Some(100.0), Some(101.0)]);
    spreads.insert(IssuerId::new("B"), vec![Some(200.0), None]);
    let panel = HistoryPanel {
        dates: vec![
            Date::from_calendar_date(2024, time::Month::January, 31).unwrap(),
            Date::from_calendar_date(2024, time::Month::February, 29).unwrap(),
        ],
        spreads,
    };
    let json = serde_json::to_string(&panel).unwrap();
    let back: HistoryPanel = serde_json::from_str(&json).unwrap();
    assert_eq!(panel, back);
}
