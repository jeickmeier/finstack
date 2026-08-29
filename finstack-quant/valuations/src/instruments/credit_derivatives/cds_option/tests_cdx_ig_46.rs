//! Quadrature-grid regression on the `cdx_ig_46_payer_atm_jun26` golden.
//!
//! Lives beside the pricer so it can reach crate-private quadrature helpers.
//! Market bootstrap goes through calibration and returns only `MarketContext`
//! (a core type), which is safe from the valuations/calibration crate cycle.

use super::bloomberg_quadrature::{
    calibrate_lognormal_mean, normal_integral, z_limit, ForwardCdsContext,
};
use super::pricer::synthetic_underlying_cds;
use super::CDSOption;
use crate::constants::bloomberg_cdso;
use finstack_quant_calibration::api::engine;
use finstack_quant_calibration::api::schema::CalibrationEnvelope;
use finstack_quant_core::dates::Date;
use finstack_quant_core::market_data::context::MarketContext;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use time::macros::date;

const FIXTURE: &str =
    "tests/golden/data/pricing/bloomberg/cds_option/cdx_ig_46_payer_atm_jun26.json";

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE)
}

fn load_fixture_json() -> Value {
    let raw = fs::read_to_string(fixture_path()).expect("read fixture");
    serde_json::from_str(&raw).expect("parse fixture")
}

fn bootstrap_market(fixture: &Value) -> MarketContext {
    let envelope: CalibrationEnvelope =
        serde_json::from_value(fixture["market"]["envelope"].clone()).expect("parse envelope");
    let result = engine::execute(&envelope).expect("calibrate");
    MarketContext::try_from(result.result.final_market).expect("rehydrate market")
}

fn load_option(fixture: &Value) -> CDSOption {
    serde_json::from_value(fixture["instrument"]["instrument"]["spec"].clone())
        .expect("parse cds option spec")
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
    let s0 = (-0.5_f64 * ctx.sigma * ctx.sigma * t_expiry).exp();
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
