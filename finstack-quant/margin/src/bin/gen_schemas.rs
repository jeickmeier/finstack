//! Generate the checked-in JSON Schema owned by the margin crate.

use std::path::{Path, PathBuf};

use finstack_quant_margin::schema::{generated_margin_schema, MARGIN_SCHEMA_FILENAME};

fn schema_path() -> PathBuf {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by Cargo");
    Path::new(&manifest_dir)
        .join("schemas")
        .join("margin")
        .join("1")
        .join(MARGIN_SCHEMA_FILENAME)
}

fn main() {
    let path = schema_path();
    let directory = path
        .parent()
        .unwrap_or_else(|| panic!("{} must have a parent directory", path.display()));
    std::fs::create_dir_all(directory)
        .unwrap_or_else(|error| panic!("create {}: {error}", directory.display()));
    let generated =
        generated_margin_schema().unwrap_or_else(|error| panic!("generate margin schema: {error}"));
    let json = serde_json::to_string_pretty(&generated)
        .unwrap_or_else(|error| panic!("encode margin schema: {error}"));
    std::fs::write(&path, json + "\n")
        .unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
    println!("updated {}", path.display());
}
