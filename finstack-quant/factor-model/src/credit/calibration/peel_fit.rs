use std::collections::BTreeMap;

use finstack_quant_core::types::IssuerId;

use super::config::{BetaShrinkage, CreditCalibrationConfig};
use super::panel::WorkingPanel;
use crate::credit::hierarchy::{
    dimension_key, CreditHierarchySpec, FitQuality, IssuerBetaMode, IssuerBetas, IssuerTags,
};
use crate::matching::{bucket_factor_id, CREDIT_GENERIC_FACTOR_ID};
use crate::FactorId;

/// `IssuerBetas` with all loadings = 1.0 (BucketOnly default).
pub(super) fn unit_betas(num_levels: usize) -> IssuerBetas {
    IssuerBetas {
        pc: 1.0,
        levels: vec![1.0; num_levels],
    }
}

/// Outcome of the PC + per-level peel (steps 4 + 5).
pub(super) struct PeelOutcome {
    /// Calibrated betas per issuer.
    pub(super) betas: BTreeMap<IssuerId, IssuerBetas>,
    /// Adder return series per issuer (length = working panel length).
    pub(super) adder_series: BTreeMap<IssuerId, Vec<Option<f64>>>,
    /// PC-fit quality stats for IssuerBeta issuers.
    pub(super) fit_quality: BTreeMap<IssuerId, FitQuality>,
    /// Per-level fit quality for IssuerBeta issuers, aligned with
    /// `betas.levels` (`None` where no fit ran: folded level or degenerate
    /// regressor). BucketOnly issuers have no entry.
    pub(super) level_fit_quality: BTreeMap<IssuerId, Vec<Option<FitQuality>>>,
    /// Per-factor return series (sparse), keyed by FactorId.
    /// Includes the generic factor (always `Some`) and every surviving bucket
    /// factor. Bucket factors carry `None` at dates where all bucket members
    /// were absent ("empty-bucket" sentinel). Use
    /// [`factor_variances`][super::statistics::factor_variances] to compute
    /// variance over only the observed entries, and flatten to
    /// `0.0`-substituted `Vec<f64>` when writing to
    /// [`FactorHistories`][crate::credit::hierarchy::FactorHistories].
    pub(super) factor_returns: BTreeMap<FactorId, Vec<Option<f64>>>,
}

