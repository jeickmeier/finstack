//! Gamma for [`CDSOption`], branched by strike kind.
//!
//! - **Spread strikes**: Bloomberg's CDSO terminal reports Γ as the
//!   (±5 bp) finite difference of the Black-76 N(d₁) delta in the
//!   displayed ATM forward spread.
//! - **Clean-price strikes**: a clean price is never fed into the Black
//!   spread `d₁`; Γ is the change in the curve-reprice price-strike delta
//!   under the same ±5 bp screen spread bump, applied as a par-quote
//!   hazard bump with rebootstrap and sticky-strike/sticky-surface rules.
//!
//! This module is the single source of truth; [`CDSOption::gamma`] is a
//! thin pass-through to [`gamma`].

use super::delta::{black_delta_ratio, delta_display_forward, price_strike_delta};
use crate::calibration::bumps::{bump_hazard_shift, bump_hazard_spreads, BumpRequest};
use crate::instruments::credit_derivatives::cds_option::bloomberg_quadrature::ForwardCdsContext;
use crate::instruments::credit_derivatives::cds_option::pricer::{
    resolve_sigma, synthetic_underlying_cds,
};
use crate::instruments::credit_derivatives::cds_option::{CDSOption, CDSOptionStrikeKind};
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::Result;

/// Screen gamma bump in forward-spread units (5 bp), for the Black `d₁`
/// spread-strike branch.
const GAMMA_SPREAD_BUMP: f64 = 0.0005;
/// Screen gamma bump in basis points, for the price-strike curve-reprice
/// branch (the same ±5 bp move expressed as a par-quote bump).
const PRICE_GAMMA_SPREAD_BUMP_BP: f64 = 5.0;

/// Bloomberg-screen `Gamma (+/-5bps)` calculator for credit options on CDS spreads.
pub(crate) struct GammaCalculator;

impl MetricCalculator for GammaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &CDSOption = context.instrument_as()?;
        gamma(option, &context.curves, context.as_of)
    }
}

/// CDS option Γ, branched by strike kind (see the module docs).
pub(crate) fn gamma(
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
        CDSOptionStrikeKind::Spread => spread_black_gamma(option, curves, as_of, t),
        CDSOptionStrikeKind::CleanPricePct => price_strike_gamma(option, curves, as_of),
    }
}

/// Bloomberg CDSO Γ — central difference of the Black-76 N(d₁) delta
/// across a ±5 bp move in the displayed ATM forward (spread strikes).
fn spread_black_gamma(
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
    let up = black_delta_ratio(
        option.option_type,
        clean_forward + GAMMA_SPREAD_BUMP,
        ctx.spread_strike()?,
        sigma,
        t,
    );
    let down = black_delta_ratio(
        option.option_type,
        (clean_forward - GAMMA_SPREAD_BUMP).max(1e-12),
        ctx.spread_strike()?,
        sigma,
        t,
    );
    Ok(up - down)
}

/// Price-strike Γ — change in the curve-reprice hedge-ratio delta across
/// a ±5 bp par-quote spread bump with rebootstrap. The surface is not
/// bumped, so each nested delta resolves the same sticky volatility.
fn price_strike_gamma(
    option: &CDSOption,
    curves: &MarketContext,
    as_of: finstack_quant_core::dates::Date,
) -> Result<f64> {
    let hazard = curves.get_hazard(&option.credit_curve_id)?;
    let bumped_market = |bump_bp: f64| -> Result<MarketContext> {
        let request = BumpRequest::Parallel(bump_bp);
        let bumped = if hazard.hazard_calibration().is_some() {
            bump_hazard_spreads(
                hazard.as_ref(),
                curves,
                &request,
                Some(&option.discount_curve_id),
                None,
                None,
            )?
        } else {
            bump_hazard_shift(hazard.as_ref(), &request)?
        };
        Ok(curves.clone().insert(bumped))
    };
    let delta_up = price_strike_delta(option, &bumped_market(PRICE_GAMMA_SPREAD_BUMP_BP)?, as_of)?;
    let delta_down =
        price_strike_delta(option, &bumped_market(-PRICE_GAMMA_SPREAD_BUMP_BP)?, as_of)?;
    Ok(delta_up - delta_down)
}
