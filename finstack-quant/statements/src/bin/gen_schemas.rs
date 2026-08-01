//! Generate checked-in JSON Schemas owned by the statements crate.

use std::path::Path;

use finstack_quant_core::schema::{run_schema_generator, SchemaArtifact, SchemaGenerationCommand};
use finstack_quant_statements::evaluator::StatementResult;
use finstack_quant_statements::FinancialModelSpec;

const ARTIFACTS: &[SchemaArtifact] = &[
    SchemaArtifact::new::<FinancialModelSpec>(
        "schemas/statements/1/financial_model_spec.schema.json",
        "https://finstack_quant.dev/schemas/statements/1/financial_model_spec.schema.json",
        "FinancialModelSpec",
        "Versioned financial statement model specification.",
    ),
    SchemaArtifact::new::<StatementResult>(
        "schemas/statements/1/statement_result.schema.json",
        "https://finstack_quant.dev/schemas/statements/1/statement_result.schema.json",
        "StatementResult",
        "Versioned financial statement evaluation result.",
    ),
];

fn main() {
    let command = SchemaGenerationCommand::from_env()
        .unwrap_or_else(|error| panic!("parse schema generator arguments: {error}"));
    run_schema_generator(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        Path::new("schemas/statements"),
        ARTIFACTS,
        &command,
    )
    .unwrap_or_else(|error| panic!("generate statement schemas: {error}"));
}