pub(super) fn run_peel(
    config: &CreditCalibrationConfig,
    panel: &WorkingPanel,
    modes: &BTreeMap<IssuerId, IssuerBetaMode>,
    bucket_paths: &BTreeMap<IssuerId, Vec<String>>,
    folded: &BTreeMap<IssuerId, Vec<bool>>,
) -> PeelOutcome {
    let n = panel.generic.len();
    let num_levels = config.hierarchy.levels.len();

    let mut betas: BTreeMap<IssuerId, IssuerBetas> = BTreeMap::new();
    let mut residuals: BTreeMap<IssuerId, Vec<Option<f64>>> = BTreeMap::new();
    let mut fit_quality: BTreeMap<IssuerId, FitQuality> = BTreeMap::new();
    let mut level_fit_quality: BTreeMap<IssuerId, Vec<Option<FitQuality>>> = BTreeMap::new();
    let mut factor_returns: BTreeMap<FactorId, Vec<Option<f64>>> = BTreeMap::new();

    // Generic factor is always fully observed; wrap in Some for type uniformity.
    factor_returns.insert(
        FactorId::new(CREDIT_GENERIC_FACTOR_ID),
        panel.generic.iter().map(|v| Some(*v)).collect(),
    );

    // Step 4 — PC peel.
    for (issuer, series) in &panel.issuers {
        let mode = modes
            .get(issuer)
            .copied()
            .unwrap_or(IssuerBetaMode::BucketOnly);
        let beta_pc = match mode {
            IssuerBetaMode::BucketOnly => 1.0,
            IssuerBetaMode::IssuerBeta => {
                let raw = ols_slope(series, &panel.generic).unwrap_or(1.0);
                let shrunk = apply_shrinkage(&config.beta_shrinkage, raw);
                // Fit-quality stats: R² and residual std on the same valid pairs.
                if let Some(fq) = compute_fit_quality(series, &panel.generic, shrunk) {
                    fit_quality.insert(issuer.clone(), fq);
                }
                level_fit_quality.insert(issuer.clone(), vec![None; num_levels]);
                shrunk
            }
        };
        let res_pc: Vec<Option<f64>> = series
            .iter()
            .enumerate()
            .map(|(t, v)| v.as_ref().map(|s| s - beta_pc * panel.generic[t]))
            .collect();
        residuals.insert(issuer.clone(), res_pc);
        // Initialize beta row with per-level betas at 0.0; the per-level peel
        // below overwrites entries for non-folded buckets. Folded buckets stay
        // at 0.0 (the contractual sentinel for "skip this level").
        betas.insert(
            issuer.clone(),
            IssuerBetas {
                pc: beta_pc,
                levels: vec![0.0; num_levels],
            },
        );
    }

    // Step 5 — per-level peel.
    // Range-based loop over the hierarchy level index `k`. `k` indexes into
    // multiple parallel structures (`bucket_paths[issuer][k]`, `folded[i][k]`,
    // `betas[issuer].levels[k]`); enumerate-iterating any one of them would
    // not eliminate indexing into the others.
    #[allow(clippy::needless_range_loop)]
    for k in 0..num_levels {
        // 5a. For each surviving (non-folded) bucket, compute factor return series.
        // Build a map: bucket_path → vector of issuer IDs participating.
        // Folded issuers contribute β=0 at this level and DO NOT participate
        // in computing the bucket factor return; they simply propagate
        // residuals unchanged.
        let mut bucket_members: BTreeMap<String, Vec<&IssuerId>> = BTreeMap::new();
        for issuer in panel.issuers.keys() {
            let folded_at_k = folded
                .get(issuer)
                .map(|f| f.get(k).copied().unwrap_or(false))
                .unwrap_or(false);
            if folded_at_k {
                continue;
            }
            let path = &bucket_paths[issuer][k];
            bucket_members.entry(path.clone()).or_default().push(issuer);
        }

        // Compute bucket factor returns f_<level_k>(g, t) = mean over members.
        // Empty buckets at date t (all members missing) emit `None` so that
        // downstream OLS and variance estimation can skip those dates rather
        // than biasing results with an imputed zero.
        let mut bucket_factor_series: BTreeMap<String, Vec<Option<f64>>> = BTreeMap::new();
        for (bucket, members) in &bucket_members {
            let mut series = Vec::with_capacity(n);
            for t in 0..n {
                let mut sum = 0.0;
                let mut count = 0usize;
                for issuer in members {
                    if let Some(Some(v)) = residuals.get(*issuer).map(|r| r[t]) {
                        sum += v;
                        count += 1;
                    }
                }
                series.push(if count > 0 {
                    Some(sum / (count as f64))
                } else {
                    None
                });
            }
            bucket_factor_series.insert(bucket.clone(), series);
        }

        // 5b. For each member, fit / set its level-k beta and update its residual.
        for (bucket, members) in &bucket_members {
            let factor_series = &bucket_factor_series[bucket];
            for issuer in members {
                let mode = modes
                    .get(*issuer)
                    .copied()
                    .unwrap_or(IssuerBetaMode::BucketOnly);
                let r_series = residuals.get(*issuer).cloned().unwrap_or_default();
                let beta_k = match mode {
                    IssuerBetaMode::BucketOnly => 1.0,
                    IssuerBetaMode::IssuerBeta => {
                        // Fit OLS on the issuer's *current* residual vs the bucket factor.
                        // `factor_series` is already `Vec<Option<f64>>`; dates where the
                        // factor is `None` (empty bucket) are skipped by `ols_slope_owned`.
                        let fitted = ols_slope_owned(&r_series, factor_series);
                        let shrunk = apply_shrinkage(&config.beta_shrinkage, fitted.unwrap_or(1.0));
                        // Record per-level fit quality only where a genuine fit
                        // ran (degenerate regressors return `None` above).
                        if fitted.is_some() {
                            if let Some(slots) = level_fit_quality.get_mut(*issuer) {
                                slots[k] =
                                    compute_fit_quality_sparse(&r_series, factor_series, shrunk);
                            }
                        }
                        shrunk
                    }
                };
                if let Some(b) = betas.get_mut(*issuer) {
                    b.levels[k] = beta_k;
                }
                // Propagate `None` when the factor is unavailable at date t.
                let new_res: Vec<Option<f64>> = r_series
                    .iter()
                    .enumerate()
                    .map(|(t, v)| match (v, factor_series[t]) {
                        (Some(x), Some(f)) => Some(x - beta_k * f),
                        _ => None,
                    })
                    .collect();
                residuals.insert((*issuer).clone(), new_res);
            }
        }

        // Folded issuers: beta_k stays at 0.0 (already initialized) and
        // residual is unchanged (no subtraction). We've simply skipped them.

        // Record bucket factor return series in the canonical FactorId form.
        // The sparse `Vec<Option<f64>>` is stored directly in `factor_returns`;
        // flattening to `Vec<f64>` for `FactorHistories` happens in
        // `build_factor_histories` (substituting `0.0` for `None`).
        // `factor_variances` computes variance over only the `Some` entries.
        for (bucket, series) in bucket_factor_series {
            // Reconstruct an IssuerTags for path → use a synthetic helper:
            // We need bucket_factor_id. The existing helper requires IssuerTags;
            // we don't have them here, but we can build a minimal tag map by
            // splitting the path on '.'.
            let tags = synth_tags_from_path(&config.hierarchy, &bucket);
            // bucket_factor_id is `Some(_)` whenever every dimension key in
            // `levels[0..=k]` appears in `tags` — which `synth_tags_from_path`
            // guarantees by construction. Fall through with `continue` defensively
            // rather than panic via `.expect()`.
            let Some(factor_id) = bucket_factor_id(&config.hierarchy, &tags, k) else {
                continue;
            };
            factor_returns.insert(factor_id, series);
        }
    }

    PeelOutcome {
        betas,
        adder_series: residuals,
        fit_quality,
        level_fit_quality,
        factor_returns,
    }
}

