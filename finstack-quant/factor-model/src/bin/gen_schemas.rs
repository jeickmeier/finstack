//! Generate checked-in JSON Schemas owned by the factor-model crate.

use std::path::{Path, PathBuf};

use finstack_quant_factor_model::credit::calibration::{
    CreditCalibrationConfig, CreditCalibrationInputs,
};
use finstack_quant_factor_model::credit::hierarchy::CreditFactorModel;
use finstack_quant_factor_model::schema::{
    generated_schema, CREDIT_CALIBRATION_CONFIG_SCHEMA_DESCRIPTION,
    CREDIT_CALIBRATION_CONFIG_SCHEMA_FILENAME, CREDIT_CALIBRATION_CONFIG_SCHEMA_TITLE,
    CREDIT_CALIBRATION_INPUTS_SCHEMA_DESCRIPTION, CREDIT_CALIBRATION_INPUTS_SCHEMA_FILENAME,
    CREDIT_CALIBRATION_INPUTS_SCHEMA_TITLE, CREDIT_FACTOR_MODEL_SCHEMA_DESCRIPTION,
    CREDIT_FACTOR_MODEL_SCHEMA_FILENAME, CREDIT_FACTOR_MODEL_SCHEMA_TITLE,
    FACTOR_MODEL_SCHEMA_DESCRIPTION, FACTOR_MODEL_SCHEMA_FILENAME, FACTOR_MODEL_SCHEMA_TITLE,
};
use finstack_quant_factor_model::FactorModelConfigEnvelope;
use schemars::JsonSchema;

fn schema_dir() -> PathBuf {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by Cargo");
    Path::new(&manifest_dir)
        .join("schemas")
        .join("factor_model")
        .join("1")
}

fn write_schema<T: JsonSchema>(directory: &Path, filename: &str, title: &str, description: &str) {
    let path = directory.join(filename);
    let generated = generated_schema::<T>(filename, title, description)
        .unwrap_or_else(|error| panic!("generate {filename}: {error}"));
    let json = serde_json::to_string_pretty(&generated)
        .unwrap_or_else(|error| panic!("encode {filename}: {error}"));
    std::fs::write(&path, json + "\n")
        .unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
    println!("updated {}", path.display());
}

fn main() {
    let directory = schema_dir();
    std::fs::create_dir_all(&directory)
        .unwrap_or_else(|error| panic!("create {}: {error}", directory.display()));
    write_schema::<FactorModelConfigEnvelope>(
        &directory,
        FACTOR_MODEL_SCHEMA_FILENAME,
        FACTOR_MODEL_SCHEMA_TITLE,
        FACTOR_MODEL_SCHEMA_DESCRIPTION,
    );
    write_schema::<CreditFactorModel>(
        &directory,
        CREDIT_FACTOR_MODEL_SCHEMA_FILENAME,
        CREDIT_FACTOR_MODEL_SCHEMA_TITLE,
        CREDIT_FACTOR_MODEL_SCHEMA_DESCRIPTION,
    );
    write_schema::<CreditCalibrationConfig>(
        &directory,
        CREDIT_CALIBRATION_CONFIG_SCHEMA_FILENAME,
        CREDIT_CALIBRATION_CONFIG_SCHEMA_TITLE,
        CREDIT_CALIBRATION_CONFIG_SCHEMA_DESCRIPTION,
    );
    write_schema::<CreditCalibrationInputs>(
        &directory,
        CREDIT_CALIBRATION_INPUTS_SCHEMA_FILENAME,
        CREDIT_CALIBRATION_INPUTS_SCHEMA_TITLE,
        CREDIT_CALIBRATION_INPUTS_SCHEMA_DESCRIPTION,
    );
}
