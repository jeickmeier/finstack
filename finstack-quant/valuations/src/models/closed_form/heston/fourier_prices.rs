use super::params::HESTON_TAIL_DIAGNOSTIC_THRESHOLD;
use super::quadrature::{
    composite_gauss_legendre_grid, heston_pj_on_grid, heston_pj_with_diagnostics,
};
use super::{HestonFourierSettings, HestonParams, HestonStripPricer};
use crate::instruments::common_impl::parameters::OptionType;
use crate::models::closed_form::vanilla::bs_price;
use tracing::warn;

/// Price a European call option under the Heston model using Fourier inversion.
///
/// # Arguments
///
/// * `spot` - Current underlying spot price in the option quote currency.
/// * `strike` - Exercise price in the same units as `spot`.
/// * `time` - Remaining time to maturity in years.
/// * `params` - Validated Heston rate, carry, variance, mean-reversion,
///   volatility-of-variance, and correlation parameters.
///
/// # Returns
///
/// Call option price
///
/// # Formula
///
/// C = S * exp(-qT) * P1 - K * exp(-rT) * P2
///
/// where P1 and P2 are risk-neutral probabilities computed via Fourier inversion.
///
/// # Integration Settings
///
/// Uses [`HestonFourierSettings::for_maturity_with_variance`] to adapt the
/// integration grid to the option's time to maturity and initial variance.
/// Short-dated and low-variance options use wider/finer grids to handle the
/// slower-decaying characteristic function. For custom control, use
/// [`heston_call_price_fourier_with_settings`].
///
/// # Example
///
/// ```text
/// use finstack_quant_valuations::models::closed_form::heston::{
///     heston_call_price_fourier, HestonParams,
/// };
///
/// let params = HestonParams::new(
///     0.05,  // risk-free rate
///     0.02,  // dividend yield
///     2.0,   // kappa (mean reversion)
///     0.04,  // theta (long-run variance)
///     0.3,   // sigma_v (vol-of-vol)
///     -0.7,  // rho (correlation)
///     0.04,  // v0 (initial variance)
/// )
/// .unwrap();
///
/// let price = heston_call_price_fourier(100.0, 100.0, 1.0, &params);
/// assert!(price > 0.0 && price < 100.0);
/// ```
#[must_use]
pub fn heston_call_price_fourier(spot: f64, strike: f64, time: f64, params: &HestonParams) -> f64 {
    heston_call_price_fourier_with_settings(
        spot,
        strike,
        time,
        params,
        &HestonFourierSettings::for_maturity_with_variance(time, params.v0),
    )
}

/// Price a strip of European call options under the Heston model using shared
/// characteristic-function precomputation.
///
/// # Arguments
///
/// * `spot` - Current underlying spot price in the option quote currency.
/// * `strikes` - Exercise prices in result order, each in the same units as
///   `spot`; the returned vector has the same length and order.
/// * `time` - Common remaining time to expiry in years.
/// * `params` - Validated Heston rate, carry, variance, mean-reversion,
///   volatility-of-variance, and correlation parameters.
#[must_use]
pub fn heston_call_prices_fourier(
    spot: f64,
    strikes: &[f64],
    time: f64,
    params: &HestonParams,
) -> Vec<f64> {
    heston_call_prices_fourier_with_settings(
        spot,
        strikes,
        time,
        params,
        &HestonFourierSettings::for_maturity_with_variance(time, params.v0),
    )
}

/// Price a strip of European call options with custom integration settings.
///
/// # Arguments
///
/// * `spot` - Current underlying spot price in the option quote currency.
/// * `strikes` - Exercise prices in result order, each in the same units as
///   `spot`; the returned vector has the same length and order.
/// * `time` - Common remaining time to expiry in years.
/// * `params` - Validated Heston rate, carry, variance, mean-reversion,
///   volatility-of-variance, and correlation parameters.
/// * `settings` - Fourier integration grid, truncation, and damping settings
///   applied consistently to the whole strike strip.
#[must_use]
pub fn heston_call_prices_fourier_with_settings(
    spot: f64,
    strikes: &[f64],
    time: f64,
    params: &HestonParams,
    settings: &HestonFourierSettings,
) -> Vec<f64> {
    if time <= 0.0 {
        return strikes
            .iter()
            .map(|&strike| (spot - strike).max(0.0))
            .collect();
    }

    if params.sigma_v < 1e-10 {
        let avg_vol = params.deterministic_avg_variance(time).sqrt();
        return strikes
            .iter()
            .map(|&strike| black_scholes_call(spot, strike, time, params.r, params.q, avg_vol))
            .collect();
    }

    if let Some(pricer) = HestonStripPricer::new(spot, time, params, settings) {
        pricer.price_calls(strikes)
    } else {
        strikes
            .iter()
            .map(|&strike| {
                heston_call_price_fourier_with_settings(spot, strike, time, params, settings)
            })
            .collect()
    }
}

