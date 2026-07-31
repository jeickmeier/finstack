//! Generate checked-in JSON Schemas owned by the statements crate.

use std::path::{Path, PathBuf};

use finstack_quant_statements::evaluator::StatementResult;
use finstack_quant_statements::schema::generated_schema;
use finstack_quant_statements::FinancialModelSpec;

fn schemas_dir() -> PathBuf {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by Cargo");
    Path::new(&manifest_dir)
        .join("schemas")
        .join("statements")
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
    write_schema::<FinancialModelSpec>(
        "financial_model_spec.schema.json",
        "FinancialModelSpec",
        "Versioned financial statement model specification.",
    );
    write_schema::<StatementResult>(
        "statement_result.schema.json",
        "StatementResult",
        "Versioned financial statement evaluation result.",
    );
}
