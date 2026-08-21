//! Par cds bump tests for scenarios.

use finstack_quant_core::currency::Currency;
use finstack_quant_core::dates::Date;
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::market_data::term_structures::DiscountCurve;
use finstack_quant_core::market_data::term_structures::HazardCurve;
use finstack_quant_core::money::Money;
use finstack_quant_scenarios::{
    CurveKind, ExecutionContext, HazardBumpMode, OperationSpec, ScenarioEngine, ScenarioSpec,
    TenorMatchMode,
};
use finstack_quant_statements::FinancialModelSpec;
use finstack_quant_valuations::calibration::bumps::HazardRecalibrationCache;
use finstack_quant_valuations::instruments::Bond;
use finstack_quant_valuations::instruments::Instrument;
use std::sync::Arc;
use time::Month;

#[test]
fn test_par_cds_bump_integration() {
    // Setup market with hazard curve and discount curve
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Create discount curve (needed for recalibration)
    let discount = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots(vec![(0.0, 1.0), (1.0, 0.95), (5.0, 0.80), (10.0, 0.60)])
        .build()
        .unwrap();

    // Create hazard curve with par spreads (needed for recalibration path)
    // Par spread ≈ hazard_rate * 10000 * (1 - recovery)
    // For 1Y: 0.01 * 10000 * 0.6 = 60 bp
    // For 5Y: 0.02 * 10000 * 0.6 = 120 bp
    let hazard = HazardCurve::builder("USD-CDS")
        .base_date(base_date)
        .recovery_rate(0.4)
        .knots(vec![(1.0, 0.01), (5.0, 0.02)])
        .par_spreads(vec![(1.0, 60.0), (5.0, 120.0)])
        .build()
        .unwrap();

    let mut market = MarketContext::new().insert(discount).insert(hazard);
    let mut model = FinancialModelSpec::new("test", vec![]);

    // Apply 10bp Par CDS bump at 5Y
    let scenario = ScenarioSpec {
        id: "par_cds_bump".into(),
        name: Some("Par CDS Bump".into()),
        description: None,
        operations: vec![OperationSpec::CurveNodeBp {
            curve_kind: CurveKind::ParCDS,
            curve_id: "USD-CDS".into(),
            discount_curve_id: None,
            nodes: vec![("5Y".to_string(), 10.0)],
            match_mode: TenorMatchMode::Exact,
        }],
        priority: 0,
        resolution_mode: Default::default(),
        hazard_bump_mode: Default::default(),
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: Some(&mut model),
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: base_date,
    };

    engine
        .apply(&scenario, &mut ctx)
        .expect("Shock application should succeed");

    // Verify result
    let bumped = market.get_hazard("USD-CDS").unwrap();

    // Check lambda at 5.0 (after recalibration, knots may have changed, so interpolate)
    let l_5y = bumped.hazard_rate(5.0);
    let original_lambda = 0.02;

    // With recalibration, the relationship is more complex than a simple shift
    // The key is that the hazard rate should increase when the par spread is bumped up
    println!("Original: {}, Bumped: {}", original_lambda, l_5y);
    assert!(
        l_5y > original_lambda,
        "Hazard rate should increase from Par CDS spread bump: original {}, got {}",
        original_lambda,
        l_5y
    );
}

#[test]
fn test_par_cds_bump_reprices_credit_bond() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();

    let discount = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots(vec![(0.0, 1.0), (1.0, 0.97), (5.0, 0.85), (10.0, 0.70)])
        .build()
        .unwrap();

    let hazard = HazardCurve::builder("USD-CDS")
        .base_date(base_date)
        .recovery_rate(0.4)
        .knots(vec![(1.0, 0.01), (5.0, 0.02)])
        .par_spreads(vec![(1.0, 60.0), (5.0, 120.0)])
        .build()
        .unwrap();

    let market = MarketContext::new().insert(discount).insert(hazard);
    let mut bond = Bond::fixed(
        "CREDIT-BOND",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        base_date,
        maturity,
        "USD-OIS",
    )
    .expect("bond");
    bond.credit_curve_id = Some("USD-CDS".into());

    let pv_base = bond.value(&market, base_date).expect("base pv").amount();

    let scenario = ScenarioSpec {
        id: "par_cds_parallel".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::CurveParallelBp {
            curve_kind: CurveKind::ParCDS,
            curve_id: "USD-CDS".into(),
            discount_curve_id: Some("USD-OIS".into()),
            bp: 25.0,
        }],
        priority: 0,
        resolution_mode: Default::default(),
        hazard_bump_mode: Default::default(),
    };

    let mut market_after = market;
    let mut model = FinancialModelSpec::new("test", vec![]);
    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market_after,
        model: Some(&mut model),
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: base_date,
    };
    engine.apply(&scenario, &mut ctx).expect("scenario apply");

    let pv_bumped = bond
        .value(&market_after, base_date)
        .expect("bumped pv")
        .amount();

    assert!(
        pv_bumped < pv_base,
        "wider par spreads should lower long bond PV: base={pv_base}, bumped={pv_bumped}"
    );
}

