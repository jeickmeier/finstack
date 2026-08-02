//! Generate checked-in JSON Schemas owned by the attribution crate.

use std::path::Path;

use finstack_quant_attribution::{AttributionEnvelope, AttributionResultEnvelope};
use finstack_quant_core::schema::{run_schema_generator, SchemaArtifact, SchemaGenerationCommand};

const ARTIFACTS: &[SchemaArtifact] = &[
    SchemaArtifact::new::<AttributionEnvelope>(
        "schemas/attribution/1/attribution.schema.json",
        "https://finstack_quant.dev/schemas/attribution/1/attribution.schema.json",
        "Finstack Quant Attribution Specification",
        "Complete specification for P&L attribution runs with instrument and market snapshots",
    ),
    SchemaArtifact::new::<AttributionResultEnvelope>(
        "schemas/attribution/1/attribution_result.schema.json",
        "https://finstack_quant.dev/schemas/attribution/1/attribution_result.schema.json",
        "Finstack Quant Attribution Result",
        "Complete result of a P&L attribution run including factor decomposition and metadata",
    ),
];

fn main() {
    let command = SchemaGenerationCommand::from_env()
        .unwrap_or_else(|error| panic!("parse schema generator arguments: {error}"));
    run_schema_generator(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        Path::new("schemas/attribution"),
        ARTIFACTS,
        &command,
    )
    .unwrap_or_else(|error| panic!("generate attribution schemas: {error}"));
}
