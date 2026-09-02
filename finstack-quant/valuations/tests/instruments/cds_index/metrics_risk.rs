//! CDS Index risk metrics tests.
//!
//! Tests cover:
//! - DV01 (interest rate sensitivity)
//! - CS01 (credit spread sensitivity)
//! - Risky PV01 (premium spread sensitivity)
//! - Hazard CS01 (hazard rate sensitivity)
//! - Bucketed DV01 (term structure sensitivity)
//! - Risk metric scaling with notional
//! - Risk metric sign conventions

use super::test_utils::*;
use finstack_quant_core::config::FinstackConfig;
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_valuations::constants::isda::STANDARD_RECOVERY_SENIOR;
use finstack_quant_valuations::instruments::Instrument;
use finstack_quant_valuations::metrics::MetricId;
use serde_json::json;
use time::macros::date;

#[test]
fn test_risky_pv01_positive() {
    // Test: Risky PV01 should be positive
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-RPV01", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);

    let result = idx
        .price_with_metrics(
            &ctx,
            as_of,
            &[MetricId::RiskyPv01],
            crate::test_support::credit::pricing_options(),
        )
        .unwrap();
    let rpv01 = *result.measures.get("risky_pv01").unwrap();

    assert_positive(rpv01, "Risky PV01");
    in_range(rpv01, 3_500.0, 5_500.0, "Risky PV01 for $10MM, 5Y");
}

#[test]
fn test_cs01_positive() {
    // Test: CS01 should be positive
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-CS01", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);

    let result = idx
        .price_with_metrics(
            &ctx,
            as_of,
            &[MetricId::Cs01Hazard],
            crate::test_support::credit::pricing_options(),
        )
        .unwrap();
    let cs01 = *result.measures.get("cs01_hazard").unwrap();

    assert_positive(cs01, "CS01");
}

#[test]
fn test_dv01_calculation() {
    // Test: DV01 (interest rate sensitivity) calculation
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-DV01", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);

    let result = idx
        .price_with_metrics(
            &ctx,
            as_of,
            &[MetricId::Dv01],
            crate::test_support::credit::pricing_options(),
        )
        .unwrap();
    let dv01 = *result.measures.get("dv01").unwrap();

    // DV01 = PV(rate+1bp) - PV(base); sign depends on instrument structure
    assert!(dv01.is_finite(), "DV01 should be finite");
}

#[test]
fn test_hazard_cs01_calculation() {
    // Test: Hazard CS01 (parallel hazard bump sensitivity)
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-HCS01", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);

    let result = idx
        .price_with_metrics(
            &ctx,
            as_of,
            &[MetricId::Cs01Hazard],
            crate::test_support::credit::pricing_options(),
        )
        .unwrap();

    // CS01 should be present
    let cs01 = result
        .measures
        .get("cs01_hazard")
        .expect("hazard CS01 should be present");
    assert!(cs01.is_finite(), "CS01 should be finite");
}

#[test]
fn test_dv01_scales_with_notional() {
    // Test: DV01 scales linearly with notional
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;
    let ctx = standard_market_context(as_of);

    let idx_10mm = standard_single_curve_index("CDX-10MM", start, end, 10_000_000.0);
    let idx_20mm = standard_single_curve_index("CDX-20MM", start, end, 20_000_000.0);

    let result_10mm = idx_10mm
        .price_with_metrics(
            &ctx,
            as_of,
            &[MetricId::Dv01],
            crate::test_support::credit::pricing_options(),
        )
        .unwrap();
    let result_20mm = idx_20mm
        .price_with_metrics(
            &ctx,
            as_of,
            &[MetricId::Dv01],
            crate::test_support::credit::pricing_options(),
        )
        .unwrap();

    let dv01_10mm = *result_10mm.measures.get("dv01").unwrap();
    let dv01_20mm = *result_20mm.measures.get("dv01").unwrap();

    assert_linear_scaling(
        dv01_10mm,
        10_000_000.0,
        dv01_20mm,
        20_000_000.0,
        "DV01",
        0.01,
    );
}

