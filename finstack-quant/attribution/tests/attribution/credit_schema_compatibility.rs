//! Schema validation tests for credit attribution payloads (PR-9).
//!
//! Named compatibility tests:
//!
//! 1. `attribution_result_without_credit_factor_detail_still_valid` — a
//!    current result without optional credit-factor details remains valid.
//! 2. `new_attribution_result_schema_accepts_credit_factor_detail` — a
//!    result payload with the new `credit_factor_detail` and
//!    `credit_carry_decomposition` fields validates.

use finstack_quant_attribution::{
    AttributionMethod, AttributionResult, AttributionResultEnvelope, PnlAttribution,
};
use finstack_quant_core::config::ResultsMeta;
use finstack_quant_core::currency::Currency;
use finstack_quant_core::money::Money;
use serde_json::Value;
use time::macros::date;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Load and parse the attribution result schema.
fn attribution_result_schema() -> Value {
    let content = include_str!("../../schemas/attribution/1/attribution_result.schema.json");
    serde_json::from_str(content).expect("attribution_result schema must be valid JSON")
}

/// Validate `instance` against `schema`, returning `Ok(())` or a descriptive error.
fn validate(instance: &Value, schema: &Value) -> Result<(), String> {
    let validator =
        jsonschema::validator_for(schema).map_err(|e| format!("Failed to compile schema: {e}"))?;
    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|e| {
            let path = e.instance_path.to_string();
            if path.is_empty() {
                e.to_string()
            } else {
                format!("{path}: {e}")
            }
        })
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Validation failed with {} error(s):\n  {}",
            errors.len(),
            errors.join("\n  ")
        ))
    }
}

fn merge_missing_fields(value: &mut Value, defaults: &Value) {
    if let (Some(value), Some(defaults)) = (value.as_object_mut(), defaults.as_object()) {
        for (key, default) in defaults {
            match value.get_mut(key) {
                Some(existing) => merge_missing_fields(existing, default),
                None => {
                    value.insert(key.clone(), default.clone());
                }
            }
        }
    }
}

fn with_current_required_fields(mut fixture: Value) -> Value {
    let attribution = PnlAttribution::new(
        Money::new(0.0, Currency::USD),
        "SCHEMA-COMPATIBILITY-FIXTURE",
        date!(2024 - 01 - 01),
        date!(2024 - 01 - 02),
        AttributionMethod::Parallel,
    );
    let defaults = serde_json::to_value(AttributionResultEnvelope::new(AttributionResult {
        attribution,
        results_meta: ResultsMeta::default(),
    }))
    .expect("canonical Rust result fixture serializes");
    merge_missing_fields(&mut fixture, &defaults);
    fixture
}

// ---------------------------------------------------------------------------
// Test 1: result without credit-factor detail still validates
// ---------------------------------------------------------------------------

/// A current attribution result without `credit_factor_detail` or
/// `credit_carry_decomposition` must validate against the result schema.
///
/// The fixture starts from the historical compact shape and fills fields added
/// since then from a canonical Rust-serialized result, avoiding hand-maintained
/// copies of rounding and result metadata defaults.
#[test]
fn attribution_result_without_credit_factor_detail_still_valid() {
    let schema = attribution_result_schema();

    // Compact historical payload with no optional credit-factor fields.
    let instance = with_current_required_fields(serde_json::json!({
        "schema": "finstack_quant.attribution/1",
        "result": {
            "attribution": {
                "total_pnl": { "amount": "1000.00", "currency": "USD" },
                "carry":           { "amount": "50.00", "currency": "USD" },
                "rates_curves_pnl": { "amount": "200.00", "currency": "USD" },
                "credit_curves_pnl": { "amount": "150.00", "currency": "USD" },
                "inflation_curves_pnl": { "amount": "0.00", "currency": "USD" },
                "correlations_pnl": { "amount": "0.00", "currency": "USD" },
                "fx_pnl":          { "amount": "100.00", "currency": "USD" },
                "vol_pnl":         { "amount": "0.00", "currency": "USD" },
                "model_params_pnl": { "amount": "0.00", "currency": "USD" },
                "market_scalars_pnl": { "amount": "0.00", "currency": "USD" },
                "residual":        { "amount": "500.00", "currency": "USD" },
                "meta": {
                    "method": "Parallel",
                    "t0": "2024-01-01",
                    "t1": "2024-01-02",
                    "instrument_id": "BOND-1",
                    "num_repricings": 0,
                    "tolerance_abs": 1.0,
                    "tolerance_pct": 0.01,
                    "residual_pct": 50.0,
                    "rounding": {}
                }
            },
            "results_meta": {}
        }
    }));

    validate(&instance, &schema).unwrap_or_else(|e| {
        panic!("Pre-feature attribution payload failed schema validation:\n{e}");
    });
}

// ---------------------------------------------------------------------------
// Test 2: new payload with credit_factor_detail validates
// ---------------------------------------------------------------------------

