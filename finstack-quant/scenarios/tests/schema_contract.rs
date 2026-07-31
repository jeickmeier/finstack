//! JSON Schema contract tests for scenario envelopes.

use finstack_quant_scenarios::schema::{
    generated_schema as assemble_schema, SCENARIO_SCHEMA_BASE, SCENARIO_SCHEMA_DESCRIPTION,
    SCENARIO_SCHEMA_FILENAME, SCENARIO_SCHEMA_TITLE,
};
use finstack_quant_scenarios::ScenarioEnvelope;
use serde_json::{json, Value};

const CANONICAL_DECIMAL_PATTERN: &str = r"^-?\d+(\.\d+)?([eE][+-]?\d+)?$";

struct CanonicalPostprocessProbe;

impl schemars::JsonSchema for CanonicalPostprocessProbe {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("CanonicalPostprocessProbe")
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "object",
            "properties": {
                "trade_date": {"type": "string"},
                "amount": {
                    "type": "string",
                    "pattern": "^-?\\d+(\\.\\d+)?([eE]\\d+)?$",
                },
            },
        })
    }
}

fn generated_schema() -> Value {
    assemble_schema::<ScenarioEnvelope>(
        SCENARIO_SCHEMA_FILENAME,
        SCENARIO_SCHEMA_TITLE,
        SCENARIO_SCHEMA_DESCRIPTION,
    )
    .expect("scenario envelope schema generates")
}

fn checked_in_schema() -> Value {
    serde_json::from_str(include_str!("../schemas/scenarios/1/scenario.schema.json"))
        .expect("checked-in scenario schema parses")
}

fn validate(schema: &Value, fixture: &Value) -> Result<(), Vec<String>> {
    let validator = jsonschema::validator_for(schema).expect("scenario schema compiles");
    let errors: Vec<_> = validator
        .iter_errors(fixture)
        .map(|error| error.to_string())
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[test]
fn checked_in_envelope_schema_validates_canonical_fixture() {
    let fixture: Value = serde_json::from_str(include_str!("data/canonical/scenario.json"))
        .expect("canonical scenario fixture parses");

    validate(&checked_in_schema(), &fixture)
        .unwrap_or_else(|errors| panic!("canonical scenario failed validation: {errors:#?}"));
}

#[test]
fn checked_in_envelope_schema_rejects_malformed_nested_operation() {
    let mut fixture: Value = serde_json::from_str(include_str!("data/canonical/scenario.json"))
        .expect("canonical scenario fixture parses");
    fixture["scenario"]["operations"][0] = json!({
        "kind": "curve_parallel_bp",
        "curve_kind": "discount",
        "curve_id": "USD-OIS",
        "bp": "fifty"
    });

    assert!(
        validate(&checked_in_schema(), &fixture).is_err(),
        "typed scenario schema must reject an invalid nested operation"
    );
}

#[test]
fn checked_in_envelope_schema_rejects_invalid_instrument_type() {
    let mut fixture: Value = serde_json::from_str(include_str!("data/canonical/scenario.json"))
        .expect("canonical scenario fixture parses");
    fixture["scenario"]["operations"][0] = json!({
        "kind": "instrument_price_pct_by_type",
        "instrument_types": ["NotAnInstrumentType"],
        "pct": -5.0
    });

    assert!(
        validate(&checked_in_schema(), &fixture).is_err(),
        "typed scenario schema must reject an unknown instrument type"
    );
}

#[test]
fn checked_in_envelope_schema_accepts_serde_instrument_type_value() {
    let mut fixture: Value = serde_json::from_str(include_str!("data/canonical/scenario.json"))
        .expect("canonical scenario fixture parses");
    fixture["scenario"]["operations"][0] = json!({
        "kind": "instrument_price_pct_by_type",
        "instrument_types": ["Bond"],
        "pct": -5.0
    });

    validate(&checked_in_schema(), &fixture)
        .unwrap_or_else(|errors| panic!("serde instrument type failed validation: {errors:#?}"));
}

#[test]
fn checked_in_envelope_schema_rejects_wrong_contract_marker() {
    let mut fixture: Value = serde_json::from_str(include_str!("data/canonical/scenario.json"))
        .expect("canonical scenario fixture parses");
    fixture["schema"] = json!("finstack_quant.scenario/999");

    assert!(
        validate(&checked_in_schema(), &fixture).is_err(),
        "scenario schema must require the current contract marker"
    );
}

#[test]
fn checked_in_schema_matches_generated_type_and_metadata() {
    let schema = checked_in_schema();

    assert_eq!(schema, generated_schema());
    assert_eq!(
        schema["$id"],
        format!("{SCENARIO_SCHEMA_BASE}{SCENARIO_SCHEMA_FILENAME}")
    );
    assert_eq!(
        schema["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
}

#[test]
fn scenario_generator_applies_canonical_decimal_and_date_normalization() {
    let schema = assemble_schema::<CanonicalPostprocessProbe>(
        "canonical_probe.schema.json",
        "Canonical probe",
        "Exercises the scenario generator's canonical post-processing path.",
    )
    .expect("canonical probe schema generates");

    assert_eq!(
        schema.pointer("/properties/amount/pattern"),
        Some(&json!(CANONICAL_DECIMAL_PATTERN))
    );
    assert_eq!(
        schema.pointer("/properties/trade_date/format"),
        Some(&json!("date"))
    );
}

#[test]
fn checked_in_schema_keeps_meaningful_nested_validation() {
    let schema = checked_in_schema();

    assert!(schema.pointer("/$defs/ScenarioSpec").is_some());
    assert!(schema
        .pointer("/$defs/InstrumentType/oneOf")
        .and_then(Value::as_array)
        .is_some_and(|variants| variants.len() > 50));
    assert!(schema
        .pointer("/$defs/OperationSpec/oneOf")
        .and_then(Value::as_array)
        .is_some_and(|variants| variants.len() > 10));
    assert_eq!(
        schema.pointer("/properties/scenario/$ref"),
        Some(&json!("#/$defs/ScenarioSpec"))
    );
}
