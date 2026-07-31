//! Generate checked-in JSON Schemas owned by the scenarios crate.

use std::path::{Path, PathBuf};

use finstack_quant_scenarios::schema::{
    generated_schema, SCENARIO_SCHEMA_DESCRIPTION, SCENARIO_SCHEMA_FILENAME, SCENARIO_SCHEMA_TITLE,
};
use finstack_quant_scenarios::ScenarioEnvelope;

fn schema_dir() -> PathBuf {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by Cargo");
    Path::new(&manifest_dir)
        .join("schemas")
        .join("scenarios")
        .join("1")
}

fn main() {
    let directory = schema_dir();
    std::fs::create_dir_all(&directory)
        .unwrap_or_else(|error| panic!("create {}: {error}", directory.display()));
    let path = directory.join(SCENARIO_SCHEMA_FILENAME);
    let generated = generated_schema::<ScenarioEnvelope>(
        SCENARIO_SCHEMA_FILENAME,
        SCENARIO_SCHEMA_TITLE,
        SCENARIO_SCHEMA_DESCRIPTION,
    )
    .unwrap_or_else(|error| panic!("generate scenario schema: {error}"));
    let json = serde_json::to_string_pretty(&generated)
        .unwrap_or_else(|error| panic!("encode scenario schema: {error}"));
    std::fs::write(&path, json + "\n")
        .unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
    println!("updated {}", path.display());
}
