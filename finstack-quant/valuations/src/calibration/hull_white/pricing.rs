use super::bond_vol::hw_bond_vol;
use super::*;

/// Price a full cap/floor with a flat normal volatility quote.
pub(crate) fn bachelier_cap_floor_price(
    discount_df: &dyn Fn(f64) -> f64,
    forward_df: &dyn Fn(f64) -> f64,
    maturity: f64,
    strike: f64,
    normal_vol: f64,
    is_cap: bool,
    frequency: SwapFrequency,
) -> f64 {
    cap_floor_periods(maturity, frequency)
        .map(|(t_start, t_end, accrual)| {
            let forward = forward_rate_from_df(forward_df, t_start, t_end);
            let df = discount_df(t_end);
            // Option expiry is the fixing time `t_start`, not the payment
            // time `t_end`: the caplet's rate is fixed at the period start
            // and accrues no vol afterwards.
            normal_caplet_price(forward, strike, normal_vol, t_start, accrual, df, is_cap)
        })
        .sum()
}

pub(super) fn cap_floor_bachelier_vega(
    discount_df: &dyn Fn(f64) -> f64,
    forward_df: &dyn Fn(f64) -> f64,
    maturity: f64,
    strike: f64,
    normal_vol: f64,
    frequency: SwapFrequency,
) -> f64 {
    cap_floor_periods(maturity, frequency)
        .map(|(t_start, t_end, accrual)| {
            let forward = forward_rate_from_df(forward_df, t_start, t_end);
            let df = discount_df(t_end);
            // Vol accrues only to the fixing time `t_start` (see
            // `bachelier_cap_floor_price`).
            normal_caplet_vega(forward, strike, normal_vol, t_start) * accrual * df
        })
        .sum()
}

/// Cap/floor shape used by HW1F pricing helpers.
#[derive(Clone, Copy)]
pub(crate) struct CapFloorPriceSpec {
    pub(super) maturity: f64,
    pub(super) strike: f64,
    pub(super) is_cap: bool,
    pub(super) frequency: SwapFrequency,
}

impl CapFloorPriceSpec {
    pub(crate) fn new(maturity: f64, strike: f64, is_cap: bool, frequency: SwapFrequency) -> Self {
        Self {
            maturity,
            strike,
            is_cap,
            frequency,
        }
    }

    pub(super) fn from_quote(quote: &CapFloorQuote, frequency: SwapFrequency) -> Self {
        Self::new(quote.maturity, quote.strike, quote.is_cap, frequency)
    }
}

/// Price a full cap/floor exactly under HW1F by pricing each caplet as a
/// zero-coupon bond option.
///
/// A caplet fixing at `T`, paying `τ·max(L(T,S) − K, 0)` at `S`, equals
/// `(1 + τK)` zero-coupon bond **puts** with strike `X = 1/(1 + τK)` on
/// `P(T,S)`, expiring at `T`; a floorlet is the corresponding bond **call**
/// (Brigo–Mercurio §2.6 / Hull §31). The ZCB option is priced with the same
/// HW1F bond-option formula used by the Jamshidian swaption decomposition
/// ([`hw_bond_vol`]). This replaces the earlier mapping of HW bond vol to an
/// approximate forward-rate normal vol, which understated the caplet vol by
/// a `(1 + τF)` factor.
///
/// Dual-curve handling: the ZCB option is evaluated on the forward
/// (projection) curve and scaled by the deterministic discount/projection
/// basis `P_d(0,S)/P_f(0,S)`; for single-curve calibration the factor is 1
/// and the price is exact.
pub(crate) fn hw1f_cap_floor_price(
    kappa: f64,
    sigma: f64,
    discount_df: &dyn Fn(f64) -> f64,
    forward_df: &dyn Fn(f64) -> f64,
    spec: CapFloorPriceSpec,
) -> f64 {
    cap_floor_periods(spec.maturity, spec.frequency)
        .map(|(t_start, t_end, accrual)| {
            hw1f_caplet_price_zcb_option(
                kappa,
                sigma,
                discount_df,
                forward_df,
                t_start,
                t_end,
                accrual,
                spec.strike,
                spec.is_cap,
            )
        })
        .sum()
}

