//! JSON Schema generation helpers for attribution contracts.

use schemars::JsonSchema;
use serde_json::Value;

use finstack_quant_core::Result;

/// Stable base URI for attribution-owned schemas.
pub const ATTRIBUTION_SCHEMA_BASE: &str = "https://finstack_quant.dev/schemas/attribution/1/";

/// Build one attribution-owned schema with canonical metadata.
///
/// This is public only so the generator binary and schema parity integration
/// test use exactly the same assembly path.
///
/// # Arguments
///
/// * `filename` - Version-directory filename appended to
///   [`ATTRIBUTION_SCHEMA_BASE`].
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
) -> Result<Value> {
    finstack_quant_core::schema::generated_schema::<T>(
        ATTRIBUTION_SCHEMA_BASE,
        filename,
        title,
        description,
    )
}