/// Price a strip of European put options under the Heston model using shared
/// characteristic-function precomputation.
///
/// # Arguments
///
/// * `spot` - Current underlying spot price in the option quote currency.
/// * `strikes` - Exercise prices in result order, each in the same units as
///   `spot`; the returned vector has the same length and order.
/// * `time` - Common remaining time to expiry in years.
/// * `params` - Validated Heston rate, carry, variance, mean-reversion,
///   volatility-of-variance, and correlation parameters.
#[must_use]
pub fn heston_put_prices_fourier(
    spot: f64,
    strikes: &[f64],
    time: f64,
    params: &HestonParams,
) -> Vec<f64> {
    heston_put_prices_fourier_with_settings(
        spot,
        strikes,
        time,
        params,
        &HestonFourierSettings::for_maturity_with_variance(time, params.v0),
    )
}

/// Price a strip of European put options with custom integration settings.
///
/// # Arguments
///
/// * `spot` - Current underlying spot price in the option quote currency.
/// * `strikes` - Exercise prices in result order, each in the same units as
///   `spot`; the returned vector has the same length and order.
/// * `time` - Common remaining time to expiry in years.
/// * `params` - Validated Heston rate, carry, variance, mean-reversion,
///   volatility-of-variance, and correlation parameters.
/// * `settings` - Fourier integration grid, truncation, and damping settings
///   applied consistently to the whole strike strip.
#[must_use]
pub fn heston_put_prices_fourier_with_settings(
    spot: f64,
    strikes: &[f64],
    time: f64,
    params: &HestonParams,
    settings: &HestonFourierSettings,
) -> Vec<f64> {
    if time <= 0.0 {
        return strikes
            .iter()
            .map(|&strike| (strike - spot).max(0.0))
            .collect();
    }

    let call_prices =
        heston_call_prices_fourier_with_settings(spot, strikes, time, params, settings);
    call_prices
        .into_iter()
        .zip(strikes.iter())
        .map(|(call_price, strike)| {
            let forward = spot * (-params.q * time).exp();
            let discount_k = *strike * (-params.r * time).exp();
            (call_price - forward + discount_k).max(0.0)
        })
        .collect()
}

/// Price a European call option with custom integration settings.
///
/// See [`heston_call_price_fourier`] for details.
///
/// # Arguments
///
/// * `spot` - Current underlying spot price in the option quote currency.
/// * `strike` - Exercise price in the same units as `spot`.
/// * `time` - Remaining time to expiry in years.
/// * `params` - Validated Heston rate, carry, variance, mean-reversion,
///   volatility-of-variance, and correlation parameters.
/// * `settings` - Fourier integration grid, truncation, and damping settings
///   for this single-option inversion.
#[must_use]
pub fn heston_call_price_fourier_with_settings(
    spot: f64,
    strike: f64,
    time: f64,
    params: &HestonParams,
    settings: &HestonFourierSettings,
) -> f64 {
    // Handle expired options
    if time <= 0.0 {
        return (spot - strike).max(0.0);
    }

    // Special case: very small vol-of-vol approaches Black-Scholes with the
    // deterministic average variance v̄(T) (σ_v → 0 collapses the variance to
    // its deterministic mean-reverting path).
    if params.sigma_v < 1e-10 {
        return black_scholes_call(
            spot,
            strike,
            time,
            params.r,
            params.q,
            params.deterministic_avg_variance(time).sqrt(),
        );
    }

    // Compute P1 and P2 via Fourier inversion, with diagnostics. The composite
    // Gauss-Legendre grid depends only on `settings`, so build it once and share
    // it across the j=1 / j=2 evaluations rather than rebuilding it twice. The
    // degenerate-settings path (grid build fails) falls back to the
    // self-contained `heston_pj_with_diagnostics`, which uses library quadrature.
    let grid =
        composite_gauss_legendre_grid(0.0, settings.u_max, settings.gl_order, settings.panels);
    let (d1, d2) = match &grid {
        Some(g) => (
            heston_pj_on_grid(1, spot, strike, time, params, settings, g),
            heston_pj_on_grid(2, spot, strike, time, params, settings, g),
        ),
        None => (
            heston_pj_with_diagnostics(1, spot, strike, time, params, settings),
            heston_pj_with_diagnostics(2, spot, strike, time, params, settings),
        ),
    };

    // Audit item 5: characteristic-function overflow corruption fallback.
    // `heston_pj_characteristic_function` reports `HestonCfStatus::Overflow`
    // for ill-formed nodes (legitimate underflow is excluded); when a large
    // fraction of integration nodes overflowed the Gil-Pelaez integral
    // silently loses mass and yields a plausible-but-wrong probability.
    // The strip pricer already detects this and falls back to Black-Scholes —
    // the scalar path must do the same rather than integrating zeros into a
    // finite-but-wrong price.
    if d1.corrupted || d2.corrupted {
        warn!(
            spot,
            strike,
            time,
            kappa = params.kappa,
            theta = params.theta,
            sigma_v = params.sigma_v,
            rho = params.rho,
            v0 = params.v0,
            "Heston scalar Fourier integrand corrupted (characteristic function \
             overflowed on too many integration nodes); falling back to a \
             Black-Scholes price at the deterministic average vol sqrt(v_bar(T))"
        );
        return black_scholes_call(
            spot,
            strike,
            time,
            params.r,
            params.q,
            params.deterministic_avg_variance(time).sqrt(),
        );
    }

    // Audit item 4: truncation-tail diagnostic. The Gil-Pelaez integral is
    // truncated at a fixed `u_max`; a non-negligible tail beyond `u_max`, or a
    // pre-clamp probability materially outside `[0, 1]`, means the truncated
    // integral mis-priced and the `[0, 1]` clamp is hiding it. Surface a
    // diagnostic so the mis-truncation is observable (short-dated wings are the
    // typical trigger) instead of being silently clamped away.
    let tail = d1.tail_estimate.max(d2.tail_estimate);
    let raw_p1_excursion = (d1.raw_probability - d1.raw_probability.clamp(0.0, 1.0)).abs();
    let raw_p2_excursion = (d2.raw_probability - d2.raw_probability.clamp(0.0, 1.0)).abs();
    let raw_excursion = raw_p1_excursion.max(raw_p2_excursion);
    if tail > HESTON_TAIL_DIAGNOSTIC_THRESHOLD || raw_excursion > HESTON_TAIL_DIAGNOSTIC_THRESHOLD {
        warn!(
            spot,
            strike,
            time,
            u_max = settings.u_max,
            tail_estimate = tail,
            raw_probability_excursion = raw_excursion,
            "Heston Gil-Pelaez integral truncated at u_max with a non-negligible \
             residual tail (or a pre-clamp probability outside [0,1]); the price \
             may be mis-truncated — consider a larger u_max (e.g. \
             HestonFourierSettings::for_maturity_with_variance for short \
             maturities or low initial variance)"
        );
    }

    // C = S * exp(-qT) * P1 - K * exp(-rT) * P2
    let call_price = spot * (-params.q * time).exp() * d1.probability
        - strike * (-params.r * time).exp() * d2.probability;

    // Non-finite Fourier integral (extreme params / CF overflow): price with
    // Black-Scholes at deterministic avg vol sqrt(v_bar(T)) instead of returning
    // zero/NaN for deep-OTM or short-dated cases.
    if !call_price.is_finite() {
        return black_scholes_call(
            spot,
            strike,
            time,
            params.r,
            params.q,
            params.deterministic_avg_variance(time).sqrt(),
        );
    }

    // Clamp to non-negative (numerical errors can cause tiny negatives for deep OTM)
    call_price.max(0.0)
}