/// Price a full cap/floor under a scheduled HW1F short-rate volatility.
pub(crate) fn hw1f_cap_floor_price_with_model(
    params: &HullWhiteModelParams,
    discount_df: &dyn Fn(f64) -> f64,
    forward_df: &dyn Fn(f64) -> f64,
    spec: CapFloorPriceSpec,
) -> finstack_quant_core::Result<f64> {
    cap_floor_periods(spec.maturity, spec.frequency)
        .map(|(t_start, t_end, accrual)| {
            hw1f_term_caplet_price_from_dfs_with_model(
                params,
                forward_df(t_start),
                forward_df(t_end),
                discount_df(t_end),
                t_start,
                t_start,
                t_end,
                accrual,
                spec.strike,
                spec.is_cap,
            )
        })
        .sum()
}

/// Exact HW1F caplet/floorlet price via the ZCB-option equivalence.
///
/// Returns NaN on pathological curve inputs (non-finite or non-positive
/// discount factors) so the calibration objective's non-finite-price error
/// contract keeps working.
#[allow(clippy::too_many_arguments)]
fn hw1f_caplet_price_zcb_option(
    kappa: f64,
    sigma: f64,
    discount_df: &dyn Fn(f64) -> f64,
    forward_df: &dyn Fn(f64) -> f64,
    t_fix: f64,
    t_pay: f64,
    accrual: f64,
    strike: f64,
    is_cap: bool,
) -> f64 {
    let pf_fix = forward_df(t_fix);
    let pf_pay = forward_df(t_pay);
    let pd_pay = discount_df(t_pay);
    hw1f_caplet_price_zcb_option_from_dfs(
        kappa, sigma, pf_fix, pf_pay, pd_pay, t_fix, t_pay, accrual, strike, is_cap,
    )
}

/// Exact HW1F term-index caplet/floorlet price from curve discount factors.
///
/// The returned value is per unit notional. `pf_fix` and `pf_pay` are
/// projection-curve discount factors relative to the valuation date; `pd_pay`
/// is the discount-curve factor to the contractual payment date.
#[allow(clippy::too_many_arguments)]
pub(crate) fn hw1f_caplet_price_zcb_option_from_dfs(
    kappa: f64,
    sigma: f64,
    pf_fix: f64,
    pf_pay: f64,
    pd_pay: f64,
    t_fix: f64,
    t_pay: f64,
    accrual: f64,
    strike: f64,
    is_cap: bool,
) -> f64 {
    let valid_df = |p: f64| p.is_finite() && p > 0.0;
    if !valid_df(pf_fix) || !valid_df(pf_pay) || !valid_df(pd_pay) {
        return f64::NAN;
    }
    // Deterministic multiplicative discount/projection basis; 1.0 when the
    // two curves coincide (single-curve calibration).
    let basis = pd_pay / pf_pay;

    let gearing = 1.0 + accrual * strike;
    if gearing <= 0.0 {
        // Strike below −1/τ: a cap is always in the money (intrinsic), a
        // floor is worthless (assuming P(T,S) > 0 ⇔ 1 + τL > 0).
        if is_cap {
            let forward = (pf_fix / pf_pay - 1.0) / accrual;
            return basis * pf_pay * accrual * (forward - strike);
        }
        return 0.0;
    }
    let x_strike = 1.0 / gearing;

    let sigma_p = hw_bond_vol(kappa, sigma, 0.0, t_fix, t_pay);
    if sigma_p < 1e-15 {
        // Degenerate (zero vol or zero time to fixing): intrinsic value.
        let zcb_intrinsic = if is_cap {
            (x_strike * pf_fix - pf_pay).max(0.0)
        } else {
            (pf_pay - x_strike * pf_fix).max(0.0)
        };
        return basis * gearing * zcb_intrinsic;
    }

    let d1 = (pf_pay / (x_strike * pf_fix)).ln() / sigma_p + 0.5 * sigma_p;
    let d2 = d1 - sigma_p;
    // Caplet = (1+τK) × ZBP(0, T, S, X); floorlet = (1+τK) × ZBC(0, T, S, X).
    let zcb_option = if is_cap {
        x_strike * pf_fix * norm_cdf(-d2) - pf_pay * norm_cdf(-d1)
    } else {
        pf_pay * norm_cdf(d1) - x_strike * pf_fix * norm_cdf(d2)
    };
    let zcb_option_clamped = if zcb_option < 0.0 { 0.0 } else { zcb_option };
    basis * gearing * zcb_option_clamped
}

