//! Delta for [`CDSOption`], branched by strike kind.
//!
//! - **Spread strikes**: Bloomberg's CDSO terminal reports Δ as the
//!   Black-76 N(d₁) sensitivity of the option premium to the displayed ATM
//!   forward spread (DOCS 2055833 §2.5 in conjunction with the closed-form
//!   lognormal-spread identity).
//! - **Clean-price strikes**: the Black spread `d₁` is not defined for a
//!   price strike, so Δ is the curve-reprice hedge ratio
//!   `option_CS01 / underlying_spread_DV01` under a symmetric par-spread
//!   bump with hazard-curve rebootstrap, sticky native strike, and sticky
//!   surface volatility.
//!
//! This module is the single source of truth for both values;
//! [`CDSOption::delta`] is a thin pass-through to [`delta`].

use crate::calibration::bumps::{bump_hazard_spreads, BumpRequest};
use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::credit_derivatives::cds::pricer::CDSPricer;
use crate::instruments::credit_derivatives::cds_option::bloomberg_quadrature::{
    self, ForwardCdsContext,
};
use crate::instruments::credit_derivatives::cds_option::pricer::{
    resolve_sigma, synthetic_underlying_cds,
};
use crate::instruments::credit_derivatives::cds_option::{CDSOption, CDSOptionStrikeKind};
use crate::instruments::OptionType;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::Result;

/// Symmetric par-spread bump (in bp) for the price-strike hedge-ratio
/// delta, matching the Bloomberg CDSW ±1 bp spread DV01 convention.
const PRICE_DELTA_SPREAD_BUMP_BP: f64 = 1.0;

/// Bloomberg-screen Delta calculator for credit options on CDS spreads.
pub(crate) struct DeltaCalculator;

impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &CDSOption = context.instrument_as()?;
        delta(option, &context.curves, context.as_of)
    }
}

/// CDS option Δ, branched by strike kind (see the module docs).
///
/// Returned as a unit-less ratio (multiply by 100 for the displayed
/// percentage). Payers Δ ≥ 0, receivers Δ ≤ 0 under both conventions.
pub(crate) fn delta(
    option: &CDSOption,
    curves: &MarketContext,
    as_of: finstack_quant_core::dates::Date,
) -> Result<f64> {
    option.validate_supported_configuration()?;
    let t = option.time_to_expiry(as_of)?;
    if t <= 0.0 {
        return Ok(0.0);
    }
    match option.strike.kind() {
        CDSOptionStrikeKind::Spread => spread_black_delta(option, curves, as_of, t),
        CDSOptionStrikeKind::CleanPricePct => price_strike_delta(option, curves, as_of),
    }
}

/// Bloomberg CDSO Δ — closed-form Black-76 N(d₁) on the displayed ATM
/// forward spread (spread strikes only).
fn spread_black_delta(
    option: &CDSOption,
    curves: &MarketContext,
    as_of: finstack_quant_core::dates::Date,
    t: f64,
) -> Result<f64> {
    let sigma = resolve_sigma(option, curves, as_of)?;
    let cds = synthetic_underlying_cds(option, as_of)?;
    let disc = curves.get_discount(&option.discount_curve_id)?;
    let surv = curves.get_hazard(&option.credit_curve_id)?;
    let ctx = ForwardCdsContext::build(option, disc.as_ref(), surv.as_ref(), &cds, as_of, sigma)?;
    let clean_forward = delta_display_forward(option, curves, &ctx, as_of)?;
    Ok(black_delta_ratio(
        option.option_type,
        clean_forward,
        ctx.spread_strike()?,
        sigma,
        t,
    ))
}

