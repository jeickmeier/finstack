use super::*;

#[test]
fn test_parse_strict_known_metric() {
    // Test lowercase
    let dv01 = MetricId::parse_strict("dv01").unwrap();
    assert_eq!(dv01, MetricId::Dv01);
    assert!(!dv01.is_custom());

    for noncanonical in ["THETA", "Cs01", " dv01", "dv-01"] {
        assert!(MetricId::parse_strict(noncanonical).is_err());
    }

    // Test various standard metrics
    let delta = MetricId::parse_strict("delta").unwrap();
    assert_eq!(delta, MetricId::Delta);

    let ytm = MetricId::parse_strict("ytm").unwrap();
    assert_eq!(ytm, MetricId::Ytm);

    let convexity = MetricId::parse_strict("convexity").unwrap();
    assert_eq!(convexity, MetricId::Convexity);
}

#[test]
fn test_parse_strict_unknown_metric() {
    // Unknown metric should fail
    let result = MetricId::parse_strict("dv01x");
    assert!(result.is_err());

    // Check error contains metric name
    let err = result.unwrap_err();
    let err_msg = format!("{}", err);
    assert!(err_msg.to_lowercase().contains("dv01x"));

    // Test other typos
    assert!(MetricId::parse_strict("theta2").is_err());
    assert!(MetricId::parse_strict("cs_01").is_err());
    assert!(MetricId::parse_strict("unknown_metric").is_err());
}

#[test]
fn test_parse_strict_error_includes_available_metrics() {
    let result = MetricId::parse_strict("invalid_metric");
    assert!(result.is_err());

    // The error should be UnknownMetric variant
    match result.unwrap_err() {
        finstack_quant_core::Error::UnknownMetric {
            metric_id,
            available,
        } => {
            assert_eq!(metric_id, "invalid_metric");
            // Should include standard metrics
            assert!(!available.is_empty());
            assert!(available.contains(&"dv01".to_string()));
            assert!(available.contains(&"theta".to_string()));
            assert!(available.contains(&"cs01".to_string()));
        }
        _ => panic!("Expected UnknownMetric error"),
    }
}

#[test]
fn test_from_str_still_permissive() {
    // FromStr should still accept unknown metrics
    let known = MetricId::from_str("dv01").unwrap();
    assert_eq!(known, MetricId::Dv01);
    assert!(!known.is_custom());

    // Unknown metric becomes custom (no error)
    let custom = MetricId::from_str("my_custom_metric").unwrap();
    assert!(custom.is_custom());
    assert_eq!(custom.as_str(), "my_custom_metric");

    // Another unknown metric
    let custom2 = MetricId::from_str("user_defined_123").unwrap();
    assert!(custom2.is_custom());
}

#[test]
fn test_parse_strict_vs_from_str_behavior() {
    // Known metric: both work the same
    let strict = MetricId::parse_strict("theta").unwrap();
    let permissive = MetricId::from_str("theta").unwrap();
    assert_eq!(strict, permissive);

    // Unknown metric: strict fails, permissive creates custom
    let strict_result = MetricId::parse_strict("custom_metric");
    assert!(strict_result.is_err());

    let permissive_result = MetricId::from_str("custom_metric").unwrap();
    assert!(permissive_result.is_custom());
}

#[test]
fn test_custom_metric_creation() {
    let custom = MetricId::custom("my_metric");
    assert!(custom.is_custom());
    assert_eq!(custom.as_str(), "my_metric");

    // Custom metrics not in ALL_STANDARD
    assert!(!MetricId::ALL_STANDARD.contains(&custom));
}

#[test]
fn test_all_standard_metrics_parseable_strict() {
    // Every standard metric should be parseable via parse_strict
    for metric in MetricId::ALL_STANDARD {
        let parsed = MetricId::parse_strict(metric.as_str()).unwrap();
        assert_eq!(&parsed, metric);
        assert!(!parsed.is_custom());
    }
}

#[test]
fn test_carry_decomposition_metrics_are_standard_and_parseable() {
    for name in [
        "carry_total",
        "coupon_income",
        "pull_to_par",
        "roll_down",
        "funding_cost",
    ] {
        assert!(MetricId::ALL_STANDARD
            .iter()
            .any(|metric| metric.as_str() == name));

        let parsed = MetricId::parse_strict(name).unwrap();
        assert_eq!(parsed.as_str(), name);
        assert!(!parsed.is_custom());
    }
}