/// Reconstruct an [`IssuerTags`] from a dotted bucket path so that callers
/// can re-use [`bucket_factor_id`].
///
/// The path has `k+1` segments aligned with `hierarchy.levels[0..=k]`.
fn synth_tags_from_path(hierarchy: &CreditHierarchySpec, path: &str) -> IssuerTags {
    let segments: Vec<&str> = path.split('.').collect();
    let mut map = BTreeMap::new();
    for (i, seg) in segments.iter().enumerate() {
        if let Some(dim) = hierarchy.levels.get(i) {
            map.insert(dimension_key(dim), (*seg).to_owned());
        }
    }
    IssuerTags(map)
}

/// Apply shrinkage rule to an OLS β estimate.
fn apply_shrinkage(rule: &BetaShrinkage, beta_fit: f64) -> f64 {
    match rule {
        BetaShrinkage::None => beta_fit,
        BetaShrinkage::TowardOne { alpha } => (1.0 - *alpha) * beta_fit + *alpha * 1.0,
    }
}

/// Relative variance floor below which a regressor is treated as degenerate.
///
/// A hierarchy level that does not refine its parent partition produces a
/// bucket factor that is identically zero *in exact arithmetic* but carries
/// float noise around `1e-15` in practice. `finstack_quant_analytics::beta`
/// only rejects an *exactly* zero-variance regressor, so the OLS slope
/// `Cov(y, x) / Var(x)` divides residual-scale noise by squared rounding
/// noise and returns finite garbage on the order of `1e13`. Requiring
/// `Var(x) > tol · Var(y)` rejects that case while keeping any regressor
/// whose variation is economically meaningful relative to the response.
const DEGENERATE_REGRESSOR_REL_TOL: f64 = 1e-12;

