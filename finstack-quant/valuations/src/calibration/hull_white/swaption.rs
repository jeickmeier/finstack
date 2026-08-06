use super::bond_vol::{hw_b, hw_bond_vol, hw_ln_a};
use super::targets::{
    floor_vega_and_record, reject_at_bound_params, HullWhiteSwaptionTarget, PreparedSwaption,
    HW_NUM_RESTARTS, HW_PERTURB_SCALE, HW_VALIDATION_TOLERANCE, SWAPTION_VEGA_FLOOR,
};
use super::*;

/// Calibrate Hull-White 1-factor parameters to European swaption market data.
///
/// Fits κ (mean reversion) and σ (short rate volatility) by minimising
/// squared differences between model and market swaption prices.
///
/// # Arguments
///
/// * `df` - Discount factor function: `df(t)` returns P(0, t). Must satisfy `df(0) ≈ 1`.
/// * `quotes` - Swaption market data.
/// * `frequency` - Coupon frequency of the underlying swap (e.g., semi-annual for USD,
///   annual for EUR). This materially affects the annuity factor and forward swap rate.
/// * `initial_guess` - Optional seed for (κ, σ). Pass `None` to use built-in defaults.
///
/// # Returns
///
/// Calibrated [`HullWhiteParams`] and a [`CalibrationReport`] with residual diagnostics.
///
/// # Algorithm
///
/// 1. For each swaption quote, compute the market price from the quoted vol.
/// 2. Model prices are computed analytically via the Jamshidian (1989) decomposition.
/// 3. The Levenberg-Marquardt solver minimises the sum of squared price errors,
///    routed through `GlobalFitOptimizer` so HW1F shares the same numeric
///    plumbing (multi-start, diagnostics, error reporting) as curve calibration.
/// 4. Uses the unconstrained parameterisation: `(ln κ, ln σ)`.
///
/// # Residual scaling (ATM assumption)
///
/// Each per-quote residual is `(price_model − price_mkt) / vega`, where
/// `vega` is the *ATM* Bachelier / Black-76 vega evaluated via
/// `swaption_atm_vega` (strike = forward swap rate). This linearisation
/// converges to the right minimiser when the calibration set is at-the-money
/// or close to it: at ATM the strike-vol slope is small and the ATM-vega
/// is a good proxy for the true `dPrice/dVol`. For materially off-ATM
/// quotes (deep ITM/OTM swaptions), the ATM-vega proxy under- or over-scales
/// the residual depending on the smile, and the LM objective is then a
/// *distorted* (but still descent-compatible) surface. If you need to
/// calibrate to a smile, weight or down-weight off-ATM quotes externally,
/// or invest in true implied-vol-error iteration as in Andersen-Piterbarg
/// (*Interest Rate Modeling* Vol III §3.3). A `vega_floor_hits` count is
/// reported in the result metadata; investigate any non-zero count before
/// trusting the calibrated `(κ, σ)`.
///
/// # Errors
///
/// Returns an error if:
/// - Fewer than 2 quotes are provided (2 free parameters)
/// - Calibration fails to converge
/// - Discount function returns invalid values
///
/// # Examples
///
/// ```
/// use finstack_quant_valuations::calibration::hull_white::{
///     calibrate_hull_white_to_swaptions, SwaptionQuote, SwapFrequency,
/// };
///
/// let quotes = vec![
///     SwaptionQuote { expiry: 1.0, tenor: 5.0, volatility: 0.005, is_normal_vol: true },
///     SwaptionQuote { expiry: 5.0, tenor: 5.0, volatility: 0.006, is_normal_vol: true },
///     SwaptionQuote { expiry: 10.0, tenor: 5.0, volatility: 0.005, is_normal_vol: true },
/// ];
///
/// // Flat 3% discount curve, semi-annual USD convention
/// let df = |t: f64| (-0.03 * t).exp();
/// let (params, report) = calibrate_hull_white_to_swaptions(
///     &df, &quotes, SwapFrequency::SemiAnnual, None,
/// ).unwrap();
/// assert!(report.success);
/// ```
pub fn calibrate_hull_white_to_swaptions(
    df: &dyn Fn(f64) -> f64,
    quotes: &[SwaptionQuote],
    frequency: SwapFrequency,
    initial_guess: Option<HullWhiteParams>,
) -> finstack_quant_core::Result<(HullWhiteParams, CalibrationReport)> {
    calibrate_hull_white_to_swaptions_core(df, quotes, frequency, None, initial_guess, None)
}

