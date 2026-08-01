//! Embedded JSON Schemas owned by the statements crate.

use std::sync::OnceLock;

use serde_json::Value;

use crate::{Error, Result};
use finstack_quant_core::schema::SerdeSchema;

/// Stable base URI for statements-owned schemas.
pub const STATEMENTS_SCHEMA_BASE: &str = "https://finstack_quant.dev/schemas/statements/1/";

/// Build one statements-owned schema with canonical metadata.
///
/// This is public only so the generator binary and schema parity integration
/// test use exactly the same assembly path.
///
/// # Arguments
///
/// * `filename` - Version-directory filename appended to
///   [`STATEMENTS_SCHEMA_BASE`].
/// * `title` - Stable schema title for the generated root type.
/// * `description` - Stable schema description for the persisted contract.
///
/// # Errors
///
/// Returns [`Error::Serde`] if the schemars output cannot be represented as a
/// JSON object.
#[doc(hidden)]
pub fn generated_schema<T: SerdeSchema>(
    filename: &str,
    title: &str,
    description: &str,
) -> Result<Value> {
    finstack_quant_core::schema::generated_schema::<T>(
        STATEMENTS_SCHEMA_BASE,
        filename,
        title,
        description,
    )
    .map_err(|error| Error::Serde(error.to_string()))
}

fn parse_schema(
    cache: &'static OnceLock<std::result::Result<Value, String>>,
    raw: &'static str,
    filename: &'static str,
) -> Result<&'static Value> {
    cache
        .get_or_init(|| {
            serde_json::from_str(raw)
                .map_err(|error| format!("invalid statements schema JSON at {filename}: {error}"))
        })
        .as_ref()
        .map_err(|error| Error::Serde(error.clone()))
}

/// Return the checked-in schema for [`crate::FinancialModelSpec`].
///
/// # Errors
///
/// Returns [`Error::Serde`] if the embedded schema JSON is malformed.
pub fn financial_model_spec_schema() -> Result<&'static Value> {
    static SCHEMA: OnceLock<std::result::Result<Value, String>> = OnceLock::new();
    parse_schema(
        &SCHEMA,
        include_str!("../schemas/statements/1/financial_model_spec.schema.json"),
        "financial_model_spec.schema.json",
    )
}

/// Return the checked-in schema for [`crate::evaluator::StatementResult`].
///
/// # Errors
///
/// Returns [`Error::Serde`] if the embedded schema JSON is malformed.
pub fn statement_result_schema() -> Result<&'static Value> {
    static SCHEMA: OnceLock<std::result::Result<Value, String>> = OnceLock::new();
    parse_schema(
        &SCHEMA,
        include_str!("../schemas/statements/1/statement_result.schema.json"),
        "statement_result.schema.json",
    )
}
