//! wasm-bindgen-test suite for `api::portfolio`.
//!
//! Covers portfolio_result_get_metric and apply_scenario_and_revalue
//! which return JsValue.

#![cfg(target_arch = "wasm32")]

use finstack_quant_wasm::api::portfolio::*;
use finstack_quant_wasm::api::scenarios::build_scenario_spec;
use finstack_quant_wasm::utils::{contract_to_js_error, materialization_to_js_error};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

fn portfolio_spec_json() -> String {
    r#"{"id":"test_portfolio","name":"Test","base_currency":"USD","as_of":"2024-01-15","entities":{},"positions":[]}"#.to_string()
}

fn empty_market_json() -> String {
    let ctx = finstack_quant_core::market_data::context::MarketContext::new();
    serde_json::to_string(&ctx).unwrap()
}

#[wasm_bindgen_test]
fn portfolio_result_get_metric_returns_undefined_for_missing() {
    let spec = portfolio_spec_json();
    let market = empty_market_json();
    let valuation_json = value_portfolio(&spec, &market, false).unwrap();
    let result = finstack_quant_portfolio::results::PortfolioResult::new(
        serde_json::from_str(&valuation_json).unwrap(),
        Default::default(),
        Default::default(),
    );
    let result_json = serde_json::to_string(&result).unwrap();
    let v = portfolio_result_get_metric(&result_json, "nonexistent").unwrap();
    assert!(v == JsValue::UNDEFINED);
}

#[wasm_bindgen_test]
fn mo24_liquidity_estimators_return_none_for_missing_estimates() {
    assert_eq!(
        roll_effective_spread("[0.01]").unwrap(),
        None,
        "Roll estimator should map missing estimate to undefined"
    );
    assert_eq!(
        amihud_illiquidity("[0.01]", "[0.0]").unwrap(),
        None,
        "Amihud estimator should map missing estimate to undefined"
    );
    assert_eq!(
        kyle_lambda("[0.0]", "[0.01]", 100.0).unwrap(),
        None,
        "Kyle estimator should map missing estimate to undefined"
    );
}

#[wasm_bindgen_test]
fn kyle_lambda_calibrates_in_price_space() {
    let lambda = kyle_lambda("[100.0, 200.0]", "[0.01, -0.02]", 50.0)
        .unwrap()
        .expect("valid price-space inputs");
    assert!((lambda - 0.005).abs() < 1e-15);
}

#[wasm_bindgen_test]
fn apply_scenario_and_revalue_empty_portfolio() {
    let spec = portfolio_spec_json();
    let scenario = build_scenario_spec("stress", "[]", None, None, 0, None).unwrap();
    let market = empty_market_json();
    let result = apply_scenario_and_revalue(&spec, &scenario, &market).unwrap();
    let obj: serde_json::Value = serde_wasm_bindgen::from_value(result).unwrap();
    assert!(obj["valuation"].is_object());
    assert!(obj["report"].is_object());
}

#[wasm_bindgen_test]
fn apply_scenario_and_revalue_built_empty_portfolio() {
    let spec = portfolio_spec_json();
    let portfolio = JsPortfolio::from_spec(&spec).unwrap();
    let scenario = build_scenario_spec("stress", "[]", None, None, 0, None).unwrap();
    let market = empty_market_json();
    let result = apply_scenario_and_revalue_built(&portfolio, &scenario, &market).unwrap();
    let obj: serde_json::Value = serde_wasm_bindgen::from_value(result).unwrap();
    assert!(obj["valuation"].is_object());
    assert!(obj["report"].is_object());
}

#[wasm_bindgen_test]
fn contract_error_js_kinds_follow_variants_not_messages() {
    use finstack_quant_core::contract::{ContractError, ValidationReport};

    let cases = [
        (
            ContractError::UnsupportedVersion {
                contract: "missing curve".to_string(),
                found: 3,
                min: 1,
                max: 2,
            },
            "unsupported_version",
        ),
        (
            ContractError::MissingVersion {
                contract: "malformed validation".to_string(),
            },
            "missing_version",
        ),
        (
            ContractError::MalformedSchema {
                value: "missing curve".to_string(),
                expected: "not found".to_string(),
            },
            "malformed_schema",
        ),
        (
            ContractError::LimitExceeded {
                what: "malformed validation",
                found: 2,
                limit: 1,
            },
            "limit_exceeded",
        ),
        (
            ContractError::Report(Box::new(ValidationReport::default())),
            "report",
        ),
        (
            ContractError::Core(finstack_quant_core::Error::Internal(
                "missing curve malformed validation".to_string(),
            )),
            "core",
        ),
    ];

    for (error, expected) in cases {
        let js_error = contract_to_js_error(error);
        let kind = js_sys::Reflect::get(&js_error, &JsValue::from_str("kind"))
            .unwrap()
            .as_string()
            .unwrap();
        assert_eq!(kind, expected);
    }
}

#[wasm_bindgen_test]
fn materialization_limit_and_invalid_input_have_distinct_js_kinds() {
    let cases = [
        (
            finstack_quant_portfolio::Error::ContractLimitExceeded {
                what: "bytes".to_string(),
                found: 2,
                limit: 1,
            },
            "limit_exceeded",
        ),
        (
            finstack_quant_portfolio::Error::InvalidInput(
                "unrelated validation failure".to_string(),
            ),
            "invalid_input",
        ),
    ];

    for (error, expected) in cases {
        let js_error = materialization_to_js_error(error);
        let kind = js_sys::Reflect::get(&js_error, &JsValue::from_str("kind"))
            .unwrap()
            .as_string()
            .unwrap();
        assert_eq!(kind, expected);
    }
}
