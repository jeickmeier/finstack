//! Generate checked-in JSON Schemas owned by `finstack-quant-core`.

use std::path::Path;

use finstack_quant_core::market_data::context::MarketContextState;
use finstack_quant_core::schema::{run_schema_generator, SchemaArtifact, SchemaGenerationCommand};

const ARTIFACTS: &[SchemaArtifact] = &[SchemaArtifact::new::<MarketContextState>(
    "schemas/market_data/1/market_context_state.schema.json",
    "https://finstack_quant.dev/schemas/market_data/1/market_context_state.schema.json",
    "Market Context State",
    "Canonical v1 persisted snapshot of a complete market-data context.",
)];

fn main() {
    let command = SchemaGenerationCommand::from_env()
        .unwrap_or_else(|error| panic!("parse schema generator arguments: {error}"));
    run_schema_generator(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        Path::new("schemas/market_data"),
        ARTIFACTS,
        &command,
    )
    .unwrap_or_else(|error| panic!("generate core schemas: {error}"));
}
