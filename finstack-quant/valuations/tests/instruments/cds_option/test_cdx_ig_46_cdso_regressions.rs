//! Public-surface regression for the `cdx_ig_46_payer_atm_jun26` Bloomberg
//! CDSO golden: supplied-curve NPV and a zero-bump hazard rebootstrap.

#![allow(clippy::expect_used)]

use finstack_quant_calibration::api::engine;
use finstack_quant_calibration::api::schema::CalibrationEnvelope;
use finstack_quant_calibration::recalibration::bump_hazard_spreads;
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_valuations::instruments::credit_derivatives::cds::CdsValuationConvention;
use finstack_quant_valuations::instruments::credit_derivatives::cds_option::CDSOption;
use finstack_quant_valuations::instruments::Instrument;
use finstack_quant_valuations::market::conventions::ids::CdsDocClause;
use finstack_quant_valuations::recalibration::QuoteBump;
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

#[test]
fn cdx_ig_46_reported_npv_uses_supplied_curve_not_zero_rebootstrap() {
    let fixture = load_fixture_json();
    let as_of = date!(2026 - 05 - 07);
    let market = bootstrap_market(&fixture);
    let option = load_option(&fixture);
    let supplied_pv = option
        .value(&market, as_of)
        .expect("supplied market npv")
        .amount();

    let hazard = market.get_hazard(&option.credit_curve_id).expect("hazard");
    let zero_hazard = bump_hazard_spreads(
        hazard.as_ref(),
        &market,
        &QuoteBump::ParallelBp(0.0),
        Some(&option.discount_curve_id),
        Some(CdsDocClause::IsdaNa),
        Some(CdsValuationConvention::BloombergCdswClean),
    )
    .expect("zero-bump hazard rebootstrap");
    let zero_market = market.insert(zero_hazard);
    let zero_pv = option
        .value(&zero_market, as_of)
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
