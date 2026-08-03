//! JSON Schema generation helpers for factor-model contracts.

use std::sync::OnceLock;

use serde_json::Value;

/// Stable base URI for factor-model-owned schemas.
///
/// This retains the published underscore URI used by the original credit
/// factor-model schemas even though the Rust crate name is hyphenated.
pub const FACTOR_MODEL_SCHEMA_BASE: &str = "https://finstack_quant.dev/schemas/factor_model/1/";
/// Filename of the published factor-model configuration schema.
pub const FACTOR_MODEL_SCHEMA_FILENAME: &str = "factor_model_config.schema.json";
/// Canonical title of the factor-model configuration schema.
pub const FACTOR_MODEL_SCHEMA_TITLE: &str = "Finstack Quant Factor Model Configuration";
/// Canonical description of the factor-model configuration schema.
pub const FACTOR_MODEL_SCHEMA_DESCRIPTION: &str =
    "Versioned factor-model configuration with typed factors, covariance, matching, pricing, and risk settings.";
/// Filename of the published credit factor-model artifact schema.
pub const CREDIT_FACTOR_MODEL_SCHEMA_FILENAME: &str = "credit_factor_model.schema.json";
/// Canonical title of the credit factor-model artifact schema.
pub const CREDIT_FACTOR_MODEL_SCHEMA_TITLE: &str = "CreditFactorModel";
/// Canonical description of the credit factor-model artifact schema.
pub const CREDIT_FACTOR_MODEL_SCHEMA_DESCRIPTION: &str =
    "Fully self-contained credit factor hierarchy model artifact.";
/// Filename of the published credit calibration configuration schema.
pub const CREDIT_CALIBRATION_CONFIG_SCHEMA_FILENAME: &str = "credit_calibration_config.schema.json";
/// Canonical title of the credit calibration configuration schema.
pub const CREDIT_CALIBRATION_CONFIG_SCHEMA_TITLE: &str = "CreditCalibrationConfig";
/// Canonical description of the credit calibration configuration schema.
pub const CREDIT_CALIBRATION_CONFIG_SCHEMA_DESCRIPTION: &str =
    "Configuration for the deterministic credit factor-model calibrator.";
/// Filename of the published credit calibration input schema.
pub const CREDIT_CALIBRATION_INPUTS_SCHEMA_FILENAME: &str = "credit_calibration_inputs.schema.json";
/// Canonical title of the credit calibration input schema.
pub const CREDIT_CALIBRATION_INPUTS_SCHEMA_TITLE: &str = "CreditCalibrationInputs";
/// Canonical description of the credit calibration input schema.
pub const CREDIT_CALIBRATION_INPUTS_SCHEMA_DESCRIPTION: &str =
    "Typed issuer histories, tags, generic factor series, anchor date, and overrides for one credit calibration run.";

fn parse_schema(
    cache: &'static OnceLock<std::result::Result<Value, String>>,
    raw: &'static str,
    filename: &'static str,
) -> finstack_quant_core::Result<&'static Value> {
    cache
        .get_or_init(|| {
            serde_json::from_str(raw)
                .map_err(|error| format!("invalid factor-model schema JSON at {filename}: {error}"))
        })
        .as_ref()
        .map_err(|error| finstack_quant_core::Error::Internal(error.clone()))
}

/// Return the checked-in schema for [`crate::FactorModelConfigEnvelope`].
///
/// # Errors
///
/// Returns [`finstack_quant_core::Error::Internal`] if the embedded schema is
/// malformed JSON.
pub fn factor_model_config_schema() -> finstack_quant_core::Result<&'static Value> {
    static SCHEMA: OnceLock<std::result::Result<Value, String>> = OnceLock::new();
    parse_schema(
        &SCHEMA,
        include_str!("../schemas/factor_model/1/factor_model_config.schema.json"),
        FACTOR_MODEL_SCHEMA_FILENAME,
    )
}

/// Return the checked-in schema for
/// [`crate::credit::hierarchy::CreditFactorModel`].
///
/// # Errors
///
/// Returns [`finstack_quant_core::Error::Internal`] if the embedded schema is
/// malformed JSON.
pub fn credit_factor_model_schema() -> finstack_quant_core::Result<&'static Value> {
    static SCHEMA: OnceLock<std::result::Result<Value, String>> = OnceLock::new();
    parse_schema(
        &SCHEMA,
        include_str!("../schemas/factor_model/1/credit_factor_model.schema.json"),
        CREDIT_FACTOR_MODEL_SCHEMA_FILENAME,
    )
}

/// Return the checked-in schema for
/// [`crate::credit::calibration::CreditCalibrationConfig`].
///
/// # Errors
///
/// Returns [`finstack_quant_core::Error::Internal`] if the embedded schema is
/// malformed JSON.
pub fn credit_calibration_config_schema() -> finstack_quant_core::Result<&'static Value> {
    static SCHEMA: OnceLock<std::result::Result<Value, String>> = OnceLock::new();
    parse_schema(
        &SCHEMA,
        include_str!("../schemas/factor_model/1/credit_calibration_config.schema.json"),
        CREDIT_CALIBRATION_CONFIG_SCHEMA_FILENAME,
    )
}

/// Return the checked-in schema for
/// [`crate::credit::calibration::CreditCalibrationInputs`].
///
/// # Errors
///
/// Returns [`finstack_quant_core::Error::Internal`] if the embedded schema is
/// malformed JSON.
pub fn credit_calibration_inputs_schema() -> finstack_quant_core::Result<&'static Value> {
    static SCHEMA: OnceLock<std::result::Result<Value, String>> = OnceLock::new();
    parse_schema(
        &SCHEMA,
        include_str!("../schemas/factor_model/1/credit_calibration_inputs.schema.json"),
        CREDIT_CALIBRATION_INPUTS_SCHEMA_FILENAME,
    )
}
