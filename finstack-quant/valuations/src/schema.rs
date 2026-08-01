//! JSON-Schema helpers for Finstack Quant types.
//!
//! Schemas are generated from the crate's serde-friendly types and checked in
//! under `schemas/`. These helpers expose them as `serde_json::Value` for use
//! in validation, UI forms, and contract generation.
//!
//! # Error Handling
//!
//! All schema accessors return `Result<&'static Value>` instead of panicking,
//! allowing callers to handle schema loading failures gracefully.

use serde_json::Value;
use std::sync::OnceLock;

#[cfg(test)]
const JSON_SCHEMA_DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";
const COMMON_SCHEMA_BASE: &str = finstack_quant_core::schema::COMMON_SCHEMA_BASE;

const PRICING_OVERRIDE_SCHEMA_DEFINITIONS:
    &[finstack_quant_core::schema::ExternalSchemaDefinition] = &[
    finstack_quant_core::schema::ExternalSchemaDefinition::new::<
        crate::instruments::InstrumentPricingOverrides,
    >(
        "InstrumentPricingOverrides",
        "https://finstack_quant.dev/schemas/common/1/instrument_pricing_overrides.schema.json",
    ),
    finstack_quant_core::schema::ExternalSchemaDefinition::new::<
        crate::instruments::MetricPricingOverrides,
    >(
        "MetricPricingOverrides",
        "https://finstack_quant.dev/schemas/common/1/metric_pricing_overrides.schema.json",
    ),
    finstack_quant_core::schema::ExternalSchemaDefinition::new::<
        crate::instruments::ScenarioPricingOverrides,
    >(
        "ScenarioPricingOverrides",
        "https://finstack_quant.dev/schemas/common/1/scenario_pricing_overrides.schema.json",
    ),
];

/// Package a derived valuation schema using canonical shared definitions.
///
/// This pass changes only reference placement: common and cashflow definitions
/// are replaced by their equivalent published `$id`, then newly unreachable
/// local definitions are removed.
///
/// # Arguments
///
/// * `schema` - Complete schema generated from a valuation serde type.
#[doc(hidden)]
pub fn package_valuations_schema(schema: &mut Value) -> finstack_quant_core::Result<()> {
    let mut definitions = finstack_quant_core::schema::COMMON_SCHEMA_DEFINITIONS.to_vec();
    definitions.extend_from_slice(PRICING_OVERRIDE_SCHEMA_DEFINITIONS);
    definitions.extend_from_slice(finstack_quant_cashflows::schema::CASHFLOW_SCHEMA_DEFINITIONS);
    finstack_quant_core::schema::externalize_schema_definitions(schema, &definitions)
}

/// Parse embedded JSON schema at compile time, returning a Result.
/// The JSON is embedded via `include_str!` so the content is always present,
/// but parsing can still fail if the JSON is malformed.
macro_rules! try_include_schema {
    ($path:expr) => {
        serde_json::from_str::<Value>(include_str!($path))
            .map_err(|e| format!("invalid schema JSON at {}: {}", $path, e))
    };
}

/// Get JSON-Schema for Bond configuration.
///
/// Sourced from the generated instrument schemas under `schemas/instruments/1/`.
///
/// # Errors
///
/// Returns `Error::Validation` if the embedded schema JSON is malformed.
#[allow(dead_code)] // Public API, used in tests
pub fn bond_schema() -> finstack_quant_core::Result<&'static Value> {
    static SCHEMA: OnceLock<Result<Value, String>> = OnceLock::new();
    SCHEMA
        .get_or_init(|| {
            try_include_schema!("../schemas/instruments/1/fixed_income/bond.schema.json")
        })
        .as_ref()
        .map_err(|e| finstack_quant_core::Error::Validation(e.clone()))
}

/// Get the JSON Schema for the instrument envelope.
///
/// # Errors
///
/// Returns `Error::Validation` if the embedded schema JSON is malformed.
pub fn instrument_envelope_schema() -> finstack_quant_core::Result<&'static Value> {
    static SCHEMA: OnceLock<Result<Value, String>> = OnceLock::new();
    SCHEMA
        .get_or_init(|| try_include_schema!("../schemas/instruments/1/instrument.schema.json"))
        .as_ref()
        .map_err(|e| finstack_quant_core::Error::Validation(e.clone()))
}

fn instrument_schema_cache(
) -> &'static std::collections::BTreeMap<&'static str, Result<Value, String>> {
    static CACHE: OnceLock<std::collections::BTreeMap<&'static str, Result<Value, String>>> =
        OnceLock::new();
    CACHE.get_or_init(|| {
        crate::instruments::json_loader::instrument_registry()
            .into_iter()
            .map(|entry| (entry.tag, entry.load_embedded_schema()))
            .collect()
    })
}