/// OLS slope on aligned valid pairs of `(y_i, x_i)` where `y` is a sparse
/// `Option<f64>` series and `x` is dense.
///
/// Returns `None` if fewer than 3 valid pairs are available (mirroring the
/// `n < 3` `NaN` return of `finstack_quant_analytics::beta`) or when the
/// regressor is degenerate per [`DEGENERATE_REGRESSOR_REL_TOL`]; callers
/// fall back to the unit-beta convention.
fn ols_slope(y: &[Option<f64>], x: &[f64]) -> Option<f64> {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for (yi, xi) in y.iter().zip(x.iter()) {
        if let Some(v) = yi {
            xs.push(*xi);
            ys.push(*v);
        }
    }
    ols_slope_pairs(&ys, &xs)
}

/// OLS slope when both series are sparse; align on positions where both are `Some`.
///
/// Same return contract as [`ols_slope`].
fn ols_slope_owned(y: &[Option<f64>], x: &[Option<f64>]) -> Option<f64> {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for (yi, xi) in y.iter().zip(x.iter()) {
        if let (Some(yv), Some(xv)) = (yi, xi) {
            ys.push(*yv);
            xs.push(*xv);
        }
    }
    ols_slope_pairs(&ys, &xs)
}

/// Shared OLS core over aligned pairs: degenerate-regressor gate, then
/// delegate to `finstack_quant_analytics::beta`.
fn ols_slope_pairs(ys: &[f64], xs: &[f64]) -> Option<f64> {
    let n = xs.len();
    if n >= 2 {
        let nf = n as f64;
        let mean_x = xs.iter().sum::<f64>() / nf;
        let mean_y = ys.iter().sum::<f64>() / nf;
        let var_x = xs.iter().map(|v| (v - mean_x).powi(2)).sum::<f64>();
        let var_y = ys.iter().map(|v| (v - mean_y).powi(2)).sum::<f64>();
        // Shared (n − 1) denominator cancels in the ratio test.
        if var_x <= DEGENERATE_REGRESSOR_REL_TOL * var_y {
            return None;
        }
    }
    let result = finstack_quant_analytics::beta(ys, xs);
    if result.beta.is_nan() {
        None
    } else {
        Some(result.beta)
    }
}

/// R², residual std, and n_obs for the PC fit (used as the regression diagnostic).
///
/// `residual_std` is a population std dev (sum of squared residuals divided by `n`).
/// Population variance is acceptable because calibration windows are required to be
/// ≥ 24 observations (`min_history` default = 24), keeping the bias negligible.
fn compute_fit_quality(y: &[Option<f64>], x: &[f64], beta: f64) -> Option<FitQuality> {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for (yi, xi) in y.iter().zip(x.iter()) {
        if let Some(v) = yi {
            xs.push(*xi);
            ys.push(*v);
        }
    }
    fit_quality_from_pairs(&ys, &xs, beta)
}

/// [`compute_fit_quality`] for a sparse regressor: pairs align on positions
/// where both series are `Some` (the same alignment [`ols_slope_owned`] uses).
fn compute_fit_quality_sparse(
    y: &[Option<f64>],
    x: &[Option<f64>],
    beta: f64,
) -> Option<FitQuality> {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for (yi, xi) in y.iter().zip(x.iter()) {
        if let (Some(yv), Some(xv)) = (yi, xi) {
            ys.push(*yv);
            xs.push(*xv);
        }
    }
    fit_quality_from_pairs(&ys, &xs, beta)
}

/// Shared fit-quality core over aligned pairs.
fn fit_quality_from_pairs(ys: &[f64], xs: &[f64], beta: f64) -> Option<FitQuality> {
    let n = xs.len();
    if n < 3 {
        return None;
    }
    let nf = n as f64;
    let mean_x = xs.iter().sum::<f64>() / nf;
    let mean_y = ys.iter().sum::<f64>() / nf;
    let alpha = mean_y - beta * mean_x;
    let mut tss = 0.0;
    let mut rss = 0.0;
    for i in 0..n {
        let resid = ys[i] - alpha - beta * xs[i];
        rss += resid * resid;
        let dy = ys[i] - mean_y;
        tss += dy * dy;
    }
    let r_squared = if tss > 0.0 { 1.0 - rss / tss } else { 0.0 };
    let residual_std = (rss / nf).sqrt();
    Some(FitQuality {
        r_squared,
        residual_std,
        n_obs: n,
    })
}
