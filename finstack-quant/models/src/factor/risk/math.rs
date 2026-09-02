//! Shared numerical helpers for factor-model risk decomposition.
//!
//! These helpers live here (rather than in each decomposer) so that the
//! parametric portfolio and position decomposers reuse bit-identical
//! constants, formulas, and the rank-tolerant Cholesky factorization. The
//! `normal_quantile` implementation is Peter Acklam's rational approximation
//! to the inverse standard-normal CDF (sometimes misattributed to
//! Beasley–Springer–Moro, a different algorithm); `normal_pdf` is the
//! standard-normal density. Keep the constants byte-identical — changing
//! them is a numerical-behaviour change, not a cleanup.

/// Rational approximation for the inverse standard-normal CDF (probit).
pub(super) fn normal_quantile(probability: f64) -> f64 {
    const A1: f64 = -3.969_683_028_665_376e1;
    const A2: f64 = 2.209_460_984_245_205e2;
    const A3: f64 = -2.759_285_104_469_687e2;
    const A4: f64 = 1.383_577_518_672_69e2;
    const A5: f64 = -3.066_479_806_614_716e1;
    const A6: f64 = 2.506_628_277_459_239;
    const B1: f64 = -5.447_609_879_822_406e1;
    const B2: f64 = 1.615_858_368_580_409e2;
    const B3: f64 = -1.556_989_798_598_866e2;
    const B4: f64 = 6.680_131_188_771_972e1;
    const B5: f64 = -1.328_068_155_288_572e1;
    const C1: f64 = -7.784_894_002_430_293e-3;
    const C2: f64 = -3.223_964_580_411_365e-1;
    const C3: f64 = -2.400_758_277_161_838;
    const C4: f64 = -2.549_732_539_343_734;
    const C5: f64 = 4.374_664_141_464_968;
    const C6: f64 = 2.938_163_982_698_783;
    const D1: f64 = 7.784_695_709_041_462e-3;
    const D2: f64 = 3.224_671_290_700_398e-1;
    const D3: f64 = 2.445_134_137_142_996;
    const D4: f64 = 3.754_408_661_907_416;
    const P_LOW: f64 = 0.024_25;
    const P_HIGH: f64 = 1.0 - P_LOW;

    if probability < P_LOW {
        let q = (-2.0 * probability.ln()).sqrt();
        (((((C1 * q + C2) * q + C3) * q + C4) * q + C5) * q + C6)
            / ((((D1 * q + D2) * q + D3) * q + D4) * q + 1.0)
    } else if probability > P_HIGH {
        let q = (-2.0 * (1.0 - probability).ln()).sqrt();
        -(((((C1 * q + C2) * q + C3) * q + C4) * q + C5) * q + C6)
            / ((((D1 * q + D2) * q + D3) * q + D4) * q + 1.0)
    } else {
        let q = probability - 0.5;
        let r = q * q;
        (((((A1 * r + A2) * r + A3) * r + A4) * r + A5) * r + A6) * q
            / (((((B1 * r + B2) * r + B3) * r + B4) * r + B5) * r + 1.0)
    }
}

