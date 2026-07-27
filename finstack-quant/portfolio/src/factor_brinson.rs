//! Factor-Brinson unified attribution (Jeet & Partani 2023).
//!
//! Classical Brinson-Fachler ([`crate::brinson`]) requires assets to be
//! partitioned into disjoint sectors. Factor-Brinson generalizes the same
//! allocation/selection split to *continuous* factor exposures (durations,
//! spread betas, quality scores, ...) by replacing the sector partition with
//! a factor-exposure matrix and a factor-return vector.
//!
//! # Notation
//!
//! * `h_p`, `h_b` — portfolio / benchmark asset weight vectors (length
//!   `n_assets`).
//! * `X` — asset x factor exposure matrix (`n_assets x n_factors`).
//! * `f_b` — benchmark factor returns (caller-supplied; see below).
//! * `r` — realized asset returns.
//! * `ε_b = r − X f_b` — per-asset specific return implied by `f_b`.
//!
//! # Decomposition
//!
//! ```text
//! w  = X'(h_p − h_b)          // active factor loadings
//! FC = w'f_b                   // factor contribution ("allocation")
//! SC = (h_p − h_b)'ε_b         // specific contribution ("selection")
//! FC + SC ≡ h_p'r − h_b'r = active_return   (algebraic identity, any f_b)
//! ```
//!
//! `FC + SC` reconstructs the active return for *any* `f_b` — that identity
//! is pure algebra (`w'f_b + (h_p-h_b)'ε_b = (h_p-h_b)'(Xf_b + ε_b) =
//! (h_p-h_b)'r`) and holds even for a nonsensical `f_b`. What makes `FC`
//! and `SC` interpretable as Brinson-style allocation/selection effects is
//! the **completeness condition** `h_b'ε_b ≈ 0`: the benchmark's specific
//! returns implied by `f_b` must average to zero under benchmark weights,
//! i.e. `f_b` must fully explain the benchmark's realized return. Binary
//! (0/1) factor exposures forming a sector partition, with `f_b` set to
//! each sector's benchmark-weighted-mean return, satisfy this exactly and
//! reduce `FC`/`SC` to classical Brinson-Fachler allocation/(selection +
//! interaction) — see
//! [`binary_factors_reproduce_brinson_fachler_exactly`] in this module's
//! tests. For continuous exposures, `f_b` must come from a fit that
//! enforces the completeness condition, e.g.
//! [`finstack_quant_analytics::regression::constrained_least_squares`],
//! which this crate does not depend on: factor returns are always
//! caller-supplied, and [`factor_brinson_attribution`] only checks the
//! condition, it does not fit `f_b` itself.
//!
//! # Design decisions
//!
//! * **Factor returns are caller-supplied.** They typically come from a
//!   risk model or from
//!   `finstack_quant_analytics::regression::constrained_least_squares`
//!   (this crate does not depend on `finstack-quant-analytics`: the
//!   dependency direction runs analytics -> ... -> portfolio, so adding a
//!   reverse edge purely to reference a function in doc comments and error
//!   text is unnecessary and would be an unused dependency).
//! * **Per-factor contributions are undemeaned** (`w_k * f_k`, not
//!   `w_k * (f_k - mean(f))`): demeaning is a presentation-layer choice
//!   and is ill-defined for continuous (non-partition) loadings — paper
//!   footnote 10.
//! * **Completeness tolerance.** The paper's completeness condition is
//!   `h_b'ε_b = 0` exactly for a properly fit `f_b`. In practice, `f_b`
//!   reaches this function as literal floating-point numbers — often
//!   copied from a risk-model report or rounded to a handful of
//!   significant digits — so a residual at the scale of that rounding
//!   (empirically ~1e-8 for 8-significant-digit inputs; see this module's
//!   `continuous_factors_match_hand_derived_estimator_output` test) is
//!   expected, not a modeling failure. [`factor_brinson_attribution`]
//!   therefore uses [`COMPLETENESS_TOLERANCE`] (`1e-6`, matching this
//!   crate's other weight-sum tolerances) scaled by `max(1, |r_b|)`,
//!   rather than a tolerance tight enough to reject that rounding — while
//!   still rejecting `f_b` values that do not explain the benchmark at
//!   all (residuals several orders of magnitude larger; see
//!   `incomplete_factor_returns_fail_closed`).
//!
//! # References
//!
//! * Jeet, V., & Partani, A. (2023). "Brinson-Style Attribution over
//!   Continuous Factors." *The Journal of Portfolio Management*,
//!   Quantitative Special Issue 2023, 216-223.
//!   `docs/REFERENCES.md#jeet-partani-2023`

