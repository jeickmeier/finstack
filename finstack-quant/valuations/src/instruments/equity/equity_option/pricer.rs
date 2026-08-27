//! Equity option Black–Scholes pricing engine and greeks.
//!
//! Provides deterministic PV and greeks for `EquityOption` using the
//! Black–Scholes model with continuous dividend yield. Volatility is
//! sourced from a surface (clamped) unless overridden. This mirrors the
//! structure used by `fx_option` and keeps pricing logic separate from
//! instrument definitions.

use crate::instruments::common_impl::helpers::year_fraction;
use crate::instruments::common_impl::parameters::{OptionMarketParams, OptionType};
use crate::instruments::equity::equity_option::types::EquityOption;
use crate::instruments::{ExerciseStyle, SettlementType};
use crate::pricer::{ModelKey, PricingError, PricingErrorContext};
use finstack_quant_core::currency::Currency;
use finstack_quant_core::dates::{Date, DayCount};
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::money::Money;
use finstack_quant_core::Result;
use finstack_quant_models::closed_form::vanilla::{bs_greeks_unchecked, bs_price_unchecked};
use finstack_quant_models::trees::binomial_tree::BinomialTree;

/// Reject exercise styles that a selected model does not actually model.
pub(crate) fn require_european(inst: &EquityOption, model: &str) -> Result<()> {
    if !matches!(inst.exercise_style, ExerciseStyle::European) {
        return Err(finstack_quant_core::Error::Validation(format!(
            "{model} supports European EquityOption exercise only; got {:?}",
            inst.exercise_style
        )));
    }
    Ok(())
}

/// Present value using Black–Scholes; result currency is the instrument currency.
pub(crate) fn compute_pv(
    inst: &EquityOption,
    curves: &MarketContext,
    as_of: Date,
) -> Result<Money> {
    if let Some(value) = resolve_lifecycle_value(inst, curves, as_of)? {
        return Ok(value);
    }
    let ccy = option_currency(inst);
    let unit_price = match inst.exercise_style {
        ExerciseStyle::European => {
            let (spot, r, q, sigma, t) = collect_inputs(inst, curves, as_of)?;
            bs_price_unchecked(spot, inst.strike, r, q, sigma, t, inst.option_type)
        }
        ExerciseStyle::American => {
            let steps = inst
                .instrument_pricing_overrides
                .model_config
                .tree_steps
                .unwrap_or(201);
            let tree = BinomialTree::leisen_reimer(steps);
            let (params, dividends) = early_exercise_market_params(inst, curves, as_of)?;
            if dividends.is_empty() {
                tree.price_american(&params)?
            } else {
                tree.price_american_with_discrete_dividends(&params, &dividends)?
            }
        }
        ExerciseStyle::Bermudan => {
            let schedule = inst.exercise_schedule.as_ref().ok_or_else(|| {
                finstack_quant_core::Error::Validation(
                    "Bermudan equity option requires exercise_schedule".to_string(),
                )
            })?;
            let steps = inst
                .instrument_pricing_overrides
                .model_config
                .tree_steps
                .unwrap_or(201);
            let tree = BinomialTree::leisen_reimer(steps);
            let (params, dividends) = early_exercise_market_params(inst, curves, as_of)?;
            let exercise_times: Vec<f64> = schedule
                .iter()
                .filter_map(|date| {
                    let year_fraction = DayCount::Act365F
                        .year_fraction(as_of, *date, Default::default())
                        .ok()?;
                    (year_fraction > 0.0 && year_fraction <= params.time_to_expiry)
                        .then_some(year_fraction)
                })
                .collect();
            if exercise_times.is_empty() {
                return Err(finstack_quant_core::Error::Validation(
                    "Bermudan equity option has no exercise dates remaining after valuation date"
                        .to_string(),
                ));
            }
            if dividends.is_empty() {
                tree.price_bermudan(&params, &exercise_times)?
            } else {
                tree.price_bermudan_with_discrete_dividends(&params, &exercise_times, &dividends)?
            }
        }
    };

    let unit_price = finstack_quant_models::closed_form::checked_closed_form_value(
        unit_price,
        "equity option unit price",
    )?;
    Ok(Money::new(unit_price * inst.notional.amount(), ccy))
}

pub(crate) fn option_currency(inst: &EquityOption) -> Currency {
    inst.notional.currency()
}

/// Resolve a fixed exercise/expiry state before running a live option model.
///
/// Returns `None` while the option is live. From an observed exercise date
/// onward it returns the remaining cash-settlement or physical-delivery value.
/// At or after expiry, a missing observation is an error rather than an
/// implicit auto-exercise assumption.
pub(crate) fn resolve_lifecycle_value(
    inst: &EquityOption,
    market: &MarketContext,
    as_of: Date,
) -> Result<Option<Money>> {
    let Some(exercise) = inst.exercise else {
        if as_of >= inst.expiry {
            return Err(finstack_quant_core::Error::Validation(format!(
                "EquityOption '{}' requires an exercise/expiry observation from expiry {} onward",
                inst.id, inst.expiry
            )));
        }
        return Ok(None);
    };

    if as_of < exercise.date {
        return Ok(None);
    }
    let currency = option_currency(inst);
    if !exercise.exercised || as_of > exercise.settlement_date {
        return Ok(Some(Money::new(0.0, currency)));
    }

    let discount_curve = market.get_discount(inst.discount_curve_id.as_str())?;
    let settlement_df = discount_curve.df_between_dates(as_of, exercise.settlement_date)?;
    let unit_value = match inst.settlement {
        SettlementType::Cash => {
            let intrinsic = match inst.option_type {
                OptionType::Call => (exercise.spot - inst.strike).max(0.0),
                OptionType::Put => (inst.strike - exercise.spot).max(0.0),
            };
            intrinsic * settlement_df
        }
        SettlementType::Physical => {
            let spot = crate::instruments::common_impl::helpers::scalar_price_amount(
                market.get_price(&inst.spot_id)?,
                currency,
            )?;
            let t = year_fraction(DayCount::Act365F, as_of, exercise.settlement_date)?;
            let future_dividends = inst
                .discrete_dividends
                .iter()
                .filter(|(date, _)| *date > as_of && *date <= exercise.settlement_date)
                .collect::<Vec<_>>();
            let prepaid_forward = if future_dividends.is_empty() {
                let q = if let Some(dividend_yield_id) = &inst.div_yield_id {
                    match market.get_price(dividend_yield_id.as_str())? {
                        finstack_quant_core::market_data::scalars::MarketScalar::Unitless(
                            value,
                        ) => *value,
                        finstack_quant_core::market_data::scalars::MarketScalar::Price(_) => {
                            return Err(finstack_quant_core::Error::Validation(format!(
                                "Dividend yield '{}' must be unitless",
                                dividend_yield_id
                            )));
                        }
                    }
                } else {
                    0.0
                };
                spot * (-q * t).exp()
            } else {
                let mut dividend_pv = finstack_quant_core::math::NeumaierAccumulator::new();
                for (date, amount) in future_dividends {
                    dividend_pv.add(*amount * discount_curve.df_between_dates(as_of, *date)?);
                }
                spot - dividend_pv.total()
            };
            match inst.option_type {
                OptionType::Call => prepaid_forward - inst.strike * settlement_df,
                OptionType::Put => inst.strike * settlement_df - prepaid_forward,
            }
        }
    };

    Ok(Some(Money::new(
        unit_value * inst.notional.amount(),
        currency,
    )))
}