/// A time roll combined with a discount bump and a par-CDS bump must succeed
/// even when a discount knot sits exactly on the roll tenor.
///
/// `roll_forward` shifts knot times to `t' = t − dt` and drops only knots with
/// `t' ≤ 0`, so a knot sitting on the roll tenor survives as a tiny positive
/// residual whenever the roll's realized day count does not divide the tenor
/// exactly (a 3M business-day roll of 89/90/91 days leaves `t = 0.25` at
/// `t' = 0.006164 / 0.003425 / 0.000685`).
///
/// The discount bump used to fabricate a synthetic deposit maturing
/// `round(t' · 365.25)` days out — 2, 1 or 0 days for those residuals — while
/// the synthetic index carried a two-business-day settlement lag. The accrual
/// start then landed on or after maturity and schedule construction failed with
/// `Invalid date range: start must be before end`. The bump no longer builds
/// any dates, so residual knots are harmless.
#[test]
fn roll_with_discount_and_par_cds_bump_survives_knot_on_roll_tenor() {
    let base_date = Date::from_calendar_date(2025, Month::January, 15).unwrap();

    // Each roll tenor under test has a discount knot sitting exactly on it.
    for (period, tenor_years) in [
        ("1M", 1.0_f64 / 12.0),
        ("3M", 0.25_f64),
        ("6M", 0.5_f64),
        ("1Y", 1.0_f64),
    ] {
        let discount = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots(vec![
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
            .unwrap();

        let hazard = HazardCurve::builder("CORP-HAZARD")
            .base_date(base_date)
            .recovery_rate(0.40)
            .knots(vec![(1.0, 0.02), (3.0, 0.024), (5.0, 0.028), (10.0, 0.032)])
            .par_spreads(vec![
                (1.0, 116.639125),
                (3.0, 134.717567),
                (5.0, 147.417265),
                (10.0, 166.063391),
            ])
            .build()
            .unwrap();

        let mut market = MarketContext::new().insert(discount).insert(hazard);
        let mut model = FinancialModelSpec::new("test", vec![]);

        let scenario = ScenarioSpec {
            id: "roll_and_bump".into(),
            name: None,
            description: None,
            operations: vec![
                OperationSpec::TimeRollForward {
                    period: period.into(),
                    apply_shocks: true,
                    roll_mode: finstack_quant_scenarios::TimeRollMode::BusinessDays,
                },
                OperationSpec::CurveParallelBp {
                    curve_kind: CurveKind::Discount,
                    curve_id: "USD-OIS".into(),
                    discount_curve_id: None,
                    bp: 100.0,
                },
                OperationSpec::CurveParallelBp {
                    curve_kind: CurveKind::ParCDS,
                    curve_id: "CORP-HAZARD".into(),
                    discount_curve_id: Some("USD-OIS".into()),
                    bp: 50.0,
                },
            ],
            priority: 0,
            resolution_mode: Default::default(),
            hazard_bump_mode: Default::default(),
        };

        let engine = ScenarioEngine::new();
        let mut ctx = ExecutionContext {
            market: &mut market,
            model: Some(&mut model),
            instruments: None,
            rate_bindings: None,
            calendar: None,
            as_of: base_date,
        };

        engine
            .apply(&scenario, &mut ctx)
            .unwrap_or_else(|e| panic!("roll {period} + discount bump + par-CDS bump failed: {e}"));

        // The discount bump must still be faithful on the rolled grid.
        let bumped = market.get_discount("USD-OIS").unwrap();
        let rolled_base = bumped.base_date();
        assert!(
            rolled_base > base_date,
            "roll {period} should advance the base date",
        );
        let probe = tenor_years.max(2.0);
        assert!(
            bumped.df(probe) > 0.0 && bumped.df(probe) < 1.0,
            "roll {period}: bumped DF({probe}) out of range: {}",
            bumped.df(probe),
        );
    }
}

fn par_cds_market() -> (finstack_quant_core::dates::Date, MarketContext) {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let discount = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots(vec![(0.0, 1.0), (1.0, 0.95), (5.0, 0.80), (10.0, 0.60)])
        .build()
        .unwrap();
    let hazard = HazardCurve::builder("USD-CDS")
        .base_date(base_date)
        .recovery_rate(0.4)
        .knots(vec![(1.0, 0.01), (5.0, 0.02)])
        .par_spreads(vec![(1.0, 60.0), (5.0, 120.0)])
        .build()
        .unwrap();
    (
        base_date,
        MarketContext::new().insert(discount).insert(hazard),
    )
}

fn apply_par_cds(market: &mut MarketContext, as_of: Date, spec: &ScenarioSpec) {
    let mut model = FinancialModelSpec::new("test", vec![]);
    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market,
        model: Some(&mut model),
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of,
    };
    engine.apply(spec, &mut ctx).expect("apply");
}

#[test]
fn first_order_par_cds_shifts_hazard_knots_without_solve_to_par() {
    let (base_date, market) = par_cds_market();
    let original = market.get_hazard("USD-CDS").unwrap().hazard_rate(5.0);

    let mut shifted = market.clone();
    apply_par_cds(
        &mut shifted,
        base_date,
        &ScenarioSpec {
            id: "first_order".into(),
            name: None,
            description: None,
            operations: vec![OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::ParCDS,
                curve_id: "USD-CDS".into(),
                discount_curve_id: Some("USD-OIS".into()),
                bp: 25.0,
            }],
            priority: 0,
            resolution_mode: Default::default(),
            hazard_bump_mode: HazardBumpMode::FirstOrderShift,
        },
    );
    let mut solved = market;
    apply_par_cds(
        &mut solved,
        base_date,
        &ScenarioSpec {
            id: "solve".into(),
            name: None,
            description: None,
            operations: vec![OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::ParCDS,
                curve_id: "USD-CDS".into(),
                discount_curve_id: Some("USD-OIS".into()),
                bp: 25.0,
            }],
            priority: 0,
            resolution_mode: Default::default(),
            hazard_bump_mode: HazardBumpMode::SolveToPar,
        },
    );

    let shifted_h = shifted.get_hazard("USD-CDS").unwrap().hazard_rate(5.0);
    let solved_h = solved.get_hazard("USD-CDS").unwrap().hazard_rate(5.0);
    assert!(
        (shifted_h - (original + 0.0025)).abs() < 1e-10,
        "first-order should add 25bp to the 5Y hazard: original={original} shifted={shifted_h}"
    );
    assert!(
        (solved_h - shifted_h).abs() > 1e-6,
        "solve-to-par must differ from the first-order knot shift: solved={solved_h} shifted={shifted_h}"
    );
}