#[test]
fn test_cs01_increases_with_maturity() {
    // Test: CS01 increases with longer maturity
    let start = date!(2025 - 01 - 01);
    let as_of = start;
    let ctx = standard_market_context(as_of);

    let idx_3y = standard_single_curve_index("CDX-3Y", start, date!(2028 - 01 - 01), 10_000_000.0);
    let idx_5y = standard_single_curve_index("CDX-5Y", start, date!(2030 - 01 - 01), 10_000_000.0);

    let result_3y = idx_3y
        .price_with_metrics(
            &ctx,
            as_of,
            &[MetricId::Cs01Hazard],
            crate::test_support::credit::pricing_options(),
        )
        .unwrap();
    let result_5y = idx_5y
        .price_with_metrics(
            &ctx,
            as_of,
            &[MetricId::Cs01Hazard],
            crate::test_support::credit::pricing_options(),
        )
        .unwrap();

    let cs01_3y = *result_3y.measures.get("cs01_hazard").unwrap();
    let cs01_5y = *result_5y.measures.get("cs01_hazard").unwrap();

    assert!(
        cs01_3y < cs01_5y,
        "CS01 should increase with maturity: 3Y={}, 5Y={}",
        cs01_3y,
        cs01_5y
    );
}

#[test]
fn test_standard_cs01_requires_replay_recipe() {
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-CS01", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);
    let provider = finstack_quant_calibration::recalibration::CachedRecalibrationProvider::new();

    let direct_error = idx
        .cs01(&ctx, as_of, &provider)
        .expect_err("standard CS01 requires quote-space replay");
    assert!(direct_error.to_string().contains("calibration recipe"));

    let result = idx
        .price_with_metrics(
            &ctx,
            as_of,
            &[MetricId::Cs01],
            finstack_quant_valuations::instruments::PricingOptions::default()
                .with_recalibration_provider(std::sync::Arc::new(provider)),
        )
        .expect_err("standard CS01 metric requires quote-space replay");
    assert!(result.to_string().contains("calibration recipe"));
}

#[test]
fn test_risky_pv01_single_vs_constituents() {
    // Test: Risky PV01 consistency across pricing modes
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;
    let ctx = multi_constituent_market_context(as_of, 5);

    let idx_single = standard_single_curve_index("CDX-SINGLE", start, end, 10_000_000.0);
    let idx_const = standard_constituents_index("CDX-CONST", start, end, 10_000_000.0, 5);

    let result_single = idx_single
        .price_with_metrics(
            &ctx,
            as_of,
            &[MetricId::RiskyPv01],
            crate::test_support::credit::pricing_options(),
        )
        .unwrap();
    let result_const = idx_const
        .price_with_metrics(
            &ctx,
            as_of,
            &[MetricId::RiskyPv01],
            crate::test_support::credit::pricing_options(),
        )
        .unwrap();

    let rpv01_single = *result_single.measures.get("risky_pv01").unwrap();
    let rpv01_const = *result_const.measures.get("risky_pv01").unwrap();

    relative_eq(rpv01_single, rpv01_const, 0.05, "Risky PV01 parity");
}

#[test]
fn test_cs01_single_vs_constituents() {
    // Test: CS01 consistency across pricing modes
    //
    // Both modes use identical hazard rates (0.015) and recovery (40%).
    // CS01 is computed by bumping hazard curves by 1bp and repricing.
    // - Single-curve: bumps HZ-INDEX
    // - Constituents: bumps each HZ1..HZ5 independently and sums
    //
    // With identical curves, both should produce similar results.
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;
    let ctx = multi_constituent_market_context(as_of, 5);

    let idx_single = standard_single_curve_index("CDX-SINGLE", start, end, 10_000_000.0);
    let idx_const = standard_constituents_index("CDX-CONST", start, end, 10_000_000.0, 5);

    let result_single = idx_single
        .price_with_metrics(
            &ctx,
            as_of,
            &[MetricId::Cs01Hazard],
            crate::test_support::credit::pricing_options(),
        )
        .unwrap();
    let result_const = idx_const
        .price_with_metrics(
            &ctx,
            as_of,
            &[MetricId::Cs01Hazard],
            crate::test_support::credit::pricing_options(),
        )
        .unwrap();

    let cs01_single = *result_single.measures.get("cs01_hazard").unwrap();
    let cs01_const = *result_const.measures.get("cs01_hazard").unwrap();

    // 5% tolerance: aggregation of per-constituent CS01 vs single curve
    relative_eq(cs01_single, cs01_const, 0.05, "CS01 parity");
}

