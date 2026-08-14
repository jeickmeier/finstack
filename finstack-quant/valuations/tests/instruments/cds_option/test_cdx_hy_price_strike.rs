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
            None,
            None,
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
                None,
                None,
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

// Pricing behaviour across index states, moneyness, vol, and time

fn context_for(option: &CDSOption, market: &MarketContext, as_of: Date) -> ForwardCdsContext {
    let cds = synthetic_underlying_cds(option, as_of).expect("synthetic cds");
    let disc = market.get_discount("USD-OIS").expect("discount");
    let hazard = market.get_hazard("HZ-SN").expect("hazard");
    ForwardCdsContext::build(option, disc.as_ref(), hazard.as_ref(), &cds, as_of, VOL)
        .expect("context")
}

/// Build a price-struck option with explicit factor/loss state.
fn hy_option_with_state(
    option_type: OptionType,
    strike_price_pct: f64,
    strike_factor: f64,
    current_factor: f64,
    realized_loss: f64,
    as_of: Date,
) -> CDSOption {
    let mut builder = CDSOptionBuilder::new()
        .id("CDX-HY-STATE")
        .clean_price_strike(strike_price_pct)
        .with_index(current_factor)
        .strike_index_factor(strike_factor)
        .realized_index_loss(realized_loss)
        .underlying_cds_coupon_bp(HY_COUPON_BP)
        .implied_vol(VOL);
    builder = match option_type {
        OptionType::Call => builder.call(),
        OptionType::Put => builder.put(),
    };
    builder.build(as_of)
}

/// Multiple settled defaults with nontrivial recoveries: three 1%-weight
/// names at recoveries 10%, 30%, and 50% give
/// `L = 0.9·0.01 + 0.7·0.01 + 0.5·0.01 = 0.021` with `f0 − f = 0.03`.
/// The direct H_price form must still equal the loss-omitting
/// adjusted-strike oracle, and payer/receiver parity must hold.
#[test]
fn multiple_defaults_direct_form_matches_oracle_and_parity() {
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);
    let (k_pct, f0, f) = (105.0, 1.0, 0.97);
    let loss = 0.9 * 0.01 + 0.7 * 0.01 + 0.5 * 0.01;
    let k_adj_pct = 100.0 * (1.0 - ((1.0 - k_pct / 100.0) * f0 - loss) / f);

    for option_type in [OptionType::Call, OptionType::Put] {
        let direct = hy_option_with_state(option_type, k_pct, f0, f, loss, as_of);
        let oracle = hy_option_with_state(option_type, k_adj_pct, f, f, 0.0, as_of);
        let npv_direct = direct.value(&market, as_of).expect("direct npv").amount();
        let npv_oracle = oracle.value(&market, as_of).expect("oracle npv").amount();
        assert!(
            (npv_direct - npv_oracle).abs() <= 1e-8 * npv_direct.abs().max(1.0),
            "{option_type:?}: multi-default direct={npv_direct} oracle={npv_oracle}"
        );
    }

    // Parity: payer − receiver = N · df · (f·F0 + (K−1)·f0 + L + f·FEP).
    let payer = hy_option_with_state(OptionType::Call, k_pct, f0, f, loss, as_of);
    let receiver = hy_option_with_state(OptionType::Put, k_pct, f0, f, loss, as_of);
    let ctx = context_for(&payer, &market, as_of);
    let expected = payer.notional.amount()
        * ctx.df_to_expiry
        * (f * ctx.no_knockout_forward()
            + (k_pct / 100.0 - 1.0) * f0
            + loss
            + f * ctx.front_end_protection);
    let lhs = payer.value(&market, as_of).unwrap().amount()
        - receiver.value(&market, as_of).unwrap().amount();
    assert_approx_eq(lhs, expected, 1e-6, "multi-default payer/receiver parity");
}

