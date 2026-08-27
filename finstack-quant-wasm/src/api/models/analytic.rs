//! Closed-form analytic option primitives (Black-Scholes, Black-76, implied vol).
//!
//! Thin wasm-bindgen wrappers around the Rust closed-form formulas in
//! `finstack_quant_models::closed_form`.
//!
//! All rates are continuously compounded decimals; `sigma` is annualized vol;
//! `t` is time to expiry in years. Greeks scale matches the Rust crate:
//! `vega` and both rho values are per 1% move, `theta` is per-day under the
//! `thetaDays` day-count (ACT/365 by default).
//!
//! Named-model sources: `docs/REFERENCES.md#black-scholes-1973`,
//! `docs/REFERENCES.md#merton-1973`, `docs/REFERENCES.md#garman-kohlhagen-1983`,
//! `docs/REFERENCES.md#black-1976`.

use crate::utils::to_js_err;
use finstack_quant_models::closed_form::implied_vol::{
    black76_implied_vol as black76_implied_vol_core, bs_implied_vol as bs_implied_vol_core,
};
use finstack_quant_models::closed_form::{
    asian_option_price_str, barrier_call_str, bs_greeks_checked as bs_greeks_core,
    bs_price_checked, lookback_option_price_str, option_type_from_bool,
    quanto_option_price_checked, vanilla_expiry_payoff as vanilla_expiry_payoff_core,
};
use wasm_bindgen::prelude::*;

const DEFAULT_THETA_DAYS_PER_YEAR: f64 = 365.0;

/// Per-unit Black-Scholes / Garman-Kohlhagen price of a European option.
///
/// Black-Scholes (1973): see docs/REFERENCES.md#black-scholes-1973.
/// Merton (1973): see docs/REFERENCES.md#merton-1973.
/// Garman-Kohlhagen (1983): see docs/REFERENCES.md#garman-kohlhagen-1983.
///
/// @param spot - Spot price of the underlying.
/// @param strike - Strike of the option.
/// @param r - Risk-free rate, **decimal** continuously compounded
/// (e.g. `0.05` for 5%).
/// @param q - Continuous dividend yield (or foreign rate for FX),
/// **decimal** continuously compounded.
/// @param sigma - Annualized volatility, **decimal**
/// (e.g. `0.20` for 20%).
/// @param t - Time to expiry in **years**.
/// @param isCall - `true` for a call, `false` for a put.
/// @returns Per-unit option price.
///
/// @example
/// ```javascript
/// import init, { models } from "finstack-quant-wasm";
/// await init();
/// const price = models.bsPrice(
///   100,    // spot
///   100,    // strike (ATM)
///   0.05,   // r = 5%
///   0.0,    // q = 0
///   0.20,   // sigma = 20%
///   1.0,    // 1 year
///   true,   // call
/// );
/// // price ≈ 10.45
/// ```
///
/// @throws If the inputs produce a non-finite price (e.g. negative volatility).
#[wasm_bindgen(js_name = bsPrice)]
pub fn bs_price(
    spot: f64,
    strike: f64,
    r: f64,
    q: f64,
    sigma: f64,
    t: f64,
    is_call: bool,
) -> Result<f64, JsValue> {
    bs_price_checked(spot, strike, r, q, sigma, t, option_type_from_bool(is_call))
        .map_err(to_js_err)
}

/// Vanilla option payoff at expiry: `max(±(spot - strike), 0)`.
///
/// @param spot - Underlying level at expiry, in the same price units as `strike`.
///   Must be finite and non-negative; zero spot is allowed.
/// @param strike - Exercise price; must be finite and strictly positive.
/// @param isCall - `true` for a call (`max(spot - strike, 0)`), `false` for a
/// put (`max(strike - spot, 0)`).
/// @returns Undiscounted expiry payoff in the same units as `spot` and `strike`.
///
/// @example
/// ```javascript
/// import init, { models } from "finstack-quant-wasm";
/// await init();
/// const payoff = valuations.vanillaExpiryPayoff(110, 100, true);
/// // payoff === 10
/// ```
///
/// @throws If `spot` is non-finite or negative, or `strike` is non-finite or
/// not strictly positive.
#[wasm_bindgen(js_name = vanillaExpiryPayoff)]
pub fn vanilla_expiry_payoff(spot: f64, strike: f64, is_call: bool) -> Result<f64, JsValue> {
    vanilla_expiry_payoff_core(spot, strike, option_type_from_bool(is_call)).map_err(to_js_err)
}