/// Exact HW1F term-index caplet/floorlet price under a scheduled volatility.
#[allow(clippy::too_many_arguments)]
pub(crate) fn hw1f_term_caplet_price_from_dfs_with_model(
    params: &HullWhiteModelParams,
    pf_start: f64,
    pf_end: f64,
    pd_pay: f64,
    t_fix: f64,
    t_start: f64,
    t_end: f64,
    accrual: f64,
    strike: f64,
    is_cap: bool,
) -> finstack_quant_core::Result<f64> {
    let valid_df = |value: f64| value.is_finite() && value > 0.0;
    if !valid_df(pf_start)
        || !valid_df(pf_end)
        || !valid_df(pd_pay)
        || accrual <= 0.0
        || t_end <= t_start
        || t_start < t_fix
    {
        return Err(finstack_quant_core::Error::Validation(
            "invalid term caplet discount factors or times".into(),
        ));
    }
    let ratio_forward = pf_start / pf_end;
    let ratio_strike = 1.0 + accrual * strike;
    if ratio_strike <= 0.0 {
        return if is_cap {
            Ok(pd_pay * (ratio_forward - ratio_strike))
        } else {
            Ok(0.0)
        };
    }

    let ratio_vol = (hw_bond_vol_with_model(params, 0.0, t_fix, t_end)?
        - hw_bond_vol_with_model(params, 0.0, t_fix, t_start)?)
    .abs();
    if ratio_vol < 1.0e-15 {
        let intrinsic = if is_cap {
            (ratio_forward - ratio_strike).max(0.0)
        } else {
            (ratio_strike - ratio_forward).max(0.0)
        };
        return Ok(pd_pay * intrinsic);
    }

    let d1 = (ratio_forward / ratio_strike).ln() / ratio_vol + 0.5 * ratio_vol;
    let d2 = d1 - ratio_vol;
    let option = if is_cap {
        ratio_forward * norm_cdf(d1) - ratio_strike * norm_cdf(d2)
    } else {
        ratio_strike * norm_cdf(-d2) - ratio_forward * norm_cdf(-d1)
    };
    Ok(pd_pay * option.max(0.0))
}

/// Return the flat normal vol that reproduces the HW1F cap/floor model price.
#[cfg(test)]
pub(crate) fn hw1f_cap_floor_implied_normal_vol(
    kappa: f64,
    sigma: f64,
    discount_df: &dyn Fn(f64) -> f64,
    forward_df: &dyn Fn(f64) -> f64,
    spec: CapFloorPriceSpec,
) -> f64 {
    let target = hw1f_cap_floor_price(kappa, sigma, discount_df, forward_df, spec);
    let residual = |vol: f64| -> f64 {
        bachelier_cap_floor_price(
            discount_df,
            forward_df,
            spec.maturity,
            spec.strike,
            vol,
            spec.is_cap,
            spec.frequency,
        ) - target
    };
    let mut hi = sigma.max(0.01);
    while residual(hi) < 0.0 && hi < 1.0 {
        hi *= 2.0;
    }
    BrentSolver::new()
        .tolerance(1e-12)
        .bracket_bounds(1e-10, hi)
        .solve(residual, hi * 0.5)
        .unwrap_or(hi)
}

