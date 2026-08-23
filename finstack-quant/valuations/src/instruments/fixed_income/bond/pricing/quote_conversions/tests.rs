use super::*;
use crate::instruments::fixed_income::bond::Bond;
use finstack_quant_core::currency::Currency;
use finstack_quant_core::dates::{DayCount, DayCountContext, Tenor};
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_quant_core::money::Money;
use time::macros::date;

#[test]
fn asset_swap_forward_paths_use_discount_factor_implied_rates() {
    let base = date!(2025 - 01 - 01);
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .day_count(finstack_quant_core::dates::DayCount::Act360)
        .knots([(0.0, 1.0), (1.0, 0.95)])
        .build()
        .expect("discount curve should build");
    let fwd = ForwardCurve::builder("USD-3M", 0.25)
        .base_date(base)
        .day_count(finstack_quant_core::dates::DayCount::Act360)
        .knots([(0.0, 0.01), (1.0, 0.21)])
        .build()
        .expect("forward curve should build");
    let schedule = [base, date!(2025 - 07 - 01), date!(2026 - 01 - 01)];
    let mut expected_float_pv = 0.0;
    let mut integrated_float_pv = 0.0;
    for dates in schedule.windows(2) {
        let t1 = fwd
            .day_count()
            .year_fraction(base, dates[0], DayCountContext::default())
            .expect("valid start time");
        let t2 = fwd
            .day_count()
            .year_fraction(base, dates[1], DayCountContext::default())
            .expect("valid end time");
        let yf = fwd
            .day_count()
            .year_fraction(dates[0], dates[1], DayCountContext::default())
            .expect("valid accrual fraction");
        let df = disc
            .df_on_date_curve(dates[1])
            .expect("valid discount factor");
        expected_float_pv += fwd.rate_between(t1, t2).expect("valid term forward") * yf * df;
        integrated_float_pv += fwd.rate_period(t1, t2) * yf * df;
    }

    let (float_pv, fixed_ann, _) = asset_swap_forward_components(
        &disc,
        &fwd,
        finstack_quant_core::dates::DayCount::Act360,
        None,
        &schedule,
        0.0,
    )
    .expect("asset-swap components should succeed");
    let (par_rate, par_ann) = par_rate_and_annuity_from_forward(
        &disc,
        &fwd,
        finstack_quant_core::dates::DayCount::Act360,
        None,
        &schedule,
        0.0,
    )
    .expect("forward par rate should succeed");

    assert!((expected_float_pv - integrated_float_pv).abs() > 1e-6);
    assert!((float_pv - expected_float_pv).abs() < 1e-14);
    assert!((par_ann - fixed_ann).abs() < 1e-14);
    assert!((par_rate - expected_float_pv / fixed_ann).abs() < 1e-14);
}

#[test]
fn overnight_asset_swap_forward_paths_use_observation_average() {
    let base = date!(2025 - 01 - 01);
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .day_count(finstack_quant_core::dates::DayCount::Act360)
        .knots([(0.0, 1.0), (1.0, 0.95)])
        .build()
        .expect("discount curve should build");
    let fwd = ForwardCurve::builder("USD-SOFR", 1.0 / 360.0)
        .base_date(base)
        .day_count(finstack_quant_core::dates::DayCount::Act360)
        .knots([(0.0, 0.01), (1.0, 0.21)])
        .build()
        .expect("forward curve should build");
    let schedule = [base, date!(2026 - 01 - 01)];
    let t2 = fwd
        .day_count()
        .year_fraction(base, schedule[1], DayCountContext::default())
        .expect("valid end time");
    let yf = t2;
    let df = disc
        .df_on_date_curve(schedule[1])
        .expect("valid discount factor");
    let expected_float_pv = fwd.rate_period(0.0, t2) * yf * df;
    let term_float_pv = fwd.rate_between(0.0, t2).expect("valid term forward") * yf * df;

    let (float_pv, _, _) = asset_swap_forward_components(
        &disc,
        &fwd,
        finstack_quant_core::dates::DayCount::Act360,
        None,
        &schedule,
        0.0,
    )
    .expect("asset-swap components should succeed");

    assert!((expected_float_pv - term_float_pv).abs() > 1e-6);
    assert!((float_pv - expected_float_pv).abs() < 1e-14);
}

