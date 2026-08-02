//! Invariant tests for the curve/surface bump paths.
//!
//! These tests pin the three properties that any bump implementation must
//! satisfy, and whose absence allowed a silent correctness defect in the
//! discount-curve synthetic bump path:
//!
//! 1. **Zero-shock identity** — a `0 bp` bump reproduces the original object
//!    exactly, at knot *and* non-knot tenors.
//! 2. **Faithfulness** — an `X bp` parallel bump moves the shocked quantity by
//!    exactly `X bp`.
//! 3. **Additivity** — `+X bp` followed by `−X bp` returns to the base.
//!
//! The historical defect: `bump_discount_curve_synthetic` implied synthetic
//! deposit quotes with a hardcoded `Act365F` day count and re-bootstrapped them
//! through an index registered as `Act360`, on a maturity grid
//! (`base + round(t · 365.25)` days) that did not map back to the original knot
//! times. The round trip was not the identity, so a `0 bp` shock moved
//! continuously-compounded zeros by roughly +4.4 bp to +5.5 bp.

use finstack_quant_core::currency::Currency;
use finstack_quant_core::dates::{Date, DayCount, Tenor, TenorUnit};
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::market_data::surfaces::VolSurface;
use finstack_quant_core::market_data::term_structures::{
    DiscountCurve, HazardCurve, InflationCurve, ParInterp, Seniority,
};
use finstack_quant_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use finstack_quant_core::HashMap;
use finstack_quant_valuations::calibration::bumps::{
    bump_discount_curve_synthetic, bump_hazard_shift, bump_hazard_spreads, bump_inflation_rates,
    bump_vol_surface, BumpRequest, VolBumpRequest,
};
use finstack_quant_valuations::market::conventions::ids::{CdsConventionKey, CdsDocClause};
use finstack_quant_valuations::market::quotes::cds::CdsQuote;
use finstack_quant_valuations::market::quotes::ids::Pillar;
use finstack_quant_valuations::market::{build_cds_instrument, BuildCtx};
use time::Month;

/// Tolerance for quantities that must agree to floating-point round-off.
const EXACT_TOL: f64 = 1e-12;

/// Knot and non-knot probe tenors. 0.75/1.5/4.0/7.5 fall strictly between
/// knots, so they exercise the interpolation path rather than stored knots.
const PROBE_TENORS: [f64; 11] = [0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 4.0, 5.0, 7.5, 10.0];

const KNOT_TENORS: [f64; 6] = [0.5, 1.0, 2.0, 3.0, 5.0, 10.0];

fn base_date() -> Date {
    Date::from_calendar_date(2025, Month::January, 15).expect("valid base date")
}

/// The exact curve from the defect reproduction.
///
/// Built with log-linear interpolation so that a parallel shift of the
/// continuously-compounded zero curve is exactly representable between knots:
/// log-DF is affine in `t`, and the shift `−δt` is affine, so the shift
/// survives interpolation unchanged.
fn sample_discount_curve() -> DiscountCurve {
    DiscountCurve::builder("USD-OIS")
        .base_date(base_date())
        .day_count(DayCount::Act365F)
        .interp(InterpStyle::LogLinear)
        .knots([
            (0.0, 1.0),
            (0.5, 0.980),
            (1.0, 0.960),
            (2.0, 0.920),
            (3.0, 0.880),
            (5.0, 0.800),
            (10.0, 0.650),
        ])
        .build()
        .expect("sample discount curve")
}

fn empty_market() -> MarketContext {
    MarketContext::new()
}

fn bump_discount(curve: &DiscountCurve, bump: &BumpRequest) -> DiscountCurve {
    bump_discount_curve_synthetic(curve, &empty_market(), bump, base_date(), Currency::USD)
        .expect("discount bump succeeds")
}

// ---------------------------------------------------------------------------
// Discount curve — the path that carried the defect
// ---------------------------------------------------------------------------

