//! Generate checked-in JSON Schemas owned by the portfolio crate.

use std::path::{Path, PathBuf};

use finstack_quant_core::schema::{generated_schema, write_schema};
use finstack_quant_portfolio::PortfolioMaterializationEnvelope;

const PORTFOLIO_SCHEMA_BASE: &str = "https://finstack_quant.dev/schemas/portfolio/1/";

fn schemas_dir() -> PathBuf {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by Cargo");
    Path::new(&manifest_dir)
        .join("schemas")
        .join("portfolio")
        .join("1")
}

fn main() {
    let filename = "portfolio_materialization.schema.json";
    let path = schemas_dir().join(filename);
    let schema = generated_schema::<PortfolioMaterializationEnvelope>(
        PORTFOLIO_SCHEMA_BASE,
        filename,
        "Portfolio Materialization Envelope",
        "Strict, versioned, content-addressed portfolio materialization bundle.",
    )
    .unwrap_or_else(|error| panic!("generate portfolio materialization schema: {error}"));
    write_schema(&path, &schema)
        .unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
    println!("updated {}", path.display());
}