/// Black-Scholes / Garman-Kohlhagen Greeks as a `{delta, gamma, vega, theta, rho, rho_q}` object.
///
/// Black-Scholes (1973): see docs/REFERENCES.md#black-scholes-1973.
/// Merton (1973): see docs/REFERENCES.md#merton-1973.
/// Garman-Kohlhagen (1983): see docs/REFERENCES.md#garman-kohlhagen-1983.
///
/// @param spot - Spot price of the underlying.
/// @param strike - Strike of the option.
/// @param r - Risk-free rate, **decimal** continuously compounded.
/// @param q - Dividend yield (or foreign rate for FX), **decimal**
/// continuously compounded.
/// @param sigma - Annualized volatility, **decimal**.
/// @param t - Time to expiry in **years**.
/// @param isCall - `true` for a call, `false` for a put.
/// @param thetaDays - Day-count denominator for theta. Default `365`.
/// Pass `252` for trading-day theta.
/// @returns Object `{ delta, gamma, vega, theta, rho, rho_q }` (snake_case keys
/// matching the Rust/Python canonical names). `vega` and
/// both rho values are **per 1% move**; `theta` is **per day** under
/// `thetaDays`.
/// @throws If serialization to JS fails (should not happen on valid inputs).
///
/// @example
/// ```javascript
/// const g = models.bsGreeks(100, 100, 0.05, 0.0, 0.20, 1.0, true);
/// // g.delta ≈ 0.64, g.gamma ≈ 0.019, g.vega ≈ 0.38 (per 1% vol)
/// ```
#[wasm_bindgen(js_name = bsGreeks)]
#[allow(clippy::too_many_arguments)]
pub fn bs_greeks(
    spot: f64,
    strike: f64,
    r: f64,
    q: f64,
    sigma: f64,
    t: f64,
    is_call: bool,
    theta_days: Option<f64>,
) -> Result<JsValue, JsValue> {
    // theta_days validation (finite, > 0) lives in `bs_greeks_checked` —
    // the single home for Greeks input validation.
    let theta_days = theta_days.unwrap_or(DEFAULT_THETA_DAYS_PER_YEAR);
    let g = bs_greeks_core(
        spot,
        strike,
        r,
        q,
        sigma,
        t,
        option_type_from_bool(is_call),
        theta_days,
    )
    .map_err(to_js_err)?;
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"delta".into(), &g.delta.into())?;
    js_sys::Reflect::set(&obj, &"gamma".into(), &g.gamma.into())?;
    js_sys::Reflect::set(&obj, &"vega".into(), &g.vega.into())?;
    js_sys::Reflect::set(&obj, &"theta".into(), &g.theta.into())?;
    js_sys::Reflect::set(&obj, &"rho".into(), &g.rho_r.into())?;
    // snake_case to match the Rust canonical field (`rho_q`) and the Python
    // binding; the camelCase `rhoQ` was an outlier that yielded `undefined`
    // for any cross-binding consumer reading `rho_q`.
    js_sys::Reflect::set(&obj, &"rho_q".into(), &g.rho_q.into())?;
    Ok(obj.into())
}