/// 31 CFR Part 356, Appendix B, section II.C long-first-period example.
///
/// 8.5% note issued 1990-03-01, first payment 1990-11-15, maturity
/// 1995-05-15, priced at an 8.53% Treasury yield: 99.805118 per 100.
#[test]
fn treasury_actual_matches_cfr_long_first_coupon_example() {
    let as_of = date!(1990 - 03 - 01);
    let coupon = 8.50 / 2.0;
    let fractional_coupon = coupon * 75.0 / 181.0;
    let flows = vec![
        (
            date!(1990 - 11 - 15),
            Money::new(coupon + fractional_coupon, Currency::USD),
        ),
        (date!(1991 - 05 - 15), Money::new(coupon, Currency::USD)),
        (date!(1991 - 11 - 15), Money::new(coupon, Currency::USD)),
        (date!(1992 - 05 - 15), Money::new(coupon, Currency::USD)),
        (date!(1992 - 11 - 15), Money::new(coupon, Currency::USD)),
        (date!(1993 - 05 - 15), Money::new(coupon, Currency::USD)),
        (date!(1993 - 11 - 15), Money::new(coupon, Currency::USD)),
        (date!(1994 - 05 - 15), Money::new(coupon, Currency::USD)),
        (date!(1994 - 11 - 15), Money::new(coupon, Currency::USD)),
        (
            date!(1995 - 05 - 15),
            Money::new(100.0 + coupon, Currency::USD),
        ),
    ];

    let price = price_from_ytm_compounded_params(
        DayCount::ActActIsma,
        Tenor::semi_annual(),
        &flows,
        as_of,
        0.0853,
        YieldCompounding::TreasuryActual,
    )
    .expect("Treasury Appendix B price");

    assert!(
        (price - 99.805118).abs() < 5e-7,
        "CFR long-first price mismatch: {price}"
    );
}

#[test]
fn treasury_actual_zero_coupon_round_trips() {
    let as_of = date!(2025 - 01 - 01);
    let flows = vec![(date!(2025 - 09 - 01), Money::new(100.0, Currency::USD))];
    let frequency = Tenor::semi_annual();
    let day_count = DayCount::Act365F;
    let expected_yield = 0.05;
    let price = price_from_ytm_compounded_params(
        day_count,
        frequency,
        &flows,
        as_of,
        expected_yield,
        YieldCompounding::TreasuryActual,
    )
    .expect("price");

    let solved = crate::instruments::fixed_income::bond::pricing::ytm_solver::solve_ytm(
        &flows,
        as_of,
        Money::new(price, Currency::USD),
        crate::instruments::fixed_income::bond::pricing::ytm_solver::YtmPricingSpec {
            day_count,
            notional: Money::new(100.0, Currency::USD),
            coupon_rate: 0.0,
            compounding: YieldCompounding::TreasuryActual,
            frequency,
        },
    )
    .expect("yield");
    assert!((solved - expected_yield).abs() < 1e-11);
}

#[test]
fn compute_quotes_returns_zeroes_for_effectively_zero_notional() {
    let as_of = date!(2025 - 01 - 01);
    let bond = Bond::fixed(
        "QE-NEAR-ZERO-NOTIONAL",
        Money::new(1e-12, Currency::USD),
        0.05,
        as_of,
        date!(2030 - 01 - 01),
        finstack_quant_core::dates::StubKind::ShortFront,
        "USD-OIS",
    )
    .expect("bond");
    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (5.0, 0.8)])
        .build()
        .expect("curve");

    let quotes = compute_quotes(
        &bond,
        &MarketContext::new().insert(curve),
        as_of,
        BondQuoteInput::CleanPricePct(99.0),
    )
    .expect("quote conversion");

    assert_eq!(quotes.clean_price_currency, 0.0);
    assert_eq!(quotes.clean_price_pct, 0.0);
    assert_eq!(quotes.dirty_price_currency, 0.0);
    assert!(quotes.ytm.is_none());
}