/// Collected market inputs for equity option pricing.
///
/// The effective rate `r` reproduces the curve-native date-to-date discount
/// factor when applied over the ACT/365F model time `t_vol`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EquityOptionInputs {
    /// Spot price of the underlying
    pub(crate) spot: f64,
    /// Effective risk-free rate consistent with `t_vol`
    pub(crate) r: f64,
    /// Dividend yield
    pub(crate) q: f64,
    /// Implied volatility
    pub(crate) sigma: f64,
    /// Time to expiry for vol calculations (ACT/365F standard)
    pub(crate) t_vol: f64,
}

/// Collect standard inputs (spot, risk-free, dividend yield, vol, time to expiry).
///
/// **Day Count Convention Handling:**
/// - Discount factors use the discount curve's own day count
/// - Vol surface lookups and model time use ACT/365F (equity market standard)
///
/// This separation ensures consistent pricing when discount curves use different
/// conventions (e.g., OIS curves with ACT/360) than the vol surface.
pub(crate) fn collect_inputs(
    inst: &EquityOption,
    curves: &MarketContext,
    as_of: Date,
) -> Result<(f64, f64, f64, f64, f64)> {
    let inputs = collect_inputs_extended(inst, curves, as_of)?;
    // Return t_vol as the primary time for the simplified interface
    Ok((inputs.spot, inputs.r, inputs.q, inputs.sigma, inputs.t_vol))
}

/// Collect inputs with curve-native discounting and ACT/365F model time.
///
/// The curve's day count is consumed by its date-to-date discount-factor
/// lookup; `t_vol` uses ACT/365F for volatility lookup and option pricing.
///
/// # Discrete Dividend Handling
///
/// When `discrete_dividends` is non-empty and contains future dividends (ex-date > as_of
/// and ex-date <= expiry), the escrowed dividend model is applied:
/// - Spot is adjusted: `S* = S - Σ D_i × e^{-r × t_i}`
/// - Dividend yield `q` is set to 0.0 (dividends are already priced into S*)
///
/// This is the QuantLib-standard approach for discrete dividends in Black-Scholes.
/// Extract future discrete dividends as `(time_to_ex_date, amount)` pairs.
///
/// Only dividends with an ex-date strictly after `as_of` and on or before
/// `inst.expiry` are returned (past and post-expiry dividends do not affect the
/// option). Times use ACT/365F (the equity-vol market standard). The returned
/// slice drives the escrowed-dividend spot adjustment and its rho correction.
pub(crate) fn has_future_discrete_dividends(inst: &EquityOption, as_of: Date) -> bool {
    inst.discrete_dividends
        .iter()
        .any(|(ex_date, _)| *ex_date > as_of && *ex_date <= inst.expiry)
}

pub(crate) fn reject_future_discrete_dividends_for_stochastic_vol(
    inst: &EquityOption,
    as_of: Date,
    model: ModelKey,
    model_name: &str,
) -> std::result::Result<(), PricingError> {
    if has_future_discrete_dividends(inst, as_of) {
        return Err(PricingError::model_failure_with_context(
            format!(
                "{model_name} pricing does not support discrete dividends: the \
                 escrowed-dividend spot adjustment is a Black-Scholes-only construct \
                 and is invalid under stochastic volatility. Use the Black-Scholes \
                 pricer for discrete dividends, or supply a continuous dividend yield \
                 instead."
            ),
            PricingErrorContext::from_instrument(inst).model(model),
        ));
    }
    Ok(())
}

fn future_dividends(
    inst: &EquityOption,
    disc_curve: &finstack_quant_core::market_data::term_structures::DiscountCurve,
    as_of: Date,
) -> Result<Vec<(f64, f64)>> {
    if inst.discrete_dividends.is_empty() {
        return Ok(Vec::new());
    }
    let divs = inst
        .discrete_dividends
        .iter()
        .filter(|(ex_date, _)| *ex_date > as_of && *ex_date <= inst.expiry)
        .map(|(ex_date, amount)| {
            let t_div = year_fraction(DayCount::Act365F, as_of, *ex_date)?;
            let df = disc_curve.df_between_dates(as_of, *ex_date)?;
            Ok((t_div, *amount * df))
        })
        .collect::<finstack_quant_core::Result<Vec<_>>>()?
        .into_iter()
        .filter(|(t_div, _)| *t_div > 0.0)
        .collect();
    Ok(divs)
}

pub(crate) fn collect_inputs_extended(
    inst: &EquityOption,
    curves: &MarketContext,
    as_of: Date,
) -> Result<EquityOptionInputs> {
    // The curve evaluates the economic discount factor on its own day-count
    // clock. Black–Scholes and the vol surface use ACT/365F, so bridge the two
    // clocks with an effective rate satisfying exp(-r * t_vol) = df.
    let disc_curve = curves.get_discount(inst.discount_curve_id.as_str())?;
    let df = disc_curve.df_between_dates(as_of, inst.expiry)?;

    // Vol time uses ACT/365F (equity market standard for vol surfaces)
    // This is consistent with how equity volatility is quoted in the market
    let t_vol = year_fraction(DayCount::Act365F, as_of, inst.expiry)?;
    // Effective BSM rate on the vol clock — see the two-clock note above.
    // A non-positive/non-finite df means a corrupted curve; error rather than
    // derive an infinite rate that would poison the Black–Scholes price.
    let r = crate::instruments::common_impl::helpers::zero_rate_from_df(
        df,
        t_vol,
        "EquityOption discount curve",
    )?;

    let raw_spot = crate::instruments::common_impl::helpers::scalar_price_amount(
        curves.get_price(&inst.spot_id)?,
        inst.notional.currency(),
    )?;

    // Check for discrete dividends — if present, adjust spot and zero out q
    let future_divs = future_dividends(inst, disc_curve.as_ref(), as_of)?;

    let (spot, q) = if !future_divs.is_empty() {
        // Escrowed dividend model: adjust spot, set q=0
        // Dividend amounts are already discounted with their own ex-date DFs.
        let s_adj = adjust_spot_for_discrete_dividends(raw_spot, 0.0, &future_divs)?;
        (s_adj, 0.0)
    } else {
        // Continuous dividend yield from scalar id if provided
        //
        // When a dividend yield ID is explicitly provided, we require the lookup to succeed
        // and return a unitless scalar. Silent fallback to 0.0 would mask market data
        // configuration errors.
        let q = if let Some(div_id) = &inst.div_yield_id {
            let ms = curves.get_price(div_id.as_str()).map_err(|e| {
                finstack_quant_core::Error::Validation(format!(
                    "Failed to fetch dividend yield '{}': {}",
                    div_id, e
                ))
            })?;
            match ms {
                finstack_quant_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                finstack_quant_core::market_data::scalars::MarketScalar::Price(m) => {
                    return Err(finstack_quant_core::Error::Validation(format!(
                        "Dividend yield '{}' should be a unitless scalar, got Price({})",
                        div_id,
                        m.currency()
                    )));
                }
            }
        } else {
            0.0
        };
        (raw_spot, q)
    };

    let sigma = crate::instruments::common_impl::vol_resolution::resolve_sigma_at(
        &inst.instrument_pricing_overrides.market_quotes,
        curves,
        inst.vol_surface_id.as_str(),
        t_vol,
        inst.strike,
    )?;

    Ok(EquityOptionInputs {
        spot,
        r,
        q,
        sigma,
        t_vol,
    })
}

