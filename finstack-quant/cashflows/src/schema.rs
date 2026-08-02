//! Embedded JSON Schema resources owned by the cashflows crate.

use finstack_quant_core::{Error, Result};
use serde_json::Value;

/// Stable base URI for cashflow component schemas.
pub const CASHFLOW_SCHEMA_BASE: &str = "https://finstack_quant.dev/schemas/cashflow/1/";

/// Typed cashflow definitions eligible for assertion-checked externalization.
pub const CASHFLOW_SCHEMA_DEFINITIONS: &[finstack_quant_core::schema::ExternalSchemaDefinition] = &[
    finstack_quant_core::schema::ExternalSchemaDefinition::new::<crate::builder::DefaultModelSpec>(
        "DefaultModelSpec",
        "https://finstack_quant.dev/schemas/cashflow/1/default_model_spec.schema.json",
    ),
    finstack_quant_core::schema::ExternalSchemaDefinition::new::<crate::builder::FeeSpec>(
        "FeeSpec",
        "https://finstack_quant.dev/schemas/cashflow/1/fee_specs.schema.json",
    ),
    finstack_quant_core::schema::ExternalSchemaDefinition::new::<crate::builder::FixedCouponSpec>(
        "FixedCouponSpec",
        "https://finstack_quant.dev/schemas/cashflow/1/coupon_specs.schema.json",
    ),
    finstack_quant_core::schema::ExternalSchemaDefinition::new::<crate::builder::PrepaymentModelSpec>(
        "PrepaymentModelSpec",
        "https://finstack_quant.dev/schemas/cashflow/1/prepayment_model_spec.schema.json",
    ),
    finstack_quant_core::schema::ExternalSchemaDefinition::new::<crate::builder::RecoveryModelSpec>(
        "RecoveryModelSpec",
        "https://finstack_quant.dev/schemas/cashflow/1/recovery_model_spec.schema.json",
    ),
    finstack_quant_core::schema::ExternalSchemaDefinition::new::<crate::builder::ScheduleParams>(
        "ScheduleParams",
        "https://finstack_quant.dev/schemas/cashflow/1/schedule_params.schema.json",
    ),
];

/// Package a derived cashflow schema using canonical shared definitions.
///
/// This pass changes only reference placement: shared definitions are
/// replaced by their equivalent published `$id`, then newly unreachable local
/// definitions are removed.
///
/// # Arguments
///
/// * `schema` - Complete schema generated from a cashflow serde type.
#[doc(hidden)]
pub fn package_cashflow_schema(schema: &mut Value) -> Result<()> {
    finstack_quant_core::schema::externalize_schema_definitions(
        schema,
        finstack_quant_core::schema::COMMON_SCHEMA_DEFINITIONS,
    )
}

const SCHEMAS: [(&str, &str); 7] = [
    (
        "amortization_spec.schema.json",
        include_str!("../schemas/cashflow/1/amortization_spec.schema.json"),
    ),
    (
        "coupon_specs.schema.json",
        include_str!("../schemas/cashflow/1/coupon_specs.schema.json"),
    ),
    (
        "default_model_spec.schema.json",
        include_str!("../schemas/cashflow/1/default_model_spec.schema.json"),
    ),
    (
        "fee_specs.schema.json",
        include_str!("../schemas/cashflow/1/fee_specs.schema.json"),
    ),
    (
        "prepayment_model_spec.schema.json",
        include_str!("../schemas/cashflow/1/prepayment_model_spec.schema.json"),
    ),
    (
        "recovery_model_spec.schema.json",
        include_str!("../schemas/cashflow/1/recovery_model_spec.schema.json"),
    ),
    (
        "schedule_params.schema.json",
        include_str!("../schemas/cashflow/1/schedule_params.schema.json"),
    ),
];

/// Parse the embedded schemas once per process.
///
/// Errors are cached as `String` because `finstack_quant_core::Error` is not
/// `Clone`.
fn parsed_schemas() -> &'static std::result::Result<Vec<(String, jsonschema::Resource)>, String> {
    static CACHE: std::sync::OnceLock<
        std::result::Result<Vec<(String, jsonschema::Resource)>, String>,
    > = std::sync::OnceLock::new();

    CACHE.get_or_init(|| {
        SCHEMAS
            .into_iter()
            .map(|(filename, raw)| {
                let schema = serde_json::from_str::<Value>(raw)
                    .map_err(|err| format!("invalid cashflow schema JSON at {filename}: {err}"))?;
                let resource = jsonschema::Resource::from_contents(schema).map_err(|err| {
                    format!("invalid cashflow schema resource at {filename}: {err}")
                })?;
                Ok((format!("{CASHFLOW_SCHEMA_BASE}{filename}"), resource))
            })
            .collect()
    })
}

/// Return the embedded cashflow schemas as JSON-Schema resolver resources.
///
/// # Errors
///
/// Returns a validation error if a checked-in schema is malformed.
pub fn resources() -> Result<Vec<(String, jsonschema::Resource)>> {
    match parsed_schemas() {
        Ok(entries) => Ok(entries.clone()),
        Err(err) => Err(Error::Validation(err.clone())),
    }
}