/// An attribution result payload that includes the new PR-9 fields
/// (`credit_factor_detail` and `credit_carry_decomposition`) must validate
/// against the extended schema.
///
/// This confirms the schema correctly describes the new fields.
#[test]
fn new_attribution_result_schema_accepts_credit_factor_detail() {
    let schema = attribution_result_schema();

    let instance = with_current_required_fields(serde_json::json!({
        "schema": "finstack_quant.attribution/1",
        "result": {
            "attribution": {
                "total_pnl": { "amount": "1000.00", "currency": "USD" },
                "carry":           { "amount": "50.00", "currency": "USD" },
                "rates_curves_pnl": { "amount": "200.00", "currency": "USD" },
                "credit_curves_pnl": { "amount": "150.00", "currency": "USD" },
                "inflation_curves_pnl": { "amount": "0.00", "currency": "USD" },
                "correlations_pnl": { "amount": "0.00", "currency": "USD" },
                "fx_pnl":          { "amount": "100.00", "currency": "USD" },
                "vol_pnl":         { "amount": "0.00", "currency": "USD" },
                "model_params_pnl": { "amount": "0.00", "currency": "USD" },
                "market_scalars_pnl": { "amount": "0.00", "currency": "USD" },
                "residual":        { "amount": "500.00", "currency": "USD" },
                // New PR-9 field: credit factor hierarchy decomposition
                "credit_factor_detail": {
                    "model_id": "2024-03-31/abcdef0123456789",
                    "generic_pnl": { "amount": "80.00", "currency": "USD" },
                    "levels": [
                        {
                            "level_name": "rating",
                            "total": { "amount": "40.00", "currency": "USD" },
                            "by_bucket": {
                                "IG": { "amount": "40.00", "currency": "USD" }
                            }
                        },
                        {
                            "level_name": "region",
                            "total": { "amount": "20.00", "currency": "USD" }
                        }
                    ],
                    "adder_pnl_total": { "amount": "10.00", "currency": "USD" },
                    "adder_pnl_by_issuer": {
                        "ISSUER-A": { "amount": "10.00", "currency": "USD" }
                    }
                },
                // New PR-9 field: credit carry decomposition
                "credit_carry_decomposition": {
                    "model_id": "2024-03-31/abcdef0123456789",
                    "rates_carry_total": { "amount": "30.00", "currency": "USD" },
                    "credit_carry_total": { "amount": "20.00", "currency": "USD" },
                    "credit_by_level": {
                        "generic": { "amount": "10.00", "currency": "USD" },
                        "levels": [
                            {
                                "level_name": "rating",
                                "total": { "amount": "8.00", "currency": "USD" }
                            }
                        ],
                        "adder_total": { "amount": "2.00", "currency": "USD" }
                    }
                },
                "meta": {
                    "method": "MetricsBased",
                    "t0": "2024-01-01",
                    "t1": "2024-01-02",
                    "instrument_id": "BOND-A",
                    "num_repricings": 0,
                    "tolerance_abs": 1.0,
                    "tolerance_pct": 0.01,
                    "residual_pct": 50.0,
                    "rounding": {}
                }
            },
            "results_meta": {}
        }
    }));

    validate(&instance, &schema).unwrap_or_else(|e| {
        panic!("New attribution payload with credit_factor_detail failed schema validation:\n{e}");
    });
}

// ---------------------------------------------------------------------------
// Bonus: carry_detail with legacy Money shape still validates
// ---------------------------------------------------------------------------

/// The extended schema allows `coupon_income` and `roll_down` in `carry_detail`
/// to be either a legacy bare `Money` object (`{amount, currency}`) or the new
/// `SourceLine` shape (`{total, ...}`). Verify the legacy shape is still accepted.
#[test]
fn attribution_result_carry_detail_accepts_legacy_money_shape() {
    let schema = attribution_result_schema();

    let instance = with_current_required_fields(serde_json::json!({
        "schema": "finstack_quant.attribution/1",
        "result": {
            "attribution": {
                "total_pnl": { "amount": "1000.00", "currency": "USD" },
                "carry":           { "amount": "50.00", "currency": "USD" },
                "rates_curves_pnl": { "amount": "200.00", "currency": "USD" },
                "credit_curves_pnl": { "amount": "150.00", "currency": "USD" },
                "inflation_curves_pnl": { "amount": "0.00", "currency": "USD" },
                "correlations_pnl": { "amount": "0.00", "currency": "USD" },
                "fx_pnl":          { "amount": "100.00", "currency": "USD" },
                "vol_pnl":         { "amount": "0.00", "currency": "USD" },
                "model_params_pnl": { "amount": "0.00", "currency": "USD" },
                "market_scalars_pnl": { "amount": "0.00", "currency": "USD" },
                "residual":        { "amount": "500.00", "currency": "USD" },
                // carry_detail with legacy bare-Money shape for coupon_income and roll_down
                "carry_detail": {
                    "total": { "amount": "50.00", "currency": "USD" },
                    "coupon_income": { "amount": "30.00", "currency": "USD" },
                    "roll_down": { "amount": "20.00", "currency": "USD" }
                },
                "meta": {
                    "method": "MetricsBased",
                    "t0": "2024-01-01",
                    "t1": "2024-01-02",
                    "instrument_id": "BOND-A",
                    "num_repricings": 0,
                    "tolerance_abs": 1.0,
                    "tolerance_pct": 0.01,
                    "residual_pct": 5.0,
                    "rounding": {}
                }
            },
            "results_meta": {}
        }
    }));

    validate(&instance, &schema).unwrap_or_else(|e| {
        panic!("Legacy Money shape in carry_detail failed schema validation:\n{e}");
    });
}