#[test]
fn test_bucketed_cs01_requires_replay_recipe() {
    // Manually built curves have no replay recipe.
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-BKT-SC", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);

    let error = idx
        .price_with_metrics(
            &ctx,
            as_of,
            &[MetricId::Cs01, MetricId::BucketedCs01],
            crate::test_support::credit::pricing_options(),
        )
        .expect_err("standard CDS index CS01 requires quote-space replay");
    assert!(error.to_string().contains("calibration recipe"));
}

#[test]
#[ignore = "slow: covered by mise rust-test-slow"]
fn bucketed_cs01_quote_single_curve_uses_each_off_grid_replay_quote_once() {
    let as_of = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let source = MarketContext::new().insert(flat_discount_curve("USD-OIS", as_of, 0.03));
    let hazard = crate::test_support::credit::calibrated_hazard_curve_with_pillars(
        &source,
        as_of,
        "HZ-INDEX",
        "INDEX",
        "USD-OIS",
        &[
            (365, 80.0),
            (3 * 365, 100.0),
            (4 * 365, 115.0),
            (5 * 365, 125.0),
            (10 * 365, 140.0),
        ],
    )
    .expect("off-grid index hazard calibration");
    let expected_count = hazard
        .hazard_calibration()
        .expect("replayable index hazard")
        .spread_risk_inputs
        .len();
    let market = source.insert(hazard);
    let index = standard_single_curve_index("CDX-QUOTE-OFFGRID", as_of, end, 10_000_000.0);

    let result = index
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::Cs01, MetricId::BucketedCs01],
            crate::test_support::credit::pricing_options(),
        )
        .expect("single-curve quote-space bucketed CS01");
    let prefix = "bucketed_cs01::HZ-INDEX::";
    let buckets: Vec<_> = result
        .measures
        .iter()
        .filter(|(key, _)| key.as_str().starts_with(prefix))
        .collect();
    let bucket_sum: f64 = buckets.iter().map(|(_, value)| **value).sum();

    assert_eq!(
        buckets.len(),
        expected_count,
        "each index replay quote must appear once: {buckets:?}"
    );
    assert!(
        buckets
            .iter()
            .any(|(key, _)| key.as_str() == "bucketed_cs01::HZ-INDEX::4y"),
        "off-grid 4Y index quote must be represented: {buckets:?}"
    );
    relative_eq(
        bucket_sum,
        result.measures[MetricId::Cs01.as_str()],
        0.02,
        "single-curve quote buckets vs parallel CS01",
    );
}

#[test]
#[ignore = "slow: covered by mise rust-test-slow"]
fn bucketed_cs01_quote_constituents_use_each_off_grid_replay_quote_once() {
    let as_of = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let source = MarketContext::new().insert(flat_discount_curve("USD-OIS", as_of, 0.03));
    let hz1 = crate::test_support::credit::calibrated_hazard_curve_with_pillars(
        &source,
        as_of,
        "HZ1",
        "NAME1",
        "USD-OIS",
        &[
            (365, 80.0),
            (3 * 365, 100.0),
            (4 * 365, 115.0),
            (5 * 365, 125.0),
            (10 * 365, 140.0),
        ],
    )
    .expect("first off-grid constituent hazard calibration");
    let hz2 = crate::test_support::credit::calibrated_hazard_curve_with_pillars(
        &source,
        as_of,
        "HZ2",
        "NAME2",
        "USD-OIS",
        &[
            (365, 90.0),
            (3 * 365, 105.0),
            (4 * 365, 118.0),
            (5 * 365, 130.0),
            (10 * 365, 145.0),
        ],
    )
    .expect("second off-grid constituent hazard calibration");
    let expected_per_curve = hz1
        .hazard_calibration()
        .expect("replayable constituent hazard")
        .spread_risk_inputs
        .len();
    let market = source.insert(hz1).insert(hz2).insert(flat_hazard_curve(
        "HZ-INDEX",
        as_of,
        STANDARD_RECOVERY_SENIOR,
        STANDARD_HAZARD_RATE,
    ));
    let index = standard_constituents_index("CDX-CONSTITUENT-OFFGRID", as_of, end, 10_000_000.0, 2);

    let result = index
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::Cs01, MetricId::BucketedCs01],
            crate::test_support::credit::pricing_options(),
        )
        .expect("constituent quote-space bucketed CS01");
    let curve_buckets = |curve_id: &str| {
        let prefix = format!("bucketed_cs01::{curve_id}::");
        result
            .measures
            .iter()
            .filter(|(key, _)| key.as_str().starts_with(&prefix))
            .collect::<Vec<_>>()
    };
    let hz1_buckets = curve_buckets("HZ1");
    let hz2_buckets = curve_buckets("HZ2");
    let bucket_sum: f64 = hz1_buckets
        .iter()
        .chain(&hz2_buckets)
        .map(|(_, value)| **value)
        .sum();

    assert_eq!(hz1_buckets.len(), expected_per_curve);
    assert_eq!(hz2_buckets.len(), expected_per_curve);
    assert!(hz1_buckets
        .iter()
        .any(|(key, _)| key.as_str() == "bucketed_cs01::HZ1::4y"));
    assert!(hz2_buckets
        .iter()
        .any(|(key, _)| key.as_str() == "bucketed_cs01::HZ2::4y"));
    relative_eq(
        bucket_sum,
        result.measures[MetricId::Cs01.as_str()],
        0.02,
        "constituent quote buckets vs parallel CS01",
    );
}

