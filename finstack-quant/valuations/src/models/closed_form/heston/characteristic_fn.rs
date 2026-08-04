use super::params::{HESTON_EXPONENT_REAL_LIMIT, HESTON_G_DENOM_EPS};
use super::HestonParams;
use num_complex::Complex;

/// Status of a Heston characteristic-function evaluation at one φ node.
///
/// Distinguishes the two ways ψ_j(φ) can come back as `Complex::ZERO`:
///
/// - **Overflow**: an intermediate quantity was non-finite or the exponent
///   exceeded the overflow guard — the value is *corrupt* and the node must
///   count toward the corruption diagnostic.
/// - **Underflow**: every intermediate was well-formed but |ψ| underflowed to
///   zero (deep in the decayed tail). Contributing exactly 0 to the
///   Gil-Pelaez integral is the *correct* value there, so such nodes must
///   not trip the corruption fallback.
///
/// Conflating the two made long-dated / high-κθ surfaces (where the CF
/// legitimately underflows over much of the grid) needlessly fall back to a
/// Black-Scholes price.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HestonCfStatus {
    /// Finite, non-zero value.
    Ok,
    /// Well-formed inputs; |ψ| underflowed to exactly zero (legitimate).
    Underflow,
    /// Non-finite intermediate or exponent guard hit; value zeroed (corrupt).
    Overflow,
}

/// Heston probability characteristic function ψ_j(φ) for j ∈ {1, 2}.
///
/// Uses the "Little Heston Trap" formulation from Albrecher et al. (2007)
/// to avoid branch-cut discontinuities and overflow from `exp(+dT)`.
///
/// The key change vs. the original Heston (1993) is:
/// - `g⁻ = (b - ρσφi - d) / (b - ρσφi + d)` (swapped numerator/denominator)
/// - `exp(-dT)` instead of `exp(+dT)` (avoids overflow for large T or Re(d) > 0)
///
/// # Arguments
///
/// * `j` - Probability index (1 or 2)
/// * `phi` - Fourier variable
/// * `time` - Time to maturity
/// * `log_spot` - Natural log of spot price
/// * `params` - Heston model parameters
///
/// # Returns
///
/// `(ψ_j(φ), status)`: the complex value (zeroed on overflow/underflow) and a
/// [`HestonCfStatus`] telling the caller whether a zero is legitimate
/// underflow or corruption.
///
/// # References
///
/// - Albrecher et al. (2007) — "The Little Heston Trap"
pub(super) fn heston_pj_characteristic_function(
    j: u8,
    phi: f64,
    time: f64,
    log_spot: f64,
    params: &HestonParams,
) -> (Complex<f64>, HestonCfStatus) {
    let kappa = params.kappa;
    let theta = params.theta;
    let sigma = params.sigma_v;
    let rho = params.rho;
    let v0 = params.v0;
    let r = params.r;
    let q = params.q;

    let i = Complex::new(0.0, 1.0);
    let zero = Complex::new(0.0, 0.0);

    // For P1: u = 0.5, b = kappa - rho*sigma
    // For P2: u = -0.5, b = kappa
    let (u, b) = if j == 1 {
        (0.5, kappa - rho * sigma)
    } else {
        (-0.5, kappa)
    };

    let a = kappa * theta;
    let sigma_sq = sigma * sigma;

    // d = sqrt((rho*sigma*phi*i - b)^2 - sigma^2*(2*u*phi*i - phi^2))
    let d_sq = (rho * sigma * phi * i - b).powi(2) - sigma_sq * (2.0 * u * phi * i - phi * phi);
    let d = d_sq.sqrt();

    // Little Heston Trap formulation (Albrecher et al. 2007):
    // g⁻ = (b - rho*sigma*phi*i - d) / (b - rho*sigma*phi*i + d)
    // Uses exp(-dT) to avoid overflow
    let b_minus_rsi = b - rho * sigma * phi * i;
    let g_denom = b_minus_rsi + d;
    let g_denom_limit = HESTON_G_DENOM_EPS * (1.0 + b_minus_rsi.norm() + d.norm());
    if !g_denom.is_finite() || g_denom.norm() <= g_denom_limit {
        return (zero, HestonCfStatus::Overflow);
    }
    let g_minus = (b_minus_rsi - d) / g_denom;
    if !g_minus.is_finite() {
        return (zero, HestonCfStatus::Overflow);
    }

    // exp(-d*T) — bounded, avoids the overflow of exp(+dT)
    let exp_minus_dt = (-d * time).exp();
    if !exp_minus_dt.is_finite() {
        return (zero, HestonCfStatus::Overflow);
    }

    let one = Complex::new(1.0, 0.0);

    // C = (r-q)*phi*i*T + (a/sigma^2) * [(b - rho*sigma*phi*i - d)*T
    //     - 2*ln((1 - g⁻*exp(-dT)) / (1 - g⁻))]
    let c = (r - q) * phi * i * time
        + (a / sigma_sq)
            * ((b_minus_rsi - d) * time
                - 2.0 * ((one - g_minus * exp_minus_dt) / (one - g_minus)).ln());

    // D = (b - rho*sigma*phi*i - d) / sigma^2
    //     * (1 - exp(-dT)) / (1 - g⁻*exp(-dT))
    let d_val =
        (b_minus_rsi - d) / sigma_sq * (one - exp_minus_dt) / (one - g_minus * exp_minus_dt);
    if !c.is_finite() || !d_val.is_finite() {
        return (zero, HestonCfStatus::Overflow);
    }

    // ψ_j(φ) = exp(C + D*v0 + i*φ*ln(S))
    let exponent = c + d_val * v0 + i * phi * log_spot;
    if !exponent.is_finite() || exponent.re > HESTON_EXPONENT_REAL_LIMIT {
        return (zero, HestonCfStatus::Overflow);
    }

    let psi = exponent.exp();
    if !psi.is_finite() {
        return (zero, HestonCfStatus::Overflow);
    }
    if psi.norm_sqr() == 0.0 {
        // Inputs were well-formed and the exponent finite: |ψ| genuinely
        // underflowed to zero in the decayed tail. Contributing 0 to the
        // integral is correct — not a corrupted node.
        return (zero, HestonCfStatus::Underflow);
    }
    (psi, HestonCfStatus::Ok)
}