fn calibrate_hull_white_to_swaptions_core(
    df: &dyn Fn(f64) -> f64,
    quotes: &[SwaptionQuote],
    frequency: SwapFrequency,
    schedules: Option<&[Vec<f64>]>,
    initial_guess: Option<HullWhiteParams>,
    schedule_source: Option<&'static str>,
) -> finstack_quant_core::Result<(HullWhiteParams, CalibrationReport)> {
    if quotes.len() < 2 {
        return Err(finstack_quant_core::Error::Validation(format!(
            "Need at least 2 swaption quotes for HW1F calibration (2 free parameters), got {}",
            quotes.len()
        )));
    }
    if let Some(schedules) = schedules {
        if schedules.len() != quotes.len() {
            return Err(finstack_quant_core::Error::Validation(format!(
                "schedules.len() ({}) must match quotes.len() ({})",
                schedules.len(),
                quotes.len()
            )));
        }
    }
    for (i, q) in quotes.iter().enumerate() {
        if q.expiry <= 0.0 || q.tenor <= 0.0 || q.volatility <= 0.0 {
            return Err(finstack_quant_core::Error::Validation(format!(
                "Invalid swaption quote at index {i}: expiry={}, tenor={}, vol={}",
                q.expiry, q.tenor, q.volatility
            )));
        }
    }

    let n_quotes = quotes.len();
    let ppy = frequency.periods_per_year();

    // Pre-compute market data once; the LM hot loop only does numeric ops.
    let mut prepared = Vec::with_capacity(n_quotes);
    let mut fwd_swap_rates = Vec::with_capacity(n_quotes);
    let mut vega_floor_hits: Vec<String> = Vec::new();
    let mut schedule_fallbacks: Vec<String> = Vec::new();
    for (idx, q) in quotes.iter().enumerate() {
        // Validate the per-quote schedule up-front with the same predicate
        // the pricer uses (`valid_swap_accruals`), so the metadata stamp
        // reflects what the calibration actually consumed. A malformed
        // schedule falls back to the synthetic constant-dt recipe — that
        // fallback is kept, but it is no longer silent: it is warned about
        // and stamped per quote in the report metadata.
        let n_periods = (q.tenor * ppy as f64).round().max(1.0) as usize;
        let accruals_slice =
            schedules.and_then(|s| valid_swap_accruals(Some(s[idx].as_slice()), n_periods));
        if schedules.is_some() && accruals_slice.is_none() {
            let label = format!("{}Yx{}Y", q.expiry, q.tenor);
            tracing::warn!(
                quote = label.as_str(),
                "HW1F swaption calibration: per-quote accrual schedule is malformed \
                 (wrong length or non-positive entries); falling back to the synthetic \
                 constant-dt schedule for this quote"
            );
            schedule_fallbacks.push(label);
        }
        let (annuity, fwd_rate) = if let Some(accruals) = accruals_slice {
            compute_swap_annuity_and_rate_inner(df, q.expiry, q.tenor, ppy, Some(accruals))
        } else {
            compute_swap_annuity_and_rate(df, q.expiry, q.tenor, ppy)
        };
        let market_price = compute_swaption_market_price(
            annuity,
            fwd_rate,
            q.expiry,
            q.volatility,
            q.is_normal_vol,
        );
        let raw_vega =
            swaption_atm_vega(annuity, fwd_rate, q.expiry, q.volatility, q.is_normal_vol);
        let label = format!("{}Yx{}Y", q.expiry, q.tenor);
        let vega =
            floor_vega_and_record(raw_vega, SWAPTION_VEGA_FLOOR, &label, &mut vega_floor_hits);
        prepared.push(PreparedSwaption {
            market_price,
            fwd_swap_rate: fwd_rate,
            vega,
            accruals: accruals_slice.map(|s| s.to_vec().into_boxed_slice()),
        });
        fwd_swap_rates.push(fwd_rate);
    }

    let (default_kappa_init, default_sigma_init) = infer_hw_initial_guess(quotes, &fwd_swap_rates);
    let kappa_init: f64 = initial_guess.map(|p| p.kappa).unwrap_or(default_kappa_init);
    let sigma_init: f64 = initial_guess.map(|p| p.sigma).unwrap_or(default_sigma_init);
    let x0 = [kappa_init.ln(), sigma_init.ln()];

    let target = HullWhiteSwaptionTarget {
        df,
        ppy,
        initial_x0: x0,
        prepared,
    };

    // Use solver tolerance 1e-12 (matches the prior hand-rolled LM
    // settings) and validation tolerance 1e-6 (the historical
    // accept/reject threshold for HW1F price residuals).
    let mut config = CalibrationConfig::default();
    config.solver = config.solver.with_tolerance(1e-12).with_max_iterations(300);

    let multi_start = MultiStartConfig {
        num_restarts: HW_NUM_RESTARTS,
        perturbation_scale: HW_PERTURB_SCALE,
    };

    let (params, mut report) = GlobalFitOptimizer::optimize_with_multi_start(
        &target,
        quotes,
        &config,
        Some(HW_VALIDATION_TOLERANCE),
        Some(&multi_start),
    )?;

    // Override the report type tag (stored in metadata["type"]) and add
    // HW-specific metadata. The framework reports a generic "global_fit"
    // type; HW consumers expect "hull_white_1f" for serialization stability.
    report = report
        .with_model_version(finstack_quant_core::versions::HULL_WHITE_1F)
        .with_metadata("type", "hull_white_1f".to_string())
        .with_metadata("kappa", format!("{:.6}", params.kappa))
        .with_metadata("sigma", format!("{:.6}", params.sigma))
        .with_metadata("initial_kappa", format!("{kappa_init:.6}"))
        .with_metadata("initial_sigma", format!("{sigma_init:.6}"))
        .with_metadata("multi_start_restarts", HW_NUM_RESTARTS.to_string())
        .with_metadata(
            "residual_weighting",
            "1/vega (vega-weighted price residual)".to_string(),
        )
        .with_metadata("swap_frequency", frequency.to_string())
        .with_metadata("vega_floor_hits", vega_floor_hits.len().to_string());
    if let Some(schedule_source) = schedule_source {
        // Stamp the actual per-basket source: if any quote fell back to the
        // synthetic schedule, the basket is "mixed" and the fallback quotes
        // are listed so the analyst can see exactly which quotes were not
        // priced on real day counts.
        let actual_source = if schedule_fallbacks.is_empty() {
            schedule_source.to_string()
        } else if schedule_fallbacks.len() == quotes.len() {
            "synthetic_constant_dt".to_string()
        } else {
            "mixed".to_string()
        };
        report = report.with_metadata("schedule_source", actual_source);
        if !schedule_fallbacks.is_empty() {
            report = report
                .with_metadata(
                    "schedule_fallback_count",
                    schedule_fallbacks.len().to_string(),
                )
                .with_metadata("schedule_fallback_quotes", schedule_fallbacks.join("; "));
        }
    }
    if !vega_floor_hits.is_empty() {
        report = report.with_metadata("vega_floor_hits_detail", vega_floor_hits.join("; "));
    }

    reject_at_bound_params(
        params.kappa,
        params.sigma,
        "Hull-White swaption calibration",
    )?;

    // Final validation of (κ, σ) > 0 — `HullWhiteParams::new` is the
    // canonical gate.
    let params = HullWhiteParams::new(params.kappa, params.sigma)?;
    Ok((params, report))
}

