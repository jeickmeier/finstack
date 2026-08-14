//! Regression tests for the `cdx_ig_46_payer_atm_jun26` Bloomberg CDSO golden.

#![allow(clippy::expect_used)]

use finstack_quant_core::dates::Date;
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_valuations::calibration::api::engine;
use finstack_quant_valuations::calibration::api::schema::CalibrationEnvelope;
use finstack_quant_valuations::calibration::bumps::{bump_hazard_spreads, BumpRequest};
use finstack_quant_valuations::constants::bloomberg_cdso;
use finstack_quant_valuations::instruments::credit_derivatives::cds::CdsValuationConvention;
use finstack_quant_valuations::instruments::credit_derivatives::cds_option::bloomberg_quadrature::{
    calibrate_lognormal_mean, normal_integral, npv, z_limit, ForwardCdsContext,
};
use finstack_quant_valuations::instruments::credit_derivatives::cds_option::pricer::synthetic_underlying_cds;
use finstack_quant_valuations::instruments::credit_derivatives::cds_option::CDSOption;
use finstack_quant_valuations::market::conventions::ids::CdsDocClause;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use time::macros::date;

const FIXTURE: &str =
    "tests/golden/data/pricing/bloomberg/cds_option/cdx_ig_46_payer_atm_jun26.json";
const BBG_NPV: f64 = 118_781.76;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE)
}

fn load_fixture_json() -> Value {
    let raw = fs::read_to_string(fixture_path()).expect("read fixture");
    serde_json::from_str(&raw).expect("parse fixture")
}

fn fixture_market_envelope(fixture: &Value) -> &Value {
    &fixture["market"]["envelope"]
}

fn fixture_instrument_spec(fixture: &Value) -> &Value {
    &fixture["instrument"]["instrument"]["spec"]
}

fn bootstrap_market(fixture: &Value) -> MarketContext {
    let envelope: CalibrationEnvelope =
        serde_json::from_value(fixture_market_envelope(fixture).clone()).expect("parse envelope");
    let result = engine::execute_with_diagnostics(&envelope).expect("calibrate");
    MarketContext::try_from(result.result.final_market).expect("rehydrate market")
}

fn load_option(fixture: &Value) -> CDSOption {
    serde_json::from_value(fixture_instrument_spec(fixture).clone()).expect("parse cds option spec")
}

fn context_for(
    option: &CDSOption,
    market: &MarketContext,
    as_of: Date,
    sigma: f64,
) -> ForwardCdsContext {
    let cds = synthetic_underlying_cds(option, as_of).expect("synthetic cds");
    let discount = market
        .get_discount(&option.discount_curve_id)
        .expect("discount");
    let hazard = market.get_hazard(&option.credit_curve_id).expect("hazard");
    ForwardCdsContext::build(
        option,
        discount.as_ref(),
        hazard.as_ref(),
        &cds,
        as_of,
        sigma,
    )
    .expect("forward cds context")
}

#[test]
fn cdx_ig_46_production_integrand_converges_at_quadrature_step() {
    let fixture = load_fixture_json();
    let as_of = date!(2026 - 05 - 07);
    let market = bootstrap_market(&fixture);
    let option = load_option(&fixture);
    let ctx = context_for(&option, &market, as_of, 0.3603);
    let m = calibrate_lognormal_mean(&ctx).expect("calibrate lognormal mean");
    let t_expiry = ctx.t_expiry.max(0.0);
    let s0 = (-0.5 * ctx.sigma * ctx.sigma * t_expiry).exp();
    let sigma_sqrt_t = ctx.sigma * t_expiry.sqrt();
    let integrand = |z: f64| {
        let s = m * s0 * (sigma_sqrt_t * z).exp();
        ctx.swap_value_per_n(s)
    };

    let production = normal_integral(
        bloomberg_cdso::Z_STEP,
        z_limit(ctx.sigma, t_expiry),
        integrand,
    );
    let fine = normal_integral(
        bloomberg_cdso::Z_STEP * 0.5,
        z_limit(ctx.sigma, t_expiry),
        integrand,
    );
    let dollar_diff = (production - fine).abs() * option.notional.amount();

    assert!(
        dollar_diff < 0.01,
        "production quadrature grid should be sub-cent stable on V_te(s): diff=${dollar_diff:.8}",
    );
}

#[test]
fn cdx_ig_46_reported_npv_uses_supplied_curve_not_zero_rebootstrap() {
    let fixture = load_fixture_json();
    let as_of = date!(2026 - 05 - 07);
    let market = bootstrap_market(&fixture);
    let option = load_option(&fixture);
    let cds = synthetic_underlying_cds(&option, as_of).expect("synthetic cds");
    let supplied_pv = npv(&option, &cds, &market, 0.3603, as_of)
        .expect("supplied market npv")
        .amount();

    let hazard = market.get_hazard(&option.credit_curve_id).expect("hazard");
    let zero_hazard = bump_hazard_spreads(
        hazard.as_ref(),
        &market,
        &BumpRequest::Parallel(0.0),
        Some(&option.discount_curve_id),
        Some(CdsDocClause::IsdaNa),
        Some(CdsValuationConvention::BloombergCdswClean),
    )
    .expect("zero-bump hazard rebootstrap");
    let zero_market = market.insert(zero_hazard);
    let zero_pv = npv(&option, &cds, &zero_market, 0.3603, as_of)
        .expect("zero-bump market npv")
        .amount();

    // $6 band: matches the golden fixture tolerance. Removing the ARRC 2-day
    // lookback from cleared-OIS presets (2026-06 moderate-fix pass) shifted
    // the bootstrapped USD swap curve, leaving a documented -$5.32 residual
    // versus the Bloomberg screen value.
    assert!(
        (supplied_pv - BBG_NPV).abs() < 6.0,
        "reported NPV should remain anchored to the supplied fixture market: supplied={supplied_pv}, target={BBG_NPV}",
    );
    // A zero-bump rebootstrap must preserve the supplied curve.
    assert!(
        (zero_pv - supplied_pv).abs() < 1e-6,
        "zero-bump rebootstrap should reproduce the supplied-curve NPV: supplied={supplied_pv}, zero={zero_pv}",
    );
}
