//! Generate checked-in JSON Schemas owned by the portfolio crate.

use std::path::Path;

use finstack_quant_core::schema::{
    externalize_schema_definitions, run_schema_generator, ExternalSchemaDefinition, SchemaArtifact,
    SchemaGenerationCommand,
};
use finstack_quant_core::Result;
use finstack_quant_portfolio::{
    optimization::PortfolioOptimizationResultWire, PortfolioMaterializationEnvelope,
};
use finstack_quant_valuations::instruments::InstrumentEnvelope;
use serde_json::Value;

const INSTRUMENT_SCHEMA_URI: &str =
    "https://finstack_quant.dev/schemas/instrument/1/instrument.schema.json";

const EXTERNAL_DEFINITIONS: &[ExternalSchemaDefinition] =
    &[ExternalSchemaDefinition::new::<InstrumentEnvelope>(
        "InstrumentEnvelope",
        INSTRUMENT_SCHEMA_URI,
    )];

fn package_materialization_schema(schema: &mut Value) -> Result<()> {
    externalize_schema_definitions(schema, EXTERNAL_DEFINITIONS)
}

const ARTIFACTS: &[SchemaArtifact] = &[
    SchemaArtifact::new::<PortfolioMaterializationEnvelope>(
        "schemas/portfolio/1/portfolio_materialization.schema.json",
        "https://finstack_quant.dev/schemas/portfolio/1/portfolio_materialization.schema.json",
        "Portfolio Materialization Envelope",
        "Strict, versioned, content-addressed portfolio materialization bundle.",
    )
    .with_packager(package_materialization_schema),
    SchemaArtifact::new::<PortfolioOptimizationResultWire>(
        "schemas/portfolio/1/portfolio_optimization_result.schema.json",
        "https://finstack_quant.dev/schemas/portfolio/1/portfolio_optimization_result.schema.json",
        "Portfolio Optimization Result",
        "Canonical typed v1 output from portfolio optimization.",
    ),
];

fn main() {
    let command = SchemaGenerationCommand::from_env()
        .unwrap_or_else(|error| panic!("parse schema generator arguments: {error}"));
    run_schema_generator(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        Path::new("schemas/portfolio"),
        ARTIFACTS,
        &command,
    )
    .unwrap_or_else(|error| panic!("generate portfolio schemas: {error}"));
}