/// Calibrate HW1F to swaptions using *real* per-period accrual year fractions.
///
/// Functionally identical to [`calibrate_hull_white_to_swaptions`] but takes
/// per-quote accrual schedules so the synthetic constant-`dt` schedule is
/// replaced by genuine market day-counts (e.g. Act/360 USD SOFR, 30/360 EUR
/// EURIBOR). This brings calibrated `(κ, σ)` into tight parity with
/// vendor models (Bloomberg VCUB, QuantLib `Gaussian1dSwaptionEngine`) that
/// use real schedules.
///
/// # Arguments
///
/// * `df` - Discount-factor function where `df(t)` returns `P(0,t)` for a
///   year fraction `t` measured on the same time axis as the quote expiries
///   and tenors. It must return finite positive factors and `df(0) ≈ 1`.
/// * `quotes` - ATM European swaption quotes to fit. Their expiries, tenors,
///   and volatility conventions must be consistent with `df` and `frequency`.
/// * `frequency` - Fixed-leg payment frequency used to construct each
///   underlying swap's schedule and annuity.
/// * `schedules` - Per-quote accrual year fractions, where `schedules[i]`
///   corresponds to `quotes[i]`.
///   Must contain `(quotes[i].tenor * frequency.periods_per_year()).round()`
///   strictly-positive values; their sum must equal `quotes[i].tenor` to
///   within numerical precision. If any schedule is malformed, the calibrator
///   falls back to the constant-`dt` recipe for that quote, emits a
///   `tracing::warn!`, and stamps the report metadata: `schedule_source`
///   becomes `"mixed"` (or `"synthetic_constant_dt"` when every quote fell
///   back) and `schedule_fallback_quotes` lists the affected quotes.
/// * `initial_guess` - Optional starting `(kappa, sigma)` parameters for the
///   optimizer. `None` uses the calibrator's bounded default seed.
///
/// # OIS-Specific Limitations
///
/// HW1F swaption calibration here treats every leg as a vanilla fixed-vs.-
/// IBOR swap. For OIS swaptions (compounded-in-arrears), the daily compounding
/// inside each accrual period is approximated by a single forward rate — the
/// HW1F r* equation does not capture the daily reset structure. This is
/// acceptable for ATM or near-ATM calibration (the loss is well below typical
/// market vol-of-vol noise) but is not appropriate for term-RFR-strict
/// calibration. The cap/floor path uses the analytical HW1F caplet vol
/// formula and is unaffected.
pub fn calibrate_hull_white_to_swaptions_with_schedules(
    df: &dyn Fn(f64) -> f64,
    quotes: &[SwaptionQuote],
    frequency: SwapFrequency,
    schedules: &[Vec<f64>],
    initial_guess: Option<HullWhiteParams>,
) -> finstack_quant_core::Result<(HullWhiteParams, CalibrationReport)> {
    calibrate_hull_white_to_swaptions_core(
        df,
        quotes,
        frequency,
        Some(schedules),
        initial_guess,
        Some("real_day_count"),
    )
}