pub(crate) fn hw1f_caplet_forward_rate_normal_vol(
    kappa: f64,
    sigma: f64,
    t_fix: f64,
    accrual: f64,
) -> f64 {
    if sigma <= 0.0 || t_fix <= 0.0 || accrual <= 0.0 {
        return 0.0;
    }
    const SMALL_KAPPA: f64 = 1e-8;
    let accrual_factor = if kappa.abs() < SMALL_KAPPA {
        1.0
    } else {
        (1.0 - (-kappa * accrual).exp()) / (kappa * accrual)
    };
    let integrated_variance_time = if kappa.abs() < SMALL_KAPPA {
        t_fix
    } else {
        (1.0 - (-2.0 * kappa * t_fix).exp()) / (2.0 * kappa)
    };
    sigma * accrual_factor * (integrated_variance_time / t_fix).sqrt()
}

/// Caplet periods `(t_start, t_end, accrual)` for a spot-start cap quote.
///
/// The first (spot-start) caplet is **excluded**: its rate fixes at `t = 0`,
/// so it carries no optionality, and standard market cap quotes exclude it.
/// Both the market (Bachelier) and model (HW1F) legs use this iterator, so
/// the convention is applied consistently to both sides of the calibration.
pub(super) fn cap_floor_periods(
    maturity: f64,
    frequency: SwapFrequency,
) -> impl Iterator<Item = (f64, f64, f64)> {
    let periods = (maturity * frequency.periods_per_year() as f64)
        .round()
        .max(1.0) as usize;
    let accrual = maturity / periods as f64;
    (1..periods).map(move |idx| {
        let start = idx as f64 * accrual;
        let end = (idx + 1) as f64 * accrual;
        (start, end, accrual)
    })
}

/// Simple forward rate between `start` and `end` from a discount-factor
/// function.
///
/// Non-finite or non-positive discount factors propagate as `NaN` instead of
/// being clamped: callers (`HullWhiteCapFloorTarget::calculate_residuals`,
/// `solve_cap_floor_sigma_for_fixed_kappa`) rely on the non-finite-price
/// check to detect broken curves, and `f64::max` would silently absorb a NaN
/// (`NaN.max(1e-12) == 1e-12`), defeating that error contract.
pub(super) fn forward_rate_from_df(df: &dyn Fn(f64) -> f64, start: f64, end: f64) -> f64 {
    let accrual = (end - start).max(1e-12);
    let p_start = df(start);
    let p_end = df(end);
    if !p_start.is_finite() || !p_end.is_finite() || p_start <= 0.0 || p_end <= 0.0 {
        return f64::NAN;
    }
    (p_start / p_end - 1.0) / accrual
}

pub(super) fn normal_caplet_price(
    forward: f64,
    strike: f64,
    vol: f64,
    expiry: f64,
    accrual: f64,
    df: f64,
    is_cap: bool,
) -> f64 {
    let annuity = accrual * df;
    if vol <= 0.0 || expiry <= 0.0 {
        let intrinsic = if is_cap {
            (forward - strike).max(0.0)
        } else {
            (strike - forward).max(0.0)
        };
        return intrinsic * annuity;
    }
    let sqrt_t = expiry.sqrt();
    let d = (forward - strike) / (vol * sqrt_t);
    let undiscounted = if is_cap {
        (forward - strike) * norm_cdf(d) + vol * sqrt_t * norm_pdf(d)
    } else {
        (strike - forward) * norm_cdf(-d) + vol * sqrt_t * norm_pdf(d)
    };
    undiscounted * annuity
}

fn normal_caplet_vega(forward: f64, strike: f64, vol: f64, expiry: f64) -> f64 {
    if vol <= 0.0 || expiry <= 0.0 {
        return 0.0;
    }
    let d = (forward - strike) / (vol * expiry.sqrt());
    expiry.sqrt() * norm_pdf(d)
}
