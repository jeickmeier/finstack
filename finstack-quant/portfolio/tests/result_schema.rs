//! JSON Schema coverage for maintained portfolio result outputs.

use std::collections::BTreeSet;

use finstack_quant_portfolio::optimization::PortfolioOptimizationResult;
use finstack_quant_portfolio::PortfolioResult;
use serde_json::Value;

fn schema_value<T: schemars::JsonSchema>() -> Value {
    serde_json::to_value(schemars::schema_for!(T)).expect("schema serializes")
}

fn property_names(schema: &Value) -> BTreeSet<&str> {
    schema["properties"]
        .as_object()
        .expect("root schema has properties")
        .keys()
        .map(String::as_str)
        .collect()
}

#[test]
fn portfolio_result_schema_covers_nested_wire_shape() {
    let schema = schema_value::<PortfolioResult>();

    assert_eq!(
        property_names(&schema),
        BTreeSet::from(["schema_version", "valuation", "metrics", "meta"])
    );
    let rendered = serde_json::to_string(&schema).expect("schema renders");
    for nested_field in [
        "position_values",
        "aggregated",
        "by_position",
        "numeric_mode",
    ] {
        assert!(
            rendered.contains(&format!("\"{nested_field}\"")),
            "schema must describe nested field {nested_field}"
        );
    }
}

#[test]
fn optimization_result_schema_matches_manual_serialization() {
    let schema = schema_value::<PortfolioOptimizationResult>();

    assert_eq!(
        property_names(&schema),
        BTreeSet::from([
            "schema_version",
            "status",
            "status_label",
            "is_feasible",
            "objective_value",
            "turnover",
            "optimal_weights",
            "current_weights",
            "weight_deltas",
            "implied_quantities",
            "metric_values",
            "trades",
            "dual_values",
            "constraint_slacks",
            "binding_constraints",
            "label",
        ])
    );
    assert!(schema["required"]
        .as_array()
        .expect("required is an array")
        .iter()
        .any(|field| field == "schema_version"));
    assert_eq!(schema["properties"]["schema_version"]["const"], 1);
    assert!(schema
        .get("properties")
        .and_then(|properties| properties.get("problem"))
        .is_none());
}