/// ATM vega for a swaption expressed in the same volatility units as the
/// quote (Bachelier σ for normal vol, Black-76 σ for lognormal).
///
/// Used as the per-quote weight in the vega-weighted price residual; see
/// the module-level note in `calibrate_hull_white_to_swaptions`.
fn swaption_atm_vega(annuity: f64, fwd_rate: f64, expiry: f64, vol: f64, is_normal: bool) -> f64 {
    if is_normal {
        annuity
            * finstack_quant_core::math::volatility::bachelier_vega(fwd_rate, fwd_rate, vol, expiry)
    } else {
        annuity * finstack_quant_core::math::volatility::black_vega(fwd_rate, fwd_rate, vol, expiry)
    }
}

/// Compute annuity and forward swap rate for a swap starting at `t0`
/// with given `tenor` and `periods_per_year` coupon payments.
///
/// The schedule is synthetic (constant `dt = tenor/n_periods`). For real
/// market day-counts (Act/360 USD SOFR, 30/360 EUR EURIBOR, etc.), use
/// [`compute_swap_annuity_and_rate_with_accruals`] and pass the actual
/// per-period year fractions.
pub(crate) fn compute_swap_annuity_and_rate(
    df: &dyn Fn(f64) -> f64,
    t0: f64,
    tenor: f64,
    periods_per_year: usize,
) -> (f64, f64) {
    compute_swap_annuity_and_rate_inner(df, t0, tenor, periods_per_year, None)
}