#[test]
#[ignore = "slow: covered by mise rust-test-slow"]
fn test_bucketed_cs01_reconciles_to_parallel_constituents() {
    // Same reconciliation in `Constituents` mode: the bucketed calculator bumps
    // every constituent curve at each tenor and reprices the index end-to-end.
    // Expensive under parallel CI load (N curves × tenors × central-diff reprices).
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;
    let ctx = multi_constituent_market_context(as_of, 5);

    let idx = standard_constituents_index("CDX-BKT-CONST", start, end, 10_000_000.0, 5);

    let result = idx
        .price_with_metrics(
            &ctx,
            as_of,
            &[MetricId::Cs01Hazard, MetricId::BucketedCs01Hazard],
            crate::test_support::credit::pricing_options(),
        )
        .unwrap();

    let cs01 = *result.measures.get("cs01_hazard").expect("cs01 present");
    let bucketed = *result
        .measures
        .get("bucketed_cs01_hazard")
        .expect("bucketed_cs01 present");
    assert!(
        cs01.is_finite() && bucketed.is_finite(),
        "CS01 metrics must be finite (cs01={cs01}, bucketed={bucketed})"
    );
    relative_eq(
        bucketed,
        cs01,
        0.02,
        "BucketedCs01 total vs parallel Cs01 (constituents)",
    );

    let series_sum: f64 = result
        .measures
        .iter()
        .filter(|(k, _)| k.as_str().starts_with("bucketed_cs01_hazard::"))
        .map(|(_, v)| *v)
        .sum();
    relative_eq(
        series_sum,
        cs01,
        0.02,
        "per-tenor series vs parallel Cs01 (constituents)",
    );
}

#[test]
fn bucketed_hazard_cs01_reports_each_distinct_constituent_curve() {
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;
    let index = standard_constituents_index("CDX-BKT-DISTINCT", start, end, 10_000_000.0, 2);
    let market = MarketContext::new()
        .insert(flat_discount_curve("USD-OIS", as_of, 0.03))
        .insert(flat_hazard_curve(
            "HZ-INDEX",
            as_of,
            STANDARD_RECOVERY_SENIOR,
            STANDARD_HAZARD_RATE,
        ))
        .insert(flat_hazard_curve(
            "HZ1",
            as_of,
            STANDARD_RECOVERY_SENIOR,
            0.01,
        ))
        .insert(flat_hazard_curve(
            "HZ2",
            as_of,
            STANDARD_RECOVERY_SENIOR,
            0.03,
        ));
    let mut config = FinstackConfig::default();
    config
        .extensions
        .insert(
            "valuations.sensitivities.v1",
            json!({"cs01_buckets_years": [1.0, 5.0, 10.0]}),
        )
        .expect("valid sensitivity configuration");

    let result = index
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::Cs01Hazard, MetricId::BucketedCs01Hazard],
            crate::test_support::credit::pricing_options().with_config(&config),
        )
        .expect("constituent hazard CS01 should compute");

    let parallel = result.measures[MetricId::Cs01Hazard.as_str()];
    let bucketed = result.measures[MetricId::BucketedCs01Hazard.as_str()];
    let curve_total = |curve_id: &str| {
        [1, 5, 10]
            .into_iter()
            .map(|tenor| {
                let key = format!("bucketed_cs01_hazard::{curve_id}::{tenor}y");
                *result
                    .measures
                    .get(key.as_str())
                    .unwrap_or_else(|| panic!("missing constituent bucket {key}"))
            })
            .sum::<f64>()
    };
    let hz1_total = curve_total("HZ1");
    let hz2_total = curve_total("HZ2");

    assert!(
        hz1_total.is_finite() && hz1_total.abs() > 1.0,
        "HZ1 bucketed hazard CS01 must be finite and non-zero: {hz1_total}"
    );
    assert!(
        hz2_total.is_finite() && hz2_total.abs() > 1.0,
        "HZ2 bucketed hazard CS01 must be finite and non-zero: {hz2_total}"
    );
    assert!(
        (hz1_total - hz2_total).abs() > 1.0,
        "distinct constituent curves must retain distinct bucket values: \
         HZ1={hz1_total}, HZ2={hz2_total}"
    );
    relative_eq(
        bucketed,
        hz1_total + hz2_total,
        1e-12,
        "bucketed aggregate vs constituent curve totals",
    );
    relative_eq(
        bucketed,
        parallel,
        0.02,
        "bucketed constituent hazard CS01 vs parallel hazard CS01",
    );
}