fn common_schema_resource(
    filename: &'static str,
    raw: &'static str,
) -> finstack_quant_core::Result<(String, jsonschema::Resource)> {
    let schema = serde_json::from_str::<Value>(raw).map_err(|e| {
        finstack_quant_core::Error::Validation(format!(
            "invalid common schema JSON at {filename}: {e}"
        ))
    })?;
    let resource = jsonschema::Resource::from_contents(schema).map_err(|e| {
        finstack_quant_core::Error::Validation(format!(
            "invalid common schema resource at {filename}: {e}"
        ))
    })?;
    Ok((format!("{COMMON_SCHEMA_BASE}{filename}"), resource))
}

fn common_schema_resources() -> finstack_quant_core::Result<Vec<(String, jsonschema::Resource)>> {
    [
        (
            "attributes.schema.json",
            include_str!("../schemas/common/1/attributes.schema.json"),
        ),
        (
            "business_day_convention.schema.json",
            include_str!("../schemas/common/1/business_day_convention.schema.json"),
        ),
        (
            "currency.schema.json",
            include_str!("../schemas/common/1/currency.schema.json"),
        ),
        (
            "date.schema.json",
            include_str!("../schemas/common/1/date.schema.json"),
        ),
        (
            "day_count.schema.json",
            include_str!("../schemas/common/1/day_count.schema.json"),
        ),
        (
            "decimal.schema.json",
            include_str!("../schemas/common/1/decimal.schema.json"),
        ),
        (
            "id.schema.json",
            include_str!("../schemas/common/1/id.schema.json"),
        ),
        (
            "money.schema.json",
            include_str!("../schemas/common/1/money.schema.json"),
        ),
        (
            "instrument_pricing_overrides.schema.json",
            include_str!("../schemas/common/1/instrument_pricing_overrides.schema.json"),
        ),
        (
            "metric_pricing_overrides.schema.json",
            include_str!("../schemas/common/1/metric_pricing_overrides.schema.json"),
        ),
        (
            "scenario_pricing_overrides.schema.json",
            include_str!("../schemas/common/1/scenario_pricing_overrides.schema.json"),
        ),
        (
            "tenor.schema.json",
            include_str!("../schemas/common/1/tenor.schema.json"),
        ),
    ]
    .into_iter()
    .map(|(filename, raw)| common_schema_resource(filename, raw))
    .collect()
}

fn embedded_instrument_schema_resources(
) -> finstack_quant_core::Result<Vec<(String, jsonschema::Resource)>> {
    let mut resources = std::collections::BTreeMap::new();
    for (tag, schema_result) in instrument_schema_cache() {
        let schema = schema_result.as_ref().map_err(|e| {
            finstack_quant_core::Error::Validation(format!(
                "invalid instrument schema JSON for {tag}: {e}"
            ))
        })?;
        let id = schema.get("$id").and_then(Value::as_str).ok_or_else(|| {
            finstack_quant_core::Error::Validation(format!(
                "instrument schema for {tag} is missing $id"
            ))
        })?;
        let resource = jsonschema::Resource::from_contents(schema.clone()).map_err(|e| {
            finstack_quant_core::Error::Validation(format!(
                "invalid instrument schema resource for {tag}: {e}"
            ))
        })?;
        resources.insert(id.to_string(), resource);
    }

    Ok(resources.into_iter().collect())
}

fn external_schema_resources() -> finstack_quant_core::Result<Vec<(String, jsonschema::Resource)>> {
    let mut resources = common_schema_resources()?;
    resources.extend(finstack_quant_cashflows::schema::resources()?);
    resources.extend(embedded_instrument_schema_resources()?);
    Ok(resources)
}

/// Return canonical instrument discriminators from the tagged-JSON registry.
///
/// The registry is the source of truth for decoding and schema generation, so
/// this accessor does not infer type names by parsing checked-in `$ref` paths.
pub fn instrument_types() -> finstack_quant_core::Result<Vec<String>> {
    Ok(crate::instruments::json_loader::registry_tags()
        .iter()
        .map(|tag| (*tag).to_string())
        .collect())
}

/// Get the JSON Schema for a single instrument type.
///
/// Returns the dedicated generated schema for a canonical registry tag.
///
/// # Errors
///
/// Returns `Error::Validation` if the embedded schema JSON is malformed or the
/// requested instrument type is not supported.
///
/// # Arguments
///
/// * `instrument_type` - Canonical registered tagged-instrument type string.
pub fn instrument_schema(instrument_type: &str) -> finstack_quant_core::Result<Value> {
    if let Some(schema) = instrument_schema_cache().get(instrument_type) {
        return schema
            .as_ref()
            .cloned()
            .map_err(|e| finstack_quant_core::Error::Validation(e.clone()));
    }

    Err(finstack_quant_core::Error::Validation(format!(
        "unknown instrument type '{instrument_type}'"
    )))
}

