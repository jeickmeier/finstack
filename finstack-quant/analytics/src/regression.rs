//! Equality-constrained least squares for factor-Brinson attribution.
//!
//! Ordinary least squares regresses realized asset returns onto factor
//! exposures without any guarantee that the fitted factor returns, when
//! carried back through the *benchmark* (or portfolio) weights, reproduce the
//! benchmark's actual realized return. Factor-Brinson attribution needs that
//! guarantee: the attribution effects must sum exactly to the return being
//! explained, or the decomposition does not close.
//!
//! [`constrained_least_squares`] solves the single linear equality constraint
//! `w'Xf = w'r` on top of an unconstrained OLS fit, using the closed-form
//! Lagrangian correction from Jeet & Partani (2023), Appendix A. This module
//! is a standalone numerical building block; it does not know about
//! portfolios, sectors, or attribution effects — those are composed by
//! downstream callers (see `finstack-quant-portfolio`).
//!
//! # Quick Example
//! ```rust
//! use finstack_quant_analytics::regression::constrained_least_squares;
//!
//! // 3 assets, 2 binary factors (Exhibit 1 of Jeet & Partani 2023).
//! let exposures = [0.0, 1.0, 1.0, 0.0, 0.0, 1.0];
//! let returns = [0.05, 0.02, 0.01];
//! let weights = [0.6, 0.3, 0.1];
//!
//! let f = constrained_least_squares(&exposures, 2, &returns, &weights).unwrap();
//! assert_eq!(f.len(), 2);
//! ```
//!
//! # References
//! - Jeet & Partani (2023): see `docs/REFERENCES.md#jeet-partani-2023`

use finstack_quant_core::error::{InputError, NonFiniteKind};
use nalgebra::{DMatrix, DVector};

/// Relative tolerance, on the Cauchy-Schwarz ratio `|w'Xg| / (‖w‖‖Xg‖)`,
/// below which the constraint-restoring denominator `w'Xg` is treated as
/// numerically zero, i.e. the equality constraint `w'Xf = w'r` cannot be
/// enforced by any scalar Lagrangian correction.
///
/// This ratio is the cosine of the angle between `w` and `Xg`; it is
/// dimensionless and invariant under any uniform rescaling of `w` (e.g.
/// holdings expressed as percent vs. decimal fractions), unlike a bound on
/// the raw `w'Xg` magnitude, which scales as `‖w‖²` under such a rescaling.
/// When either `w` or `Xg` is the zero vector the ratio is (by convention)
/// treated as `0`, which correctly flags that case as degenerate too.
const CONSTRAINT_DEGENERACY_RELATIVE_TOLERANCE: f64 = 1e-10;

/// Relative singular-value tolerance used to detect rank-deficient design
/// matrices in the underlying no-intercept least-squares solves.
const RANK_TOLERANCE_FACTOR: f64 = 1.0e-10;