#[test]
fn sequential_same_curve_par_cds_ops_compound_with_shared_cache() {
    let (base_date, market) = par_cds_market();
    let cache = Arc::new(HazardRecalibrationCache::new());
    let engine = ScenarioEngine::new().with_hazard_cache(Arc::clone(&cache));

    let once = ScenarioSpec {
        id: "once".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::CurveParallelBp {
            curve_kind: CurveKind::ParCDS,
            curve_id: "USD-CDS".into(),
            discount_curve_id: Some("USD-OIS".into()),
            bp: 25.0,
        }],
        priority: 0,
        resolution_mode: Default::default(),
        hazard_bump_mode: HazardBumpMode::SolveToPar,
    };
    let twice = ScenarioSpec {
        id: "twice".into(),
        name: None,
        description: None,
        operations: vec![
            OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::ParCDS,
                curve_id: "USD-CDS".into(),
                discount_curve_id: Some("USD-OIS".into()),
                bp: 25.0,
            },
            OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::ParCDS,
                curve_id: "USD-CDS".into(),
                discount_curve_id: Some("USD-OIS".into()),
                bp: 25.0,
            },
        ],
        priority: 0,
        resolution_mode: Default::default(),
        hazard_bump_mode: HazardBumpMode::SolveToPar,
    };

    let mut once_market = market.clone();
    let mut twice_market = market;
    let mut model = FinancialModelSpec::new("test", vec![]);
    let mut ctx = ExecutionContext {
        market: &mut once_market,
        model: Some(&mut model),
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: base_date,
    };
    engine.apply(&once, &mut ctx).expect("once");
    drop(ctx);
    let mut model = FinancialModelSpec::new("test", vec![]);
    let mut ctx = ExecutionContext {
        market: &mut twice_market,
        model: Some(&mut model),
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: base_date,
    };
    engine.apply(&twice, &mut ctx).expect("twice");

    let h_once = once_market.get_hazard("USD-CDS").unwrap().hazard_rate(5.0);
    let h_twice = twice_market.get_hazard("USD-CDS").unwrap().hazard_rate(5.0);
    assert!(
        h_twice > h_once + 1e-6,
        "second sequential bump must not reuse the first cached curve: once={h_once} twice={h_twice}"
    );
}

