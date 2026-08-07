//! CDX HY clean-price-strike CDS option tests.
//!
//! Covers the price-strike risk metrics (curve-reprice delta/gamma,
//! implied-vol round trips, vega) and end-to-end pricing behaviour that is
//! specific to the clean-price strike convention. The factor/loss payoff
//! algebra unit tests live next to the quadrature in
//! `bloomberg_quadrature.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::common::*;
use finstack_quant_core::dates::Date;
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_valuations::calibration::bumps::{bump_hazard_spreads, BumpRequest};
use finstack_quant_valuations::instruments::credit_derivatives::cds_option::bloomberg_quadrature::ForwardCdsContext;
use finstack_quant_valuations::instruments::credit_derivatives::cds_option::pricer::synthetic_underlying_cds;
use finstack_quant_valuations::instruments::credit_derivatives::cds_option::CDSOption;
use finstack_quant_valuations::instruments::Instrument;
use finstack_quant_valuations::instruments::OptionType;
use time::macros::date;

const VOL: f64 = 0.40;
const HY_COUPON_BP: f64 = 500.0;
const STRIKE_FACTOR: f64 = 1.0;
const CURRENT_FACTOR: f64 = 0.99;
const REALIZED_LOSS: f64 = 0.004;

fn hy_option(option_type: OptionType, strike_price_pct: f64, as_of: Date) -> CDSOption {
    let mut builder = CDSOptionBuilder::new()
        .id("CDX-HY-PRICE-STRIKE")
        .clean_price_strike(strike_price_pct)
        .with_index(CURRENT_FACTOR)
        .strike_index_factor(STRIKE_FACTOR)
        .realized_index_loss(REALIZED_LOSS)
        .underlying_cds_coupon_bp(HY_COUPON_BP)
        .implied_vol(VOL);
    builder = match option_type {
        OptionType::Call => builder.call(),
        OptionType::Put => builder.put(),
    };
    builder.build(as_of)
}

/// The native ATM-forward clean-price coordinate for the standard market.
fn atm_price_pct(market: &MarketContext, as_of: Date) -> f64 {
    let seed = hy_option(OptionType::Call, 105.0, as_of);
    let cds = synthetic_underlying_cds(&seed, as_of).expect("synthetic cds");
    let disc = market.get_discount("USD-OIS").expect("discount");
    let hazard = market.get_hazard("HZ-SN").expect("hazard");
    let ctx = ForwardCdsContext::build(&seed, disc.as_ref(), hazard.as_ref(), &cds, as_of, VOL)
        .expect("context");
    ctx.native_atm_forward_clean_price_pct()
        .expect("ATM coordinate")
}

#[test]
fn price_delta_signs_payer_positive_receiver_negative() {
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);
    let atm = atm_price_pct(&market, as_of);

    let payer_delta = hy_option(OptionType::Call, atm, as_of)
        .delta(&market, as_of)
        .expect("payer delta");
    let receiver_delta = hy_option(OptionType::Put, atm, as_of)
        .delta(&market, as_of)
        .expect("receiver delta");

    assert_positive(payer_delta, "ATM payer price-strike delta");
    assert!(
        receiver_delta < 0.0,
        "ATM receiver price-strike delta must be negative, got {receiver_delta}"
    );
    assert_in_range(payer_delta, 0.0, 1.3, "ATM payer delta magnitude");
    assert_in_range(receiver_delta, -1.3, 0.0, "ATM receiver delta magnitude");
}

#[test]
fn price_delta_deep_itm_near_one_deep_otm_near_zero() {
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);
    let atm = atm_price_pct(&market, as_of);

    // Payers gain value as the price strike rises: deep ITM at ATM + 15,
    // deep OTM at ATM − 15. Receivers are mirrored.
    let payer_itm = hy_option(OptionType::Call, atm + 15.0, as_of)
        .delta(&market, as_of)
        .expect("deep ITM payer delta");
    let payer_otm = hy_option(OptionType::Call, atm - 15.0, as_of)
        .delta(&market, as_of)
        .expect("deep OTM payer delta");
    assert!(
        payer_itm > 0.75,
        "deep ITM payer hedge ratio should approach 1, got {payer_itm}"
    );
    assert!(
        payer_otm < 0.15,
        "deep OTM payer hedge ratio should approach 0, got {payer_otm}"
    );

    let receiver_itm = hy_option(OptionType::Put, atm - 15.0, as_of)
        .delta(&market, as_of)
        .expect("deep ITM receiver delta");
    let receiver_otm = hy_option(OptionType::Put, atm + 15.0, as_of)
        .delta(&market, as_of)
        .expect("deep OTM receiver delta");
    assert!(
        receiver_itm < -0.75,
        "deep ITM receiver hedge ratio should approach −1, got {receiver_itm}"
    );
    assert!(
        receiver_otm > -0.15,
        "deep OTM receiver hedge ratio should approach 0, got {receiver_otm}"
    );
}