fn future_dividend_amounts(inst: &EquityOption, as_of: Date) -> Result<Vec<(f64, f64)>> {
    inst.discrete_dividends
        .iter()
        .filter(|(date, _)| *date > as_of && *date <= inst.expiry)
        .map(|(date, amount)| Ok((year_fraction(DayCount::Act365F, as_of, *date)?, *amount)))
        .collect()
}

fn early_exercise_market_params(
    inst: &EquityOption,
    curves: &MarketContext,
    as_of: Date,
) -> Result<(OptionMarketParams, Vec<(f64, f64)>)> {
    let inputs = collect_inputs_extended(inst, curves, as_of)?;
    let dividends = future_dividend_amounts(inst, as_of)?;
    let spot = if dividends.is_empty() {
        inputs.spot
    } else {
        crate::instruments::common_impl::helpers::scalar_price_amount(
            curves.get_price(&inst.spot_id)?,
            inst.notional.currency(),
        )?
    };
    Ok((
        OptionMarketParams {
            spot,
            strike: inst.strike,
            rate: inputs.r,
            dividend_yield: if dividends.is_empty() { inputs.q } else { 0.0 },
            volatility: inputs.sigma,
            time_to_expiry: inputs.t_vol,
            option_type: inst.option_type,
        },
        dividends,
    ))
}

/// Adjust spot price for discrete dividends using the present-value method.
///
/// This is the QuantLib-standard approach for handling discrete dividends in
/// the Black-Scholes framework. The adjusted spot replaces the original spot
/// in all BS formulas (pricing, Greeks, implied vol):
///
/// ```text
/// S_adj = S - Σ D_i × e^{-r × t_i}
/// ```
///
/// where:
/// - `S` = current spot price
/// - `D_i` = dividend amount at time `t_i`
/// - `r` = risk-free rate
/// - `t_i` = time to dividend payment in years (only dividends before expiry)
///
/// # Arguments
///
/// * `spot` - Current spot price of the underlying
/// * `rate` - Risk-free rate (annualized, continuous compounding)
/// * `dividends` - Slice of `(time_to_payment, dividend_amount)` pairs
///   where `time_to_payment` is in years from valuation date
///
/// # Errors
///
/// Returns a validation error when spot is non-finite/non-positive or when
/// the present value of future dividends is greater than or equal to spot.
///
/// # References
///
/// - Hull, J. C. (2018). *Options, Futures, and Other Derivatives*, Chapter 15. `docs/REFERENCES.md#hull-options-futures`
/// - QuantLib: `DividendVanillaOption` with `AnalyticEuropeanEngine`
pub(crate) fn adjust_spot_for_discrete_dividends(
    spot: f64,
    rate: f64,
    dividends: &[(f64, f64)],
) -> Result<f64> {
    if !spot.is_finite() || spot <= 0.0 {
        return Err(finstack_quant_core::Error::Validation(format!(
            "discrete-dividend spot must be finite and positive, got {spot}"
        )));
    }
    let pv_dividends: f64 = dividends
        .iter()
        .filter(|(t, _)| *t > 0.0)
        .map(|(t, d)| d * (-rate * t).exp())
        .sum();
    let adjusted = spot - pv_dividends;
    if !adjusted.is_finite() || adjusted <= 0.0 {
        return Err(finstack_quant_core::Error::Validation(format!(
            "escrowed-dividend model invalid: spot={spot}, PV(dividends)={pv_dividends}, \
             adjusted_spot={adjusted}; use an explicit dividend-jump model"
        )));
    }
    Ok(adjusted)
}

/// Sensitivity of the escrowed (dividend-adjusted) spot to the risk-free rate.
///
/// With the escrowed-dividend model `S* = S − Σ D_i · e^{−r·t_i}`, the adjusted
/// spot itself depends on `r`:
///
/// ```text
/// ∂S*/∂r = Σ D_i · t_i · e^{−r·t_i}
/// ```
///
/// This term is required to obtain a correct rho: the Black–Scholes `rho`
/// computed from `S*` holds `S*` fixed and therefore misses the
/// `∂V/∂S* · ∂S*/∂r` contribution. Total rho is
/// `rho_total = rho_BS(S*) + delta(S*) · ∂S*/∂r`.
///
/// Returns `0.0` when no future dividends are present (the adjusted spot is
/// then rate-independent). Invalid non-positive adjusted spots are rejected by
/// [`adjust_spot_for_discrete_dividends`] before this derivative is used.
#[must_use]
pub(crate) fn escrowed_spot_drho(rate: f64, dividends: &[(f64, f64)]) -> f64 {
    dividends
        .iter()
        .filter(|(t, _)| *t > 0.0)
        .map(|(t, d)| d * t * (-rate * t).exp())
        .sum()
}

/// Cash greeks for an equity option (scaled by contract size; vega per 1% vol).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EquityOptionGreeks {
    /// Delta: sensitivity to underlying price (scaled by contract size)
    pub delta: f64,
    /// Gamma: rate of change of delta with respect to underlying price
    pub gamma: f64,
    /// Vega: sensitivity to 1% change in volatility
    pub vega: f64,
    /// Theta: time decay per day
    pub theta: f64,
    /// Rho: sensitivity to 1% change in risk-free rate
    pub rho: f64,
}