/// Payer premiums rise across OTM → ATM → ITM strikes; receiver premiums
/// mirror. Every premium dominates its clean intrinsic (time value ≥ 0).
#[test]
fn itm_atm_otm_premium_ordering_and_intrinsic_bound() {
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);
    let atm = atm_price_pct(&market, as_of);

    let payer_values: Vec<(f64, f64)> = [atm - 8.0, atm, atm + 8.0]
        .iter()
        .map(|&k| {
            let option = hy_option(OptionType::Call, k, as_of);
            (k, option.value(&market, as_of).unwrap().amount())
        })
        .collect();
    assert_increasing(&payer_values, "clean-price strike", "payer premium");

    let receiver_values: Vec<(f64, f64)> = [atm - 8.0, atm, atm + 8.0]
        .iter()
        .map(|&k| {
            let option = hy_option(OptionType::Put, k, as_of);
            (k, option.value(&market, as_of).unwrap().amount())
        })
        .collect();
    assert_decreasing(&receiver_values, "clean-price strike", "receiver premium");

    for option_type in [OptionType::Call, OptionType::Put] {
        for &k in &[atm - 8.0, atm, atm + 8.0] {
            let option = hy_option(option_type, k, as_of);
            let ctx = context_for(&option, &market, as_of);
            let intrinsic = option.notional.amount()
                * ctx.scale
                * ctx.df_to_expiry
                * (ctx.sign() * ctx.no_knockout_forward()
                    + ctx.signed_strike_adjustment_per_n()
                    + ctx.signed_loss_settlement_per_n())
                .max(0.0);
            let premium = option.value(&market, as_of).unwrap().amount();
            // Tolerance: one cent on a 10MM notional — the trapezoid grid's
            // discretization noise, far below any economic violation.
            assert!(
                premium >= intrinsic - 1e-2,
                "{option_type:?} K={k}: premium {premium} must dominate clean \
                 intrinsic {intrinsic}"
            );
        }
    }
}

/// Premiums are increasing in volatility and, near ATM, in time to expiry.
#[test]
fn premium_monotonic_in_vol_and_time() {
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);
    let atm = atm_price_pct(&market, as_of);

    for option_type in [OptionType::Call, OptionType::Put] {
        let vol_values: Vec<(f64, f64)> = [0.25, 0.40, 0.60]
            .iter()
            .map(|&vol| {
                let mut option = hy_option(option_type, atm, as_of);
                option
                    .instrument_pricing_overrides
                    .market_quotes
                    .implied_volatility = Some(vol);
                (vol, option.value(&market, as_of).unwrap().amount())
            })
            .collect();
        assert_increasing(&vol_values, "volatility", "premium");
    }

    // ATM premium grows with expiry (each expiry priced at its own ATM).
    let time_values: Vec<(f64, f64)> = [6, 12, 18]
        .iter()
        .map(|&months| {
            let seed = CDSOptionBuilder::new()
                .clean_price_strike(105.0)
                .with_index(CURRENT_FACTOR)
                .strike_index_factor(STRIKE_FACTOR)
                .realized_index_loss(REALIZED_LOSS)
                .underlying_cds_coupon_bp(HY_COUPON_BP)
                .implied_vol(VOL)
                .expiry_months(months)
                .call()
                .build(as_of);
            let ctx = context_for(&seed, &market, as_of);
            let atm_here = ctx
                .native_atm_forward_clean_price_pct()
                .expect("ATM coordinate");
            let option = CDSOptionBuilder::new()
                .clean_price_strike(atm_here)
                .with_index(CURRENT_FACTOR)
                .strike_index_factor(STRIKE_FACTOR)
                .realized_index_loss(REALIZED_LOSS)
                .underlying_cds_coupon_bp(HY_COUPON_BP)
                .implied_vol(VOL)
                .expiry_months(months)
                .call()
                .build(as_of);
            (
                f64::from(months),
                option.value(&market, as_of).unwrap().amount(),
            )
        })
        .collect();
    assert_increasing(&time_values, "months to expiry", "ATM premium");
}

/// Expiry-boundary and missing-curve behaviour.
#[test]
fn expiry_boundary_and_missing_curves_fail_or_degrade_explicitly() {
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);
    let option = hy_option(OptionType::Call, 105.0, as_of);

    // Cash-settled valuation AT expiry degenerates to the discounted
    // intrinsic (t = 0) and stays finite and non-negative.
    let at_expiry = option
        .value(&market, option.expiry)
        .expect("cash valuation at expiry")
        .amount();
    assert!(
        at_expiry.is_finite() && at_expiry >= 0.0,
        "expiry-boundary value must be finite and non-negative, got {at_expiry}"
    );

    // Missing hazard curve: explicit error.
    let no_hazard = MarketContext::new().insert(flat_discount("USD-OIS", as_of, 0.03));
    option
        .value(&no_hazard, as_of)
        .expect_err("missing hazard curve must fail");

    // Missing discount curve: explicit error.
    let no_discount = MarketContext::new().insert(flat_hazard("HZ-SN", as_of, 0.4, 0.02));
    option
        .value(&no_discount, as_of)
        .expect_err("missing discount curve must fail");
}