#[test]
fn spread_equivalent_metrics_are_unique_and_standard() {
    let mut seen = std::collections::HashSet::new();
    for m in MetricId::SPREAD_EQUIVALENT_METRICS {
        assert!(
            seen.insert(m.as_str()),
            "duplicate spread-equivalent metric: {}",
            m.as_str()
        );
        assert!(
            !m.is_custom(),
            "spread-equivalent metric must be standard: {}",
            m.as_str()
        );
        assert!(
            MetricId::ALL_STANDARD.contains(m),
            "spread-equivalent metric missing from ALL_STANDARD: {}",
            m.as_str()
        );
    }
}

#[test]
fn test_cross_gamma_metric_ids_exist_and_parse() {
    let pairs = [
        (MetricId::CrossGammaRatesCredit, "cross_gamma_rates_credit"),
        (MetricId::CrossGammaRatesVol, "cross_gamma_rates_vol"),
        (MetricId::CrossGammaSpotVol, "cross_gamma_spot_vol"),
        (MetricId::CrossGammaSpotCredit, "cross_gamma_spot_credit"),
        (MetricId::CrossGammaFxVol, "cross_gamma_fx_vol"),
        (MetricId::CrossGammaFxRates, "cross_gamma_fx_rates"),
        (MetricId::CrossGammaCreditVol, "cross_gamma_credit_vol"),
    ];
    for (id, expected_str) in &pairs {
        assert_eq!(id.as_str(), *expected_str);
        let parsed = MetricId::parse_strict(expected_str).unwrap();
        assert_eq!(&parsed, id);
        assert!(!parsed.is_custom());
    }
}

#[test]
fn test_case_sensitive_canonical_ids() {
    let lower = MetricId::parse_strict("dv01").unwrap();
    assert_eq!(lower, MetricId::Dv01);
    assert!(MetricId::parse_strict("DV01").is_err());
    assert!(MetricId::parse_strict("Dv01").is_err());
}

#[test]
fn test_every_standard_metric_in_exactly_one_group() {
    let mut grouped: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for group in MetricGroup::ALL {
        for metric in group.metrics() {
            assert!(
                grouped.insert(metric.as_str()),
                "metric '{}' appears in multiple groups (duplicate found in {:?})",
                metric.as_str(),
                group,
            );
        }
    }
    for metric in MetricId::ALL_STANDARD {
        assert!(
            grouped.contains(metric.as_str()),
            "metric '{}' from ALL_STANDARD is not assigned to any MetricGroup",
            metric.as_str(),
        );
    }
}

#[test]
fn test_group_union_equals_all_standard() {
    let mut from_groups: Vec<&str> = MetricGroup::ALL
        .iter()
        .flat_map(|g| g.metrics().iter().map(|m| m.as_str()))
        .collect();
    from_groups.sort();
    let mut from_all: Vec<&str> = MetricId::ALL_STANDARD.iter().map(|m| m.as_str()).collect();
    from_all.sort();
    assert_eq!(
        from_groups, from_all,
        "union of all MetricGroup arrays must equal ALL_STANDARD"
    );
}

#[test]
fn composite_codec_round_trips_utf8_empty_and_reserved_components() {
    let key = MetricId::composite(&MetricId::BucketedDv01, &["USD-OIS", "10_y", "", "Δ"]);

    assert_eq!(
        key.as_str(),
        "bucketed_dv01::USD_x2dOIS::10_x5fy::_empty::_xce_x94"
    );
    assert_eq!(
        key.decode_components(&MetricId::BucketedDv01),
        Some(vec![
            "USD-OIS".to_string(),
            "10_y".to_string(),
            String::new(),
            "Δ".to_string(),
        ])
    );
}

#[test]
fn composite_codec_matches_only_the_exact_base_identifier() {
    let key = MetricId::composite(&MetricId::BucketedDv01, &["USD-OIS", "10y"]);

    assert_eq!(key.decode_components(&MetricId::Dv01), None);
    assert_eq!(
        MetricId::BucketedDv01.decode_components(&MetricId::BucketedDv01),
        None
    );
}

#[test]
fn composite_codec_preserves_legacy_and_malformed_escape_markers_literally() {
    for component in ["curve_xray", "curve_x", "curve_xg1", "curve_x2"] {
        let key = MetricId::custom(format!("bucketed_dv01::{component}"));
        assert_eq!(
            key.decode_components(&MetricId::BucketedDv01),
            Some(vec![component.to_string()])
        );
    }
}

#[test]
fn composite_codec_decodes_a_genuine_escaped_delimiter_component() {
    let key = MetricId::composite(&MetricId::BucketedDv01, &["USD::OIS"]);

    assert_eq!(key.as_str(), "bucketed_dv01::USD_x3a_x3aOIS");
    assert_eq!(
        key.decode_components(&MetricId::BucketedDv01),
        Some(vec!["USD::OIS".to_string()])
    );
}