use crate::error::{Error, Result};
use finstack_quant_core::math::summation::NeumaierAccumulator;
use serde::{Deserialize, Serialize};

/// Tolerance for the requirement that each side's weights sum to `1.0`.
const WEIGHT_TOLERANCE: f64 = 1e-6;

/// Tolerance on the Jeet-Partani completeness residual `h_b'ε_b`, scaled by
/// `max(1, |r_b|)`. See the module-level "Completeness tolerance" note for
/// why this is `1e-6` and not a tighter bound.
const COMPLETENESS_TOLERANCE: f64 = 1e-6;

/// Inputs to [`factor_brinson_attribution`]: per-asset returns, a factor
/// exposure matrix, and portfolio/benchmark weights.
///
/// Weights may be negative (short positions); each side's weights must sum
/// to `1.0` within [`WEIGHT_TOLERANCE`].
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactorBrinsonInput {
    /// Asset identifiers, length `n_assets`.
    pub asset_ids: Vec<String>,
    /// Realized asset returns (decimal, e.g. `0.02` = 2%), length `n_assets`.
    pub asset_returns: Vec<f64>,
    /// Row-major factor exposure matrix, `n_assets x n_factors`: asset `i`'s
    /// exposure to factor `j` is `exposures[i * n_factors + j]`.
    pub exposures: Vec<f64>,
    /// Factor names, length `n_factors` (defines `n_factors`).
    pub factor_names: Vec<String>,
    /// Portfolio weight per asset, length `n_assets`. Must sum to `1.0`.
    pub portfolio_weights: Vec<f64>,
    /// Benchmark weight per asset, length `n_assets`. Must sum to `1.0`.
    pub benchmark_weights: Vec<f64>,
}

/// One factor's contribution to the factor (allocation) effect.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactorContribution {
    /// Factor name (mirrors [`FactorBrinsonInput::factor_names`]).
    pub factor: String,
    /// Active factor loading `w_k = Σ_i X_ik (h_p,i − h_b,i)`.
    pub active_loading: f64,
    /// Benchmark factor return `f_k` supplied by the caller.
    pub factor_return: f64,
    /// Contribution `w_k * f_k` (undemeaned; see module docs).
    pub contribution: f64,
}

/// One asset's contribution to the specific (selection) effect.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetSpecificContribution {
    /// Asset identifier (mirrors [`FactorBrinsonInput::asset_ids`]).
    pub asset: String,
    /// Specific return implied by the supplied factor returns,
    /// `ε_b,i = r_i − (X f_b)_i`.
    pub specific_return: f64,
    /// Active weight `h_p,i − h_b,i`.
    pub active_weight: f64,
    /// Contribution `(h_p,i − h_b,i) * ε_b,i`.
    pub contribution: f64,
}

/// Factor-Brinson unified attribution result.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactorBrinsonResult {
    /// Portfolio total return, `h_p'r`.
    pub portfolio_return: f64,
    /// Benchmark total return, `h_b'r`.
    pub benchmark_return: f64,
    /// Active return, `portfolio_return − benchmark_return`.
    pub active_return: f64,
    /// Factor (allocation) contribution, `FC = w'f_b` where
    /// `w = X'(h_p − h_b)`.
    pub allocation: f64,
    /// Specific (selection) contribution, `SC = (h_p − h_b)'ε_b`.
    pub selection: f64,
    /// Per-factor breakdown of `allocation`, in `factor_names` order.
    pub factor_contributions: Vec<FactorContribution>,
    /// Per-asset breakdown of `selection`, in `asset_ids` order.
    pub asset_contributions: Vec<AssetSpecificContribution>,
}