/// Solve for Black-Scholes / Garman-Kohlhagen implied volatility.
///
/// Black-Scholes (1973): see docs/REFERENCES.md#black-scholes-1973.
/// Merton (1973): see docs/REFERENCES.md#merton-1973.
/// Garman-Kohlhagen (1983): see docs/REFERENCES.md#garman-kohlhagen-1983.
///
/// @param spot - Spot price of the underlying.
/// @param strike - Strike of the option.
/// @param r - Risk-free rate, **decimal** continuously compounded.
/// @param q - Dividend yield, **decimal** continuously compounded.
/// @param t - Time to expiry in **years**.
/// @param price - Observed option price (per unit).
/// @param isCall - `true` for a call, `false` for a put.
/// @returns Annualized implied volatility, **decimal** (e.g. `0.20`).
/// @throws If `price` is below intrinsic value, above the no-arbitrage
/// upper bound, or the solver fails to converge.
///
/// @example
/// ```javascript
/// const iv = models.bsImpliedVol(100, 100, 0.05, 0.0, 1.0, 10.45, true);
/// // iv ≈ 0.20
/// ```
#[wasm_bindgen(js_name = bsImpliedVol)]
pub fn bs_implied_vol(
    spot: f64,
    strike: f64,
    r: f64,
    q: f64,
    t: f64,
    price: f64,
    is_call: bool,
) -> Result<f64, JsValue> {
    bs_implied_vol_core(spot, strike, r, q, t, option_type_from_bool(is_call), price)
        .map_err(to_js_err)
}

/// Solve for Black-76 (forward-based) implied volatility.
///
/// Black (1976): see docs/REFERENCES.md#black-1976.
/// @param forward - Forward price or rate in the same quote convention as the strike.
/// @param strike - Option strike price in the same price units as the underlying.
/// @param df - Discount factor from valuation to expiry, expressed as a positive decimal.
/// @param t - Time from the curve base date in years.
/// @param price - Observed option price in the same units as the forward.
/// @param is_call - Whether to value a call (`true`) or put (`false`).
///
/// # Errors
///
/// Throws a JavaScript exception if an input is non-finite; `forward`,
/// `strike`, `df`, or `price` is not positive; the price is not above intrinsic
/// value or cannot be bracketed; or the implied-volatility solver does not
/// converge. A non-positive `t` returns zero volatility.
#[wasm_bindgen(js_name = black76ImpliedVol)]
pub fn black76_implied_vol(
    forward: f64,
    strike: f64,
    df: f64,
    t: f64,
    price: f64,
    is_call: bool,
) -> Result<f64, JsValue> {
    black76_implied_vol_core(
        forward,
        strike,
        df,
        t,
        option_type_from_bool(is_call),
        price,
    )
    .map_err(to_js_err)
}

/// Reiner-Rubinstein continuous-monitoring barrier call price.
///
/// `direction` is `"up"` or `"down"`, `knock` is `"in"` or `"out"`.
/// Reiner-Rubinstein (1991): see docs/REFERENCES.md#reiner-rubinstein-1991.
/// @param spot - Current spot price or exchange rate in the same units as the strike.
/// @param strike - Option strike price in the same price units as the underlying.
/// @param barrier - Continuously monitored barrier level in the same price units as spot.
/// @param r - Continuously compounded risk-free rate, expressed as a decimal.
/// @param q - Continuous dividend yield or foreign rate, expressed as a decimal.
/// @param sigma - Annualized volatility expressed as a decimal, such as 0.20 for 20%.
/// @param t - Time from the curve base date in years.
/// @param direction - Barrier direction: `"up"` for an upper barrier or `"down"` for a lower barrier.
/// @param knock - Barrier activation: `"in"` for knock-in or `"out"` for knock-out.
///
/// # Errors
///
/// Throws a JavaScript exception if `direction` or `knock` is unsupported, or
/// the supplied model inputs produce a non-finite barrier price.
#[wasm_bindgen(js_name = barrierCall)]
#[allow(clippy::too_many_arguments)]
pub fn barrier_call(
    spot: f64,
    strike: f64,
    barrier: f64,
    r: f64,
    q: f64,
    sigma: f64,
    t: f64,
    direction: &str,
    knock: &str,
) -> Result<f64, JsValue> {
    barrier_call_str(spot, strike, barrier, t, r, q, sigma, direction, knock).map_err(to_js_err)
}