/// Factor returns `f` satisfying the single equality constraint `w'Xf = w'r`
/// (Jeet & Partani 2023, Appendix A).
///
/// Ordinary least squares fits `f̂ = argmin ‖r − Xf‖²`, the return-explaining
/// factor returns. That fit generally does *not* satisfy `w'Xf̂ = w'r`: the
/// weighted (e.g. benchmark) return implied by the fitted factor returns need
/// not equal the weighted realized return. This function adds the minimal
/// correction along the constraint-restoring direction
/// `g = argmin ‖w − Xg‖² = (X'X)⁻¹X'w` so that the corrected factor returns
/// `f = f̂ + λg` satisfy the constraint exactly, where the Lagrange multiplier
///
/// ```text
/// λ = w'(r − Xf̂) / (w'Xg) = w'ε̂ / (w'Xg)
/// ```
///
/// is the ratio of the weighted OLS residual to the weighted constraint
/// direction. Both `f̂` and `g` are computed as no-intercept least-squares
/// solves of the same design matrix `X` against `r` and `w` respectively.
///
/// # Arguments
///
/// * `exposures` - Row-major factor exposure matrix, `n_assets × n_factors`:
///   asset `i`'s exposure to factor `j` is `exposures[i * n_factors + j]`.
/// * `n_factors` - Number of factor columns in `exposures`.
/// * `returns` - Realized asset returns, length `n_assets`.
/// * `weights` - Holding weights whose weighted return `w'r` must be fully
///   reproduced by `w'Xf` (e.g. benchmark weights for a benchmark-return
///   attribution).
///
/// # Returns
///
/// The constrained factor returns `f`, one per factor, satisfying
/// `w'Xf = w'r` to numerical precision.
///
/// # Errors
///
/// Returns an error when:
/// - `n_factors` is `0`, or `returns` (equivalently `n_assets`) is empty.
/// - `exposures.len() != n_assets * n_factors` or `weights.len() != n_assets`
///   (dimension mismatch).
/// - any input value is `NaN` or infinite.
/// - the design matrix `X` is rank-deficient (either least-squares solve is
///   underdetermined).
/// - `|w'Xg|` is numerically zero *relative to* `‖w‖·‖Xg‖` (a `1e-10`
///   bound on the Cauchy-Schwarz ratio `|w'Xg| / (‖w‖‖Xg‖)`), meaning `w`
///   and the constraint direction `Xg` are (numerically) orthogonal and the
///   constraint cannot be restored by any scalar correction. This test is
///   scale-invariant: rescaling `weights` uniformly (e.g. percent vs.
///   decimal) does not change whether it fires.
///
/// # Ill-Conditioning
///
/// The degeneracy guard above is a bound on how close `w` and `Xg` are to
/// *exactly* orthogonal; it does not bound how close they are to that
/// boundary. A `w` just inside the guard can still make `w'Xg` small enough
/// that `λ = w'ε̂ / (w'Xg)`, and therefore `f`, is very large in magnitude.
/// That large `f` is the mathematically correct constrained solution for an
/// ill-conditioned `(X, w)` pair, not a bug: `|f| ≫ 1` (e.g. an implied
/// factor return far outside any plausible range) is a signal to inspect the
/// conditioning of the inputs — near-collinear factor exposures or a weight
/// vector nearly orthogonal to the constraint direction — rather than
/// evidence that this function mis-solved the constraint.
///
/// # Examples
///
/// ```rust
/// use finstack_quant_analytics::regression::constrained_least_squares;
///
/// let exposures = [1.2, -0.8, 0.5, 1.2, -0.7, 0.7];
/// let returns = [0.05, 0.02, 0.01];
/// let weights = [0.6, 0.3, 0.1];
///
/// let f = constrained_least_squares(&exposures, 2, &returns, &weights).unwrap();
///
/// // The constraint holds exactly (to numerical precision).
/// let wxf: f64 = (0..3)
///     .map(|i| weights[i] * (exposures[i * 2] * f[0] + exposures[i * 2 + 1] * f[1]))
///     .sum();
/// let wr: f64 = (0..3).map(|i| weights[i] * returns[i]).sum();
/// assert!((wxf - wr).abs() < 1e-9);
/// ```
///
/// # References
///
/// - Jeet, V., & Partani, A. (2023). "Brinson-Style Attribution over
///   Continuous Factors." *The Journal of Portfolio Management*, Quantitative
///   Special Issue 2023, 216-223. See `docs/REFERENCES.md#jeet-partani-2023`.
pub fn constrained_least_squares(
    exposures: &[f64],
    n_factors: usize,
    returns: &[f64],
    weights: &[f64],
) -> crate::Result<Vec<f64>> {
    let n_assets = returns.len();
    if n_assets == 0 || n_factors == 0 {
        return Err(InputError::Invalid.into());
    }
    if exposures.len() != n_assets * n_factors || weights.len() != n_assets {
        return Err(InputError::DimensionMismatch.into());
    }
    ensure_finite(exposures)?;
    ensure_finite(returns)?;
    ensure_finite(weights)?;

    let x_matrix = DMatrix::from_row_slice(n_assets, n_factors, exposures);
    let r_vector = DVector::from_row_slice(returns);
    let w_vector = DVector::from_row_slice(weights);

    // f̂ = argmin ‖r − Xf‖² (unconstrained OLS fit).
    let f_hat = no_intercept_least_squares(&x_matrix, &r_vector)?;
    // g = argmin ‖w − Xg‖² = (X'X)⁻¹X'w (constraint-restoring direction).
    let g = no_intercept_least_squares(&x_matrix, &w_vector)?;

    // ε̂ = r − Xf̂ (OLS residual).
    let residual = &r_vector - &x_matrix * &f_hat;
    let w_dot_residual = w_vector.dot(&residual);

    let x_g = &x_matrix * &g;
    let w_dot_xg = w_vector.dot(&x_g);

    // Scale-invariant degeneracy test: compare |w'Xg| against a bound
    // relative to ‖w‖·‖Xg‖ rather than an absolute epsilon (see the
    // constant's doc comment). By Cauchy-Schwarz, |w'Xg| <= ‖w‖·‖Xg‖
    // always, so this is equivalent to bounding the cosine of the angle
    // between `w` and `Xg`. Using `<=` (not `<`) also catches the exact-zero
    // case (either vector is zero) without a separate branch: the bound
    // itself becomes `0`, and `0.abs() <= 0` is true.
    let degeneracy_bound = CONSTRAINT_DEGENERACY_RELATIVE_TOLERANCE * w_vector.norm() * x_g.norm();
    if !w_dot_xg.is_finite() || w_dot_xg.abs() <= degeneracy_bound {
        return Err(InputError::ConstraintUnreachable {
            denominator: w_dot_xg,
        }
        .into());
    }

    let lambda = w_dot_residual / w_dot_xg;
    let f = f_hat + g * lambda;

    Ok(f.iter().copied().collect())
}