/// A `0 bp` bump must be a no-op at every tenor, knot and non-knot.
///
/// This is the test that would have caught the original defect: before the
/// fix, this shifted zeros by +4.42 bp (0.5y) through +5.55 bp (1y).
#[test]
fn discount_zero_shock_is_identity_at_knot_and_non_knot_tenors() {
    let base = sample_discount_curve();
    let bumped = bump_discount(&base, &BumpRequest::Parallel(0.0));

    for &t in &PROBE_TENORS {
        let shift_bp = (bumped.zero(t) - base.zero(t)) * 1e4;
        assert!(
            shift_bp.abs() < EXACT_TOL,
            "0bp bump must not move the zero curve at t={t}, got {shift_bp} bp",
        );
        assert!(
            (bumped.df(t) - base.df(t)).abs() < EXACT_TOL,
            "0bp bump must not move DF at t={t}",
        );
    }

    // The knot grid itself must be preserved: the defect also relocated knots
    // (t=0.5 became 0.50137, t=2.0 became 2.01096, ...).
    assert_eq!(
        base.knots(),
        bumped.knots(),
        "0bp bump must not relocate curve knots",
    );
}

/// An `X bp` parallel bump must move continuously-compounded zeros by exactly
/// `X bp` — that is what "parallel" means.
#[test]
fn discount_parallel_bump_is_faithful_at_knot_and_non_knot_tenors() {
    let base = sample_discount_curve();

    for requested_bp in [-25.0, -5.0, 1.0, 5.0, 50.0] {
        let bumped = bump_discount(&base, &BumpRequest::Parallel(requested_bp));
        for &t in &PROBE_TENORS {
            let realized_bp = (bumped.zero(t) - base.zero(t)) * 1e4;
            assert!(
                (realized_bp - requested_bp).abs() < 1e-9,
                "requested {requested_bp} bp at t={t}, realized {realized_bp} bp",
            );
        }
    }
}

/// `+X bp` then `−X bp` must return to the base curve.
#[test]
fn discount_bump_is_additive() {
    let base = sample_discount_curve();
    let up = bump_discount(&base, &BumpRequest::Parallel(5.0));
    let round_trip = bump_discount(&up, &BumpRequest::Parallel(-5.0));

    for &t in &PROBE_TENORS {
        assert!(
            (round_trip.zero(t) - base.zero(t)).abs() < EXACT_TOL,
            "+5bp then -5bp must return to base at t={t}",
        );
    }
}

/// A tenor-targeted bump moves the targeted knot and leaves the others alone.
#[test]
fn discount_tenor_bump_targets_closest_knot_only() {
    let base = sample_discount_curve();
    let bumped = bump_discount(&base, &BumpRequest::Tenors(vec![(5.0, 10.0)]));

    for &t in &KNOT_TENORS {
        let realized_bp = (bumped.zero(t) - base.zero(t)) * 1e4;
        let expected_bp = if (t - 5.0).abs() < EXACT_TOL {
            10.0
        } else {
            0.0
        };
        assert!(
            (realized_bp - expected_bp).abs() < 1e-9,
            "tenor bump at 5y: expected {expected_bp} bp at t={t}, got {realized_bp} bp",
        );
    }
}

/// A zero-shock tenor bump is also an exact identity.
#[test]
fn discount_zero_shock_tenor_bump_is_identity() {
    let base = sample_discount_curve();
    let bumped = bump_discount(&base, &BumpRequest::Tenors(vec![(1.0, 0.0), (5.0, 0.0)]));

    for &t in &PROBE_TENORS {
        assert!(
            (bumped.zero(t) - base.zero(t)).abs() < EXACT_TOL,
            "0bp tenor bump must not move the zero curve at t={t}",
        );
    }
}

/// The bumped curve keeps the original identifier so it can replace the base
/// curve in a `MarketContext` (the scenarios engine relies on this).
#[test]
fn discount_bump_preserves_curve_id_and_metadata() {
    let base = sample_discount_curve();
    let bumped = bump_discount(&base, &BumpRequest::Parallel(5.0));

    assert_eq!(base.id(), bumped.id(), "bump must preserve the curve id");
    assert_eq!(base.day_count(), bumped.day_count());
    assert_eq!(base.interp_style(), bumped.interp_style());
    assert_eq!(base.base_date(), bumped.base_date());
}

