//! Tests for BaseCorrelationCurve functionality.
//!
//! This module covers:
//! - Builder validation and construction
//! - Serialization roundtrips

use finstack_quant_core::market_data::term_structures::BaseCorrelationCurve;
use time::{Date, Month};

fn _test_date() -> Date {
    Date::from_calendar_date(2025, Month::January, 15).unwrap()
}

// Serialization Tests

mod serde_tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let curve = BaseCorrelationCurve::builder("CDX")
            .knots([(3.0, 0.25), (7.0, 0.45), (10.0, 0.60)])
            .build()
            .unwrap();

        let json = serde_json::to_string_pretty(&curve).unwrap();
        let deserialized: BaseCorrelationCurve = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id(), curve.id());
        assert_eq!(deserialized.detachment_points(), curve.detachment_points());
        assert_eq!(deserialized.correlations(), curve.correlations());
    }
}

#[test]
fn builder_rejects_non_monotonic_without_explicit_opt_in() {
    let result = BaseCorrelationCurve::builder("CDX")
        .knots([(3.0, 0.50), (7.0, 0.40), (10.0, 0.60)])
        .build();

    assert!(
        result.is_err(),
        "base-correlation builder should reject non-monotonic curves by default"
    );
}

#[test]
fn builder_rejects_correlation_outside_unit_interval() {
    let result = BaseCorrelationCurve::builder("CDX")
        .knots([(3.0, 0.25), (7.0, 1.20)])
        .build();

    assert!(
        result.is_err(),
        "base-correlation builder should reject correlations outside [0, 1]"
    );
}

#[test]
fn bucket_bump_filters_and_clamps_correlations() {
    let curve = BaseCorrelationCurve::builder("CDX")
        .knots([(3.0, 0.25), (7.0, 0.45), (10.0, 0.60)])
        .build()
        .unwrap();

    let bumped = curve
        .apply_bucket_bump(Some(&[7.0]), 0.75)
        .expect("filtered bucket bump should rebuild the curve");

    assert_eq!(bumped.correlations()[0], 0.25);
    assert_eq!(bumped.correlations()[1], 1.0);
    assert_eq!(bumped.correlations()[2], 0.60);
    assert!(
        !bumped.validate_arbitrage_free().is_arbitrage_free,
        "single-bucket stress can intentionally break monotonicity"
    );
}