#[test]
fn bucketed_hazard_cs01_uses_each_non_aligned_curve_node_once() {
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;
    let index = standard_constituents_index("CDX-BKT-NON-ALIGNED", start, end, 10_000_000.0, 1);
    let constituent_hazard =
        finstack_quant_core::market_data::term_structures::HazardCurve::builder("HZ1")
            .base_date(as_of)
            .recovery_rate(STANDARD_RECOVERY_SENIOR)
            .knots([(0.75, 0.01), (2.5, 0.0175), (4.5, 0.025)])
            .build()
            .expect("non-aligned constituent hazard curve");
    let market = MarketContext::new()
        .insert(flat_discount_curve("USD-OIS", as_of, 0.03))
        .insert(flat_hazard_curve(
            "HZ-INDEX",
            as_of,
            STANDARD_RECOVERY_SENIOR,
            STANDARD_HAZARD_RATE,
        ))
        .insert(constituent_hazard);

    let result = index
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::Cs01Hazard, MetricId::BucketedCs01Hazard],
            crate::test_support::credit::pricing_options(),
        )
        .expect("non-aligned constituent hazard CS01 should compute");
    let constituent_buckets: Vec<_> = result
        .measures
        .iter()
        .filter(|(key, _)| key.as_str().starts_with("bucketed_cs01_hazard::HZ1::"))
        .collect();
    let bucket_sum: f64 = constituent_buckets.iter().map(|(_, value)| **value).sum();

    assert_eq!(
        constituent_buckets.len(),
        3,
        "three hazard nodes must produce exactly three effective buckets: {constituent_buckets:?}"
    );
    relative_eq(
        result.measures[MetricId::BucketedCs01Hazard.as_str()],
        bucket_sum,
        1e-12,
        "non-aligned bucket aggregate vs series",
    );
    relative_eq(
        bucket_sum,
        result.measures[MetricId::Cs01Hazard.as_str()],
        0.02,
        "non-aligned hazard nodes vs parallel hazard CS01",
    );
}

#[test]
fn bucketed_hazard_cs01_single_node_is_not_repeated() {
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;
    let index = standard_single_curve_index("CDX-BKT-SINGLE-NODE", start, end, 10_000_000.0);
    let single_node =
        finstack_quant_core::market_data::term_structures::HazardCurve::builder("HZ-INDEX")
            .base_date(as_of)
            .recovery_rate(STANDARD_RECOVERY_SENIOR)
            .knots([(5.0, 0.02)])
            .build()
            .expect("single-node hazard curve");
    let market = MarketContext::new()
        .insert(flat_discount_curve("USD-OIS", as_of, 0.03))
        .insert(single_node);

    let result = index
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::Cs01Hazard, MetricId::BucketedCs01Hazard],
            crate::test_support::credit::pricing_options(),
        )
        .expect("single-node hazard CS01 should compute");
    let buckets: Vec<_> = result
        .measures
        .iter()
        .filter(|(key, _)| key.as_str().starts_with("bucketed_cs01_hazard::HZ-INDEX::"))
        .collect();

    assert_eq!(
        buckets.len(),
        1,
        "one hazard node must produce one bucket, not repeated parallel bumps: {buckets:?}"
    );
    relative_eq(
        result.measures[MetricId::BucketedCs01Hazard.as_str()],
        result.measures[MetricId::Cs01Hazard.as_str()],
        1e-12,
        "single-node bucketed vs parallel hazard CS01",
    );
}