pub(super) fn compute_swap_annuity_and_rate_inner(
    df: &dyn Fn(f64) -> f64,
    t0: f64,
    tenor: f64,
    periods_per_year: usize,
    accruals: Option<&[f64]>,
) -> (f64, f64) {
    let n_periods = (tenor * periods_per_year as f64).round().max(1.0) as usize;

    let real_accruals = valid_swap_accruals(accruals, n_periods);

    let mut annuity = 0.0;
    let mut t_running = t0;
    if let Some(accruals) = real_accruals {
        for &tau in accruals {
            t_running += tau;
            annuity += tau * df(t_running);
        }
    } else {
        let dt = tenor / n_periods as f64;
        for i in 1..=n_periods {
            let t_i = t0 + i as f64 * dt;
            annuity += dt * df(t_i);
        }
        t_running = t0 + tenor;
    }

    let t_n = t_running;
    let fwd_rate = if annuity > 1e-15 {
        (df(t0) - df(t_n)) / annuity
    } else {
        let p0 = df(t0).max(1e-12);
        let p_n = df(t_n).max(1e-12);
        ((p0 / p_n).ln() / tenor.max(1e-8)).max(0.0)
    };

    (annuity, fwd_rate)
}

#[inline]
pub(super) fn valid_swap_accruals(accruals: Option<&[f64]>, n_periods: usize) -> Option<&[f64]> {
    accruals.filter(|a| a.len() == n_periods && a.iter().all(|x| x.is_finite() && *x > 0.0))
}

pub(super) fn infer_hw_initial_guess(
    quotes: &[SwaptionQuote],
    fwd_swap_rates: &[f64],
) -> (f64, f64) {
    let horizon = if quotes.is_empty() {
        5.0
    } else {
        quotes.iter().map(|q| q.expiry + 0.5 * q.tenor).sum::<f64>() / quotes.len() as f64
    };
    // Average ABSOLUTE-rate vol, branched per quote on the vol regime so
    // the σ seed never conflates Bachelier and Black quotes (W-39):
    //  - normal (Bachelier) quote: the vol is already an absolute-rate
    //    vol, so it contributes directly;
    //  - lognormal (Black) quote: the vol is dimensionless, so `vol·fwd`
    //    recovers an absolute-rate scale.
    // The HW1F σ is an absolute short-rate vol, so this average is the
    // right order of magnitude for the seed.
    let avg_abs_vol = if quotes.is_empty() {
        0.01 * 0.02 // fallback: ~1% Black vol at a 2% forward.
    } else {
        let sum: f64 = quotes
            .iter()
            .enumerate()
            .map(|(i, q)| {
                let v = q.volatility.abs();
                if q.is_normal_vol {
                    v
                } else {
                    // fwd_swap_rates is built quote-aligned by the callers;
                    // fall back to a 2% forward if the slice is short.
                    let fwd = fwd_swap_rates.get(i).map_or(0.02, |r| r.abs()).max(0.005);
                    v * fwd
                }
            })
            .sum();
        sum / quotes.len() as f64
    };

    let kappa_init = (1.0 / horizon.max(0.5)).clamp(0.01, 0.30);
    let sigma_init = avg_abs_vol.clamp(0.001, 0.05);
    (kappa_init, sigma_init)
}

