//! Generate the checked-in JSON Schema owned by the margin crate.

use std::path::Path;

use finstack_quant_core::schema::{run_schema_generator, SchemaArtifact, SchemaGenerationCommand};
use finstack_quant_margin::schema::{MARGIN_SCHEMA_DESCRIPTION, MARGIN_SCHEMA_TITLE};
use finstack_quant_margin::MarginEnvelope;

const ARTIFACTS: &[SchemaArtifact] = &[SchemaArtifact::new::<MarginEnvelope>(
    "schemas/margin/1/margin.schema.json",
    "https://finstack_quant.dev/schemas/margin/1/margin.schema.json",
    MARGIN_SCHEMA_TITLE,
    MARGIN_SCHEMA_DESCRIPTION,
)];

fn main() {
    let command = SchemaGenerationCommand::from_env()
        .unwrap_or_else(|error| panic!("parse schema generator arguments: {error}"));
    run_schema_generator(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        Path::new("schemas/margin"),
        ARTIFACTS,
        &command,
    )
    .unwrap_or_else(|error| panic!("generate margin schemas: {error}"));
}
