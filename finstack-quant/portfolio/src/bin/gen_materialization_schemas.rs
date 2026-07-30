//! Generate checked-in JSON Schemas owned by the portfolio crate.

use std::path::{Path, PathBuf};

use finstack_quant_portfolio::PortfolioMaterializationEnvelope;
use serde_json::{Map, Value};

const JSON_SCHEMA_DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";
const PORTFOLIO_SCHEMA_BASE: &str = "https://finstack_quant.dev/schemas/portfolio/1/";

fn schemas_dir() -> PathBuf {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by Cargo");
    Path::new(&manifest_dir)
        .join("schemas")
        .join("portfolio")
        .join("1")
}

fn write_schema(filename: &str, title: &str, description: &str, generated: schemars::Schema) {
    let directory = schemas_dir();
    std::fs::create_dir_all(&directory)
        .unwrap_or_else(|error| panic!("create {}: {error}", directory.display()));
    let path = directory.join(filename);
    let generated =
        serde_json::to_value(generated).expect("serialize portfolio materialization schema");
    let generated = generated
        .as_object()
        .expect("generated portfolio schema must be an object");

    let mut output = Map::new();
    output.insert(
        "$id".to_string(),
        Value::String(format!("{PORTFOLIO_SCHEMA_BASE}{filename}")),
    );
    output.insert(
        "$schema".to_string(),
        Value::String(JSON_SCHEMA_DIALECT.to_string()),
    );
    output.insert("title".to_string(), Value::String(title.to_string()));
    output.insert(
        "description".to_string(),
        Value::String(description.to_string()),
    );
    for (key, value) in generated {
        if !matches!(key.as_str(), "$id" | "$schema" | "title" | "description") {
            output.insert(key.clone(), value.clone());
        }
    }

    let json = serde_json::to_string_pretty(&Value::Object(output))
        .expect("encode portfolio materialization schema");
    std::fs::write(&path, json + "\n")
        .unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
    println!("updated {}", path.display());
}

fn main() {
    write_schema(
        "portfolio_materialization.schema.json",
        "Portfolio Materialization Envelope",
        "Strict, versioned, content-addressed portfolio materialization bundle.",
        schemars::schema_for!(PortfolioMaterializationEnvelope),
    );
}
