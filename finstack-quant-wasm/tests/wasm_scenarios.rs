//! wasm-bindgen-test suite for `api::scenarios`.
//!
//! Covers list_builtin_templates, list_template_components,
//! apply_scenario, and apply_scenario_to_market which return JsValue.

#![cfg(target_arch = "wasm32")]

use finstack_quant_wasm::api::scenarios::*;
use wasm_bindgen_test::*;

fn empty_market_json() -> String {
    let ctx = finstack_quant_core::market_data::context::MarketContext::new();
    serde_json::to_string(&ctx).unwrap()
}

fn empty_model_json() -> String {
    let model = finstack_quant_statements::FinancialModelSpec::new("test", vec![]);
    serde_json::to_string(&model).unwrap()
}

#[wasm_bindgen_test]
fn list_builtin_templates_returns_array() {
    let result = list_builtin_templates().unwrap();
    let ids: Vec<String> = serde_wasm_bindgen::from_value(result).unwrap();
    assert!(!ids.is_empty());
}

#[wasm_bindgen_test]
fn list_template_components_for_gfc() {
    let result = list_template_components("gfc_2008").unwrap();
    let ids: Vec<String> = serde_wasm_bindgen::from_value(result).unwrap();
    assert!(!ids.is_empty());
}

#[wasm_bindgen_test]
fn apply_scenario_empty_spec() {
    let scenario = build_scenario_spec("test", "[]", None, None, None, None).unwrap();
    let market = empty_market_json();
    let model = empty_model_json();
    let result = apply_scenario(&scenario, &market, &model, "2024-01-15").unwrap();
    let obj: serde_json::Value = serde_wasm_bindgen::from_value(result).unwrap();
    // `market`/`model` are nested objects now, not serialized strings: the
    // envelope used to hand back JSON-inside-JSON.
    assert!(
        obj["market"].is_object(),
        "market should be a nested object"
    );
    assert!(obj["model"].is_object(), "model should be a nested object");
    assert_eq!(obj["operations_applied"].as_u64().unwrap(), 0);
}

#[wasm_bindgen_test]
fn apply_scenario_to_market_empty_spec() {
    let scenario = build_scenario_spec("test", "[]", None, None, None, None).unwrap();
    let market = empty_market_json();
    let result = apply_scenario_to_market(&scenario, &market, "2024-06-01").unwrap();
    let obj: serde_json::Value = serde_wasm_bindgen::from_value(result).unwrap();
    assert!(
        obj["market"].is_object(),
        "market should be a nested object"
    );
    assert!(obj["model"].is_null(), "no model was supplied");
    assert_eq!(obj["operations_applied"].as_u64().unwrap(), 0);
}

#[wasm_bindgen_test]
fn build_scenario_spec_preserves_cumulative_resolution_mode() {
    let scenario = build_scenario_spec(
        "cumulative",
        "[]",
        None,
        None,
        None,
        Some("cumulative".to_string()),
    )
    .unwrap();
    let value: serde_json::Value = serde_json::from_str(&scenario).unwrap();
    assert_eq!(value["resolution_mode"], "cumulative");
}
