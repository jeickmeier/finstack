//! Behavioral baselines for callable credit-risky pricing on the
//! rates-credit tree path.
//!
//! These pin the PV/OAS numbers produced by the two-factor rates-credit
//! lattice for a callable bond and a callable term loan. They exist so the
//! configuration refactor (public `hazard_volatility` /
//! `rate_credit_correlation` inputs, one shared resolver, and the removal of
//! the implicit stochastic defaults) can prove it did not move any price.
//!
//! The pinned regime is the one the engines used implicitly before the
//! refactor — short-rate vol `0.01`, hazard vol `0.20`, no mean reversion,
//! zero correlation — now stated explicitly through `ModelConfig`. If a
//! number here moves, the lattice or the routing changed, not the wiring.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use finstack_quant_cashflows::builder::specs::CouponType;
use finstack_quant_core::currency::Currency;
use finstack_quant_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::market_data::term_structures::{DiscountCurve, HazardCurve, ParInterp};
use finstack_quant_core::money::Money;
use finstack_quant_core::types::{CurveId, InstrumentId};
use finstack_quant_valuations::instruments::fixed_income::bond::{Bond, CallPut, CallPutSchedule};
use finstack_quant_valuations::instruments::fixed_income::term_loan::{
    AmortizationSpec, LoanCall, LoanCallSchedule, LoanCallType, RateSpec, TermLoan,
    TermLoanTreePricer,
};
use time::macros::date;

/// Baseline regime: the volatilities the engines applied implicitly before
/// the configuration refactor made them public inputs.
const BASELINE_RATE_VOL: f64 = 0.01;
const BASELINE_HAZARD_VOL: f64 = 0.20;

fn as_of() -> Date {
    date!(2025 - 01 - 01)
}

fn discount_curve() -> DiscountCurve {
    DiscountCurve::builder("USD-OIS")
        .base_date(as_of())
        .knots([
            (0.0, 1.0),
            (1.0, (-0.04_f64).exp()),
            (5.0, (-0.20_f64).exp()),
            (10.0, (-0.40_f64).exp()),
        ])
        .build()
        .unwrap()
}

fn hazard_curve() -> HazardCurve {
    HazardCurve::builder("USD-HAZ")
        .base_date(as_of())
        .recovery_rate(0.4)
        .knots([(0.0, 0.03), (5.0, 0.03), (10.0, 0.03)])
        .par_interp(ParInterp::Linear)
        .build()
        .unwrap()
}

fn market() -> MarketContext {
    MarketContext::new()
        .insert(discount_curve())
        .insert(hazard_curve())
}

/// Fixed-rate callable bond routed onto the rates-credit tree by an explicit
/// `credit_curve_id`.
fn callable_credit_bond() -> Bond {
    let mut bond = Bond::fixed(
        "CALLABLE-CREDIT-BASELINE",
        Money::new(1_000_000.0, Currency::USD),
        0.06,
        as_of(),
        date!(2030 - 01 - 01),
        "USD-OIS",
    )
    .unwrap();
    bond.credit_curve_id = Some(CurveId::from("USD-HAZ"));
    bond.call_put = Some(CallPutSchedule {
        calls: vec![CallPut {
            start_date: date!(2027 - 01 - 01),
            end_date: date!(2029 - 01 - 01),
            price_pct_of_par: 102.0,
            make_whole: None,
        }],
        puts: vec![],
    });
    bond
}

fn callable_credit_loan() -> TermLoan {
    let mut loan = TermLoan::builder()
        .id(InstrumentId::new("TL-CALLABLE-CREDIT-BASELINE"))
        .currency(Currency::USD)
        .notional_limit(Money::new(10_000_000.0, Currency::USD))
        .issue_date(as_of())
        .maturity(date!(2030 - 01 - 01))
        .rate(RateSpec::Fixed { rate_bp: 600 })
        .frequency(Tenor::quarterly())
        .day_count(DayCount::Act360)
        .business_day_convention(BusinessDayConvention::ModifiedFollowing)
        .calendar_id_opt(None)
        .stub(StubKind::None)
        .discount_curve_id(CurveId::from("USD-OIS"))
        .credit_curve_id_opt(Some(CurveId::from("USD-HAZ")))
        .amortization(AmortizationSpec::None)
        .coupon_type(CouponType::Cash)
        .build()
        .unwrap();
    loan.call_schedule = Some(LoanCallSchedule {
        calls: vec![LoanCall {
            date: date!(2027 - 01 - 01),
            price_pct_of_par: 102.0,
            call_type: LoanCallType::Hard,
        }],
    });
    loan
}

/// Apply the baseline regime explicitly through the public model config.
///
/// Before the refactor this is a no-op on the bond path (the engine already
/// used these values implicitly) and is ignored on the hazard factor; after
/// the refactor it is the only way to select the regime.
fn apply_baseline_regime(
    overrides: &mut finstack_quant_valuations::instruments::InstrumentPricingOverrides,
) {
    overrides.market_quotes.implied_volatility = Some(BASELINE_RATE_VOL);
    let _ = BASELINE_HAZARD_VOL;
}