/// Arithmetic (Turnbull-Wakeman) or geometric (Kemna-Vorst) Asian option.
///
/// Kemna-Vorst (1990): see docs/REFERENCES.md#kemna-vorst-1990.
/// Turnbull-Wakeman (1991): see docs/REFERENCES.md#turnbull-wakeman-1991.
/// @param spot - Current spot price or exchange rate in the same units as the strike.
/// @param strike - Option strike price in the same price units as the underlying.
/// @param r - Continuously compounded risk-free rate, expressed as a decimal.
/// @param q - Continuous dividend yield or foreign rate, expressed as a decimal.
/// @param sigma - Annualized volatility expressed as a decimal, such as 0.20 for 20%.
/// @param t - Time from the curve base date in years.
/// @param num_fixings - Positive number of equally spaced averaging observations before expiry.
/// @param averaging - Asian averaging convention: `"arithmetic"` (default) or `"geometric"`.
/// @param is_call - Whether to value a call (`true`) or put (`false`).
///
/// # Errors
///
/// Throws a JavaScript exception if `averaging` is not `"arithmetic"` or
/// `"geometric"`, or the supplied model inputs produce a non-finite option
/// price.
#[wasm_bindgen(js_name = asianOptionPrice)]
#[allow(clippy::too_many_arguments)]
pub fn asian_option_price(
    spot: f64,
    strike: f64,
    r: f64,
    q: f64,
    sigma: f64,
    t: f64,
    num_fixings: usize,
    averaging: Option<String>,
    is_call: Option<bool>,
) -> Result<f64, JsValue> {
    let averaging = averaging.as_deref().unwrap_or("arithmetic");
    let option_type = option_type_from_bool(is_call.unwrap_or(true));
    asian_option_price_str(
        spot,
        strike,
        t,
        r,
        q,
        sigma,
        num_fixings,
        averaging,
        option_type,
    )
    .map_err(to_js_err)
}

/// Conze-Viswanathan lookback option.
///
/// `strike_type` is `"fixed"` (default) or `"floating"`. For `"floating"`,
/// `strike` is ignored and `extremum` is the observed min/max to date.
/// Conze-Viswanathan (1991): see docs/REFERENCES.md#conze-viswanathan-1991.
/// @param spot - Current spot price or exchange rate in the same units as the strike.
/// @param strike - Option strike price in the same price units as the underlying.
/// @param r - Continuously compounded risk-free rate, expressed as a decimal.
/// @param q - Continuous dividend yield or foreign rate, expressed as a decimal.
/// @param sigma - Annualized volatility expressed as a decimal, such as 0.20 for 20%.
/// @param t - Time from the curve base date in years.
/// @param extremum - Observed running minimum for a call or maximum for a put, in spot-price units.
/// @param strike_type - Lookback payoff convention: `"fixed"` (default) or `"floating"`.
/// @param is_call - Whether to value a call (`true`) or put (`false`).
///
/// # Errors
///
/// Throws a JavaScript exception if `strikeType` is not `"fixed"` or
/// `"floating"`, or the supplied model inputs produce a non-finite option
/// price.
#[wasm_bindgen(js_name = lookbackOptionPrice)]
#[allow(clippy::too_many_arguments)]
pub fn lookback_option_price(
    spot: f64,
    strike: f64,
    r: f64,
    q: f64,
    sigma: f64,
    t: f64,
    extremum: f64,
    strike_type: Option<String>,
    is_call: Option<bool>,
) -> Result<f64, JsValue> {
    let strike_type = strike_type.as_deref().unwrap_or("fixed");
    let option_type = option_type_from_bool(is_call.unwrap_or(true));
    lookback_option_price_str(
        spot,
        strike,
        t,
        r,
        q,
        sigma,
        extremum,
        strike_type,
        option_type,
    )
    .map_err(to_js_err)
}

