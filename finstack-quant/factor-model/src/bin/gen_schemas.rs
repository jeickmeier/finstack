//! Generate checked-in JSON Schemas owned by the factor-model crate.

use std::path::Path;

use finstack_quant_core::schema::{run_schema_generator, SchemaArtifact, SchemaGenerationCommand};
use finstack_quant_factor_model::credit::calibration::{
    CreditCalibrationConfig, CreditCalibrationInputs,
};
use finstack_quant_factor_model::credit::hierarchy::CreditFactorModel;
use finstack_quant_factor_model::schema::{
    CREDIT_CALIBRATION_CONFIG_SCHEMA_DESCRIPTION, CREDIT_CALIBRATION_CONFIG_SCHEMA_TITLE,
    CREDIT_CALIBRATION_INPUTS_SCHEMA_DESCRIPTION, CREDIT_CALIBRATION_INPUTS_SCHEMA_TITLE,
    CREDIT_FACTOR_MODEL_SCHEMA_DESCRIPTION, CREDIT_FACTOR_MODEL_SCHEMA_TITLE,
    FACTOR_MODEL_SCHEMA_DESCRIPTION, FACTOR_MODEL_SCHEMA_TITLE,
};
use finstack_quant_factor_model::FactorModelConfigEnvelope;
const ARTIFACTS: &[SchemaArtifact] = &[
    SchemaArtifact::new::<FactorModelConfigEnvelope>(
        "schemas/factor_model/1/factor_model_config.schema.json",
        "https://finstack_quant.dev/schemas/factor_model/1/factor_model_config.schema.json",
        FACTOR_MODEL_SCHEMA_TITLE,
        FACTOR_MODEL_SCHEMA_DESCRIPTION,
    ),
    SchemaArtifact::new::<CreditFactorModel>(
        "schemas/factor_model/1/credit_factor_model.schema.json",
        "https://finstack_quant.dev/schemas/factor_model/1/credit_factor_model.schema.json",
        CREDIT_FACTOR_MODEL_SCHEMA_TITLE,
        CREDIT_FACTOR_MODEL_SCHEMA_DESCRIPTION,
    ),
    SchemaArtifact::new::<CreditCalibrationConfig>(
        "schemas/factor_model/1/credit_calibration_config.schema.json",
        "https://finstack_quant.dev/schemas/factor_model/1/credit_calibration_config.schema.json",
        CREDIT_CALIBRATION_CONFIG_SCHEMA_TITLE,
        CREDIT_CALIBRATION_CONFIG_SCHEMA_DESCRIPTION,
    ),
    SchemaArtifact::new::<CreditCalibrationInputs>(
        "schemas/factor_model/1/credit_calibration_inputs.schema.json",
        "https://finstack_quant.dev/schemas/factor_model/1/credit_calibration_inputs.schema.json",
        CREDIT_CALIBRATION_INPUTS_SCHEMA_TITLE,
        CREDIT_CALIBRATION_INPUTS_SCHEMA_DESCRIPTION,
    ),
];

fn main() {
    let command = SchemaGenerationCommand::from_env()
        .unwrap_or_else(|error| panic!("parse schema generator arguments: {error}"));
    run_schema_generator(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        Path::new("schemas/factor_model"),
        ARTIFACTS,
        &command,
    )
    .unwrap_or_else(|error| panic!("generate factor-model schemas: {error}"));
}