#[test]
fn constituent_using_index_curve_id_is_included_once() {
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;
    let mut constituents = equal_weight_constituents(2);
    constituents[0].credit.credit_curve_id = "HZ-INDEX".into();
    let index = standard_single_curve_index("CDX-SHARED-CURVE", start, end, 10_000_000.0)
        .with_constituents(constituents);
    let market = MarketContext::new()
        .insert(flat_discount_curve("USD-OIS", as_of, 0.03))
        .insert(flat_hazard_curve(
            "HZ-INDEX",
            as_of,
            STANDARD_RECOVERY_SENIOR,
            0.0125,
        ))
        .insert(flat_hazard_curve(
            "HZ2",
            as_of,
            STANDARD_RECOVERY_SENIOR,
            0.0275,
        ));

    let result = index
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::Cs01Hazard, MetricId::BucketedCs01Hazard],
            crate::test_support::credit::pricing_options(),
        )
        .expect("shared index/constituent hazard curve risk should compute");
    let shared_buckets: Vec<_> = result
        .measures
        .iter()
        .filter(|(key, _)| key.as_str().starts_with("bucketed_cs01_hazard::HZ-INDEX::"))
        .collect();
    let other_buckets: Vec<_> = result
        .measures
        .iter()
        .filter(|(key, _)| key.as_str().starts_with("bucketed_cs01_hazard::HZ2::"))
        .collect();
    let bucket_sum: f64 = shared_buckets
        .iter()
        .chain(&other_buckets)
        .map(|(_, value)| **value)
        .sum();

    assert_eq!(
        shared_buckets.len(),
        3,
        "the shared index-level curve must appear once per effective node"
    );
    assert_eq!(other_buckets.len(), 3);
    assert!(shared_buckets.iter().any(|(_, value)| value.abs() > 1.0));
    relative_eq(
        bucket_sum,
        result.measures[MetricId::Cs01Hazard.as_str()],
        0.02,
        "unique constituent curve buckets vs parallel hazard CS01",
    );
}

#[test]
fn test_all_risk_metrics_together() {
    // Test: All risk metrics computed together
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-ALL-RISK", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);

    let metrics = vec![MetricId::RiskyPv01, MetricId::Cs01Hazard, MetricId::Dv01];

    let result = idx
        .price_with_metrics(
            &ctx,
            as_of,
            &metrics,
            crate::test_support::credit::pricing_options(),
        )
        .unwrap();

    assert!(result.measures.contains_key("risky_pv01"));
    assert!(result.measures.contains_key("cs01_hazard"));
    assert!(result.measures.contains_key("dv01"));
}

#[test]
fn test_dv01_reasonable_magnitude() {
    // Test: DV01 has reasonable magnitude
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-DV01", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);

    let result = idx
        .price_with_metrics(
            &ctx,
            as_of,
            &[MetricId::Dv01],
            crate::test_support::credit::pricing_options(),
        )
        .unwrap();
    let dv01 = *result.measures.get("dv01").unwrap();

    // DV01 computed via bump-and-reprice; magnitude should be meaningful but not a simple closed-form
    assert!(dv01.is_finite(), "DV01 should be finite");
    // DV01 can be small for credit instruments where protection leg dominates premium leg
    assert!(
        dv01.abs() > 1.0,
        "DV01 magnitude should be non-trivial for $10MM notional"
    );
}

#[test]
fn test_risk_metrics_finite() {
    // Test: All risk metrics are finite
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-FINITE", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);

    let metrics = vec![MetricId::RiskyPv01, MetricId::Cs01Hazard, MetricId::Dv01];

    let result = idx
        .price_with_metrics(
            &ctx,
            as_of,
            &metrics,
            crate::test_support::credit::pricing_options(),
        )
        .unwrap();

    for (name, value) in &result.measures {
        assert!(
            value.is_finite(),
            "Risk metric '{}' is not finite: {}",
            name,
            value
        );
    }
}