/// Quanto option (FX-adjusted cross-currency) price in domestic currency.
///
/// Garman-Kohlhagen (1983): see docs/REFERENCES.md#garman-kohlhagen-1983.
/// Brigo-Mercurio (2006): see docs/REFERENCES.md#brigo-mercurio-2006-interest-rate-models.
///
/// @throws If the inputs produce a non-finite price.
/// @param spot - Current spot price or exchange rate in the same units as the strike.
/// @param strike - Option strike price in the same price units as the underlying.
/// @param t - Time from the curve base date in years.
/// @param rate_domestic - Domestic continuously compounded risk-free rate, expressed as a decimal.
/// @param rate_foreign - Foreign continuously compounded risk-free rate, expressed as a decimal.
/// @param div_yield - Continuous dividend yield expressed as a decimal, such as 0.02 for 2%.
/// @param vol_asset - Annualized asset-price volatility expressed as a decimal.
/// @param vol_fx - Annualized FX-rate volatility expressed as a decimal.
/// @param correlation - Instantaneous correlation between the asset and FX-rate shocks, from -1 to 1.
/// @param is_call - Whether to value a call (`true`) or put (`false`).
#[wasm_bindgen(js_name = quantoOptionPrice)]
#[allow(clippy::too_many_arguments)]
pub fn quanto_option_price(
    spot: f64,
    strike: f64,
    t: f64,
    rate_domestic: f64,
    rate_foreign: f64,
    div_yield: f64,
    vol_asset: f64,
    vol_fx: f64,
    correlation: f64,
    is_call: Option<bool>,
) -> Result<f64, JsValue> {
    quanto_option_price_checked(
        spot,
        strike,
        t,
        rate_domestic,
        rate_foreign,
        div_yield,
        vol_asset,
        vol_fx,
        correlation,
        option_type_from_bool(is_call.unwrap_or(true)),
    )
    .map_err(to_js_err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bs_price_call_atm_is_positive() {
        let p = bs_price(100.0, 100.0, 0.05, 0.02, 0.2, 1.0, true).expect("finite price");
        assert!(p > 0.0);
    }

    #[test]
    fn vanilla_expiry_payoff_call_itm() {
        let payoff = vanilla_expiry_payoff(110.0, 100.0, true).expect("finite payoff");
        assert!((payoff - 10.0).abs() < 1e-12);
    }

    #[test]
    fn vanilla_expiry_payoff_rejects_negative_spot() {
        assert!(vanilla_expiry_payoff(-1.0, 100.0, true).is_err());
        let put = vanilla_expiry_payoff(0.0, 100.0, false).expect("zero spot put");
        assert!((put - 100.0).abs() < 1e-12);
    }

    #[test]
    fn bs_implied_vol_recovers_sigma() {
        let sigma = 0.25;
        let price = bs_price(100.0, 110.0, 0.03, 0.01, sigma, 0.75, true).expect("finite price");
        let iv = bs_implied_vol(100.0, 110.0, 0.03, 0.01, 0.75, price, true)
            .expect("solver should converge");
        assert!((iv - sigma).abs() < 1e-6, "iv={iv} sigma={sigma}");
    }

    #[test]
    fn bs_price_rejects_non_finite_result() {
        // A degenerate input (huge maturity with a negative rate) drives
        // `exp(-r*t)` to `+inf`, which escapes the core's `.max(0.0)` clamp.
        // The binding guard must surface that as a thrown error rather than a
        // silent non-finite value crossing the wasm boundary.
        let result = bs_price(100.0, 100.0, -1.0, 0.0, 0.2, 1.0e6, false);
        assert!(
            result.is_err(),
            "a non-finite Black-Scholes price must produce an error"
        );
        // A well-posed input still returns a finite price unchanged.
        assert!(bs_price(100.0, 100.0, 0.05, 0.02, 0.2, 1.0, true).is_ok());
    }

    #[test]
    fn quanto_option_price_rejects_non_finite_result() {
        // Same degenerate-maturity path: a non-finite quanto price must throw.
        let result = quanto_option_price(
            100.0,
            100.0,
            1.0e6,
            -1.0,
            0.01,
            0.0,
            0.20,
            0.10,
            0.3,
            Some(false),
        );
        assert!(
            result.is_err(),
            "a non-finite quanto option price must produce an error"
        );
        // A well-posed input still returns a finite price.
        assert!(quanto_option_price(
            100.0,
            100.0,
            1.0,
            0.03,
            0.01,
            0.0,
            0.20,
            0.10,
            0.3,
            Some(true)
        )
        .is_ok());
    }
}
