//! JSON Schema generation for the published margin contract.
//!
//! The root is a synthetic schema-only union. It does not represent a Rust
//! ingress envelope or imply that the margin crate provides a root loader.

use schemars::JsonSchema;
use serde_json::Value;

use crate::{CsaSpec, MarginCall, OtcMarginSpec};

/// Stable base URI for margin-owned schemas.
pub const MARGIN_SCHEMA_BASE: &str = "https://finstack_quant.dev/schemas/margin/1/";
/// Filename of the published margin schema.
pub const MARGIN_SCHEMA_FILENAME: &str = "margin.schema.json";
/// Canonical title of the published margin schema.
pub const MARGIN_SCHEMA_TITLE: &str = "Finstack Quant Margin Specification";
/// Canonical description of the published margin schema.
pub const MARGIN_SCHEMA_DESCRIPTION: &str = "OTC derivative margin specifications including CSA terms, thresholds, and collateral eligibility. Covers ISDA CSA, BCBS-IOSCO regulatory margin, and CCP clearing requirements.";
/// Version marker required by each synthetic published schema variant.
pub const MARGIN_SCHEMA_V1: &str = "finstack_quant.margin/1";

fn margin_schema_marker(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({
        "type": "string",
        "const": MARGIN_SCHEMA_V1,
        "description": "Schema version identifier",
    })
}

/// Synthetic union used only to publish the supported margin payload shapes.
struct MarginSchemaContract;

impl JsonSchema for MarginSchemaContract {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("MarginSchemaContract")
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        let schema_marker = margin_schema_marker(generator);
        let otc_margin_spec = generator.subschema_for::<OtcMarginSpec>();
        let csa_spec = generator.subschema_for::<CsaSpec>();
        let margin_call = generator.subschema_for::<MarginCall>();
        schemars::json_schema!({
            "oneOf": [
                {
                    "type": "object",
                    "required": ["schema", "otc_margin_spec"],
                    "additionalProperties": false,
                    "properties": {
                        "schema": schema_marker,
                        "otc_margin_spec": otc_margin_spec,
                    },
                },
                {
                    "type": "object",
                    "required": ["schema", "csa_spec"],
                    "additionalProperties": false,
                    "properties": {
                        "schema": schema_marker,
                        "csa_spec": csa_spec,
                    },
                },
                {
                    "type": "object",
                    "required": ["schema", "margin_call"],
                    "additionalProperties": false,
                    "properties": {
                        "schema": schema_marker,
                        "margin_call": margin_call,
                    },
                },
            ],
        })
    }
}

/// Generate the published margin schema from its Rust contract types.
///
/// The schema-only top-level union is emitted as `oneOf` to preserve the
/// existing published contract. There is no corresponding Rust envelope or
/// strict root loader.
///
/// # Errors
///
/// Returns [`finstack_quant_core::Error::Internal`] if schemars output cannot
/// be represented as a JSON object.
pub fn generated_margin_schema() -> finstack_quant_core::Result<Value> {
    finstack_quant_core::schema::generated_schema::<MarginSchemaContract>(
        MARGIN_SCHEMA_BASE,
        MARGIN_SCHEMA_FILENAME,
        MARGIN_SCHEMA_TITLE,
        MARGIN_SCHEMA_DESCRIPTION,
    )
}