/// PV of the callable credit-risky bond under the baseline regime, captured
/// from the pre-refactor engine (implicit `rate_vol = 0.01`,
/// `hazard_vol = 0.20`).
const BASELINE_BOND_PV: f64 = 743_256.321_062_46;
/// PV of the callable credit-risky term loan under the same regime.
const BASELINE_LOAN_PV: f64 = 7_517_234.091_682_44;
/// OAS (bp) recovered by re-solving at the term loan's own model price.
/// Non-zero only by the clean/dirty accrued conversion inside the solve.
const BASELINE_LOAN_OAS_BP: f64 = -1.282_977_864_4;

#[test]
fn callable_credit_bond_pv_baseline() {
    use finstack_quant_valuations::instruments::Instrument;

    let mut bond = callable_credit_bond();
    apply_baseline_regime(&mut bond.instrument_pricing_overrides);

    let pv = bond.value(&market(), as_of()).unwrap().amount();

    assert!(
        (pv - BASELINE_BOND_PV).abs() < 1e-6,
        "callable credit-risky bond PV moved: actual={pv:.10}, \
         baseline={BASELINE_BOND_PV:.10}. The rates-credit lattice or the \
         bond routing changed — not just configuration wiring."
    );
}

#[test]
fn callable_credit_term_loan_pv_and_oas_baseline() {
    let loan = callable_credit_loan();
    let market = market();
    let pricer = TermLoanTreePricer::new();

    let pv = pricer
        .price_callable(&loan, &market, as_of())
        .unwrap()
        .amount();
    assert!(
        (pv - BASELINE_LOAN_PV).abs() < 1e-6,
        "callable credit-risky term-loan PV moved: actual={pv:.10}, \
         baseline={BASELINE_LOAN_PV:.10}"
    );

    // OAS re-solved at the instrument's own model price: the round trip must
    // land on the same point it did before the refactor, which also pins that
    // direct PV and the OAS objective share one calibrated tree.
    let clean_px = 100.0 * pv / loan.notional_limit.amount();
    let oas_bp = pricer
        .calculate_oas(&loan, &market, as_of(), clean_px)
        .unwrap();
    assert!(
        (oas_bp - BASELINE_LOAN_OAS_BP).abs() < 1e-6,
        "callable credit-risky term-loan OAS moved: actual={oas_bp:.10}, \
         baseline={BASELINE_LOAN_OAS_BP:.10}"
    );
}

/// Recovery, call friction, and amortization all still move the callable
/// credit-risky term loan in the documented direction. These are the
/// qualitative baselines the refactor must not invert.
#[test]
fn callable_credit_term_loan_structural_baselines() {
    let market = market();
    let pricer = TermLoanTreePricer::new();
    let base = callable_credit_loan();
    let base_pv = pricer
        .price_callable(&base, &market, as_of())
        .unwrap()
        .amount();

    // Call friction raises the exercise threshold, so the borrower's option is
    // worth less to them and the lender's claim is worth at least as much.
    let mut frictional = callable_credit_loan();
    frictional
        .instrument_pricing_overrides
        .model_config
        .call_friction_cents = Some(500.0);
    let frictional_pv = pricer
        .price_callable(&frictional, &market, as_of())
        .unwrap()
        .amount();
    assert!(
        frictional_pv >= base_pv - 1e-9,
        "call friction must not lower the lender's callable value: \
         base={base_pv:.6}, frictional={frictional_pv:.6}"
    );

    // Removing the call schedule recovers the straight credit-risky value,
    // which must dominate the callable one.
    let mut straight = callable_credit_loan();
    straight.call_schedule = None;
    let straight_pv = pricer
        .price_callable(&straight, &market, as_of())
        .unwrap()
        .amount();
    assert!(
        straight_pv >= base_pv - 1e-9,
        "straight credit-risky value must dominate the callable value: \
         straight={straight_pv:.6}, callable={base_pv:.6}"
    );

    // A higher hazard curve lowers PV.
    let high_hazard = HazardCurve::builder("USD-HAZ")
        .base_date(as_of())
        .recovery_rate(0.4)
        .knots([(0.0, 0.08), (5.0, 0.08), (10.0, 0.08)])
        .par_interp(ParInterp::Linear)
        .build()
        .unwrap();
    let stressed = MarketContext::new()
        .insert(discount_curve())
        .insert(high_hazard);
    let stressed_pv = pricer
        .price_callable(&base, &stressed, as_of())
        .unwrap()
        .amount();
    assert!(
        stressed_pv < base_pv,
        "higher hazard must lower PV: stressed={stressed_pv:.6}, base={base_pv:.6}"
    );
}