// Recovery01 and Cs01Hazard
//
// Both are registered on the CDS Index metric calculator but were previously
// unexercised. Recovery01 is the PV sensitivity to a +1% recovery-rate bump;
// Cs01Hazard is the central-difference sensitivity to a direct parallel hazard
// shift (an alternative to the par-spread-rebootstrap `Cs01`). These tests
// guard against either metric silently regressing to zero/NaN or losing its
// linearity in notional.

#[test]
fn test_recovery01_finite_and_nonzero() {
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;
    let ctx = standard_market_context(as_of);
    let idx = standard_single_curve_index("CDX-REC01", start, end, 10_000_000.0);

    let result = idx
        .price_with_metrics(
            &ctx,
            as_of,
            &[MetricId::Recovery01],
            crate::test_support::credit::pricing_options(),
        )
        .unwrap();
    let recovery01 = *result.measures.get("recovery_01").unwrap();

    assert!(
        recovery01.is_finite(),
        "Recovery01 should be finite, got {}",
        recovery01
    );
    assert!(
        recovery01.abs() > 0.0,
        "Recovery01 should be non-zero for a live index, got {}",
        recovery01
    );
}

#[test]
fn test_recovery01_scales_with_notional() {
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;
    let ctx = standard_market_context(as_of);

    let idx_10mm = standard_single_curve_index("CDX-REC01-10", start, end, 10_000_000.0);
    let idx_20mm = standard_single_curve_index("CDX-REC01-20", start, end, 20_000_000.0);

    let rec01_10mm = *idx_10mm
        .price_with_metrics(
            &ctx,
            as_of,
            &[MetricId::Recovery01],
            crate::test_support::credit::pricing_options(),
        )
        .unwrap()
        .measures
        .get("recovery_01")
        .unwrap();
    let rec01_20mm = *idx_20mm
        .price_with_metrics(
            &ctx,
            as_of,
            &[MetricId::Recovery01],
            crate::test_support::credit::pricing_options(),
        )
        .unwrap()
        .measures
        .get("recovery_01")
        .unwrap();

    assert_linear_scaling(
        rec01_10mm,
        10_000_000.0,
        rec01_20mm,
        20_000_000.0,
        "Recovery01",
        0.05,
    );
}

#[test]
fn test_cs01_hazard_is_finite_and_nonzero() {
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;
    let ctx = standard_market_context(as_of);
    let idx = standard_single_curve_index("CDX-CS01H", start, end, 10_000_000.0);

    let result = idx
        .price_with_metrics(
            &ctx,
            as_of,
            &[MetricId::Cs01Hazard],
            crate::test_support::credit::pricing_options(),
        )
        .unwrap();
    let cs01_hazard = *result.measures.get("cs01_hazard").unwrap();

    assert!(
        cs01_hazard.is_finite(),
        "Cs01Hazard should be finite, got {}",
        cs01_hazard
    );
    assert!(
        cs01_hazard.abs() > 0.0,
        "Cs01Hazard should be non-zero for a live index, got {}",
        cs01_hazard
    );
}

#[test]
fn test_cs01_hazard_scales_with_notional() {
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;
    let ctx = standard_market_context(as_of);

    let idx_10mm = standard_single_curve_index("CDX-CS01H-10", start, end, 10_000_000.0);
    let idx_20mm = standard_single_curve_index("CDX-CS01H-20", start, end, 20_000_000.0);

    let cs01h_10mm = *idx_10mm
        .price_with_metrics(
            &ctx,
            as_of,
            &[MetricId::Cs01Hazard],
            crate::test_support::credit::pricing_options(),
        )
        .unwrap()
        .measures
        .get("cs01_hazard")
        .unwrap();
    let cs01h_20mm = *idx_20mm
        .price_with_metrics(
            &ctx,
            as_of,
            &[MetricId::Cs01Hazard],
            crate::test_support::credit::pricing_options(),
        )
        .unwrap()
        .measures
        .get("cs01_hazard")
        .unwrap();

    assert_linear_scaling(
        cs01h_10mm,
        10_000_000.0,
        cs01h_20mm,
        20_000_000.0,
        "Cs01Hazard",
        0.05,
    );
}
