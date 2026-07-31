//! JSON Schema generation helpers for scenario contracts.

use schemars::JsonSchema;
use serde_json::Value;

/// Stable base URI for scenario-owned schemas.
pub const SCENARIO_SCHEMA_BASE: &str = "https://finstack_quant.dev/schemas/scenarios/1/";
/// Filename of the published scenario envelope schema.
pub const SCENARIO_SCHEMA_FILENAME: &str = "scenario.schema.json";
/// Canonical title of the published scenario envelope schema.
pub const SCENARIO_SCHEMA_TITLE: &str = "Finstack Quant Scenario Specification";
/// Canonical description of the published scenario envelope schema.
pub const SCENARIO_SCHEMA_DESCRIPTION: &str =
    "Versioned scenario specification with typed market, instrument, statement, and time-roll operations.";

/// Build the scenario-owned schema with canonical metadata.
///
/// This is public so the generator binary and schema parity integration test
/// use exactly the same assembly path.
///
/// # Arguments
///
/// * `filename` - Version-directory filename appended to
///   [`SCENARIO_SCHEMA_BASE`].
/// * `title` - Stable human-readable title for the generated root type.
/// * `description` - Stable description for the persisted contract.
///
/// # Errors
///
/// Returns [`finstack_quant_core::Error::Internal`] if schemars output cannot
/// be serialized as a JSON object.
#[doc(hidden)]
pub fn generated_schema<T: JsonSchema>(
    filename: &str,
    title: &str,
    description: &str,
) -> finstack_quant_core::Result<Value> {
    finstack_quant_core::schema::generated_schema::<T>(
        SCENARIO_SCHEMA_BASE,
        filename,
        title,
        description,
    )
}