/// Compute greeks consistent with the pricing inputs.
///
/// Uses proper day count handling:
/// - Rate lookups use the discount curve's day count
/// - Vol time uses ACT/365F (equity market standard)
pub(crate) fn compute_greeks(
    inst: &EquityOption,
    curves: &MarketContext,
    as_of: Date,
) -> Result<EquityOptionGreeks> {
    if resolve_lifecycle_value(inst, curves, as_of)?.is_some() {
        let delta = match (inst.exercise, inst.settlement) {
            (Some(exercise), SettlementType::Physical)
                if exercise.exercised && as_of <= exercise.settlement_date =>
            {
                let has_discrete_dividend = inst
                    .discrete_dividends
                    .iter()
                    .any(|(date, _)| *date > as_of && *date <= exercise.settlement_date);
                let carry = if has_discrete_dividend {
                    1.0
                } else {
                    let q = if let Some(dividend_yield_id) = &inst.div_yield_id {
                        match curves.get_price(dividend_yield_id.as_str())? {
                            finstack_quant_core::market_data::scalars::MarketScalar::Unitless(
                                value,
                            ) => *value,
                            finstack_quant_core::market_data::scalars::MarketScalar::Price(_) => {
                                return Err(finstack_quant_core::Error::Validation(format!(
                                    "Dividend yield '{}' must be unitless",
                                    dividend_yield_id
                                )));
                            }
                        }
                    } else {
                        0.0
                    };
                    let t = year_fraction(DayCount::Act365F, as_of, exercise.settlement_date)?;
                    (-q * t).exp()
                };
                let direction = match inst.option_type {
                    OptionType::Call => 1.0,
                    OptionType::Put => -1.0,
                };
                direction * carry * inst.notional.amount()
            }
            _ => 0.0,
        };
        return Ok(EquityOptionGreeks {
            delta,
            ..Default::default()
        });
    }
    let inputs = collect_inputs_extended(inst, curves, as_of)?;
    let (spot, r, q, sigma, t) = (inputs.spot, inputs.r, inputs.q, inputs.sigma, inputs.t_vol);

    if t <= 0.0 {
        // At expiry, delta is the step function of the payoff.
        // ATM (spot == strike) uses the convention 0.5 / -0.5,
        // consistent with QuantLib and Bloomberg.
        let strike = inst.strike;
        let delta_unit = match inst.option_type {
            OptionType::Call => {
                if spot > strike {
                    1.0
                } else if (spot - strike).abs() < 1e-12 * strike.abs().max(1.0) {
                    0.5
                } else {
                    0.0
                }
            }
            OptionType::Put => {
                if spot < strike {
                    -1.0
                } else if (spot - strike).abs() < 1e-12 * strike.abs().max(1.0) {
                    -0.5
                } else {
                    0.0
                }
            }
        };
        let scale = inst.notional.amount();
        return Ok(EquityOptionGreeks {
            delta: delta_unit * scale,
            ..Default::default()
        });
    }

    match inst.exercise_style {
        ExerciseStyle::European => {
            let greeks_unit = bs_greeks_unchecked(
                spot,
                inst.strike,
                r,
                q,
                sigma,
                t,
                inst.option_type,
                inst.theta_day_basis.days_per_year(),
            );

            // Escrowed-dividend rho correction.
            //
            // Under the escrowed-dividend model the BS inputs use the adjusted
            // spot `S* = S − Σ D_i·e^{−r·t_i}`, which itself depends on `r`.
            // `bs_greeks` computes rho holding `S*` fixed, so it misses the
            // `∂V/∂S* · ∂S*/∂r` chain-rule term. Total rho is
            // rho_total = rho_BS(S*) + delta(S*) · ∂S*/∂r,
            // expressed per 1% rate move (hence the `ONE_PERCENT` factor:
            // `greeks_unit.rho_r` and `vega` are already per-1%, while
            // `delta` and `∂S*/∂r` are per-unit).
            let rho_unit = {
                let disc_curve = curves.get_discount(inst.discount_curve_id.as_str())?;
                let future_divs = future_dividends(inst, disc_curve.as_ref(), as_of)?;
                if future_divs.is_empty() {
                    greeks_unit.rho_r
                } else {
                    // `future_divs` already contains each dividend's PV.
                    let ds_star_dr = escrowed_spot_drho(0.0, &future_divs);
                    const ONE_PERCENT: f64 = 0.01;
                    greeks_unit.rho_r + greeks_unit.delta * ds_star_dr * ONE_PERCENT
                }
            };

            let scale = inst.notional.amount();
            Ok(EquityOptionGreeks {
                delta: greeks_unit.delta * scale,
                gamma: greeks_unit.gamma * scale,
                vega: greeks_unit.vega * scale,
                theta: greeks_unit.theta * scale,
                rho: rho_unit * scale,
            })
        }
        ExerciseStyle::American => {
            let steps = inst
                .instrument_pricing_overrides
                .model_config
                .tree_steps
                .unwrap_or(201);
            let tree = BinomialTree::leisen_reimer(steps);
            let (params, dividends) = early_exercise_market_params(inst, curves, as_of)?;
            let price_fn = |market_params: &OptionMarketParams| -> Result<f64> {
                if dividends.is_empty() {
                    tree.price_american(market_params)
                } else {
                    tree.price_american_with_discrete_dividends(market_params, &dividends)
                }
            };
            tree_finite_difference_greeks(
                &params,
                inst.notional.amount(),
                inst.theta_day_basis.days_per_year(),
                price_fn,
            )
        }
        ExerciseStyle::Bermudan => {
            let schedule = inst.exercise_schedule.as_ref().ok_or_else(|| {
                finstack_quant_core::Error::Validation(
                    "Bermudan equity option requires exercise_schedule".to_string(),
                )
            })?;
            let steps = inst
                .instrument_pricing_overrides
                .model_config
                .tree_steps
                .unwrap_or(201);
            let tree = BinomialTree::leisen_reimer(steps);
            let (params, dividends) = early_exercise_market_params(inst, curves, as_of)?;
            let exercise_times: Vec<f64> = schedule
                .iter()
                .filter_map(|date| {
                    let year_fraction = DayCount::Act365F
                        .year_fraction(as_of, *date, Default::default())
                        .ok()?;
                    (year_fraction > 0.0 && year_fraction <= params.time_to_expiry)
                        .then_some(year_fraction)
                })
                .collect();
            if exercise_times.is_empty() {
                return Err(finstack_quant_core::Error::Validation(
                    "Bermudan equity option has no exercise dates remaining after valuation date"
                        .to_string(),
                ));
            }
            let price_fn = |market_params: &OptionMarketParams| -> Result<f64> {
                if dividends.is_empty() {
                    tree.price_bermudan(market_params, &exercise_times)
                } else {
                    tree.price_bermudan_with_discrete_dividends(
                        market_params,
                        &exercise_times,
                        &dividends,
                    )
                }
            };
            tree_finite_difference_greeks(
                &params,
                inst.notional.amount(),
                inst.theta_day_basis.days_per_year(),
                price_fn,
            )
        }
    }
}