/// Price a European put option under the Heston model using Fourier inversion.
///
/// # Arguments
///
/// * `spot` - Current underlying spot price in the option quote currency.
/// * `strike` - Exercise price in the same units as `spot`.
/// * `time` - Remaining time to maturity in years.
/// * `params` - Validated Heston rate, carry, variance, mean-reversion,
///   volatility-of-variance, and correlation parameters.
///
/// # Returns
///
/// Put option price
///
/// # Formula
///
/// Uses put-call parity: P = C - S*exp(-qT) + K*exp(-rT)
#[must_use]
pub fn heston_put_price_fourier(spot: f64, strike: f64, time: f64, params: &HestonParams) -> f64 {
    heston_put_price_fourier_with_settings(
        spot,
        strike,
        time,
        params,
        &HestonFourierSettings::for_maturity_with_variance(time, params.v0),
    )
}

/// Price a European put option with custom integration settings.
///
/// See [`heston_put_price_fourier`] for details.
///
/// # Arguments
///
/// * `spot` - Current underlying spot price in the option quote currency.
/// * `strike` - Exercise price in the same units as `spot`.
/// * `time` - Remaining time to expiry in years.
/// * `params` - Validated Heston rate, carry, variance, mean-reversion,
///   volatility-of-variance, and correlation parameters.
/// * `settings` - Fourier integration grid, truncation, and damping settings
///   for this single-option inversion.
pub fn heston_put_price_fourier_with_settings(
    spot: f64,
    strike: f64,
    time: f64,
    params: &HestonParams,
    settings: &HestonFourierSettings,
) -> f64 {
    if time <= 0.0 {
        return (strike - spot).max(0.0);
    }

    // Use put-call parity: P = C - S*exp(-qT) + K*exp(-rT)
    let call_price = heston_call_price_fourier_with_settings(spot, strike, time, params, settings);
    let forward = spot * (-params.q * time).exp();
    let discount_k = strike * (-params.r * time).exp();

    let put_price = call_price - forward + discount_k;
    if !put_price.is_finite() {
        // Mirror the call-side fallback so put pricing degrades to BS rather than
        // returning zero on extreme parameters.
        let bs_call = black_scholes_call(
            spot,
            strike,
            time,
            params.r,
            params.q,
            params.deterministic_avg_variance(time).sqrt(),
        );
        return (bs_call - forward + discount_k).max(0.0);
    }
    put_price.max(0.0)
}

/// Black-Scholes call price (fallback for sigma_v ≈ 0).
pub(super) fn black_scholes_call(
    spot: f64,
    strike: f64,
    time: f64,
    r: f64,
    q: f64,
    vol: f64,
) -> f64 {
    bs_price(spot, strike, r, q, vol, time, OptionType::Call)
}