/// Solve `argmin_β ‖y − Xβ‖²` via SVD, with no intercept column.
///
/// Columns of `x` are scale-normalized before the SVD so that differing
/// factor magnitudes alone do not trigger false "singular" classifications,
/// mirroring `benchmark::svd_least_squares`. The returned coefficients are
/// rescaled back into the original (unscaled) units.
fn no_intercept_least_squares(x: &DMatrix<f64>, y: &DVector<f64>) -> crate::Result<DVector<f64>> {
    let (n, p) = x.shape();
    if n == 0 || p == 0 || y.len() != n {
        return Err(InputError::DimensionMismatch.into());
    }

    let mut scales = Vec::with_capacity(p);
    for column in x.column_iter() {
        let norm = column.norm();
        if !norm.is_finite() || norm <= 0.0 {
            return Err(InputError::Invalid.into());
        }
        scales.push(norm);
    }

    let mut scaled = x.clone();
    for (col_idx, scale) in scales.iter().enumerate() {
        for row_idx in 0..n {
            scaled[(row_idx, col_idx)] /= *scale;
        }
    }

    let svd = scaled.svd(true, true);
    let max_singular = svd.singular_values.iter().copied().fold(0.0_f64, f64::max);
    // No `.max(1.0)` floor needed: every column of `scaled` was normalized
    // to unit norm above, so `max_singular >= ||scaled * e_i|| = 1` for any
    // standard basis vector `e_i` — the largest singular value is always at
    // least 1 by construction, making such a floor a no-op.
    let tolerance = RANK_TOLERANCE_FACTOR * max_singular;
    // The column-norm guard above already rejected zero / non-finite
    // columns, so any remaining shortfall is rank-deficiency relative to `p`.
    let rank = svd
        .singular_values
        .iter()
        .filter(|&&value| value > tolerance)
        .count();
    if rank < p {
        return Err(InputError::Invalid.into());
    }

    let beta_scaled = svd.solve(y, tolerance).map_err(|_| InputError::Invalid)?;

    let mut beta = beta_scaled;
    for (row_idx, scale) in scales.iter().enumerate() {
        beta[row_idx] /= *scale;
    }
    Ok(beta)
}