/// Compute Jeet-Partani (2023) factor-Brinson unified attribution.
///
/// Generalizes classical Brinson-Fachler allocation/selection to continuous
/// factor exposures. See the module docs for the full decomposition and the
/// completeness condition that makes `allocation`/`selection` interpretable
/// as Brinson-style effects.
///
/// # Arguments
///
/// * `input` - Asset returns, exposures, and portfolio/benchmark weights.
/// * `factor_returns` - Caller-supplied benchmark factor returns `f_b`,
///   length `input.factor_names.len()`. Typically fit with
///   `finstack_quant_analytics::regression::constrained_least_squares` so
///   the completeness condition holds.
///
/// # Errors
///
/// Returns [`Error::InvalidInput`] when:
/// - any of `asset_returns`, `portfolio_weights`, `benchmark_weights` does
///   not have length `n_assets = asset_ids.len()`, or `n_assets == 0`.
/// - `factor_returns.len() != input.factor_names.len()`, or
///   `input.factor_names.is_empty()` (`n_factors == 0`).
/// - `exposures.len() != n_assets * n_factors`.
/// - any input value is `NaN` or infinite.
/// - portfolio or benchmark weights do not sum to `1.0` within
///   [`WEIGHT_TOLERANCE`].
/// - the completeness residual `|h_b'ε_b|` exceeds
///   [`COMPLETENESS_TOLERANCE`] * `max(1, |benchmark_return|)` — the
///   supplied `factor_returns` do not fully explain the benchmark return,
///   so `allocation`/`selection` would not be valid Brinson effects. The
///   error message directs callers to
///   `finstack_quant_analytics::regression::constrained_least_squares`.
pub fn factor_brinson_attribution(
    input: &FactorBrinsonInput,
    factor_returns: &[f64],
) -> Result<FactorBrinsonResult> {
    let n_assets = input.asset_ids.len();
    let n_factors = input.factor_names.len();

    if n_assets == 0 {
        return Err(Error::invalid_input(
            "factor-Brinson attribution requires at least one asset",
        ));
    }
    if n_factors == 0 {
        return Err(Error::invalid_input(
            "factor-Brinson attribution requires at least one factor",
        ));
    }
    for (name, len) in [
        ("asset_returns", input.asset_returns.len()),
        ("portfolio_weights", input.portfolio_weights.len()),
        ("benchmark_weights", input.benchmark_weights.len()),
    ] {
        if len != n_assets {
            return Err(Error::invalid_input(format!(
                "'{name}' has length {len}, expected {n_assets} (= asset_ids.len())"
            )));
        }
    }
    if factor_returns.len() != n_factors {
        return Err(Error::invalid_input(format!(
            "'factor_returns' has length {}, expected {n_factors} (= factor_names.len())",
            factor_returns.len()
        )));
    }
    if input.exposures.len() != n_assets * n_factors {
        return Err(Error::invalid_input(format!(
            "'exposures' has length {}, expected {} (= n_assets * n_factors)",
            input.exposures.len(),
            n_assets * n_factors
        )));
    }

    for (i, &v) in input.asset_returns.iter().enumerate() {
        if !v.is_finite() {
            return Err(Error::invalid_input(format!(
                "'asset_returns[{i}]' must be finite (got {v})"
            )));
        }
    }
    for (i, &v) in input.portfolio_weights.iter().enumerate() {
        if !v.is_finite() {
            return Err(Error::invalid_input(format!(
                "'portfolio_weights[{i}]' must be finite (got {v})"
            )));
        }
    }
    for (i, &v) in input.benchmark_weights.iter().enumerate() {
        if !v.is_finite() {
            return Err(Error::invalid_input(format!(
                "'benchmark_weights[{i}]' must be finite (got {v})"
            )));
        }
    }
    for (i, &v) in input.exposures.iter().enumerate() {
        if !v.is_finite() {
            return Err(Error::invalid_input(format!(
                "'exposures[{i}]' must be finite (got {v})"
            )));
        }
    }
    for (j, &v) in factor_returns.iter().enumerate() {
        if !v.is_finite() {
            return Err(Error::invalid_input(format!(
                "'factor_returns[{j}]' must be finite (got {v})"
            )));
        }
    }

    let sum_wp: f64 = {
        let mut acc = NeumaierAccumulator::new();
        for &w in &input.portfolio_weights {
            acc.add(w);
        }
        acc.total()
    };
    if (sum_wp - 1.0).abs() > WEIGHT_TOLERANCE {
        return Err(Error::invalid_input(format!(
            "Portfolio weights must sum to 1.0 (got {sum_wp})"
        )));
    }
    let sum_wb: f64 = {
        let mut acc = NeumaierAccumulator::new();
        for &w in &input.benchmark_weights {
            acc.add(w);
        }
        acc.total()
    };
    if (sum_wb - 1.0).abs() > WEIGHT_TOLERANCE {
        return Err(Error::invalid_input(format!(
            "Benchmark weights must sum to 1.0 (got {sum_wb})"
        )));
    }

    // Portfolio / benchmark total returns: r_p = h_p'r, r_b = h_b'r.
    let mut acc_rp = NeumaierAccumulator::new();
    let mut acc_rb = NeumaierAccumulator::new();
    for ((&hp_i, &hb_i), &r_i) in input
        .portfolio_weights
        .iter()
        .zip(input.benchmark_weights.iter())
        .zip(input.asset_returns.iter())
    {
        acc_rp.add(hp_i * r_i);
        acc_rb.add(hb_i * r_i);
    }
    let portfolio_return = acc_rp.total();
    let benchmark_return = acc_rb.total();
    let active_return = portfolio_return - benchmark_return;

    // Active weight h_p - h_b, per asset.
    let active_weight: Vec<f64> = (0..n_assets)
        .map(|i| input.portfolio_weights[i] - input.benchmark_weights[i])
        .collect();

    // Active factor loadings w = X'(h_p - h_b): one Neumaier accumulator
    // per factor, summed over assets in input order.
    let mut acc_w: Vec<NeumaierAccumulator> = vec![NeumaierAccumulator::new(); n_factors];
    for (row, &aw_i) in input
        .exposures
        .chunks_exact(n_factors)
        .zip(active_weight.iter())
    {
        for (acc, &x_ij) in acc_w.iter_mut().zip(row.iter()) {
            acc.add(x_ij * aw_i);
        }
    }
    let w: Vec<f64> = acc_w.into_iter().map(NeumaierAccumulator::total).collect();

    // Factor (allocation) contribution FC = w'f_b.
    let mut acc_fc = NeumaierAccumulator::new();
    for (&w_j, &f_j) in w.iter().zip(factor_returns.iter()) {
        acc_fc.add(w_j * f_j);
    }
    let allocation = acc_fc.total();

    // Per-asset specific returns implied by f_b: eps_b,i = r_i - (X f_b)_i.
    let specific_return: Vec<f64> = (0..n_assets)
        .map(|i| {
            let row = &input.exposures[i * n_factors..(i + 1) * n_factors];
            let mut acc_xf = NeumaierAccumulator::new();
            for (j, &x_ij) in row.iter().enumerate() {
                acc_xf.add(x_ij * factor_returns[j]);
            }
            input.asset_returns[i] - acc_xf.total()
        })
        .collect();

    // Completeness condition: h_b'eps_b must be ~0 for f_b to be a valid
    // benchmark-return-explaining fit (see module docs).
    let mut acc_completeness = NeumaierAccumulator::new();
    for (&hb_i, &eps_i) in input.benchmark_weights.iter().zip(specific_return.iter()) {
        acc_completeness.add(hb_i * eps_i);
    }
    let completeness_residual = acc_completeness.total();
    let completeness_bound = COMPLETENESS_TOLERANCE * benchmark_return.abs().max(1.0);
    if completeness_residual.abs() > completeness_bound {
        return Err(Error::invalid_input(format!(
            "Supplied factor_returns do not satisfy the Jeet-Partani completeness \
             condition: |h_b'eps_b| = {} exceeds tolerance {completeness_bound} \
             (benchmark_return = {benchmark_return}). allocation/selection would not \
             be valid Brinson effects for these factor returns. Fit factor_returns \
             with finstack_quant_analytics::regression::constrained_least_squares \
             (using benchmark weights) to enforce this condition.",
            completeness_residual.abs()
        )));
    }

    // Specific (selection) contribution SC = (h_p - h_b)'eps_b, computed
    // independently of `allocation` and `active_return` (not by
    // subtraction) so this check retains diagnostic value against a
    // mis-derived selection term.
    let mut acc_sc = NeumaierAccumulator::new();
    for (&aw_i, &eps_i) in active_weight.iter().zip(specific_return.iter()) {
        acc_sc.add(aw_i * eps_i);
    }
    let selection = acc_sc.total();

    let factor_contributions = (0..n_factors)
        .map(|j| FactorContribution {
            factor: input.factor_names[j].clone(),
            active_loading: w[j],
            factor_return: factor_returns[j],
            contribution: w[j] * factor_returns[j],
        })
        .collect();

    let asset_contributions = (0..n_assets)
        .map(|i| AssetSpecificContribution {
            asset: input.asset_ids[i].clone(),
            specific_return: specific_return[i],
            active_weight: active_weight[i],
            contribution: active_weight[i] * specific_return[i],
        })
        .collect();

    Ok(FactorBrinsonResult {
        portfolio_return,
        benchmark_return,
        active_return,
        allocation,
        selection,
        factor_contributions,
        asset_contributions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brinson::{brinson_fachler, SectorPeriod};

    fn close(actual: f64, expected: f64, tol: f64, label: &str) {
        assert!(
            (actual - expected).abs() < tol,
            "{label}: {actual} vs {expected} (tol {tol})"
        );
    }

    fn base_input(exposures: Vec<f64>) -> FactorBrinsonInput {
        FactorBrinsonInput {
            asset_ids: vec!["A".into(), "B".into(), "C".into()],
            asset_returns: vec![0.05, 0.02, 0.01],
            exposures,
            factor_names: vec!["Energy".into(), "Healthcare".into()],
            portfolio_weights: vec![1.25, -0.30, 0.05],
            benchmark_weights: vec![0.60, 0.30, 0.10],
        }
    }

    /// Paper Exhibits 1-2 in decimals, binary (0/1) sector-indicator
    /// exposures. `f_b` is each sector's benchmark-weighted-mean return:
    /// Energy 0.02 (sole asset B), Healthcare 0.31/7 = 0.0442857142857
    /// (assets A, C weighted 0.60/0.10 in the benchmark).
    #[test]
    fn binary_factors_reproduce_brinson_fachler_exactly() {
        // Rows: A -> Healthcare only, B -> Energy only, C -> Healthcare only.
        let input = base_input(vec![0.0, 1.0, 1.0, 0.0, 0.0, 1.0]);
        let f_b = [0.02, 0.31 / 7.0];
        let r = factor_brinson_attribution(&input, &f_b).expect("valid inputs");

        close(r.active_return, 0.02, 1e-12, "active_return");
        close(r.allocation, 0.0145714285714286, 1e-12, "allocation"); // paper: 1.46%
        close(r.selection, 0.0054285714285714, 1e-12, "selection"); // paper: 0.54%

        // Equivalence with the existing Brinson-Fachler implementation.
        //
        // Energy = {B}, Healthcare = {A, C}. Healthcare's portfolio sector
        // return is the portfolio-weighted mean of A and C:
        // (1.25*0.05 + 0.05*0.01) / 1.30 = 0.0484615384615.
        let sectors = vec![
            SectorPeriod {
                sector: "Energy".into(),
                portfolio_weight: -0.30,
                benchmark_weight: 0.30,
                portfolio_return: 0.02,
                benchmark_return: 0.02,
            },
            SectorPeriod {
                sector: "Healthcare".into(),
                portfolio_weight: 1.30,
                benchmark_weight: 0.70,
                portfolio_return: 0.0484615384615,
                benchmark_return: 0.31 / 7.0,
            },
        ];
        let bf = brinson_fachler(&sectors).expect("valid BF inputs");

        close(bf.total_allocation, r.allocation, 1e-12, "BF allocation");
        // NOTE: this is the one place classical BF and factor-Brinson
        // terminology diverge. Classical BF's `selection` term is
        // benchmark-weighted (`w_b,i (r_p,i - r_b,i)`) and reports the
        // *joint* effect of over/underweighting and stock-picking
        // separately as `interaction` (`(w_p,i - w_b,i)(r_p,i - r_b,i)`).
        // Factor-Brinson's specific contribution `SC = (h_p - h_b)'eps_b`
        // carries the *active* weight `h_p - h_b`, which is exactly BF's
        // `selection + interaction` combined:
        //   BF selection    = w_b,Healthcare * (r_p - r_b)
        //                    = 0.70 * 0.0041758241758 = 0.0029230769231
        //   BF interaction  = (w_p,Healthcare - w_b,Healthcare) * (r_p - r_b)
        //                    = 0.60 * 0.0041758241758 = 0.0025054945055
        //   sum             = 0.0054285714286 = r.selection
        close(
            bf.total_selection + bf.total_interaction,
            r.selection,
            1e-12,
            "BF selection + interaction",
        );
    }

    /// Exposures from paper Exhibit 6 (continuous, not a sector partition);
    /// `f_b` from Task 6's constrained-least-squares estimator (hand
    /// values, rounded to 8 significant digits as a caller would report
    /// them).
    #[test]
    fn continuous_factors_match_hand_derived_estimator_output() {
        let input = base_input(vec![1.2, -0.8, 0.5, 1.2, -0.7, 0.7]);
        let f_b = [0.04695653, 0.01130477];
        let r = factor_brinson_attribution(&input, &f_b).expect("valid inputs");

        close(r.allocation, 0.0097690, 1e-6, "allocation");
        close(r.selection, 0.0102310, 1e-6, "selection");
        // FC + SC = active_return is an algebraic identity for any f_b
        // (see module docs) — it does not by itself validate `selection`,
        // only that the two terms were computed consistently.
        close(
            r.allocation + r.selection,
            r.active_return,
            1e-14,
            "FC + SC = active_return",
        );
    }

    /// An arbitrary `f_b` that does not explain the benchmark's realized
    /// return must be rejected: `h_b'eps_b` is far outside tolerance.
    #[test]
    fn incomplete_factor_returns_fail_closed() {
        let input = base_input(vec![1.2, -0.8, 0.5, 1.2, -0.7, 0.7]);
        let f_b = [0.05, 0.01];
        let err = factor_brinson_attribution(&input, &f_b)
            .expect_err("f_b must fail the completeness condition");
        assert!(
            err.to_string().contains("constrained_least_squares"),
            "error should direct callers to the fitting function: {err}"
        );
    }

    #[test]
    fn dimension_and_weight_validation() {
        let input = base_input(vec![1.2, -0.8, 0.5, 1.2, -0.7, 0.7]);
        let f_b = [0.04695653, 0.01130477];

        // Mismatched asset_returns length.
        let mut bad = input.clone();
        bad.asset_returns = vec![0.05, 0.02];
        let err = factor_brinson_attribution(&bad, &f_b).expect_err("length mismatch");
        assert!(err.to_string().contains("asset_returns"), "{err}");

        // Mismatched exposures length.
        let mut bad = input.clone();
        bad.exposures = vec![1.0, 2.0];
        let err = factor_brinson_attribution(&bad, &f_b).expect_err("exposures mismatch");
        assert!(err.to_string().contains("exposures"), "{err}");

        // Mismatched factor_returns length.
        let err = factor_brinson_attribution(&input, &[0.05])
            .expect_err("factor_returns length mismatch");
        assert!(err.to_string().contains("factor_returns"), "{err}");

        // n_factors == 0.
        let mut bad = input.clone();
        bad.factor_names = vec![];
        bad.exposures = vec![];
        let err = factor_brinson_attribution(&bad, &[]).expect_err("zero factors");
        assert!(err.to_string().contains("factor"), "{err}");

        // n_assets == 0.
        let empty = FactorBrinsonInput {
            asset_ids: vec![],
            asset_returns: vec![],
            exposures: vec![],
            factor_names: vec!["F".into()],
            portfolio_weights: vec![],
            benchmark_weights: vec![],
        };
        let err = factor_brinson_attribution(&empty, &[0.01]).expect_err("zero assets");
        assert!(err.to_string().contains("asset"), "{err}");

        // NaN asset return.
        let mut bad = input.clone();
        bad.asset_returns[0] = f64::NAN;
        let err = factor_brinson_attribution(&bad, &f_b).expect_err("NaN return");
        assert!(err.to_string().contains("finite"), "{err}");

        // NaN exposure.
        let mut bad = input.clone();
        bad.exposures[0] = f64::INFINITY;
        let err = factor_brinson_attribution(&bad, &f_b).expect_err("infinite exposure");
        assert!(err.to_string().contains("finite"), "{err}");

        // Portfolio weights sum to 1 + 2e-6: rejected.
        let mut bad = input.clone();
        bad.portfolio_weights[0] += 2e-6;
        let err = factor_brinson_attribution(&bad, &f_b).expect_err("1 + 2e-6 must be rejected");
        assert!(err.to_string().contains("Portfolio weights"), "{err}");

        // Portfolio weights sum to 1 + 5e-7: accepted.
        let mut ok = input.clone();
        ok.portfolio_weights[0] += 5e-7;
        assert!(
            factor_brinson_attribution(&ok, &f_b).is_ok(),
            "1 + 5e-7 must be accepted at a 1e-6 tolerance"
        );

        // Benchmark weights sum to 1 + 2e-6: rejected.
        let mut bad = input.clone();
        bad.benchmark_weights[0] += 2e-6;
        let err = factor_brinson_attribution(&bad, &f_b)
            .expect_err("benchmark 1 + 2e-6 must be rejected");
        assert!(err.to_string().contains("Benchmark weights"), "{err}");

        // Benchmark weights sum to 1 + 5e-7: accepted.
        let mut ok = input;
        ok.benchmark_weights[0] += 5e-7;
        assert!(
            factor_brinson_attribution(&ok, &f_b).is_ok(),
            "benchmark 1 + 5e-7 must be accepted at a 1e-6 tolerance"
        );
    }

    /// Negative (short) weights must be supported, not rejected: the
    /// binary-factor fixture's portfolio weight on asset B is -0.30.
    #[test]
    fn negative_weights_are_supported() {
        let input = base_input(vec![0.0, 1.0, 1.0, 0.0, 0.0, 1.0]);
        assert!(input.portfolio_weights.iter().any(|&w| w < 0.0));
        let sum: f64 = input.portfolio_weights.iter().sum();
        close(sum, 1.0, 1e-12, "portfolio weights sum");
        let f_b = [0.02, 0.31 / 7.0];
        assert!(factor_brinson_attribution(&input, &f_b).is_ok());
    }

    #[test]
    fn factor_brinson_result_serde_round_trips() {
        let input = base_input(vec![0.0, 1.0, 1.0, 0.0, 0.0, 1.0]);
        let f_b = [0.02, 0.31 / 7.0];
        let r = factor_brinson_attribution(&input, &f_b).expect("valid inputs");

        let json = serde_json::to_string(&r).expect("serializes");
        let round_tripped: FactorBrinsonResult = serde_json::from_str(&json).expect("deserializes");

        close(
            round_tripped.portfolio_return,
            r.portfolio_return,
            1e-15,
            "portfolio_return",
        );
        close(
            round_tripped.benchmark_return,
            r.benchmark_return,
            1e-15,
            "benchmark_return",
        );
        close(
            round_tripped.active_return,
            r.active_return,
            1e-15,
            "active_return",
        );
        close(round_tripped.allocation, r.allocation, 1e-15, "allocation");
        close(round_tripped.selection, r.selection, 1e-15, "selection");
        assert_eq!(
            round_tripped.factor_contributions.len(),
            r.factor_contributions.len()
        );
        assert_eq!(
            round_tripped.asset_contributions.len(),
            r.asset_contributions.len()
        );
        assert_eq!(
            round_tripped.factor_contributions[0].factor,
            r.factor_contributions[0].factor
        );
    }

    #[test]
    fn factor_brinson_input_serde_denies_unknown_fields() {
        let json = r#"{
            "asset_ids": ["A"],
            "asset_returns": [0.05],
            "exposures": [1.0],
            "factor_names": ["F"],
            "portfolio_weights": [1.0],
            "benchmark_weights": [1.0]
        }"#;
        let parsed: FactorBrinsonInput = serde_json::from_str(json).expect("stable names parse");
        assert_eq!(parsed.asset_ids[0], "A");

        let bad = r#"{
            "asset_ids": ["A"],
            "asset_returns": [0.05],
            "exposures": [1.0],
            "factor_names": ["F"],
            "portfolio_weights": [1.0],
            "benchmark_weights": [1.0],
            "surprise": 1.0
        }"#;
        assert!(
            serde_json::from_str::<FactorBrinsonInput>(bad).is_err(),
            "unknown fields must be rejected"
        );
    }
}