/// Get JSON-Schema for ValuationResult.
///
/// Returns schema for valuation result envelope (PV + metrics).
///
/// # Errors
///
/// Returns `Error::Validation` if the embedded schema JSON is malformed.
#[allow(dead_code)] // Public API, used in tests
pub fn valuation_result_schema() -> finstack_quant_core::Result<&'static Value> {
    static SCHEMA: OnceLock<Result<Value, String>> = OnceLock::new();
    SCHEMA
        .get_or_init(|| try_include_schema!("../schemas/results/1/valuation_result.schema.json"))
        .as_ref()
        .map_err(|e| finstack_quant_core::Error::Validation(e.clone()))
}

/// Validate an instrument JSON value against the envelope schema.
///
/// Returns `Ok(())` if the JSON conforms to the instrument envelope schema,
/// or a detailed `Error::Validation` listing all schema violations.
///
/// # Example
///
/// ```ignore
/// use finstack_quant_valuations::schema::validate_instrument_envelope_json;
///
/// let json: serde_json::Value = serde_json::json!({
///     "schema": "finstack_quant.instrument/1",
///     "instrument": {
///         "type": "bond",
///         "spec": {
///             "id": "UST-10Y",
///             "notional": { "amount": "1000000", "currency": "USD" },
///             "issue_date": "2024-01-15",
///             "maturity": "2034-01-15",
///             "cashflow_spec": {
///                 "fixed": {
///                     "coupon_type": "cash",
///                     "frequency": { "count": 6, "unit": "months" },
///                     "day_count": "act_act_isma",
///                     "calendar_id": "sifma",
///                     "rate": "0.0425"
///                 }
///             },
///             "discount_curve_id": "USD-TREASURY",
///             "attributes": {}
///         }
///     }
/// });
/// if let Err(e) = validate_instrument_envelope_json(&json) {
///     eprintln!("Validation errors: {e}");
/// }
/// ```
///
/// # Errors
///
/// Returns `Error::Validation` if the JSON does not conform to the schema.
///
/// # Arguments
///
/// * `instance` - Parsed JSON instrument envelope to validate against the
///   canonical v1 envelope schema and its selected type schema.
pub fn validate_instrument_envelope_json(instance: &Value) -> finstack_quant_core::Result<()> {
    let schema = instrument_envelope_schema()?;
    let envelope_result = validate_against_schema(instance, schema, "instrument envelope");

    let instrument_type = instance
        .pointer("/instrument/type")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            finstack_quant_core::Error::Validation(
                "instrument envelope validation passed but instrument.type is missing".to_string(),
            )
        })?;

    if let Err(envelope_error) = envelope_result {
        if instrument_types()?.iter().any(|ty| ty == instrument_type) {
            validate_instrument_type_json(instrument_type, instance)?;
        }
        return Err(envelope_error);
    }

    validate_instrument_type_json(instrument_type, instance)
}

/// Validate a JSON value against a specific instrument type's schema.
///
/// # Errors
///
/// Returns `Error::Validation` if the JSON does not conform to the schema.
///
/// # Arguments
///
/// * `instrument_type` - Registered tagged-instrument type whose schema is
///   selected for validation.
/// * `instance` - Parsed JSON value to validate against that type schema.
pub fn validate_instrument_type_json(
    instrument_type: &str,
    instance: &Value,
) -> finstack_quant_core::Result<()> {
    let schema = instrument_schema(instrument_type)?;
    validate_against_schema(instance, &schema, instrument_type)
}