fn tree_finite_difference_greeks(
    params: &OptionMarketParams,
    scale: f64,
    theta_days_per_year: f64,
    mut price_fn: impl FnMut(&OptionMarketParams) -> Result<f64>,
) -> Result<EquityOptionGreeks> {
    let base_price = price_fn(params)?;

    // Delta: small 1%-of-spot central bump (accuracy-limited; the first
    // difference's noise is O(ε_tree / h), so a small bump is fine).
    let h_s = params.spot * 0.01;
    let mut p_up = params.clone();
    p_up.spot += h_s;
    let price_up = price_fn(&p_up)?;
    let mut p_dn = params.clone();
    p_dn.spot -= h_s;
    let price_dn = price_fn(&p_dn)?;

    let delta_unit = (price_up - price_dn) / (2.0 * h_s);

    // Gamma: a 1%-of-spot bump is too small. The central second difference
    // `(p_up − 2·base + p_dn) / h²` has noise of order `ε_tree / h²`, which a
    // 1% bump leaves noise-dominated — gamma is then noisy and biased,
    // especially for short-dated options where the tree's discrete spot grid
    // makes `P(S)` locally piecewise-flat.
    //
    // Use a wider, better-conditioned gamma bump sized to the option's natural
    // spot scale `σ·√t` (the width of the region where gamma actually lives),
    // with a 2%-of-spot floor so the bump never collapses for short-dated /
    // low-vol options. This trades a small, bounded discretisation bias for a
    // large reduction in second-difference noise. A separate, dedicated
    // re-pricing pair is used so the delta bump stays small for accuracy.
    let gamma_unit = {
        let vol_t = params.volatility * params.time_to_expiry.max(0.0).sqrt();
        let h_g = params.spot * vol_t.max(0.02);
        let mut p_g_up = params.clone();
        p_g_up.spot += h_g;
        let price_g_up = price_fn(&p_g_up)?;
        let mut p_g_dn = params.clone();
        p_g_dn.spot = (p_g_dn.spot - h_g).max(1e-8);
        let price_g_dn = price_fn(&p_g_dn)?;
        let h_dn = params.spot - p_g_dn.spot;
        // Non-uniform three-point second derivative. When the down bump is
        // clamped, a symmetric stencil would leak the first derivative into
        // gamma and can dominate the result.
        2.0 * ((price_g_up - base_price) / h_g - (base_price - price_g_dn) / h_dn) / (h_g + h_dn)
    };

    // Vega (1% vol bump)
    let h_v = 0.01;
    let mut p_v_up = params.clone();
    p_v_up.volatility += h_v;
    let price_v_up = price_fn(&p_v_up)?;
    let mut p_v_dn = params.clone();
    p_v_dn.volatility = (p_v_dn.volatility - h_v).max(1e-8);
    let price_v_dn = price_fn(&p_v_dn)?;
    let actual_vol_width = p_v_up.volatility - p_v_dn.volatility;
    let vega_unit = (price_v_up - price_v_dn) / actual_vol_width * h_v;

    // Rho (1% rate bump)
    let h_r = 0.01;
    let mut p_r_up = params.clone();
    p_r_up.rate += h_r;
    let price_r_up = price_fn(&p_r_up)?;
    let mut p_r_dn = params.clone();
    p_r_dn.rate -= h_r;
    let price_r_dn = price_fn(&p_r_dn)?;
    let rho_unit = (price_r_up - price_r_dn) / 2.0;

    // Theta: one day on the instrument's configured reporting basis.
    let dt = 1.0 / theta_days_per_year;
    let theta_unit = if params.time_to_expiry > dt {
        let mut p_t = params.clone();
        p_t.time_to_expiry -= dt;
        let price_t = price_fn(&p_t)?;
        price_t - base_price
    } else {
        0.0
    };

    Ok(EquityOptionGreeks {
        delta: delta_unit * scale,
        gamma: gamma_unit * scale,
        vega: vega_unit * scale,
        theta: theta_unit * scale,
        rho: rho_unit * scale,
    })
}

/// Registry pricer for Equity Option using Black-Scholes model
pub(crate) struct SimpleEquityOptionBlackPricer {
    model: crate::pricer::ModelKey,
}

impl SimpleEquityOptionBlackPricer {
    /// Create new Black-Scholes pricer with default model.
    ///
    /// Uses `ModelKey::Black76` which is the library-wide convention for
    /// lognormal option pricing. BSM and Black-76 are mathematically
    /// equivalent (BSM is Black-76 applied to the forward
    /// `F = S × exp((r-q)T)`), so the same model key covers both.
    pub(crate) fn new() -> Self {
        Self {
            model: crate::pricer::ModelKey::Black76,
        }
    }

    /// Create pricer with specified model key
    pub(crate) fn with_model(model: crate::pricer::ModelKey) -> Self {
        Self { model }
    }
}

impl Default for SimpleEquityOptionBlackPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::pricer::Pricer for SimpleEquityOptionBlackPricer {
    fn key(&self) -> crate::pricer::PricerKey {
        crate::pricer::PricerKey::new(crate::pricer::InstrumentType::EquityOption, self.model)
    }

    #[tracing::instrument(
        name = "equity_option.black.price_dyn",
        level = "debug",
        skip(self, instrument, market),
        fields(
            pricer = ?self.key(),
            inst_id = %instrument.id(),
            as_of = %as_of,
        ),
        err,
    )]
    fn price_dyn(
        &self,
        instrument: &dyn crate::instruments::common_impl::traits::Instrument,
        market: &finstack_quant_core::market_data::context::MarketContext,
        as_of: finstack_quant_core::dates::Date,
    ) -> std::result::Result<crate::results::ValuationResult, crate::pricer::PricingError> {
        use crate::instruments::common_impl::traits::Instrument;

        // Type-safe downcasting
        let equity_option = instrument
            .as_any()
            .downcast_ref::<crate::instruments::equity::equity_option::EquityOption>()
            .ok_or_else(|| {
                crate::pricer::PricingError::type_mismatch(
                    crate::pricer::InstrumentType::EquityOption,
                    instrument.key(),
                )
            })?;

        // Use the provided as_of date for consistency
        let pv = compute_pv(equity_option, market, as_of).map_err(|e| {
            crate::pricer::PricingError::model_failure_with_context(
                e.to_string(),
                crate::pricer::PricingErrorContext::from_instrument(equity_option)
                    .model(self.model),
            )
        })?;

        Ok(crate::results::ValuationResult::stamped(
            equity_option.id(),
            as_of,
            pv,
        ))
    }
}

use crate::instruments::common_impl::traits::Instrument;
use finstack_quant_models::closed_form::heston::{
    heston_call_price_fourier, heston_put_price_fourier,
};

/// Equity option Heston semi-analytical pricer (Fourier inversion).
pub(crate) struct EquityOptionHestonFourierPricer;

