//! Generate checked-in JSON Schemas owned by the scenarios crate.

use std::path::Path;

use finstack_quant_core::schema::{run_schema_generator, SchemaArtifact, SchemaGenerationCommand};
use finstack_quant_scenarios::schema::{SCENARIO_SCHEMA_DESCRIPTION, SCENARIO_SCHEMA_TITLE};
use finstack_quant_scenarios::ScenarioEnvelope;

const ARTIFACTS: &[SchemaArtifact] = &[SchemaArtifact::new::<ScenarioEnvelope>(
    "schemas/scenarios/1/scenario.schema.json",
    "https://finstack_quant.dev/schemas/scenarios/1/scenario.schema.json",
    SCENARIO_SCHEMA_TITLE,
    SCENARIO_SCHEMA_DESCRIPTION,
)];

fn main() {
    let command = SchemaGenerationCommand::from_env()
        .unwrap_or_else(|error| panic!("parse schema generator arguments: {error}"));
    run_schema_generator(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        Path::new("schemas/scenarios"),
        ARTIFACTS,
        &command,
    )
    .unwrap_or_else(|error| panic!("generate scenario schemas: {error}"));
}