/// Validate a JSON value against an arbitrary schema.
fn validate_against_schema(
    instance: &Value,
    schema: &Value,
    context: &str,
) -> finstack_quant_core::Result<()> {
    let validator = jsonschema::options()
        .with_resources(external_schema_resources()?.into_iter())
        .build(schema)
        .map_err(|e| {
            finstack_quant_core::Error::Validation(format!("Invalid {context} schema: {e}"))
        })?;

    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|e| {
            let path = e.instance_path.to_string();
            if path.is_empty() {
                e.to_string()
            } else {
                format!("{path}: {e}")
            }
        })
        .collect();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(finstack_quant_core::Error::Validation(format!(
            "{context} validation failed with {} error(s):\n  {}",
            errors.len(),
            errors.join("\n  ")
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn first_schema_example(schema: &Value) -> Value {
        schema
            .get("examples")
            .and_then(Value::as_array)
            .and_then(|examples| examples.first())
            .cloned()
            .expect("schema should have at least one example")
    }

    #[test]
    fn test_schema_stubs() {
        // Verify stub schemas are valid JSON and have expected structure
        let bond = bond_schema().expect("bond schema should parse");
        assert_eq!(bond["$schema"], JSON_SCHEMA_DIALECT);
        assert_eq!(bond["title"], "bond");

        let envelope =
            instrument_envelope_schema().expect("instrument envelope schema should parse");
        assert_eq!(envelope["title"], "Finstack Quant Instrument");

        let result = valuation_result_schema().expect("valuation result schema should parse");
        assert_eq!(result["title"], "Valuation Result");
    }

    #[test]
    fn test_all_schemas_parse_successfully() {
        // Ensure all embedded schemas parse without error.
        // This test catches invalid JSON at CI time rather than runtime.
        assert!(bond_schema().is_ok(), "bond_schema() should return Ok");
        assert!(
            instrument_envelope_schema().is_ok(),
            "instrument_envelope_schema() should return Ok"
        );
        assert!(
            valuation_result_schema().is_ok(),
            "valuation_result_schema() should return Ok"
        );
    }

    #[test]
    fn test_instrument_types_lists_supported_tags() {
        let types = instrument_types().expect("instrument types should parse");
        let expected: Vec<String> = crate::instruments::json_loader::registry_tags()
            .iter()
            .map(|tag| (*tag).to_string())
            .collect();
        assert_eq!(types, expected);
        assert!(types.iter().any(|ty| ty == "bond"));
        assert!(types.iter().any(|ty| ty == "cms_swap"));
    }

    #[test]
    fn test_instrument_schema_returns_dedicated_schema_when_available() {
        let schema = instrument_schema("bond").expect("bond schema should load");
        assert_eq!(schema["title"], "bond");
        assert_eq!(
            schema["$id"],
            "https://finstack_quant.dev/schemas/instrument/1/fixed_income/bond.schema.json"
        );
    }

    #[test]
    fn test_all_envelope_types_have_dedicated_schemas() {
        let types = instrument_types().expect("instrument types should parse");
        for ty in &types {
            let schema = instrument_schema(ty)
                .unwrap_or_else(|e| panic!("schema for '{ty}' should load: {e}"));
            let desc = schema["description"]
                .as_str()
                .unwrap_or_else(|| panic!("schema for '{ty}' should have a description"));
            assert!(
                !desc.trim().is_empty(),
                "schema for '{ty}' should be documented"
            );
        }
    }

    #[test]
    fn test_instrument_schema_rejects_unknown_discriminator() {
        let err = instrument_schema("not_a_supported_instrument_type").expect_err("unknown type");
        let msg = err.to_string();
        assert!(
            msg.contains("unknown instrument type"),
            "unexpected message: {msg}"
        );
    }

    #[test]
    fn test_instrument_schema_cache_covers_canonical_tags_only() {
        let bond = instrument_schema("bond").expect("bond");
        assert_eq!(bond["title"], "bond");
        let swap = instrument_schema("interest_rate_swap").expect("irs");
        assert_eq!(swap["title"], "interest_rate_swap");
        assert!(instrument_schema("interest_rate_option").is_err());
    }

    #[test]
    fn test_validate_instrument_json_accepts_valid_envelope() {
        let valid = first_schema_example(bond_schema().expect("bond schema"));
        assert!(
            validate_instrument_envelope_json(&valid).is_ok(),
            "valid bond example should pass validation"
        );
    }

    #[test]
    fn test_validate_instrument_json_rejects_empty_typed_spec() {
        let invalid = serde_json::json!({
            "schema": "finstack_quant.instrument/1",
            "instrument": {
                "type": "bond",
                "spec": {}
            }
        });
        let msg = validate_instrument_envelope_json(&invalid)
            .expect_err("empty bond spec should fail typed validation")
            .to_string();
        assert!(
            msg.contains("bond validation failed"),
            "error should mention typed bond validation: {msg}"
        );
    }

    #[test]
    fn test_validate_instrument_json_rejects_missing_schema() {
        let invalid = serde_json::json!({
            "instrument": { "type": "bond", "spec": {} }
        });
        let msg = validate_instrument_envelope_json(&invalid)
            .expect_err("missing 'schema' field should fail")
            .to_string();
        assert!(
            msg.contains("validation failed"),
            "error should mention validation: {msg}"
        );
    }

    #[test]
    fn test_validate_instrument_json_rejects_unknown_type() {
        let invalid = serde_json::json!({
            "schema": "finstack_quant.instrument/1",
            "instrument": { "type": "not_real", "spec": {} }
        });
        let err = validate_instrument_envelope_json(&invalid);
        assert!(err.is_err(), "unknown instrument type should fail");
    }
}