/// Bumping a rolled curve must work for *every* roll length, including the
/// lengths that leave a near-zero residual knot at the front.
///
/// `roll_forward` shifts knot times to `t' = t − dt` and drops only the knots
/// with `t' ≤ 0`. A knot sitting exactly on the roll tenor therefore survives as
/// a tiny positive residual whenever the roll's day count does not divide the
/// tenor exactly — e.g. a 3M business-day roll of 89/90/91 days leaves the
/// `t = 0.25` knot at `t' = 0.006164 / 0.003425 / 0.000685`.
///
/// The old synthetic path fabricated a deposit maturity at
/// `base + round(t' · 365.25)` days, which for those residuals is 2, 1, or 0
/// days out. The synthetic money-market index carries a two-business-day
/// settlement lag, so the deposit's accrual start landed on or after its
/// maturity and schedule construction failed with
/// `Invalid date range: start must be before end`.
///
/// Shifting zero rates directly never constructs a date, so a residual knot is
/// simply scaled by `exp(−δ · t') ≈ 1`.
#[test]
fn discount_bump_survives_every_roll_length() {
    // Knots sitting exactly on the 1M, 3M, 6M and 1Y roll tenors, so that every
    // common roll length produces a coincidence somewhere on the grid.
    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date())
        .day_count(DayCount::Act365F)
        .interp(InterpStyle::LogLinear)
        .knots([
            (0.0, 1.0),
            (1.0 / 12.0, 0.9963),
            (0.25, 0.9888),
            (0.5, 0.9775),
            (1.0, 0.955),
            (2.0, 0.91),
            (3.0, 0.87),
            (5.0, 0.80),
            (10.0, 0.65),
        ])
        .build()
        .expect("rolled-curve fixture");

    // Sweep every roll length through a year: this covers all four tenor
    // coincidences and, crucially, the near-zero-residual window on each side.
    for days in 0..=370i64 {
        let rolled = curve
            .roll_forward(days)
            .unwrap_or_else(|e| panic!("roll_forward({days}) failed: {e}"));

        let bumped = bump_discount_curve_synthetic(
            &rolled,
            &empty_market(),
            &BumpRequest::Parallel(100.0),
            rolled.base_date(),
            Currency::USD,
        )
        .unwrap_or_else(|e| panic!("bump after {days}d roll failed: {e}"));

        // The bump must remain faithful on the rolled grid, including at the
        // residual front knot.
        for &t in rolled.knots() {
            if t <= 0.0 {
                continue;
            }
            let realized_bp = (bumped.zero(t) - rolled.zero(t)) * 1e4;
            assert!(
                (realized_bp - 100.0).abs() < 1e-6,
                "after {days}d roll: requested 100 bp at t={t}, realized {realized_bp} bp",
            );
        }
    }
}