impl EquityOptionHestonFourierPricer {
    /// Create a new Heston Fourier transform pricer
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Default for EquityOptionHestonFourierPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::pricer::Pricer for EquityOptionHestonFourierPricer {
    fn key(&self) -> crate::pricer::PricerKey {
        crate::pricer::PricerKey::new(
            crate::pricer::InstrumentType::EquityOption,
            crate::pricer::ModelKey::HestonFourier,
        )
    }

    #[tracing::instrument(
        name = "equity_option.heston_fourier.price_dyn",
        level = "debug",
        skip(self, instrument, market),
        fields(inst_id = %instrument.id(), as_of = %as_of),
        err,
    )]
    fn price_dyn(
        &self,
        instrument: &dyn crate::instruments::common_impl::traits::Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> std::result::Result<crate::results::ValuationResult, PricingError> {
        let equity_option = instrument
            .as_any()
            .downcast_ref::<EquityOption>()
            .ok_or_else(|| {
                crate::pricer::PricingError::type_mismatch(
                    crate::pricer::InstrumentType::EquityOption,
                    instrument.key(),
                )
            })?;

        if let Some(pv) =
            resolve_lifecycle_value(equity_option, market, as_of).map_err(|error| {
                crate::pricer::PricingError::model_failure_with_context(
                    error.to_string(),
                    crate::pricer::PricingErrorContext::from_instrument(equity_option)
                        .model(crate::pricer::ModelKey::HestonFourier),
                )
            })?
        {
            return Ok(crate::results::ValuationResult::stamped(
                equity_option.id(),
                as_of,
                pv,
            ));
        }
        require_european(equity_option, "Heston Fourier").map_err(|e| {
            crate::pricer::PricingError::model_failure_with_context(
                e.to_string(),
                crate::pricer::PricingErrorContext::from_instrument(equity_option)
                    .model(crate::pricer::ModelKey::HestonFourier),
            )
        })?;

        reject_future_discrete_dividends_for_stochastic_vol(
            equity_option,
            as_of,
            crate::pricer::ModelKey::HestonFourier,
            "Heston Fourier",
        )?;

        let inputs = collect_inputs_extended(equity_option, market, as_of).map_err(|e| {
            crate::pricer::PricingError::model_failure_with_context(
                e.to_string(),
                crate::pricer::PricingErrorContext::from_instrument(equity_option)
                    .model(crate::pricer::ModelKey::HestonFourier),
            )
        })?;
        let (spot, r, q, _sigma, t) = (inputs.spot, inputs.r, inputs.q, inputs.sigma, inputs.t_vol);

        if t <= 0.0 {
            let intrinsic = match equity_option.option_type {
                OptionType::Call => (spot - equity_option.strike).max(0.0),
                OptionType::Put => (equity_option.strike - spot).max(0.0),
            };
            return Ok(crate::results::ValuationResult::stamped(
                equity_option.id(),
                as_of,
                Money::new(
                    intrinsic * equity_option.notional.amount(),
                    option_currency(equity_option),
                ),
            ));
        }

        // Source production Heston parameters from explicit market scalars.
        // Validation is still enforced inside `HestonParams::new`.
        let err_ctx = crate::pricer::PricingErrorContext::from_instrument(equity_option)
            .model(crate::pricer::ModelKey::HestonFourier);
        let params = crate::instruments::equity::equity_option::heston_market::heston_params_from_market_strict(market, r, q)
            .map_err(|e| crate::pricer::PricingError::from_core(e, err_ctx.clone()))?;

        let price = match equity_option.option_type {
            OptionType::Call => {
                heston_call_price_fourier(spot, equity_option.strike, t, &params, None)
            }
            OptionType::Put => {
                heston_put_price_fourier(spot, equity_option.strike, t, &params, None)
            }
        }
        .map_err(|error| crate::pricer::PricingError::from_core(error, err_ctx))?;

        let pv = Money::new(
            price * equity_option.notional.amount(),
            option_currency(equity_option),
        );
        Ok(crate::results::ValuationResult::stamped(
            equity_option.id(),
            as_of,
            pv,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::equity::equity_option::types::{
        EquityOption, EquityOptionExercise, ThetaDayBasis,
    };
    use crate::instruments::{Attributes, SettlementType};
    use crate::pricer::Pricer;
    use finstack_quant_core::market_data::context::MarketContext;
    use finstack_quant_core::market_data::scalars::MarketScalar;
    use finstack_quant_core::market_data::surfaces::VolSurface;
    use finstack_quant_core::market_data::term_structures::DiscountCurve;
    use finstack_quant_core::types::{CurveId, InstrumentId, PriceId};
    use time::Month;

    fn date(year: i32, month: u8, day: u8) -> Date {
        Date::from_calendar_date(year, Month::try_from(month).expect("valid month"), day)
            .expect("valid date")
    }

    fn market(as_of: Date, spot: f64, vol: f64, rate: f64, div_yield: f64) -> MarketContext {
        let curve = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .day_count(DayCount::Act365F)
            .knots([(0.0, 1.0), (10.0, (-rate * 10.0).exp())])
            .build()
            .expect("curve");
        let surface = VolSurface::builder("SPX-VOL")
            .expiries(&[0.25, 0.5, 1.0, 2.0])
            .strikes(&[80.0, 100.0, 120.0, 150.0])
            .row(&[vol, vol, vol, vol])
            .row(&[vol, vol, vol, vol])
            .row(&[vol, vol, vol, vol])
            .row(&[vol, vol, vol, vol])
            .build()
            .expect("surface");

        MarketContext::new()
            .insert(curve)
            .insert_surface(surface)
            .insert_price("SPX-SPOT", MarketScalar::Unitless(spot))
            .insert_price("SPX-DIV", MarketScalar::Unitless(div_yield))
    }

    fn option(
        expiry: Date,
        option_type: OptionType,
        exercise_style: ExerciseStyle,
    ) -> EquityOption {
        EquityOption::builder()
            .id(InstrumentId::new("EQ-OPT-TEST"))
            .underlying_ticker("SPX".to_string())
            .strike(100.0)
            .option_type(option_type)
            .exercise_style(exercise_style)
            .expiry(expiry)
            .notional(Money::new(100.0, Currency::USD))
            .day_count(DayCount::Act365F)
            .settlement(SettlementType::Cash)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id("SPX-SPOT".into())
            .vol_surface_id(CurveId::new("SPX-VOL"))
            .div_yield_id_opt(Some(PriceId::new("SPX-DIV")))
            .attributes(Attributes::new())
            .build()
            .expect("equity option")
    }

    #[test]
    fn expiry_requires_observed_lifecycle_state() {
        let expiry = date(2025, 6, 20);
        let option = option(expiry, OptionType::Call, ExerciseStyle::European);
        let error = compute_pv(&option, &market(expiry, 120.0, 0.2, 0.05, 0.0), expiry)
            .expect_err("expiry without an observation must fail");
        assert!(error.to_string().contains("exercise/expiry observation"));
    }

    #[test]
    fn cash_exercise_remains_until_payment() {
        let expiry = date(2025, 6, 20);
        let settlement = date(2025, 6, 23);
        let mut option = option(expiry, OptionType::Call, ExerciseStyle::European);
        option.exercise = Some(EquityOptionExercise::new(expiry, 120.0, settlement, true));
        let market = market(expiry, 121.0, 0.2, 0.05, 0.0);

        let pv = compute_pv(&option, &market, expiry)
            .expect("fixed cash payoff")
            .amount();
        let df = market
            .get_discount("USD-OIS")
            .expect("discount curve")
            .df_between_dates(expiry, settlement)
            .expect("settlement discount factor");
        assert!((pv - 20.0 * 100.0 * df).abs() < 1e-9);

        let after_settlement = date(2025, 6, 24);
        assert_eq!(
            compute_pv(&option, &market, after_settlement)
                .expect("settled option")
                .amount(),
            0.0
        );
    }

    #[test]
    fn physical_exercise_marks_delivery_obligation() {
        let expiry = date(2025, 6, 20);
        let settlement = date(2025, 6, 23);
        let mut option = option(expiry, OptionType::Call, ExerciseStyle::European);
        option.settlement = SettlementType::Physical;
        option.exercise = Some(EquityOptionExercise::new(expiry, 120.0, settlement, true));
        let market = market(expiry, 121.0, 0.2, 0.05, 0.0);
        let df = market
            .get_discount("USD-OIS")
            .expect("discount curve")
            .df_between_dates(expiry, settlement)
            .expect("settlement discount factor");

        let pv = compute_pv(&option, &market, expiry)
            .expect("physical delivery mark")
            .amount();
        assert!((pv - (121.0 - 100.0 * df) * 100.0).abs() < 1e-9);
        let greeks = compute_greeks(&option, &market, expiry).expect("delivery risk");
        assert!((greeks.delta - 100.0).abs() < 1e-12);
    }

    #[test]
    fn american_discrete_dividend_preserves_pre_exercise_spot() {
        let as_of = date(2025, 1, 2);
        let expiry = date(2026, 1, 2);
        let mut american = option(expiry, OptionType::Call, ExerciseStyle::American);
        american.strike = 50.0;
        american.discrete_dividends = vec![(date(2025, 2, 3), 20.0)];
        american.validate().expect("valid discrete-dividend option");
        let mut european = american.clone();
        european.exercise_style = ExerciseStyle::European;
        let market = market(as_of, 100.0, 0.2, 0.05, 0.0);

        let american_pv = compute_pv(&american, &market, as_of)
            .expect("American discrete-dividend price")
            .amount();
        let european_pv = compute_pv(&european, &market, as_of)
            .expect("European escrowed-dividend price")
            .amount();

        assert!(american_pv >= 50.0 * american.notional.amount());
        assert!(
            american_pv > european_pv,
            "large near-term dividend should create an early-exercise premium"
        );
    }

    #[test]
    fn theta_day_basis_is_explicit_and_configurable() {
        let as_of = date(2025, 1, 2);
        let expiry = date(2026, 1, 2);
        let calendar = option(expiry, OptionType::Call, ExerciseStyle::European);
        let mut trading = calendar.clone();
        trading.theta_day_basis = ThetaDayBasis::Trading252;
        let market = market(as_of, 100.0, 0.2, 0.05, 0.01);

        let calendar_theta = compute_greeks(&calendar, &market, as_of)
            .expect("calendar theta")
            .theta;
        let trading_theta = compute_greeks(&trading, &market, as_of)
            .expect("trading theta")
            .theta;
        assert!((trading_theta / calendar_theta - 365.0 / 252.0).abs() < 1e-12);
    }

    #[test]
    fn test_adjust_spot_for_discrete_dividends_single() {
        // Stock at $100, dividend of $2 in 0.25 years, r = 5%
        let s_adj = adjust_spot_for_discrete_dividends(100.0, 0.05, &[(0.25, 2.0)])
            .expect("valid adjusted spot");
        // PV(div) = 2 × e^{-0.05×0.25} ≈ 1.9751
        assert!((s_adj - 98.0248).abs() < 0.01);
    }

    #[test]
    fn test_adjust_spot_for_discrete_dividends_multiple() {
        let s_adj = adjust_spot_for_discrete_dividends(100.0, 0.05, &[(0.25, 1.5), (0.5, 1.5)])
            .expect("valid adjusted spot");
        let expected = 100.0 - 1.5 * (-0.05 * 0.25_f64).exp() - 1.5 * (-0.05 * 0.5_f64).exp();
        assert!((s_adj - expected).abs() < 1e-10);
    }

    #[test]
    fn test_adjust_spot_for_discrete_dividends_rejects_nonpositive_result() {
        let error = adjust_spot_for_discrete_dividends(1.0, 0.01, &[(0.1, 50.0)])
            .expect_err("dividend PV above spot must fail");
        assert!(error
            .to_string()
            .contains("escrowed-dividend model invalid"));
    }

    #[test]
    fn test_adjust_spot_for_discrete_dividends_empty() {
        let s_adj = adjust_spot_for_discrete_dividends(100.0, 0.05, &[]).expect("unchanged spot");
        assert!((s_adj - 100.0).abs() < 1e-12);
    }

    #[test]
    fn test_adjust_spot_for_discrete_dividends_skips_past() {
        // Dividend at t=0 or negative should be skipped
        let s_adj = adjust_spot_for_discrete_dividends(100.0, 0.05, &[(0.0, 5.0), (-0.1, 3.0)])
            .expect("past dividends ignored");
        assert!((s_adj - 100.0).abs() < 1e-12);
    }

    /// Escrowed-dividend rho must include the `∂S*/∂r` chain-rule term.
    ///
    /// With discrete dividends the BS inputs use `S* = S − Σ D·e^{−r·t}`, which
    /// depends on `r`. The analytic rho from `compute_greeks` must therefore
    /// match a finite-difference rho computed by bumping the discount-curve
    /// rate (which re-derives `S*` at the bumped rate). Before the fix, rho
    /// held `S*` fixed and disagreed with the FD rho by `delta·∂S*/∂r`.
    #[test]
    fn escrowed_dividend_rho_includes_spot_rate_sensitivity() {
        let as_of = date(2025, 1, 1);
        let expiry = date(2026, 1, 1); // ~1y
        let mut opt = option(expiry, OptionType::Call, ExerciseStyle::European);
        // A sizeable dividend mid-life makes ∂S*/∂r materially non-zero.
        opt.discrete_dividends = vec![(date(2025, 7, 1), 8.0)];

        let base_rate = 0.04;
        let analytic = compute_greeks(&opt, &market(as_of, 100.0, 0.20, base_rate, 0.0), as_of)
            .expect("analytic greeks")
            .rho;

        // Central finite-difference rho of the full PV over the curve rate.
        // compute_pv re-derives r (and hence S*) from the curve, so this FD
        // captures the ∂S*/∂r contribution that the analytic rho must match.
        let h = 1e-4; // 1bp in rate space
        let pv_up = compute_pv(&opt, &market(as_of, 100.0, 0.20, base_rate + h, 0.0), as_of)
            .expect("pv up")
            .amount();
        let pv_dn = compute_pv(&opt, &market(as_of, 100.0, 0.20, base_rate - h, 0.0), as_of)
            .expect("pv dn")
            .amount();
        // analytic rho is per 1% (100bp); FD slope per unit-rate * 0.01.
        let fd_rho = (pv_up - pv_dn) / (2.0 * h) * 0.01;

        let denom = analytic.abs().max(fd_rho.abs()).max(1e-9);
        assert!(
            (analytic - fd_rho).abs() / denom < 5e-3,
            "escrowed-dividend rho must match FD rho of the full PV (which \
             re-derives S* at the bumped rate): analytic={analytic} fd={fd_rho}"
        );

        // And it must NOT equal the naive rho that holds S* fixed.
        let inputs =
            collect_inputs_extended(&opt, &market(as_of, 100.0, 0.20, base_rate, 0.0), as_of)
                .expect("inputs");
        let naive = bs_greeks_unchecked(
            inputs.spot,
            opt.strike,
            inputs.r,
            inputs.q,
            inputs.sigma,
            inputs.t_vol,
            opt.option_type,
            opt.theta_day_basis.days_per_year(),
        )
        .rho_r
            * opt.notional.amount();
        assert!(
            (analytic - naive).abs() / denom > 1e-3,
            "the ∂S*/∂r correction must move rho away from the S*-fixed value: \
             analytic={analytic} naive={naive}"
        );
    }

    #[test]
    fn heston_fourier_rejects_future_discrete_dividend() {
        let as_of = date(2025, 1, 1);
        let expiry = date(2026, 1, 1);
        let mut opt = option(expiry, OptionType::Call, ExerciseStyle::European);
        opt.discrete_dividends = vec![(date(2025, 7, 1), 2.0)];

        let err = EquityOptionHestonFourierPricer::new()
            .price_dyn(&opt, &market(as_of, 100.0, 0.20, 0.03, 0.0), as_of)
            .expect_err("Heston Fourier must reject discrete dividends");
        let msg = err.to_string();
        assert!(
            msg.contains("discrete dividends"),
            "unexpected error message: {msg}"
        );
    }

    #[test]
    fn cash_settlement_has_zero_delta_at_expiry() {
        let as_of = date(2025, 1, 1);
        let mut call = option(as_of, OptionType::Call, ExerciseStyle::European);
        let mut put = option(as_of, OptionType::Put, ExerciseStyle::European);
        call.exercise = Some(EquityOptionExercise::new(as_of, 100.0, as_of, true));
        put.exercise = Some(EquityOptionExercise::new(as_of, 100.0, as_of, true));
        let curves = market(as_of, 100.0, 0.20, 0.03, 0.01);

        let call_greeks = compute_greeks(&call, &curves, as_of).expect("call greeks");
        let put_greeks = compute_greeks(&put, &curves, as_of).expect("put greeks");

        assert_eq!(call_greeks, EquityOptionGreeks::default());
        assert_eq!(put_greeks, EquityOptionGreeks::default());
    }

    /// Short-dated tree FD gamma must be well-conditioned.
    ///
    /// An American call on a non-dividend-paying underlying is never optimally
    /// exercised early, so its price (and gamma) equals the European value.
    /// For a short-dated near-ATM option the analytic BS gamma is therefore a
    /// reliable oracle. With the old 1%-of-spot gamma bump the tree second
    /// difference is noise-dominated and gamma drifts well off the analytic
    /// value; the wider `σ√t`-scaled bump keeps it close.
    #[test]
    fn short_dated_tree_gamma_is_well_conditioned() {
        let as_of = date(2025, 1, 1);
        // ~3-week expiry: short enough that a 1%-of-spot bump is noise-prone.
        let expiry = date(2025, 1, 22);
        let mut american = option(expiry, OptionType::Call, ExerciseStyle::American);
        american
            .instrument_pricing_overrides
            .model_config
            .tree_steps = Some(201);
        // Zero dividend yield => American call == European call.
        let curves = market(as_of, 100.0, 0.20, 0.03, 0.0);

        let tree_greeks = compute_greeks(&american, &curves, as_of).expect("tree greeks");

        // Analytic European gamma with the same inputs.
        let inputs = collect_inputs_extended(&american, &curves, as_of).expect("inputs");
        let analytic = bs_greeks_unchecked(
            inputs.spot,
            american.strike,
            inputs.r,
            inputs.q,
            inputs.sigma,
            inputs.t_vol,
            american.option_type,
            american.theta_day_basis.days_per_year(),
        )
        .gamma
            * american.notional.amount();

        assert!(
            analytic > 0.0 && tree_greeks.gamma > 0.0,
            "gamma must be positive: analytic={analytic} tree={}",
            tree_greeks.gamma
        );
        let rel_err = (tree_greeks.gamma - analytic).abs() / analytic;
        assert!(
            rel_err < 0.05,
            "short-dated tree gamma must track analytic gamma within 5%: \
             analytic={analytic} tree={} rel_err={rel_err}",
            tree_greeks.gamma
        );
    }

    #[test]
    fn test_american_call_tree_path_prices_above_european() {
        let as_of = date(2025, 1, 1);
        let expiry = date(2025, 7, 1);
        let mut european = option(expiry, OptionType::Call, ExerciseStyle::European);
        let mut american = option(expiry, OptionType::Call, ExerciseStyle::American);
        european
            .instrument_pricing_overrides
            .model_config
            .tree_steps = Some(51);
        american
            .instrument_pricing_overrides
            .model_config
            .tree_steps = Some(51);
        let curves = market(as_of, 105.0, 0.22, 0.03, 0.01);

        let european_pv = compute_pv(&european, &curves, as_of).expect("european pv");
        let american_pv = compute_pv(&american, &curves, as_of).expect("american pv");

        assert!(american_pv.amount().is_finite());
        assert!(american_pv.amount() >= european_pv.amount());
    }

    #[test]
    fn test_bermudan_schedule_filters_invalid_dates_before_tree_pricing() {
        let as_of = date(2025, 1, 1);
        let expiry = date(2025, 7, 1);
        let mut filtered = option(expiry, OptionType::Put, ExerciseStyle::Bermudan);
        let mut noisy = option(expiry, OptionType::Put, ExerciseStyle::Bermudan);
        filtered
            .instrument_pricing_overrides
            .model_config
            .tree_steps = Some(51);
        noisy.instrument_pricing_overrides.model_config.tree_steps = Some(51);
        filtered.exercise_schedule = Some(vec![date(2025, 3, 1), date(2025, 5, 1)]);
        noisy.exercise_schedule = Some(vec![
            as_of,
            date(2024, 12, 15),
            date(2025, 3, 1),
            date(2025, 5, 1),
            date(2025, 8, 1),
        ]);
        let curves = market(as_of, 95.0, 0.25, 0.03, 0.0);

        let filtered_pv = compute_pv(&filtered, &curves, as_of).expect("filtered bermudan pv");
        let noisy_pv = compute_pv(&noisy, &curves, as_of).expect("noisy bermudan pv");

        assert!((filtered_pv.amount() - noisy_pv.amount()).abs() < 1e-10);
    }

    #[test]
    fn canonical_theta_uses_calendar_365_basis() {
        let as_of = date(2025, 1, 1);
        let expiry = date(2026, 1, 1);
        let curves = market(as_of, 100.0, 0.20, 0.03, 0.0);
        let option = option(expiry, OptionType::Call, ExerciseStyle::European);
        let theta = crate::instruments::common_impl::traits::OptionGreeksProvider::option_theta(
            &option, &curves, as_of,
        )
        .expect("theta")
        .expect("supported");
        let inputs = collect_inputs_extended(&option, &curves, as_of).expect("inputs");
        let expected = bs_greeks_unchecked(
            inputs.spot,
            option.strike,
            inputs.r,
            inputs.q,
            inputs.sigma,
            inputs.t_vol,
            option.option_type,
            365.0,
        )
        .theta
            * option.notional.amount();

        assert!((theta - expected).abs() < 1e-12);
    }

    #[test]
    fn option_rejects_spot_price_in_wrong_currency() {
        let as_of = date(2025, 1, 1);
        let expiry = date(2026, 1, 1);
        let curves = market(as_of, 100.0, 0.20, 0.03, 0.0).insert_price(
            "SPX-SPOT",
            MarketScalar::Price(Money::new(100.0, Currency::EUR)),
        );
        let option = option(expiry, OptionType::Call, ExerciseStyle::European);

        assert!(matches!(
            compute_pv(&option, &curves, as_of),
            Err(finstack_quant_core::Error::CurrencyMismatch {
                expected: Currency::USD,
                actual: Currency::EUR,
            })
        ));
    }

    #[test]
    fn post_expiry_value_and_greeks_are_zero_without_market_data() {
        let expiry = date(2025, 1, 1);
        let as_of = date(2025, 1, 2);
        let mut option = option(expiry, OptionType::Call, ExerciseStyle::European);
        option.exercise = Some(EquityOptionExercise::new(expiry, 100.0, expiry, false));
        let empty = MarketContext::new();
        let pv = compute_pv(&option, &empty, as_of).expect("post-expiry PV");
        let greeks = compute_greeks(&option, &empty, as_of).expect("post-expiry greeks");
        assert_eq!(pv.amount(), 0.0);
        assert_eq!(greeks.delta, 0.0);
        assert_eq!(greeks.gamma, 0.0);
        assert_eq!(greeks.vega, 0.0);
    }
}