#[test]
fn two_independent_par_cds_curves_match_separate_applies() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let discount = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots(vec![(0.0, 1.0), (1.0, 0.95), (5.0, 0.80), (10.0, 0.60)])
        .build()
        .unwrap();
    let ig = HazardCurve::builder("IG")
        .base_date(base_date)
        .recovery_rate(0.4)
        .knots(vec![(1.0, 0.01), (5.0, 0.02)])
        .par_spreads(vec![(1.0, 60.0), (5.0, 120.0)])
        .build()
        .unwrap();
    let hy = HazardCurve::builder("HY")
        .base_date(base_date)
        .recovery_rate(0.3)
        .knots(vec![(1.0, 0.05), (5.0, 0.07)])
        .par_spreads(vec![(1.0, 350.0), (5.0, 490.0)])
        .build()
        .unwrap();
    let market = MarketContext::new().insert(discount).insert(ig).insert(hy);

    let together = ScenarioSpec {
        id: "both".into(),
        name: None,
        description: None,
        operations: vec![
            OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::ParCDS,
                curve_id: "IG".into(),
                discount_curve_id: Some("USD-OIS".into()),
                bp: 10.0,
            },
            OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::ParCDS,
                curve_id: "HY".into(),
                discount_curve_id: Some("USD-OIS".into()),
                bp: 25.0,
            },
        ],
        priority: 0,
        resolution_mode: Default::default(),
        hazard_bump_mode: HazardBumpMode::FirstOrderShift,
    };
    let mut combined = market.clone();
    apply_par_cds(&mut combined, base_date, &together);

    let mut only_ig = market.clone();
    apply_par_cds(
        &mut only_ig,
        base_date,
        &ScenarioSpec {
            id: "ig".into(),
            name: None,
            description: None,
            operations: vec![together.operations[0].clone()],
            priority: 0,
            resolution_mode: Default::default(),
            hazard_bump_mode: HazardBumpMode::FirstOrderShift,
        },
    );
    let mut only_hy = market;
    apply_par_cds(
        &mut only_hy,
        base_date,
        &ScenarioSpec {
            id: "hy".into(),
            name: None,
            description: None,
            operations: vec![together.operations[1].clone()],
            priority: 0,
            resolution_mode: Default::default(),
            hazard_bump_mode: HazardBumpMode::FirstOrderShift,
        },
    );

    assert!(
        (combined.get_hazard("IG").unwrap().hazard_rate(5.0)
            - only_ig.get_hazard("IG").unwrap().hazard_rate(5.0))
        .abs()
            < 1e-12
    );
    assert!(
        (combined.get_hazard("HY").unwrap().hazard_rate(5.0)
            - only_hy.get_hazard("HY").unwrap().hazard_rate(5.0))
        .abs()
            < 1e-12
    );
}