/// A zero-shock bump on a rolled curve is still an exact identity, including
/// when the roll leaves a near-zero residual knot.
#[test]
fn discount_zero_shock_on_rolled_curve_is_identity() {
    let curve = sample_discount_curve();

    for days in [88i64, 89, 90, 91, 92, 181, 182, 365] {
        let rolled = curve
            .roll_forward(days)
            .unwrap_or_else(|e| panic!("roll_forward({days}) failed: {e}"));
        let bumped = bump_discount_curve_synthetic(
            &rolled,
            &empty_market(),
            &BumpRequest::Parallel(0.0),
            rolled.base_date(),
            Currency::USD,
        )
        .unwrap_or_else(|e| panic!("0bp bump after {days}d roll failed: {e}"));

        assert_eq!(rolled.knots(), bumped.knots());
        for &t in rolled.knots() {
            assert!(
                (bumped.df(t) - rolled.df(t)).abs() < EXACT_TOL,
                "0bp bump after {days}d roll moved DF at t={t}",
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Sibling bump paths
// ---------------------------------------------------------------------------

fn sample_inflation_curve() -> InflationCurve {
    InflationCurve::builder("USD-CPI")
        .base_date(base_date())
        .base_cpi(300.0)
        .day_count(DayCount::Act360)
        .indexation_lag_months(3)
        .interp(InterpStyle::CubicHermite)
        .extrapolation(ExtrapolationPolicy::FlatForward)
        .knots([
            (0.0, 300.0),
            (1.0, 307.5),
            (2.0, 315.2),
            (5.0, 339.6),
            (10.0, 384.2),
        ])
        .build()
        .expect("sample inflation curve")
}

fn inflation_market() -> MarketContext {
    let discount = DiscountCurve::builder("USD-OIS")
        .base_date(base_date())
        .day_count(DayCount::Act365F)
        .knots([
            (0.0, 1.0),
            (1.0, 0.960),
            (2.0, 0.920),
            (5.0, 0.800),
            (10.0, 0.650),
        ])
        .build()
        .expect("inflation discount curve");
    MarketContext::new().insert(discount)
}

/// A `0 bp` inflation bump must reproduce the original CPI curve.
#[test]
fn inflation_zero_shock_is_identity() {
    let base = sample_inflation_curve();
    let market = inflation_market();
    let discount_id = finstack_quant_core::types::CurveId::new("USD-OIS");

    let bumped = bump_inflation_rates(
        &base,
        &market,
        &BumpRequest::Parallel(0.0),
        &discount_id,
        base_date(),
        Currency::USD,
        "3M",
    )
    .expect("inflation bump succeeds");

    for &t in &[1.0, 2.0, 5.0, 10.0] {
        let rel = (bumped.cpi(t) - base.cpi(t)).abs() / base.cpi(t);
        assert!(
            rel < 1e-10,
            "0bp inflation bump must not move CPI at t={t}: base={}, bumped={}",
            base.cpi(t),
            bumped.cpi(t),
        );
    }
}

/// An `X bp` inflation bump must move the implied zero-coupon inflation rate by
/// exactly `X bp`.
#[test]
fn inflation_parallel_bump_is_faithful() {
    let base = sample_inflation_curve();
    let market = inflation_market();
    let discount_id = finstack_quant_core::types::CurveId::new("USD-OIS");
    let requested_bp = 25.0;

    let bumped = bump_inflation_rates(
        &base,
        &market,
        &BumpRequest::Parallel(requested_bp),
        &discount_id,
        base_date(),
        Currency::USD,
        "3M",
    )
    .expect("inflation bump succeeds");

    let implied = |curve: &InflationCurve, t: f64| (curve.cpi(t) / curve.base_cpi()).powf(1.0 / t);
    for &t in &[1.0, 2.0, 5.0, 10.0] {
        let realized_bp = (implied(&bumped, t) - implied(&base, t)) * 1e4;
        assert!(
            (realized_bp - requested_bp).abs() < 1e-6,
            "requested {requested_bp} bp at t={t}, realized {realized_bp} bp",
        );
    }
}

fn implied_inflation_zero_rate(curve: &InflationCurve, t: f64) -> f64 {
    (curve.cpi(t) / curve.base_cpi()).powf(1.0 / t) - 1.0
}

/// Tenor allocation uses a strict 0.1-year window and sums every matching
/// target. Requests between or beyond pillars are no-ops unless they fall
/// inside that fuzzy window.
#[test]
fn inflation_tenor_bump_has_strict_fuzzy_additive_allocation() {
    let base = sample_inflation_curve();
    let discount_id = finstack_quant_core::types::CurveId::new("USD-OIS");
    let bumped = bump_inflation_rates(
        &base,
        &inflation_market(),
        &BumpRequest::Tenors(vec![
            (1.0, 7.0),
            (2.1, 100.0), // Exactly 0.1y from 2y: excluded by the strict bound.
            (5.0, 2.0),
            (5.05, 3.0), // Fuzzy match inside the 0.1y window.
            (5.0, 4.0),  // Repeated target: additive with both matches above.
            (3.0, 11.0), // Between pillars and outside every fuzzy window.
            (7.5, 13.0),
            (12.0, 17.0), // Beyond the last pillar.
        ]),
        &discount_id,
        base_date(),
        Currency::USD,
        "3M",
    )
    .expect("inflation tenor bump succeeds");

    assert_eq!(bumped.knots(), base.knots());
    assert_eq!(bumped.cpi_levels()[0], base.cpi_levels()[0]);
    for (t, expected_bp) in [(1.0, 7.0), (2.0, 0.0), (5.0, 9.0), (10.0, 0.0)] {
        let realized_bp =
            (implied_inflation_zero_rate(&bumped, t) - implied_inflation_zero_rate(&base, t)) * 1e4;
        assert!(
            (realized_bp - expected_bp).abs() < 1e-6,
            "expected {expected_bp} bp inflation shift at {t}y, got {realized_bp} bp",
        );
    }

    assert_eq!(bumped.id(), base.id());
    assert_eq!(bumped.base_date(), base.base_date());
    assert_eq!(bumped.base_cpi(), base.base_cpi());
    assert_eq!(bumped.day_count(), base.day_count());
    assert_eq!(bumped.indexation_lag_months(), base.indexation_lag_months());
    assert_eq!(bumped.interp_style(), base.interp_style());
    assert_eq!(bumped.extrapolation(), base.extrapolation());
}

fn hazard_base_date() -> Date {
    Date::from_calendar_date(2025, Month::March, 20).expect("valid hazard base date")
}

fn sample_hazard_curve() -> HazardCurve {
    HazardCurve::builder("ACME-SEN-USD")
        .base_date(hazard_base_date())
        .day_count(DayCount::Act360)
        .recovery_rate(0.4)
        .issuer("ACME")
        .seniority(Seniority::Subordinated)
        .currency(Currency::USD)
        .par_interp(ParInterp::LogLinear)
        .interp(InterpStyle::Linear)
        .fx_policy("single_curve_credit::USD")
        .knots([(1.0, 0.010), (3.0, 0.015), (5.0, 0.020), (10.0, 0.025)])
        .par_spreads([(1.0, 80.0), (3.0, 100.0), (5.0, 130.0), (10.0, 180.0)])
        .build()
        .expect("sample hazard curve")
}

fn assert_hazard_metadata_eq(base: &HazardCurve, bumped: &HazardCurve) {
    assert_eq!(bumped.id(), base.id());
    assert_eq!(bumped.base_date(), base.base_date());
    assert_eq!(bumped.day_count(), base.day_count());
    assert_eq!(bumped.recovery_rate(), base.recovery_rate());
    assert_eq!(bumped.issuer(), base.issuer());
    assert_eq!(bumped.seniority, base.seniority);
    assert_eq!(bumped.currency(), base.currency());
    assert_eq!(bumped.par_interp(), base.par_interp());
    assert_eq!(bumped.survival_interp_style(), base.survival_interp_style());
    assert_eq!(bumped.fx_policy(), base.fx_policy());
}

/// A `0 bp` model hazard shift must reproduce the original hazard curve.
#[test]
fn hazard_shift_zero_shock_is_identity() {
    let base = sample_hazard_curve();
    let bumped = bump_hazard_shift(&base, &BumpRequest::Parallel(0.0)).expect("hazard shift");

    for &t in &[0.5, 1.0, 2.0, 3.0, 5.0, 7.5, 10.0] {
        assert!(
            (bumped.sp(t) - base.sp(t)).abs() < EXACT_TOL,
            "0bp hazard shift must not move survival at t={t}",
        );
    }
}

/// An `X bp` model hazard shift must move hazard rates by exactly `X bp`.
#[test]
fn hazard_shift_is_faithful_and_additive() {
    let base = sample_hazard_curve();
    let requested_bp = 25.0;
    let up =
        bump_hazard_shift(&base, &BumpRequest::Parallel(requested_bp)).expect("hazard shift up");

    for &t in &[1.0, 3.0, 5.0, 10.0] {
        let realized_bp = (up.hazard_rate(t) - base.hazard_rate(t)) * 1e4;
        assert!(
            (realized_bp - requested_bp).abs() < 1e-9,
            "requested {requested_bp} bp at t={t}, realized {realized_bp} bp",
        );
    }

    let round_trip =
        bump_hazard_shift(&up, &BumpRequest::Parallel(-requested_bp)).expect("hazard shift down");
    for &t in &[1.0, 3.0, 5.0, 10.0] {
        assert!(
            (round_trip.hazard_rate(t) - base.hazard_rate(t)).abs() < EXACT_TOL,
            "hazard shift must be additive at t={t}",
        );
    }
}

/// Direct model-hazard buckets address exact pillars, otherwise the lower
/// containing segment. Requests beyond the final support are explicit no-ops.
#[test]
fn hazard_tenor_shift_pins_segment_boundaries_and_near_knot_matching() {
    let base = sample_hazard_curve();
    let base_points: Vec<(f64, f64)> = base.knot_points().collect();
    let cases = [
        (0.5, Some(0), "below first"),
        (1.0, Some(0), "first boundary"),
        (2.0, Some(0), "first containing segment"),
        (3.0, Some(1), "exact interior knot"),
        (3.0 + 5e-7, Some(1), "near interior knot"),
        (4.0, Some(1), "second containing segment"),
        (10.0, Some(3), "last boundary"),
        (10.0 + 5e-7, Some(3), "near last knot"),
        (10.0 + 2e-6, None, "beyond last"),
    ];

    for (target, expected_idx, label) in cases {
        let bumped = bump_hazard_shift(&base, &BumpRequest::Tenors(vec![(target, 7.0)]))
            .unwrap_or_else(|error| panic!("{label} bump failed: {error}"));
        let bumped_points: Vec<(f64, f64)> = bumped.knot_points().collect();
        assert_eq!(bumped_points.len(), base_points.len());
        for (idx, ((base_t, base_rate), (bumped_t, bumped_rate))) in
            base_points.iter().zip(&bumped_points).enumerate()
        {
            assert_eq!(bumped_t, base_t, "{label} relocated knot {idx}");
            let expected_rate = base_rate + if expected_idx == Some(idx) { 7e-4 } else { 0.0 };
            assert!(
                (bumped_rate - expected_rate).abs() < EXACT_TOL,
                "{label}: expected hazard {expected_rate} at knot {idx}, got {bumped_rate}",
            );
        }
        assert_hazard_metadata_eq(&base, &bumped);
        assert_eq!(
            bumped.par_spread_points().collect::<Vec<_>>(),
            base.par_spread_points().collect::<Vec<_>>()
        );
    }
}

#[test]
fn repeated_hazard_segment_targets_are_additive() {
    let base = sample_hazard_curve();
    let bumped = bump_hazard_shift(
        &base,
        &BumpRequest::Tenors(vec![(4.0, 3.0), (4.0, 5.0), (3.0, 2.0)]),
    )
    .expect("repeated segment shifts succeed");
    let base_points: Vec<_> = base.knot_points().collect();
    let bumped_points: Vec<_> = bumped.knot_points().collect();

    for (idx, ((base_t, base_rate), (bumped_t, bumped_rate))) in
        base_points.iter().zip(&bumped_points).enumerate()
    {
        assert_eq!(bumped_t, base_t);
        let expected_rate = base_rate + if idx == 1 { 10e-4 } else { 0.0 };
        assert!((bumped_rate - expected_rate).abs() < EXACT_TOL);
    }
    assert_hazard_metadata_eq(&base, &bumped);
}

fn hazard_market(hazard: HazardCurve) -> MarketContext {
    let discount = DiscountCurve::builder("USD-OIS")
        .base_date(hazard_base_date())
        .day_count(DayCount::Act360)
        .knots([
            (0.0, 1.0),
            (1.0, (-0.04_f64).exp()),
            (3.0, (-0.12_f64).exp()),
            (5.0, (-0.20_f64).exp()),
            (10.0, (-0.40_f64).exp()),
            (30.0, (-1.20_f64).exp()),
        ])
        .build()
        .expect("hazard discount curve");
    MarketContext::new().insert(discount).insert(hazard)
}

fn par_cds_quote(years: u32, spread_bp: f64) -> CdsQuote {
    CdsQuote::CdsParSpread {
        id: format!("ACME-CDS-{years}Y").into(),
        entity: "ACME".to_string(),
        convention: CdsConventionKey {
            currency: Currency::USD,
            doc_clause: CdsDocClause::IsdaNa,
        },
        pillar: Pillar::Tenor(Tenor::new(years, TenorUnit::Years)),
        spread_bp,
        recovery_rate: 0.4,
    }
}

fn par_cds_build_context() -> BuildCtx {
    let mut curve_ids = HashMap::default();
    curve_ids.insert("discount".to_string(), "USD-OIS".to_string());
    curve_ids.insert("credit".to_string(), "ACME-SEN-USD".to_string());
    BuildCtx::new(hazard_base_date(), 1.0, curve_ids)
}

/// Par-spread tenor bumps use the same strict fuzzy allocation as inflation,
/// then rebootstrap. The observable contract is quote repricing, not an
/// assumed one-for-one displacement of model hazard rates.
#[test]
fn hazard_par_spread_tenor_bump_reprices_strict_fuzzy_targets() {
    let source = sample_hazard_curve();
    let market = hazard_market(source.clone());
    let discount_id = finstack_quant_core::types::CurveId::new("USD-OIS");
    let bumped = bump_hazard_spreads(
        &source,
        &market,
        &BumpRequest::Tenors(vec![
            (1.0, 2.0),
            (1.05, 3.0),
            (1.0, 4.0),
            (3.1, 100.0), // Exactly 0.1y from 3y: excluded.
            (4.0, 11.0),  // Between stored par tenors.
            (5.05, 5.0),
            (5.05, -2.0), // Repeated fuzzy target: additive.
            (12.0, 13.0), // Beyond the final par tenor.
        ]),
        Some(&discount_id),
    )
    .expect("hazard par-spread bump succeeds");
    let expected_quotes = [(1, 89.0), (3, 100.0), (5, 133.0), (10, 180.0)];
    let repriced_spreads: Vec<f64> = bumped
        .par_spread_points()
        .map(|(_, spread)| spread)
        .collect();
    assert_eq!(repriced_spreads.len(), expected_quotes.len());
    for (&actual, (_, expected)) in repriced_spreads.iter().zip(&expected_quotes) {
        assert!((actual - expected).abs() < 1e-10);
    }
    assert_hazard_metadata_eq(&source, &bumped);

    let bumped_market = market.insert(bumped);
    let build_ctx = par_cds_build_context();
    for (years, spread_bp) in expected_quotes {
        let quote = par_cds_quote(years, spread_bp);
        let instrument = build_cds_instrument(&quote, &build_ctx).expect("build par CDS");
        let residual = instrument
            .value_raw(&bumped_market, hazard_base_date())
            .expect("reprice bumped par CDS");
        assert!(
            residual.abs() <= 5e-7,
            "{years}Y bumped par spread {spread_bp}bp repriced to residual {residual}",
        );
    }
}

/// The sum of one-bucket quote risks reconciles to a parallel quote risk at
/// the same 1 bp shock, within the suite's existing 2% CS01 tolerance.
#[test]
fn hazard_par_spread_bucket_risk_reconciles_to_parallel() {
    let source = sample_hazard_curve();
    let source_market = hazard_market(source.clone());
    let discount_id = finstack_quant_core::types::CurveId::new("USD-OIS");
    let calibrated = bump_hazard_spreads(
        &source,
        &source_market,
        &BumpRequest::Parallel(0.0),
        Some(&discount_id),
    )
    .expect("base par curve calibration");
    let market = source_market.insert(calibrated);
    let instrument = build_cds_instrument(&par_cds_quote(7, 150.0), &par_cds_build_context())
        .expect("build risk CDS");
    let base_pv = instrument
        .value_raw(&market, hazard_base_date())
        .expect("base CDS value");

    let parallel = bump_hazard_spreads(
        &source,
        &market,
        &BumpRequest::Parallel(1.0),
        Some(&discount_id),
    )
    .expect("parallel par bump");
    let parallel_risk = instrument
        .value_raw(&market.clone().insert(parallel), hazard_base_date())
        .expect("parallel bumped CDS value")
        - base_pv;

    // Every comparison starts from the same original quote grid. That removes
    // the zero-bump schedule-snap round trip from the risk and isolates only
    // the requested quote shock.
    let bucket_times: Vec<f64> = source.par_spread_points().map(|(t, _)| t).collect();
    let mut bucket_sum = 0.0;
    for tenor in bucket_times {
        let bucket = bump_hazard_spreads(
            &source,
            &market,
            &BumpRequest::Tenors(vec![(tenor, 1.0)]),
            Some(&discount_id),
        )
        .unwrap_or_else(|error| panic!("{tenor}y par bucket failed: {error}"));
        bucket_sum += instrument
            .value_raw(&market.clone().insert(bucket), hazard_base_date())
            .expect("bucket-bumped CDS value")
            - base_pv;
    }

    assert!(parallel_risk.is_finite() && parallel_risk.abs() > 1e-12);
    let tolerance = 1e-8_f64.max(0.02 * parallel_risk.abs());
    assert!(
        (bucket_sum - parallel_risk).abs() <= tolerance,
        "bucket risk {bucket_sum} must reconcile to parallel risk {parallel_risk}; tolerance {tolerance}",
    );
}

fn sample_vol_surface() -> VolSurface {
    VolSurface::from_grid(
        "SPX-VOL",
        &[0.25, 1.0, 2.0],
        &[80.0, 100.0, 120.0],
        &[
            0.28, 0.22, 0.20, //
            0.26, 0.21, 0.20, //
            0.25, 0.21, 0.21, //
        ],
    )
    .expect("sample vol surface")
}

/// A zero vol bump must reproduce the original surface.
#[test]
fn vol_zero_shock_is_identity() {
    let base = sample_vol_surface();
    let bumped = bump_vol_surface(&base, &VolBumpRequest::Parallel(0.0)).expect("vol bump");

    for &expiry in &[0.25, 0.5, 1.0, 2.0] {
        for &strike in &[80.0, 90.0, 100.0, 120.0] {
            assert!(
                (bumped.value_clamped(expiry, strike) - base.value_clamped(expiry, strike)).abs()
                    < EXACT_TOL,
                "0 vol bump must not move vol at ({expiry}, {strike})",
            );
        }
    }
}

/// A parallel vol bump is exactly additive in vol points and reverses cleanly.
#[test]
fn vol_parallel_bump_is_faithful_and_additive() {
    let base = sample_vol_surface();
    let shift = 0.01;
    let up = bump_vol_surface(&base, &VolBumpRequest::Parallel(shift)).expect("vol bump up");

    for &expiry in &[0.25, 1.0, 2.0] {
        for &strike in &[80.0, 100.0, 120.0] {
            let realized = up.value_clamped(expiry, strike) - base.value_clamped(expiry, strike);
            assert!(
                (realized - shift).abs() < EXACT_TOL,
                "requested {shift} vol at ({expiry}, {strike}), realized {realized}",
            );
        }
    }

    let round_trip = bump_vol_surface(&up, &VolBumpRequest::Parallel(-shift)).expect("vol bump dn");
    for &expiry in &[0.25, 1.0, 2.0] {
        for &strike in &[80.0, 100.0, 120.0] {
            assert!(
                (round_trip.value_clamped(expiry, strike) - base.value_clamped(expiry, strike))
                    .abs()
                    < EXACT_TOL,
                "vol bump must be additive at ({expiry}, {strike})",
            );
        }
    }
}
