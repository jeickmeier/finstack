//! JSON Schema contract tests for portfolio materialization bundles.

use std::fs;
use std::path::{Path, PathBuf};

use finstack_quant_portfolio::materialization::PortfolioMaterializationEnvelope;
use serde_json::{json, Value};

const INSTRUMENT_SCHEMA_URI: &str =
    "https://finstack_quant.dev/schemas/instrument/1/instrument.schema.json";

fn collect_schema_files(directory: &Path, paths: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("read schema directory {}: {error}", directory.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|error| {
                    panic!("read schema entry in {}: {error}", directory.display())
                })
                .path()
        })
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_schema_files(&path, paths);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
            paths.push(path);
        }
    }
}

fn external_schema_resources() -> Vec<(String, jsonschema::Resource)> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let roots = [
        manifest_dir.join("../cashflows/schemas/cashflow/1"),
        manifest_dir.join("../valuations/schemas/common/1"),
        manifest_dir.join("../valuations/schemas/instruments/1"),
    ];
    let mut paths = Vec::new();
    for root in roots {
        collect_schema_files(&root, &mut paths);
    }
    paths
        .into_iter()
        .map(|path| {
            let raw = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("read schema {}: {error}", path.display()));
            let schema: Value = serde_json::from_str(&raw)
                .unwrap_or_else(|error| panic!("parse schema {}: {error}", path.display()));
            let id = schema
                .get("$id")
                .and_then(Value::as_str)
                .unwrap_or_else(|| panic!("schema {} is missing $id", path.display()))
                .to_string();
            let resource = jsonschema::Resource::from_contents(schema)
                .unwrap_or_else(|error| panic!("build resource {}: {error}", path.display()));
            (id, resource)
        })
        .collect()
}

fn checked_in_schema() -> Value {
    serde_json::from_str(include_str!(
        "../schemas/portfolio/1/portfolio_materialization.schema.json"
    ))
    .expect("checked-in portfolio materialization schema parses")
}

fn deposit_envelope() -> Value {
    serde_json::from_str(include_str!(
        "../../valuations/tests/instruments/json_examples/deposit.json"
    ))
    .expect("canonical deposit example parses")
}

fn materialization_fixture(instrument_envelope: Value) -> Value {
    json!({
        "schema": "finstack_quant.portfolio_materialization/1",
        "portfolio": {
            "id": "schema-test",
            "base_ccy": "USD",
            "as_of": "2025-01-01",
            "entities": {}
        },
        "instruments": [{
            "artifact_id": "deposit-artifact",
            "envelope": instrument_envelope
        }],
        "positions": []
    })
}

fn validation_errors(instance: &Value) -> Vec<String> {
    let schema = checked_in_schema();
    let validator = jsonschema::options()
        .with_resources(external_schema_resources().into_iter())
        .build(&schema)
        .expect("portfolio schema and canonical instrument resources compile");
    validator
        .iter_errors(instance)
        .map(|error| error.to_string())
        .collect()
}

#[test]
fn raw_value_runtime_still_accepts_schema_invalid_instrument_content() {
    let malformed = materialization_fixture(json!({
        "schema": "finstack_quant.instrument/1",
        "instrument": {
            "type": "not_a_real_instrument",
            "spec": {}
        }
    }));

    serde_json::from_value::<PortfolioMaterializationEnvelope>(malformed)
        .expect("outer parse must preserve deferred RawValue instrument decoding");
}

#[test]
fn checked_in_schema_references_and_validates_canonical_instruments() {
    let schema = checked_in_schema();
    assert_eq!(
        schema.pointer("/$defs/InstrumentArtifact/properties/envelope/$ref"),
        Some(&Value::String(INSTRUMENT_SCHEMA_URI.to_string()))
    );

    let valid = materialization_fixture(deposit_envelope());
    assert!(
        validation_errors(&valid).is_empty(),
        "representative deposit envelope must satisfy the materialization schema"
    );

    let mut malformed_type = valid.clone();
    malformed_type["instruments"][0]["envelope"]["instrument"]["type"] =
        json!("not_a_real_instrument");
    assert!(
        !validation_errors(&malformed_type).is_empty(),
        "unknown instrument type must be rejected"
    );

    let mut malformed_spec = valid;
    malformed_spec["instruments"][0]["envelope"]["instrument"]["spec"] = json!({});
    assert!(
        !validation_errors(&malformed_spec).is_empty(),
        "malformed typed instrument spec must be rejected"
    );
}