/// Price-strike Δ — curve-reprice hedge ratio
/// `option_CS01 / underlying_spread_DV01`.
///
/// Both legs of the ratio use the same symmetric ±1 bp par-spread bump
/// with full hazard-curve rebootstrap. The option repricing is sticky in
/// the native clean-price strike and sticky in the surface volatility (σ
/// is resolved once on the input market and reused at both bumped
/// curves). The denominator reprices the current-factor-scaled synthetic
/// underlying CDS on the same bumped curves and conventions.
pub(super) fn price_strike_delta(
    option: &CDSOption,
    curves: &MarketContext,
    as_of: finstack_quant_core::dates::Date,
) -> Result<f64> {
    let sigma = resolve_sigma(option, curves, as_of)?;
    let cds = synthetic_underlying_cds(option, as_of)?;
    let hazard = curves.get_hazard(&option.credit_curve_id)?;

    let bumped_market = |bump_bp: f64| -> Result<MarketContext> {
        let bumped = bump_hazard_spreads(
            hazard.as_ref(),
            curves,
            &BumpRequest::Parallel(bump_bp),
            Some(&option.discount_curve_id),
            None,
            None,
        )?;
        Ok(curves.clone().insert(bumped))
    };
    let up_market = bumped_market(PRICE_DELTA_SPREAD_BUMP_BP)?;
    let down_market = bumped_market(-PRICE_DELTA_SPREAD_BUMP_BP)?;

    let option_up = bloomberg_quadrature::npv(option, &cds, &up_market, sigma, as_of)?.amount();
    let option_down = bloomberg_quadrature::npv(option, &cds, &down_market, sigma, as_of)?.amount();
    let underlying_up = cds.value(&up_market, as_of)?.amount();
    let underlying_down = cds.value(&down_market, as_of)?.amount();

    let option_cs01 = (option_up - option_down) / 2.0;
    let underlying_spread_dv01 = (underlying_up - underlying_down) / 2.0;
    if underlying_spread_dv01.abs() < 1e-12 {
        return Err(finstack_quant_core::Error::Validation(format!(
            "degenerate underlying spread DV01 ({underlying_spread_dv01:.3e}) for \
             price-strike delta of CDS option '{}'",
            option.id
        )));
    }
    Ok(option_cs01 / underlying_spread_dv01)
}

pub(super) fn delta_display_forward(
    option: &CDSOption,
    curves: &MarketContext,
    ctx: &ForwardCdsContext,
    as_of: finstack_quant_core::dates::Date,
) -> Result<f64> {
    let cds = synthetic_underlying_cds(option, as_of)?;
    let disc = curves.get_discount(&option.discount_curve_id)?;
    let surv = curves.get_hazard(&option.credit_curve_id)?;
    // Bloomberg CDSO Default_Leg(0, T_mat): the synthetic CDS carries the
    // `BloombergCdswClean` convention, so protection integrates from the
    // valuation date itself (step-in 0).
    let spot_protection_pv = CDSPricer::new()
        .pv_protection_leg(&cds, disc.as_ref(), surv.as_ref(), as_of)?
        .amount();
    let denom = ctx.df_to_expiry
        * ctx.survival_to_expiry
        * ctx.bootstrapped_l_at_expiry
        * cds.notional.amount();
    if denom <= 1e-12 {
        return Err(finstack_quant_core::Error::Validation(format!(
            "degenerate CDS option delta display-forward denominator: id={}, denom={denom:.6e}",
            option.id,
        )));
    }
    Ok(spot_protection_pv / denom)
}

pub(super) fn black_delta_ratio(
    option_type: OptionType,
    forward: f64,
    strike: f64,
    sigma: f64,
    t: f64,
) -> f64 {
    if forward <= 0.0 || strike <= 0.0 || sigma <= 0.0 || t <= 0.0 {
        return 0.0;
    }
    let sigma_sqrt_t = sigma * t.sqrt();
    if sigma_sqrt_t <= 1e-12 {
        return 0.0;
    }
    let d1 = ((forward / strike).ln() + 0.5 * sigma_sqrt_t * sigma_sqrt_t) / sigma_sqrt_t;

    match option_type {
        OptionType::Call => finstack_quant_core::math::norm_cdf(d1),
        OptionType::Put => -finstack_quant_core::math::norm_cdf(-d1),
    }
}