/// Compute the market swaption price from the quoted volatility.
pub(super) fn compute_swaption_market_price(
    annuity: f64,
    fwd_rate: f64,
    expiry: f64,
    vol: f64,
    is_normal: bool,
) -> f64 {
    if is_normal {
        // Bachelier: ATM payer price ≈ annuity × σ_n × √T × √(2/π) ≈ annuity × bachelier_call
        annuity
            * finstack_quant_core::math::volatility::bachelier_call(fwd_rate, fwd_rate, vol, expiry)
    } else {
        // Black-76: annuity × black_call(F, F, σ, T)
        annuity * finstack_quant_core::math::volatility::black_call(fwd_rate, fwd_rate, vol, expiry)
    }
}

/// Price a European payer swaption under HW1F using Jamshidian decomposition.
///
/// The Jamshidian decomposition expresses a swaption as a portfolio of
/// zero-coupon bond options. The key steps are:
///
/// 1. Find the critical short rate r* where the swap value equals par.
/// 2. Each leg becomes a put on a zero-coupon bond with strike K_i = P_HW(r*, T₀, T_i).
/// 3. Sum the individual zero-coupon bond put prices.
///
/// Uses a synthetic constant-`dt` schedule. The production HW1F calibrator
/// (`calibrate_hull_white_to_swaptions_with_schedules`) drives
/// [`hw1f_swaption_price_inner`] directly with real accrual fractions, so
/// this scalar-time wrapper exists primarily as a stable test harness.
#[allow(dead_code)]
pub(crate) fn hw1f_swaption_price(
    kappa: f64,
    sigma: f64,
    df: &dyn Fn(f64) -> f64,
    t0: f64,
    tenor: f64,
    swap_rate: f64,
    periods_per_year: usize,
) -> f64 {
    hw1f_swaption_price_inner(Hw1fSwaptionPriceInput {
        kappa,
        sigma,
        df,
        t0,
        tenor,
        swap_rate,
        periods_per_year,
        accruals: None,
    })
}

pub(super) struct Hw1fSwaptionPriceInput<'a> {
    pub(super) kappa: f64,
    pub(super) sigma: f64,
    pub(super) df: &'a dyn Fn(f64) -> f64,
    pub(super) t0: f64,
    pub(super) tenor: f64,
    pub(super) swap_rate: f64,
    pub(super) periods_per_year: usize,
    pub(super) accruals: Option<&'a [f64]>,
}

