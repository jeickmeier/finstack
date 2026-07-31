//! Generate checked-in JSON Schemas owned by the attribution crate.

use std::path::{Path, PathBuf};

use finstack_quant_attribution::schema::generated_schema;
use finstack_quant_attribution::{AttributionEnvelope, AttributionResultEnvelope};

fn schemas_dir() -> PathBuf {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by Cargo");
    Path::new(&manifest_dir)
        .join("schemas")
        .join("attribution")
        .join("1")
}

fn write_schema<T: schemars::JsonSchema>(filename: &str, title: &str, description: &str) {
    let directory = schemas_dir();
    std::fs::create_dir_all(&directory)
        .unwrap_or_else(|error| panic!("create {}: {error}", directory.display()));
    let path = directory.join(filename);
    let generated = generated_schema::<T>(filename, title, description)
        .unwrap_or_else(|error| panic!("generate {title} schema: {error}"));
    let json = serde_json::to_string_pretty(&generated)
        .unwrap_or_else(|error| panic!("encode {title} schema: {error}"));
    std::fs::write(&path, json + "\n")
        .unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
    println!("updated {}", path.display());
}

fn main() {
    write_schema::<AttributionEnvelope>(
        "attribution.schema.json",
        "Finstack Quant Attribution Specification",
        "Complete specification for P&L attribution runs with instrument and market snapshots",
    );
    write_schema::<AttributionResultEnvelope>(
        "attribution_result.schema.json",
        "Finstack Quant Attribution Result",
        "Complete result of a P&L attribution run including factor decomposition and metadata",
    );
}