#[test]
fn price_delta_matches_independent_cs01_ratio() {
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);
    let atm = atm_price_pct(&market, as_of);
    let option = hy_option(OptionType::Call, atm, as_of);

    // Independent reconstruction from public pieces: symmetric ±1 bp
    // par-quote bump + rebootstrap, sticky σ (the instrument override),
    // option CS01 over underlying spread DV01.
    let hazard = market.get_hazard("HZ-SN").expect("hazard");
    let cds = synthetic_underlying_cds(&option, as_of).expect("synthetic cds");
    let bumped = |bp: f64| -> MarketContext {
        let curve = bump_hazard_spreads(
            hazard.as_ref(),
            &market,
            &BumpRequest::Parallel(bp),
            Some(&option.discount_curve_id),
        )
        .expect("bumped hazard");
        market.clone().insert(curve)
    };
    let up = bumped(1.0);
    let down = bumped(-1.0);
    let option_cs01 = (option.value(&up, as_of).unwrap().amount()
        - option.value(&down, as_of).unwrap().amount())
        / 2.0;
    let underlying_dv01 =
        (cds.value(&up, as_of).unwrap().amount() - cds.value(&down, as_of).unwrap().amount()) / 2.0;
    let expected = option_cs01 / underlying_dv01;

    let delta = option.delta(&market, as_of).expect("price-strike delta");
    assert_approx_eq(delta, expected, 1e-9, "delta vs independent CS01 ratio");
}

#[test]
fn price_gamma_positive_and_consistent_with_bumped_deltas() {
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);
    let atm = atm_price_pct(&market, as_of);

    for option_type in [OptionType::Call, OptionType::Put] {
        let option = hy_option(option_type, atm, as_of);
        let gamma = option.gamma(&market, as_of).expect("price-strike gamma");
        assert_finite(gamma, "price-strike gamma");
        assert_positive(gamma, "long-option gamma");

        // Nested finite-difference check: gamma must equal the change in
        // delta across the ±5 bp par-quote bump with rebootstrap.
        let hazard = market.get_hazard("HZ-SN").expect("hazard");
        let bumped = |bp: f64| -> MarketContext {
            let curve = bump_hazard_spreads(
                hazard.as_ref(),
                &market,
                &BumpRequest::Parallel(bp),
                Some(&option.discount_curve_id),
            )
            .expect("bumped hazard");
            market.clone().insert(curve)
        };
        let delta_up = option.delta(&bumped(5.0), as_of).expect("delta up");
        let delta_down = option.delta(&bumped(-5.0), as_of).expect("delta down");
        assert_approx_eq(
            gamma,
            delta_up - delta_down,
            1e-9,
            "gamma vs nested delta difference",
        );
    }
}

#[test]
fn price_strike_implied_vol_round_trips() {
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);
    let atm = atm_price_pct(&market, as_of);

    for option_type in [OptionType::Call, OptionType::Put] {
        let option = hy_option(option_type, atm, as_of);
        let target = option.value(&market, as_of).expect("npv").amount();
        assert_positive(target, "ATM option premium");
        let recovered = option
            .implied_vol(&market, as_of, target, None)
            .expect("implied vol");
        assert_approx_eq(recovered, VOL, 1e-6, "implied-vol round trip");
    }
}

#[test]
fn price_strike_implied_vol_rejects_unattainable_targets() {
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);
    let atm = atm_price_pct(&market, as_of);
    let option = hy_option(OptionType::Call, atm, as_of);

    // A premium approaching full notional is far above anything the model
    // can produce at the vol ceiling: must fail with a bracket error, not
    // return a plausible-looking volatility.
    let err = option
        .implied_vol(&market, as_of, 0.9 * option.notional.amount(), None)
        .expect_err("unattainable premium must fail");
    assert!(
        err.to_string().contains("outside model bounds"),
        "expected bracket error, got: {err}"
    );
}

#[test]
fn price_strike_vega_is_positive() {
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);
    let atm = atm_price_pct(&market, as_of);

    for option_type in [OptionType::Call, OptionType::Put] {
        let vega = hy_option(option_type, atm, as_of)
            .vega(&market, as_of)
            .expect("price-strike vega");
        assert_positive(vega, "near-ATM vega");
    }
}