pub(super) fn hw1f_swaption_price_inner(
    Hw1fSwaptionPriceInput {
        kappa,
        sigma,
        df,
        t0,
        tenor,
        swap_rate,
        periods_per_year,
        accruals,
    }: Hw1fSwaptionPriceInput<'_>,
) -> f64 {
    let n_periods = (tenor * periods_per_year as f64).round().max(1.0) as usize;

    let real_accruals = valid_swap_accruals(accruals, n_periods);

    // Payment dates and cashflows
    let mut payment_times = Vec::with_capacity(n_periods);
    let mut cashflows = Vec::with_capacity(n_periods);
    if let Some(accruals) = real_accruals {
        let mut t_running = t0;
        for (i, &tau) in accruals.iter().enumerate() {
            t_running += tau;
            payment_times.push(t_running);
            let cf = if i + 1 < n_periods {
                swap_rate * tau
            } else {
                1.0 + swap_rate * tau
            };
            cashflows.push(cf);
        }
    } else {
        let dt = tenor / n_periods as f64;
        for i in 1..=n_periods {
            let t_i = t0 + i as f64 * dt;
            payment_times.push(t_i);
            let cf = if i < n_periods {
                swap_rate * dt
            } else {
                1.0 + swap_rate * dt
            };
            cashflows.push(cf);
        }
    }

    // Pre-compute B and ln A for each payment date
    let b_vals: Vec<f64> = payment_times
        .iter()
        .map(|&t_i| hw_b(kappa, t0, t_i))
        .collect();
    let ln_a_vals: Vec<f64> = payment_times
        .iter()
        .map(|&t_i| hw_ln_a(kappa, sigma, t0, t_i, df))
        .collect();

    // Find r* such that Σ c_i × A_i × exp(−B_i × r*) = 1
    // g(r) = Σ c_i exp(ln_A_i − B_i r) − 1
    // g'(r) = −Σ c_i B_i exp(ln_A_i − B_i r)
    let g = |r: f64| -> f64 {
        let mut sum = 0.0;
        for i in 0..n_periods {
            sum += cashflows[i] * (ln_a_vals[i] - b_vals[i] * r).exp();
        }
        sum - 1.0
    };

    let g_prime = |r: f64| -> f64 {
        let mut sum = 0.0;
        for i in 0..n_periods {
            sum -= cashflows[i] * b_vals[i] * (ln_a_vals[i] - b_vals[i] * r).exp();
        }
        sum
    };

    // Natural magnitude scale of `g'(r)` at a given `r`: the sum of the *absolute
    // values* of the per-cashflow terms that make up `g'`. `g'` itself is a signed sum
    // and can suffer catastrophic cancellation; comparing `|g'|` against this scale
    // (rather than a fixed absolute floor) detects a numerically near-flat objective.
    let g_prime_scale = |r: f64| -> f64 {
        let mut sum = 0.0;
        for i in 0..n_periods {
            sum += (cashflows[i] * b_vals[i] * (ln_a_vals[i] - b_vals[i] * r).exp()).abs();
        }
        sum
    };

    // Initial guess: the instantaneous forward rate at t0
    let h = (t0 * 1e-3).clamp(1e-6, 1e-3);
    let f0t0 = if t0 > h {
        -(df(t0 + h).ln() - df(t0 - h).ln()) / (2.0 * h)
    } else {
        -(df(h).ln()) / h
    };

    // Newton iterations to find r*.
    //
    // Derivative guard (item 5): a fixed `|g'| < 1e-15` *absolute* floor is the wrong
    // criterion. A `g'` of ~1e-10 — a near-flat objective — sails straight past it, and
    // `step = g / g'` then explodes to a ~1e8-scale jump that throws the iterate far
    // outside any plausible short-rate range. Two scale-aware guards replace it:
    //
    //  1. A *relative* derivative-magnitude guard: `|g'|` must be a non-trivial fraction
    //     of its own term-wise magnitude scale `Σ|c_i B_i e^…|`. This catches the
    //     catastrophic-cancellation regime where the signed sum `g'` collapses toward
    //     zero while its constituent terms are not.
    //  2. A safeguarded step bound: even a "large enough" `g'` can yield an absurd step
    //     when the objective is flat. A Newton step that would move `r` by more than
    //     `NEWTON_MAX_STEP` is untrustworthy; we hand off to the bracketed Brent
    //     fallback instead of accepting the jump.
    let mut r_star = f0t0;
    let mut newton_converged = false;
    const NEWTON_DERIV_REL_EPS: f64 = 1e-10;
    // Cap on a single Newton step in absolute short-rate units. A short rate moving by
    // more than 5.0 (500%) in one step is non-physical; the Brent fallback bracket is
    // sized to cover the plausible range under HW1F dynamics.
    const NEWTON_MAX_STEP: f64 = 5.0;
    for _ in 0..50 {
        let gv = g(r_star);
        let gp = g_prime(r_star);
        let gp_scale = g_prime_scale(r_star);
        // Near-flat / fully-cancelled derivative: hand off to Brent rather than take an
        // unbounded Newton step.
        if !gp.is_finite() || gp.abs() <= NEWTON_DERIV_REL_EPS * gp_scale.max(f64::MIN_POSITIVE) {
            break;
        }
        let step = gv / gp;
        // A non-finite or absurdly large step means the local linearisation is
        // unreliable (near-flat objective); stop and let Brent bracket the root.
        if !step.is_finite() || step.abs() > NEWTON_MAX_STEP {
            break;
        }
        r_star -= step;
        if step.abs() < 1e-12 {
            newton_converged = true;
            break;
        }
    }
    // Newton may have walked the iterate to a non-finite value before the step-size
    // convergence test fired; treat that as non-convergence so the Brent fallback runs.
    if !r_star.is_finite() {
        newton_converged = false;
    }

    // Brent fallback if Newton didn't converge.
    //
    // Bracket width must scale with both rate level and HW1F vol-to-expiry to
    // stay valid under negative-rate (EUR) and distressed-sovereign regimes.
    // The previous fixed `±0.20` bracket was too narrow for f0 ≈ 15% sovereign
    // yields and too narrow at long expiries where σ√t0 dominates.
    //
    // Heuristic: half-width = max(0.5, 5·σ√t0) — covers ±5σ of the short-rate
    // distribution under HW1F (more than enough to bracket r*) plus a 50bp
    // floor for short-expiry, low-vol cases.
    if !newton_converged {
        tracing::warn!(
            "HW1F r* Newton solver did not converge (kappa={kappa:.4}, sigma={sigma:.4}), \
             falling back to Brent"
        );
        let half_width = (5.0 * sigma * t0.sqrt()).max(0.5);
        let bracket_lo = f0t0 - half_width;
        let bracket_hi = f0t0 + half_width;
        let brent = BrentSolver::new()
            .tolerance(1e-12)
            .bracket_bounds(bracket_lo, bracket_hi);
        match brent.solve(g, f0t0) {
            Ok(r) => r_star = r,
            Err(_) => {
                tracing::warn!("HW1F r* Brent fallback also failed; returning NaN");
                r_star = f64::NAN;
            }
        }
    }

    // r* solver failure (NaN) and pathological discount factors must propagate
    // as NaN to the caller — `.max(0.0)` would silently turn NaN into 0.0
    // because IEEE 754 `max(NaN, 0.0) == 0.0`, fooling the LM closure into
    // treating the input as a legitimate zero-price swaption.
    if !r_star.is_finite() {
        return f64::NAN;
    }

    // Compute strike prices K_i = A_i × exp(−B_i × r*)
    let k_strikes: Vec<f64> = (0..n_periods)
        .map(|i| (ln_a_vals[i] - b_vals[i] * r_star).exp())
        .collect();

    // Sum zero-coupon bond put prices (payer swaption = portfolio of bond puts)
    // ZBO_put(0, T₀, T_i, K_i) = K_i P(0,T₀) N(−d₂) − P(0,T_i) N(−d₁)
    let p0_t0 = df(t0);
    if !(p0_t0 > 0.0 && p0_t0.is_finite()) {
        return f64::NAN;
    }
    let mut swaption_price = 0.0;

    for i in 0..n_periods {
        let t_i = payment_times[i];
        let p0_ti = df(t_i);
        if !(p0_ti > 0.0 && p0_ti.is_finite()) {
            return f64::NAN;
        }
        let sigma_p = hw_bond_vol(kappa, sigma, 0.0, t0, t_i);

        if sigma_p < 1e-15 {
            // Degenerate: intrinsic value. `< 0.0` is false for NaN so NaN
            // would propagate, but inputs are positive-finite by the checks
            // above, so the subtraction is safe.
            let put_intrinsic_raw = k_strikes[i] * p0_t0 - p0_ti;
            let put_intrinsic = if put_intrinsic_raw < 0.0 {
                0.0
            } else {
                put_intrinsic_raw
            };
            swaption_price += cashflows[i] * put_intrinsic;
            continue;
        }

        let d1 = ((p0_ti / (k_strikes[i] * p0_t0)).ln() + 0.5 * sigma_p * sigma_p) / sigma_p;
        let d2 = d1 - sigma_p;

        let put_price = k_strikes[i] * p0_t0 * norm_cdf(-d2) - p0_ti * norm_cdf(-d1);
        // Preserve NaN: `put_price < 0.0` is false for NaN, so NaN flows
        // through; only genuinely-negative numerical noise gets clamped.
        let put_price_clamped = if put_price < 0.0 { 0.0 } else { put_price };
        swaption_price += cashflows[i] * put_price_clamped;
    }

    if swaption_price < 0.0 {
        0.0
    } else {
        swaption_price
    }
}

// Tests
