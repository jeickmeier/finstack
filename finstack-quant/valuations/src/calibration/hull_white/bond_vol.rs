use super::*;

// Futures convexity adjustment

/// Compute the Hull-White 1-factor futures convexity adjustment.
///
/// Returns the adjustment (in rate terms) to convert a futures rate to a forward rate:
/// `forward = futures_rate - convexity_adjustment`.
///
/// The full HW1F futures-forward adjustment (Hull, Technical Note #1;
/// Kirikos-Novak 1997):
///
/// $$
/// \text{CA} = \frac{\sigma^2}{4\kappa} \cdot \frac{B(T_1, T_2)}{T_2 - T_1}
/// \left[ B(T_1, T_2)\,\bigl(1 - e^{-2\kappa T_1}\bigr)
///      + 2\kappa\,B(0, T_1)^2 \right]
/// $$
///
/// where:
/// - $T_1$ = futures settlement time (years from today)
/// - $T_2$ = futures end time (maturity, years from today)
/// - $\sigma$ = HW1F short-rate volatility
/// - $\kappa$ = HW1F mean-reversion speed
/// - $B(t_1, t_2) = (1 - e^{-\kappa(t_2 - t_1)}) / \kappa$
///
/// In the $\kappa \to 0$ (Ho-Lee) limit this reduces to
/// $\tfrac{1}{2}\sigma^2 T_1 T_2$, which is handled by an explicit branch to
/// avoid $0/0$ cancellation.
///
/// # Arguments
/// * `kappa` - Mean-reversion speed
/// * `sigma` - Short-rate volatility
/// * `t_settle` - Settlement time in years ($T_1$)
/// * `t_end` - End/maturity time in years ($T_2$)
///
/// # Returns
/// The convexity adjustment in the same rate units as sigma.
pub fn hw1f_convexity_adjustment(kappa: f64, sigma: f64, t_settle: f64, t_end: f64) -> f64 {
    let tau = t_end - t_settle;
    if t_settle <= 0.0 || tau <= 0.0 {
        return 0.0;
    }
    const SMALL_KAPPA: f64 = 1e-8;
    if kappa.abs() < SMALL_KAPPA {
        // Ho-Lee limit: B(t1,t2) -> t2-t1 and the bracket collapses to
        // 2κ·T1·T2, cancelling the 1/(4κ) prefactor.
        return 0.5 * sigma * sigma * t_settle * t_end;
    }
    let b_0s = hw_b(kappa, 0.0, t_settle);
    let b_se = hw_b(kappa, t_settle, t_end);
    let bracket = b_se * (1.0 - (-2.0 * kappa * t_settle).exp()) + 2.0 * kappa * b_0s * b_0s;
    sigma * sigma / (4.0 * kappa) * (b_se / tau) * bracket
}

// Internal helpers

/// B(t₁, t₂) = (1 − e^{−κ(t₂−t₁)}) / κ
pub(crate) fn hw_b(kappa: f64, t1: f64, t2: f64) -> f64 {
    let tau = t2 - t1;
    if kappa.abs() < 1e-10 {
        tau
    } else {
        (1.0 - (-kappa * tau).exp()) / kappa
    }
}

/// Zero-coupon bond option volatility:
/// σ_P(t, T, S) = B(T,S) × σ × √((1 − e^{−2κ(T−t)}) / (2κ))
pub(super) fn hw_bond_vol(kappa: f64, sigma: f64, t: f64, big_t: f64, s: f64) -> f64 {
    let b = hw_b(kappa, big_t, s);
    let var_factor = if kappa.abs() < 1e-10 {
        big_t - t
    } else {
        (1.0 - (-2.0 * kappa * (big_t - t)).exp()) / (2.0 * kappa)
    };
    b * sigma * var_factor.max(0.0).sqrt()
}

/// Zero-coupon bond-option volatility under a scheduled HW1F volatility.
///
/// The variance kernel is integrated exactly over every volatility segment:
/// `B(T,S)² ∫ₜᵀ σ(u)² exp(-2κ(T-u)) du`.
pub(crate) fn hw_bond_vol_with_model(
    params: &HullWhiteModelParams,
    t: f64,
    big_t: f64,
    s: f64,
) -> finstack_quant_core::Result<f64> {
    if !t.is_finite() || !big_t.is_finite() || !s.is_finite() || t < 0.0 || big_t < t || s < big_t {
        return Err(finstack_quant_core::Error::Validation(format!(
            "invalid HW bond option times t={t}, T={big_t}, S={s}"
        )));
    }
    let b = hw_b(params.kappa, big_t, s);
    let variance = params
        .volatility
        .integrate_squared_exp_weight(params.kappa, big_t, t, big_t)?;
    Ok(b * variance.max(0.0).sqrt())
}

/// Compute ln A(t, T) for the HW1F affine bond price model.
///
/// ln A(t,T) = ln(P(0,T)/P(0,t)) + B(t,T) f(0,t) − (σ²/4κ)(1−e^{−2κt}) B(t,T)²
///
/// The instantaneous forward `f(0,t)` is approximated by a central finite
/// difference on `ln P(0,t)`. The FD error is benign for the Jamshidian
/// swaption decomposition: the same `ln A` enters both the strike
/// `K_i = A_i e^{−B_i r*}` (through the `r*` solve) and the bond-put
/// moneyness ratio `P(0,T_i)/(K_i P(0,T₀))`, so the `B(t,T)·f(0,t)` term —
/// and any error in it — cancels exactly between the two. An earlier
/// `forward_analytic` hook for supplying the curve's analytical forward was
/// removed for this reason: it was never wired and could not change prices.
pub(super) fn hw_ln_a(kappa: f64, sigma: f64, t: f64, big_t: f64, df: &dyn Fn(f64) -> f64) -> f64 {
    let p0t = df(t);
    let p0_big_t = df(big_t);
    let b = hw_b(kappa, t, big_t);

    // Instantaneous forward rate: f(0,t) ≈ −d/dt ln P(0,t)
    let f0t = fd_forward_rate(df, t);

    let var_term = if kappa.abs() < 1e-10 {
        sigma * sigma * t * b * b / 2.0
    } else {
        sigma * sigma / (4.0 * kappa) * (1.0 - (-2.0 * kappa * t).exp()) * b * b
    };

    (p0_big_t / p0t).ln() + b * f0t - var_term
}

#[inline]
fn fd_forward_rate(df: &dyn Fn(f64) -> f64, t: f64) -> f64 {
    let h = (t * 1e-3).clamp(1e-6, 1e-3);
    if t > h {
        -(df(t + h).ln() - df(t - h).ln()) / (2.0 * h)
    } else {
        // Near t = 0: use forward difference.
        -(df(h).ln()) / h
    }
}