/// Returns an error if any value in `values` is `NaN` or infinite.
fn ensure_finite(values: &[f64]) -> crate::Result<()> {
    for &value in values {
        if value.is_nan() {
            return Err(InputError::NonFiniteValue {
                kind: NonFiniteKind::NaN,
            }
            .into());
        }
        if value.is_infinite() {
            let kind = if value.is_sign_positive() {
                NonFiniteKind::PosInfinity
            } else {
                NonFiniteKind::NegInfinity
            };
            return Err(InputError::NonFiniteValue { kind }.into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::constrained_least_squares;

    #[test]
    fn binary_factor_case_matches_hand_closed_form() {
        // Paper Exhibit 1 data. X binary: A=(0,1), B=(1,0), C=(0,1); r=(0.05,0.02,0.01);
        // w = benchmark holdings (0.6,0.3,0.1).
        // f̂=(0.02,0.03); g=(0.3,0.35)/… hand: g=(0.003,0.0035)·100 — in decimals g=(0.3,0.35)
        // scaled: X'y=(0.3,0.7), X'X=diag(1,2) ⇒ g=(0.3,0.35); λ=(w'ε̂)/(w'Xg)=0.01/0.335·…
        // In decimals: w'ε̂=0.010, w'Xg=0.335 ⇒ λ=0.0298507463.
        // f = (0.02 + 0.3λ, 0.03 + 0.35λ) = (0.0289552239, 0.0404477612).
        let x = [0.0, 1.0, 1.0, 0.0, 0.0, 1.0];
        let f = constrained_least_squares(&x, 2, &[0.05, 0.02, 0.01], &[0.6, 0.3, 0.1]).unwrap();
        assert!((f[0] - 0.0289552239).abs() < 1e-9);
        assert!((f[1] - 0.0404477612).abs() < 1e-9);
    }

    #[test]
    fn continuous_case_satisfies_constraint_and_matches_hand_derivation() {
        // Paper Exhibit 6 exposures; returns/holdings in decimals.
        // Hand-derived (this plan, NOT the paper's unreproducible Exhibit 8):
        // f ≈ (0.04695653, 0.01130477).
        let x = [1.2, -0.8, 0.5, 1.2, -0.7, 0.7];
        let r = [0.05, 0.02, 0.01];
        let w = [0.6, 0.3, 0.1];
        let f = constrained_least_squares(&x, 2, &r, &w).unwrap();
        assert!((f[0] - 0.04695653).abs() < 1e-6);
        assert!((f[1] - 0.01130477).abs() < 1e-6);
        // Exact identity: w'Xf == w'r
        let wxf = w[0] * (x[0] * f[0] + x[1] * f[1])
            + w[1] * (x[2] * f[0] + x[3] * f[1])
            + w[2] * (x[4] * f[0] + x[5] * f[1]);
        let wr = 0.6 * 0.05 + 0.3 * 0.02 + 0.1 * 0.01;
        assert!((wxf - wr).abs() < 1e-12);
    }

    #[test]
    fn orthogonal_weights_fail_closed() {
        // X = [1, −1] (2 assets × 1 factor), w = (0.5, 0.5) ⇒ w'Xg = 0 ⇒ error.
        let err =
            constrained_least_squares(&[1.0, -1.0], 1, &[0.02, 0.01], &[0.5, 0.5]).unwrap_err();
        assert!(
            err.to_string().to_lowercase().contains("constraint"),
            "{err}"
        );
    }

    #[test]
    fn dimension_mismatch_on_exposures_fails_closed() {
        // exposures has 5 entries but n_assets(3) * n_factors(2) = 6.
        let err = constrained_least_squares(
            &[0.0, 1.0, 1.0, 0.0, 0.0],
            2,
            &[0.05, 0.02, 0.01],
            &[0.6, 0.3, 0.1],
        )
        .unwrap_err();
        assert!(
            err.to_string().to_lowercase().contains("dimension"),
            "{err}"
        );
    }

    #[test]
    fn dimension_mismatch_on_weights_fails_closed() {
        let x = [0.0, 1.0, 1.0, 0.0, 0.0, 1.0];
        let err = constrained_least_squares(&x, 2, &[0.05, 0.02, 0.01], &[0.6, 0.4]).unwrap_err();
        assert!(
            err.to_string().to_lowercase().contains("dimension"),
            "{err}"
        );
    }

    #[test]
    fn empty_assets_fails_closed() {
        // n_assets = returns.len() = 0 with n_factors = 2: exposures.len()
        // (0) and weights.len() (0) both match n_assets * n_factors / n_assets
        // trivially, so this trips the dedicated `n_assets == 0` guard (which
        // renders as `InputError::Invalid`, "Invalid input data"), not a
        // dimension-mismatch check.
        let err = constrained_least_squares(&[], 2, &[], &[]).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("invalid"), "{err}");
    }

    /// Two factor columns in an exact `2x` proportion (perfectly collinear)
    /// make the 3-asset x 2-factor design matrix rank-deficient: after
    /// column normalization both columns are numerically identical, so the
    /// underlying `no_intercept_least_squares` SVD solve sees only one
    /// non-negligible singular value for two factor columns and must fail
    /// closed rather than silently returning an arbitrary least-norm split
    /// between the two collinear factors.
    #[test]
    fn rank_deficient_exposures_fail_closed() {
        let x = [1.0, 2.0, 2.0, 4.0, 3.0, 6.0]; // factor 2 = 2 * factor 1
        let r = [0.05, 0.02, 0.01];
        let w = [0.6, 0.3, 0.1];
        let err = constrained_least_squares(&x, 2, &r, &w).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("invalid"), "{err}");
    }

    #[test]
    fn non_finite_return_fails_closed() {
        let x = [0.0, 1.0, 1.0, 0.0, 0.0, 1.0];
        let err = constrained_least_squares(&x, 2, &[f64::NAN, 0.02, 0.01], &[0.6, 0.3, 0.1])
            .unwrap_err();
        assert!(err.to_string().to_lowercase().contains("finite"), "{err}");
    }

    #[test]
    fn non_finite_exposures_and_weights_fail_closed_with_finite_value_error() {
        // Parametrized over {exposures, weights} x {NaN, +Inf, -Inf}. Each
        // combination must be rejected by `ensure_finite` itself (message
        // mentions "finite"), not merely by some downstream guard that
        // happens to also reject it under a different, less specific error
        // (e.g. "Invalid input data" or "constraint ... degenerate").
        let good_x = [0.0, 1.0, 1.0, 0.0, 0.0, 1.0];
        let good_r = [0.05, 0.02, 0.01];
        let good_w = [0.6, 0.3, 0.1];

        for &bad in &[f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut x = good_x;
            x[0] = bad;
            let err = constrained_least_squares(&x, 2, &good_r, &good_w).unwrap_err();
            assert!(
                err.to_string().to_lowercase().contains("finite"),
                "exposures[0] = {bad}: {err}"
            );

            let mut w = good_w;
            w[0] = bad;
            let err = constrained_least_squares(&good_x, 2, &good_r, &w).unwrap_err();
            assert!(
                err.to_string().to_lowercase().contains("finite"),
                "weights[0] = {bad}: {err}"
            );
        }
    }

    #[test]
    fn near_degenerate_boundary_is_scale_invariant() {
        // X = [1, -1] (2 assets x 1 factor); w = c * (0.5, 0.5 + delta).
        // The Cauchy-Schwarz ratio |w'Xg| / (||w|| ||Xg||) that the
        // degeneracy guard tests depends only on `delta`, not on the
        // overall scale `c` (both the numerator and denominator scale as
        // c^2 under w -> c*w, so c cancels exactly). A caller supplying
        // weights as percent (c = 100) rather than decimal (c = 1) must
        // therefore see the same accept/reject outcome for the same delta.
        let x = [1.0, -1.0];
        let r = [0.02, 0.01];

        let classify = |delta: f64, c: f64| -> bool {
            let w = [c * 0.5, c * (0.5 + delta)];
            constrained_least_squares(&x, 1, &r, &w).is_err()
        };

        // Below the relative-ratio tolerance (~delta): still degenerate,
        // regardless of scale. Exercises the raw-zero test's blind spot:
        // the old absolute threshold on w'Xg could not distinguish this
        // near-zero-but-nonzero denominator from the exact w'Xg == 0 case
        // that `orthogonal_weights_fail_closed` already covers.
        assert!(classify(1e-12, 1.0), "delta=1e-12, c=1 should error");
        assert!(classify(1e-12, 100.0), "delta=1e-12, c=100 should error");

        // Above the tolerance: no longer treated as degenerate, in either
        // scale. Under the old *absolute* threshold this pair would have
        // disagreed (raw w'Xg scales as c^2, so only the c=1 case passed);
        // the relative criterion agrees for both.
        assert!(!classify(1e-8, 1.0), "delta=1e-8, c=1 should NOT error");
        assert!(!classify(1e-8, 100.0), "delta=1e-8, c=100 should NOT error");
    }
}