/// Independent premium cross-check: reimplement DOCS 2055833 Eq. 2.5 for
/// the price-strike payoff from the context's public deterministic inputs
/// — flat credit-triangle annuity, direct `H_price`, loss/FEP settlement,
/// outer factor scale — and integrate with an independent Simpson rule on
/// a finer grid. This validates the production quadrature wiring end to
/// end. It is a model-internal cross-check, NOT a vendor market golden:
/// an independent Bloomberg/Markit CDX HY premium still needs to be
/// captured as a golden fixture with provenance.
#[test]
fn independent_simpson_integration_reproduces_quadrature_premium() {
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);
    let atm = atm_price_pct(&market, as_of);

    for option_type in [OptionType::Call, OptionType::Put] {
        for &k_pct in &[atm - 8.0, atm, atm + 8.0] {
            let option = hy_option(option_type, k_pct, as_of);
            let ctx = context_for(&option, &market, as_of);
            let m = finstack_quant_valuations::instruments::credit_derivatives::cds_option::bloomberg_quadrature::calibrate_lognormal_mean(&ctx)
                .expect("calibrated lognormal mean");

            // Independent payoff reimplementation from public context data.
            let sign = match option_type {
                OptionType::Call => 1.0,
                OptionType::Put => -1.0,
            };
            let flat_annuity = |s: f64| -> f64 {
                let lambda = s / ctx.lgd;
                let mut acc = -ctx.accrual_pcd_to_expiry;
                for ((alpha, t), fwd_df) in ctx
                    .accrual_factors
                    .iter()
                    .zip(ctx.times_from_expiry.iter())
                    .zip(ctx.fwd_discount_factors.iter())
                {
                    acc += alpha * (-lambda * t).exp() * fwd_df;
                }
                acc
            };
            let k = k_pct / 100.0;
            let h_price = sign * (k - 1.0) * STRIKE_FACTOR / CURRENT_FACTOR;
            let d_loss = sign * (REALIZED_LOSS / CURRENT_FACTOR + ctx.front_end_protection);
            let s0 = (-0.5 * VOL * VOL * ctx.t_expiry).exp();
            let sig_sqrt_t = VOL * ctx.t_expiry.sqrt();
            let integrand = |z: f64| -> f64 {
                let s = m * s0 * (sig_sqrt_t * z).exp();
                let v = (s - ctx.coupon) * flat_annuity(s);
                (sign * v + h_price + d_loss).max(0.0) * (-0.5 * z * z).exp()
                    / (2.0 * std::f64::consts::PI).sqrt()
            };
            // Composite Simpson with 2400 intervals on the SAME [-6, 6]
            // production domain (different rule, much finer grid). Sharing
            // the domain isolates the comparison to payoff assembly and
            // integration wiring; the only residual difference is the
            // trapezoid's O(h²) error at the exercise kink.
            let (lo, hi, n) = (-6.0_f64, 6.0_f64, 2400_usize);
            let h = (hi - lo) / n as f64;
            let mut acc = integrand(lo) + integrand(hi);
            for i in 1..n {
                let weight = if i % 2 == 1 { 4.0 } else { 2.0 };
                acc += weight * integrand(lo + i as f64 * h);
            }
            let expected_premium =
                option.notional.amount() * CURRENT_FACTOR * ctx.df_to_expiry * (acc * h / 3.0);

            let premium = option.value(&market, as_of).unwrap().amount();
            // A mis-assembled H/D term would shift premiums by O($10k) on
            // this notional; the trapezoid kink error is O($1). The mixed
            // absolute/relative threshold cleanly separates the two.
            let diff = (premium - expected_premium).abs();
            let threshold = (1e-4 * premium.abs()).max(2.0);
            assert!(
                diff <= threshold,
                "quadrature premium vs independent Simpson: premium={premium}, \
                 independent={expected_premium}, diff={diff}, threshold={threshold}"
            );
        }
    }
}
