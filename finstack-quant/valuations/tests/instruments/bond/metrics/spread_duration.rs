//! Spread duration tests for bonds.
//!
//! `SpreadDurationCalculator` is instrument-agnostic — it normalizes CS01 by
//! the base NPV (`-CS01 / (NPV × 1bp)`) — so it is registered for `Bond` as
//! well as structured credit. These tests pin the two properties consumers
//! rely on: the sign convention agrees between the bond and structured-credit
//! CS01 paths, and for a fixed-rate bullet with no credit curve (where CS01
//! falls back to a z-spread bump) spread duration reproduces modified
//! duration.

use finstack_quant_core::currency::Currency;
use finstack_quant_core::dates::{DayCount, Tenor};
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::market_data::term_structures::{DiscountCurve, HazardCurve};
use finstack_quant_core::money::Money;
use finstack_quant_core::types::CurveId;
use finstack_quant_valuations::instruments::fixed_income::bond::{Bond, CashflowSpec};
use finstack_quant_valuations::instruments::{Instrument, InstrumentPricingOverrides};
use finstack_quant_valuations::metrics::MetricId;
use time::macros::date;

/// Build a 5Y fixed-rate bullet corporate bond quoted at 96.5.
fn bullet_5y(credit: bool) -> (Bond, MarketContext) {
    let as_of = date!(2025 - 01 - 06);
    let mut bond = Bond::fixed(
        "SD-BULLET",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        as_of,
        date!(2030 - 01 - 06),
        "USD-OIS",
    )
    .expect("bond builds");
    bond.cashflow_spec = CashflowSpec::fixed_rate(0.05.into(), Tenor::annual(), DayCount::Act365F)
        .expect("finite coupon");
    bond.instrument_pricing_overrides =
        InstrumentPricingOverrides::default().with_quoted_clean_price(96.5);

    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .day_count(DayCount::Act365F)
        .knots([(0.0, 1.0), (7.0, 0.80)])
        .build()
        .expect("curve builds");
    let mut market = MarketContext::new().insert(disc);

    if credit {
        bond.credit_curve_id = Some(CurveId::new("USD-CREDIT"));
        let hazard = HazardCurve::builder("USD-CREDIT")
            .base_date(as_of)
            .recovery_rate(0.4)
            .knots([(0.0, 0.02), (7.0, 0.02)])
            .build()
            .expect("hazard builds");
        market = market.insert(hazard);
    }
    (bond, market)
}

fn priced(credit: bool) -> (f64, f64) {
    let (bond, market) = bullet_5y(credit);
    let result = bond
        .price_with_metrics(
            &market,
            date!(2025 - 01 - 06),
            &[
                MetricId::DurationMod,
                MetricId::ZSpread,
                MetricId::Cs01,
                MetricId::SpreadDuration,
            ],
            finstack_quant_valuations::instruments::PricingOptions::default(),
        )
        .expect("bond must emit spread_duration");
    let dur = *result
        .measures
        .get("duration_mod")
        .expect("duration_mod measure");
    let sd = *result
        .measures
        .get("spread_duration")
        .expect("spread_duration measure");
    (dur, sd)
}

/// A fixed-rate bullet with no credit curve takes the z-spread CS01 fallback,
/// so spread duration must reproduce modified duration (both measure the same
/// yield-basis sensitivity) — and must be **positive**, confirming the bond
/// CS01 path shares the structured-credit sign convention.
#[test]
fn fixed_rate_bullet_spread_duration_matches_modified_duration() {
    let (duration_mod, spread_duration) = priced(false);

    assert!(
        spread_duration > 0.0,
        "spread_duration must be positive for a long bond (CS01 sign convention), got {spread_duration}"
    );
    assert!(
        (3.0..6.0).contains(&spread_duration),
        "a 5Y bullet's spread duration must be a sane number of years, got {spread_duration}"
    );

    let rel = (spread_duration - duration_mod).abs() / duration_mod;
    assert!(
        rel < 0.01,
        "spread_duration ({spread_duration}) must track duration_mod ({duration_mod}) \
         for a fixed-rate bullet; relative difference {rel:.3e}"
    );
}

/// With a hazard curve, CS01 is the canonical par-CDS-spread bump on
/// survival-weighted cashflows rather than a z-spread bump, so spread duration
/// is legitimately smaller than modified duration. It must still be positive
/// and expressed in years.
#[test]
fn credit_curve_bond_spread_duration_is_positive_and_below_modified_duration() {
    let (duration_mod, spread_duration) = priced(true);

    assert!(
        spread_duration > 0.0,
        "spread_duration must be positive for a long bond, got {spread_duration}"
    );
    assert!(
        spread_duration < duration_mod,
        "par-CDS spread duration ({spread_duration}) should sit below modified duration \
         ({duration_mod}) because survival weighting damps the sensitivity"
    );
}