/// Standard-normal probability density function.
pub(super) fn normal_pdf(x: f64) -> f64 {
    (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt()
}

/// Relative tolerance for symmetry, semi-definiteness, and rank detection.
///
/// Applied against the matrix's own scale rather than as an absolute bound.
/// A *covariance* matrix carries the units of its factors squared, so its
/// entries span many orders of magnitude — daily equity-return variances sit
/// around `1e-4`, while rate factors quoted in basis points reach `1e4`. A
/// fixed absolute threshold is therefore wrong at both ends: on a small-scale
/// matrix it declares genuinely positive directions rank-deficient (or rejects
/// a valid matrix as indefinite), and on a large-scale one it sits below
/// accumulated rounding and admits a matrix that is not positive
/// semi-definite.
///
/// This mirrors [`finstack_quant_core::math::linalg::PIVOT_TOLERANCE_RELATIVE`],
/// which core adopted for the same reason.
const MATRIX_TOLERANCE_RELATIVE: f64 = 1e-10;

/// Scale a covariance matrix is measured at: the largest absolute diagonal
/// entry, i.e. the biggest factor variance present.
///
/// Floored at `f64::MIN_POSITIVE` only so an all-zero matrix yields a positive
/// tolerance rather than zero; it is deliberately *not* floored at 1.0, since
/// that would make the threshold absolute again for any matrix whose variances
/// are below unity — which is the common case for return-space covariances.
fn matrix_scale(data: &[f64], n: usize) -> f64 {
    (0..n)
        .map(|i| data[i * n + i].abs())
        .fold(0.0_f64, f64::max)
        .max(f64::MIN_POSITIVE)
}

/// Cholesky decomposition returning a lower-triangular matrix `L` such that `L * L' = A`.
pub(crate) fn cholesky(data: &[f64], n: usize) -> finstack_quant_core::Result<Vec<f64>> {
    if data.len() != n * n {
        return Err(finstack_quant_core::Error::Validation(format!(
            "Covariance storage length {} does not match matrix dimension {n}x{n}",
            data.len()
        )));
    }

    if data.iter().any(|entry| !entry.is_finite()) {
        return Err(finstack_quant_core::Error::Validation(
            "Covariance matrix entries must be finite".to_string(),
        ));
    }

    let tolerance = MATRIX_TOLERANCE_RELATIVE * matrix_scale(data, n);

    // Verify symmetry so callers need not pre-validate.
    for i in 0..n {
        for j in (i + 1)..n {
            if (data[i * n + j] - data[j * n + i]).abs() > tolerance {
                return Err(finstack_quant_core::Error::Validation(format!(
                    "Covariance matrix must be symmetric at ({i}, {j})"
                )));
            }
        }
    }

    let mut lower = vec![0.0; n * n];

    for i in 0..n {
        for j in 0..=i {
            let mut sum = 0.0;
            for k in 0..j {
                sum += lower[i * n + k] * lower[j * n + k];
            }

            if i == j {
                let diagonal = data[i * n + i] - sum;
                if diagonal < -tolerance {
                    return Err(finstack_quant_core::Error::Validation(
                        "Covariance matrix is not positive semi-definite".to_string(),
                    ));
                }
                lower[i * n + j] = diagonal.max(0.0).sqrt();
            } else {
                let denominator = lower[j * n + j];
                let value = data[i * n + j] - sum;
                // `lower` holds square roots of variances, so a rank-deficient
                // pivot is small on the *standard-deviation* scale, not the
                // variance scale — compare against sqrt(tolerance). The
                // residual `value`, however, lives on the covariance
                // (variance) scale: for a PSD matrix a zero pivot forces the
                // entire remaining column to zero, so any residual above the
                // variance-scale `tolerance` proves the matrix is indefinite
                // and must not be silently dropped.
                let pivot_tolerance = tolerance.sqrt();
                if denominator.abs() <= pivot_tolerance {
                    if value.abs() > tolerance {
                        return Err(finstack_quant_core::Error::Validation(
                            "Covariance matrix is not positive semi-definite".to_string(),
                        ));
                    }
                    lower[i * n + j] = 0.0;
                } else {
                    lower[i * n + j] = value / denominator;
                }
            }
        }
    }

    Ok(lower)
}

#[cfg(test)]
mod tests {
    use super::cholesky;

    type TestResult = finstack_quant_core::Result<()>;

    /// The factorization must behave identically on a matrix and on the same
    /// matrix rescaled — an absolute tolerance breaks exactly this property.
    #[test]
    fn cholesky_is_scale_invariant() -> TestResult {
        // Rank-deficient by construction: row 2 = 2 x row 1.
        let base = [1.0, 2.0, 2.0, 4.0];

        for scale in [1e-8_f64, 1e-4, 1.0, 1e4, 1e8] {
            let scaled: Vec<f64> = base.iter().map(|v| v * scale).collect();
            let lower = cholesky(&scaled, 2)?;

            // L * L' must reproduce the input at the input's own scale.
            for i in 0..2 {
                for j in 0..2 {
                    let product: f64 = (0..2).map(|k| lower[i * 2 + k] * lower[j * 2 + k]).sum();
                    let expected = scaled[i * 2 + j];
                    assert!(
                        (product - expected).abs() <= 1e-9 * scale,
                        "scale {scale:e}: L*L'[{i}][{j}] = {product} != {expected}"
                    );
                }
            }

            // The dependent direction collapses to ~zero at every scale, not
            // just where an absolute 1e-10 happened to land.
            //
            // The bound is `sqrt(eps)`-relative, not `eps`-relative: this
            // pivot is the square root of a fully cancelled quantity
            // (`4s − (2*sqrt(s))^2`), and taking a square root maps a relative
            // error of `eps` in the radicand to `sqrt(eps)` ~ 1.5e-8 in the
            // result. Asserting an `eps`-scale bound here would be asserting
            // something floating point cannot deliver.
            assert!(
                lower[3].abs() <= 1e-7 * scale.sqrt(),
                "scale {scale:e}: dependent direction should have ~zero pivot, got {}",
                lower[3]
            );
        }
        Ok(())
    }

    /// A genuinely indefinite matrix is rejected regardless of its scale.
    #[test]
    fn cholesky_rejects_indefinite_at_every_scale() {
        for scale in [1e-8_f64, 1.0, 1e8] {
            let indefinite: Vec<f64> = [1.0, 3.0, 3.0, 1.0].iter().map(|v| v * scale).collect();
            assert!(
                cholesky(&indefinite, 2).is_err(),
                "scale {scale:e}: indefinite matrix should be rejected"
            );
        }
    }

    /// Asymmetry is judged against the matrix's own scale.
    #[test]
    fn cholesky_symmetry_check_is_relative() {
        // At scale 1e8 an absolute 1e-10 rule would reject this valid matrix,
        // whose asymmetry is 1e-4 of nothing in relative terms (1e-12).
        let large = [1e8, 2e8, 2e8 + 1e-4, 4e8];
        assert!(
            cholesky(&large, 2).is_ok(),
            "rounding-scale asymmetry on a large matrix should be tolerated"
        );

        // At scale 1e-8 a 1e-9 discrepancy is 10% of the matrix — a real defect
        // that an absolute 1e-10 rule would also catch, but for the wrong reason.
        let small = [1e-8, 2e-8, 2e-8 + 1e-9, 4e-8];
        assert!(
            cholesky(&small, 2).is_err(),
            "material asymmetry on a small matrix should be rejected"
        );
    }

    #[test]
    fn test_cholesky_2x2_example() -> TestResult {
        let lower = cholesky(&[4.0, 2.0, 2.0, 5.0], 2)?;

        assert!((lower[0] - 2.0).abs() < 1e-12);
        assert!(lower[1].abs() < 1e-12);
        assert!((lower[2] - 1.0).abs() < 1e-12);
        assert!((lower[3] - 2.0).abs() < 1e-12);

        Ok(())
    }
}
