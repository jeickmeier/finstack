use super::yield_price::df_treasury_actual_with_first_period;
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

/// W-30: a Treasury with a LONG first coupon period (8 months on a
/// semi-annual bond) must have its first-period stub flagged from the
/// cashflow schedule. The price returned by `price_from_ytm_compounded_params`
/// for `TreasuryActual` must apply simple interest over the actual 8-month
/// first period — not the time-based `t <= 1/m` heuristic which would
/// split it into a regular 6-month period plus a 2-month stub.
#[test]
fn treasury_actual_long_first_coupon_uses_schedule_stub() {
    let as_of = date!(2025 - 01 - 01);
    let day_count = DayCount::Act365F;
    let frequency = Tenor::semi_annual();
    let ytm = 0.05;

    // Long first coupon: ~8 months to first flow, then semi-annual.
    let flows: Vec<(finstack_quant_core::dates::Date, Money)> = vec![
        (date!(2025 - 09 - 01), Money::new(3.0, Currency::USD)),
        (date!(2026 - 03 - 01), Money::new(3.0, Currency::USD)),
        (date!(2026 - 09 - 01), Money::new(103.0, Currency::USD)),
    ];

    let price_actual = price_from_ytm_compounded_params(
        day_count,
        frequency,
        &flows,
        as_of,
        ytm,
        YieldCompounding::TreasuryActual,
    )
    .expect("treasury-actual price");

    // First-period length flagged from the schedule (yf to first flow).
    let first_period_len = day_count
        .year_fraction(as_of, flows[0].0, DayCountContext::default())
        .expect("first period yf");
    assert!(
        first_period_len > 0.5,
        "test precondition: first coupon must be a LONG stub (>1 regular period), got {first_period_len}"
    );
    let m = periods_per_year(frequency).expect("m").max(1.0);

    // Schedule-aware reference: simple interest over the actual first period.
    let mut expected_schedule = 0.0;
    // Buggy time-based reference: the old `df_from_yield` heuristic.
    let mut buggy_time_based = 0.0;
    for &(date, amount) in &flows {
        let t = day_count
            .year_fraction(as_of, date, DayCountContext::default())
            .expect("yf");
        expected_schedule += amount.amount()
            * df_treasury_actual_with_first_period(ytm, t, m, first_period_len)
                .expect("schedule df");
        buggy_time_based += amount.amount()
            * df_from_yield(ytm, t, YieldCompounding::TreasuryActual, frequency)
                .expect("time-based df");
    }

    assert!(
        (price_actual - expected_schedule).abs() < 1e-9,
        "price {price_actual} must use the schedule-flagged stub {expected_schedule}"
    );
    assert!(
        (price_actual - buggy_time_based).abs() > 1e-4,
        "price {price_actual} must NOT match the time-based heuristic {buggy_time_based}"
    );
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
