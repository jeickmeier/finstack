//! Deterministic JSON Schema assembly helpers.

use schemars::JsonSchema;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::Path;

use crate::{Error, Result};

/// JSON Schema dialect used by generated Finstack contracts.
pub const JSON_SCHEMA_DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";
const DECIMAL_PATTERN: &str = r"^-?\d+(\.\d+)?([eE][+-]?\d+)?$";
const SCHEMARS_DECIMAL_PATTERN: &str = r"^-?\d+(\.\d+)?([eE]\d+)?$";

/// Build a generated schema with stable canonical metadata.
///
/// The default `serde_json::Map` representation serializes keys in
/// lexicographic order, making pretty-printed output deterministic across
/// repeated runs. The generated document always passes through
/// [`postprocess_schema`] with local definitions retained, so decimal patterns,
/// date formats, reference traversal, and unreachable-definition pruning cannot
/// be bypassed by callers using this helper. Types that emit absolute `$ref`
/// values keep those external references unchanged.
///
/// # Arguments
///
/// * `schema_base` - Canonical URI prefix for the owning schema family,
///   including its trailing slash.
/// * `filename` - Version-directory filename appended to `schema_base`.
/// * `title` - Stable human-readable title for the generated root type.
/// * `description` - Stable description for the persisted contract.
///
/// # Errors
///
/// Returns [`Error::Internal`] if schemars output cannot be serialized as a
/// JSON object.
pub fn generated_schema<T: JsonSchema>(
    schema_base: &str,
    filename: &str,
    title: &str,
    description: &str,
) -> Result<Value> {
    let mut generated = serde_json::to_value(schemars::schema_for!(T))
        .map_err(|error| Error::Internal(format!("serialize generated {title} schema: {error}")))?;
    postprocess_schema(&mut generated, |_| None);
    let generated = generated.as_object().ok_or_else(|| {
        Error::Internal(format!("generated {title} schema must be a JSON object"))
    })?;
    let mut output = Map::new();
    output.insert(
        "$id".to_string(),
        Value::String(format!("{schema_base}{filename}")),
    );
    output.insert(
        "$schema".to_string(),
        Value::String(JSON_SCHEMA_DIALECT.to_string()),
    );
    output.insert("title".to_string(), Value::String(title.to_string()));
    output.insert(
        "description".to_string(),
        Value::String(description.to_string()),
    );
    for (key, value) in generated {
        if !matches!(key.as_str(), "$id" | "$schema" | "title" | "description") {
            output.insert(key.clone(), value.clone());
        }
    }
    Ok(Value::Object(output))
}

/// Apply deterministic Finstack post-processing to a schemars document.
///
/// The traversal normalizes `rust_decimal` patterns, annotates conventional
/// date fields, replaces selected local definition references, and removes
/// definitions that are no longer reachable from the document root.
///
/// # Arguments
///
/// * `schema` - Mutable schemars JSON document to normalize in place.
/// * `external_ref` - Resolver called with each local `$defs` name. Returning
///   a URI externalizes that definition and preserves any nested JSON Pointer
///   suffix as a fragment; returning `None` keeps it local. Definition names
///   are JSON Pointer-decoded before the resolver is called.
pub fn postprocess_schema(schema: &mut Value, external_ref: impl Fn(&str) -> Option<String>) {
    walk_schema(schema, &external_ref);
    prune_unreachable_defs(schema);
}

/// Assemble a deterministic schema document from selected fields.
///
/// The canonical JSON Schema dialect is always inserted. Missing selected
/// fields are skipped, allowing callers to preserve optional metadata such as
/// examples without fabricating it.
///
/// # Arguments
///
/// * `metadata` - JSON object containing existing or caller-supplied metadata.
/// * `generated` - JSON object produced by schemars after any post-processing.
/// * `metadata_keys` - Metadata fields to copy into the assembled document.
/// * `generated_keys` - Generated schema keywords to copy into the document.
///
/// # Errors
///
/// Returns [`Error::Internal`] when `metadata` or `generated` is not a JSON
/// object.
pub fn assemble_schema(
    metadata: &Value,
    generated: &Value,
    metadata_keys: &[&str],
    generated_keys: &[&str],
) -> Result<Value> {
    let metadata = metadata
        .as_object()
        .ok_or_else(|| Error::Internal("schema metadata must be a JSON object".to_string()))?;
    let generated = generated
        .as_object()
        .ok_or_else(|| Error::Internal("generated schema must be a JSON object".to_string()))?;
    let mut output = Map::new();
    for key in metadata_keys {
        if let Some(value) = metadata.get(*key) {
            output.insert((*key).to_string(), value.clone());
        }
    }
    output.insert(
        "$schema".to_string(),
        Value::String(JSON_SCHEMA_DIALECT.to_string()),
    );
    for key in generated_keys {
        if let Some(value) = generated.get(*key) {
            output.insert((*key).to_string(), value.clone());
        }
    }
    Ok(Value::Object(output))
}

/// Write a schema as deterministic pretty JSON with a trailing newline.
///
/// Parent directories are created when needed.
///
/// # Arguments
///
/// * `path` - Destination file for the generated schema document.
/// * `schema` - JSON Schema value to serialize.
///
/// # Errors
///
/// Returns [`Error::Internal`] if the destination directory cannot be created,
/// the schema cannot be serialized, or the file cannot be written.
pub fn write_schema(path: &Path, schema: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            Error::Internal(format!(
                "create schema directory {}: {error}",
                parent.display()
            ))
        })?;
    }
    let json = serde_json::to_string_pretty(schema)
        .map_err(|error| Error::Internal(format!("serialize schema: {error}")))?;
    std::fs::write(path, json + "\n")
        .map_err(|error| Error::Internal(format!("write schema {}: {error}", path.display())))
}

fn is_date_like(name: &str) -> bool {
    name == "date"
        || name.ends_with("_date")
        || name == "as_of"
        || name.starts_with("as_of_")
        || name == "maturity"
        || name.ends_with("_maturity")
        || name == "expiry"
        || name.ends_with("_expiry")
}

fn accepts_string(schema: &Value) -> bool {
    type_accepts_string(schema.get("type"))
}

fn type_accepts_string(schema_type: Option<&Value>) -> bool {
    match schema_type {
        Some(Value::String(kind)) => kind == "string",
        Some(Value::Array(kinds)) => kinds.iter().any(|kind| kind == "string"),
        _ => false,
    }
}

fn walk_schema(value: &mut Value, external_ref: &impl Fn(&str) -> Option<String>) {
    match value {
        Value::Object(map) => {
            if let Some(properties) = map.get_mut("properties").and_then(Value::as_object_mut) {
                for (name, schema) in properties {
                    if is_date_like(name) && accepts_string(schema) {
                        if let Some(schema) = schema.as_object_mut() {
                            schema
                                .entry("format".to_string())
                                .or_insert_with(|| Value::String("date".to_string()));
                        }
                    }
                }
            }
            if type_accepts_string(map.get("type"))
                && map.get("pattern").and_then(Value::as_str) == Some(SCHEMARS_DECIMAL_PATTERN)
            {
                map.insert(
                    "pattern".to_string(),
                    Value::String(DECIMAL_PATTERN.to_string()),
                );
            }
            let replacement = map
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(local_definition_ref)
                .and_then(|(name, suffix)| {
                    external_ref(&name).map(|reference| match suffix {
                        Some(suffix) => format!("{reference}#/{suffix}"),
                        None => reference,
                    })
                });
            if let Some(reference) = replacement {
                map.insert("$ref".to_string(), Value::String(reference));
            }
            for child in map.values_mut() {
                walk_schema(child, external_ref);
            }
        }
        Value::Array(items) => {
            for item in items {
                walk_schema(item, external_ref);
            }
        }
        _ => {}
    }
}

fn local_definition_ref(reference: &str) -> Option<(String, Option<&str>)> {
    let rest = reference.strip_prefix("#/$defs/")?;
    let (name, suffix) = match rest.split_once('/') {
        Some((name, suffix)) => (name, Some(suffix)),
        None => (rest, None),
    };
    if name.is_empty() {
        return None;
    }
    Some((name.replace("~1", "/").replace("~0", "~"), suffix))
}

fn collect_local_def_refs(value: &Value, refs: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            if let Some(name) = map
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(local_definition_ref)
                .map(|(name, _)| name)
            {
                refs.insert(name);
            }
            for child in map.values() {
                collect_local_def_refs(child, refs);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_local_def_refs(item, refs);
            }
        }
        _ => {}
    }
}

fn prune_unreachable_defs(value: &mut Value) {
    let Some(defs) = value.get("$defs").and_then(Value::as_object).cloned() else {
        return;
    };
    let mut root = value.clone();
    if let Some(root) = root.as_object_mut() {
        root.remove("$defs");
    }
    let mut pending = BTreeSet::new();
    collect_local_def_refs(&root, &mut pending);
    let mut reachable = BTreeSet::new();
    while let Some(name) = pending.pop_first() {
        if reachable.insert(name.clone()) {
            if let Some(definition) = defs.get(&name) {
                collect_local_def_refs(definition, &mut pending);
            }
        }
    }
    if let Some(defs) = value.get_mut("$defs").and_then(Value::as_object_mut) {
        defs.retain(|name, _| reachable.contains(name));
        if defs.is_empty() {
            if let Some(value) = value.as_object_mut() {
                value.remove("$defs");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn postprocess_rewrites_refs_normalizes_formats_and_prunes_defs() {
        let mut schema = json!({
            "properties": {
                "trade_date": {
                    "type": "string"
                },
                "amount": {
                    "type": ["string", "null"],
                    "pattern": "^-?\\d+(\\.\\d+)?([eE]\\d+)?$"
                },
                "money": {
                    "$ref": "#/$defs/Money"
                },
                "local": {
                    "$ref": "#/$defs/Local"
                }
            },
            "$defs": {
                "Money": {
                    "type": "object"
                },
                "Local": {
                    "$ref": "#/$defs/Nested"
                },
                "Nested": {
                    "type": "string"
                },
                "Unused": {
                    "type": "boolean"
                }
            }
        });

        postprocess_schema(&mut schema, |name| {
            (name == "Money").then(|| "https://example.test/money.schema.json".to_string())
        });

        assert_eq!(schema["properties"]["trade_date"]["format"], "date");
        assert_eq!(
            schema["properties"]["amount"]["pattern"],
            r"^-?\d+(\.\d+)?([eE][+-]?\d+)?$"
        );
        assert_eq!(
            schema["properties"]["money"]["$ref"],
            "https://example.test/money.schema.json"
        );
        assert!(schema["$defs"].get("Money").is_none());
        assert!(schema["$defs"].get("Unused").is_none());
        assert!(schema["$defs"].get("Local").is_some());
        assert!(schema["$defs"].get("Nested").is_some());
    }

    #[test]
    fn postprocess_decodes_nested_definition_name_and_preserves_suffix() {
        let seen = std::cell::RefCell::new(Vec::new());
        let mut schema = json!({
            "properties": {
                "nested": {
                    "$ref": "#/$defs/Foo~1Bar~0Baz/properties/value"
                }
            },
            "$defs": {
                "Foo/Bar~Baz": {
                    "properties": {
                        "value": {
                            "type": "string"
                        }
                    },
                    "type": "object"
                }
            }
        });

        postprocess_schema(&mut schema, |name| {
            seen.borrow_mut().push(name.to_string());
            (name == "Foo/Bar~Baz").then(|| "https://example.test/foo.schema.json".to_string())
        });

        assert_eq!(seen.into_inner(), vec!["Foo/Bar~Baz"]);
        assert_eq!(
            schema["properties"]["nested"]["$ref"],
            "https://example.test/foo.schema.json#/properties/value"
        );
        assert!(schema.get("$defs").is_none());
    }

    #[test]
    fn postprocess_keeps_escaped_nested_local_definition_reachable() {
        let mut schema = json!({
            "properties": {
                "nested": {
                    "$ref": "#/$defs/Foo~1Bar~0Baz/properties/value"
                }
            },
            "$defs": {
                "Foo/Bar~Baz": {
                    "properties": {
                        "value": {
                            "type": "string"
                        }
                    },
                    "type": "object"
                },
                "Unused": {
                    "type": "boolean"
                }
            }
        });

        postprocess_schema(&mut schema, |_| None);

        assert!(schema["$defs"].get("Foo/Bar~Baz").is_some());
        assert!(schema["$defs"].get("Unused").is_none());
        assert_eq!(
            schema["properties"]["nested"]["$ref"],
            "#/$defs/Foo~1Bar~0Baz/properties/value"
        );
    }

    #[test]
    fn generated_schema_always_applies_canonical_postprocessing() {
        struct CanonicalProbe;

        impl JsonSchema for CanonicalProbe {
            fn schema_name() -> std::borrow::Cow<'static, str> {
                std::borrow::Cow::Borrowed("CanonicalProbe")
            }

            fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
                schemars::json_schema!({
                    "type": "object",
                    "properties": {
                        "trade_date": {"type": "string"},
                        "as_of_t0": {"type": "string"},
                        "amount": {
                            "type": "string",
                            "pattern": SCHEMARS_DECIMAL_PATTERN,
                        },
                    },
                })
            }
        }

        let schema = generated_schema::<CanonicalProbe>(
            "https://example.test/schema/",
            "probe.schema.json",
            "Canonical probe",
            "Exercises canonical schema post-processing.",
        )
        .expect("probe schema generates");

        assert_eq!(
            schema["properties"]["trade_date"]["format"],
            Value::String("date".to_string())
        );
        assert_eq!(
            schema["properties"]["as_of_t0"]["format"],
            Value::String("date".to_string())
        );
        assert_eq!(
            schema["properties"]["amount"]["pattern"],
            Value::String(DECIMAL_PATTERN.to_string())
        );
    }

    #[test]
    fn assemble_schema_preserves_selected_metadata_and_generated_keywords() {
        let metadata = json!({
            "$id": "https://example.test/widget.schema.json",
            "title": "Widget",
            "description": "A stable widget.",
            "examples": [{"name": "example"}],
            "ignored": true
        });
        let generated = json!({
            "type": "object",
            "properties": {"name": {"type": "string"}},
            "required": ["name"],
            "ignored": true
        });

        let assembled = assemble_schema(
            &metadata,
            &generated,
            &["$id", "title", "description", "examples"],
            &["type", "properties", "required"],
        )
        .expect("schema assembles");

        assert_eq!(assembled["$id"], metadata["$id"]);
        assert_eq!(assembled["examples"], metadata["examples"]);
        assert_eq!(assembled["$schema"], JSON_SCHEMA_DIALECT);
        assert_eq!(assembled["properties"], generated["properties"]);
        assert!(assembled.get("ignored").is_none());
    }

    #[test]
    fn write_schema_emits_stable_pretty_json_with_trailing_newline() {
        let directory =
            std::env::temp_dir().join(format!("finstack-core-schema-test-{}", std::process::id()));
        let path = directory.join("nested").join("schema.json");
        let schema = json!({"type": "object", "$schema": JSON_SCHEMA_DIALECT});

        write_schema(&path, &schema).expect("schema writes");
        let first = std::fs::read(&path).expect("schema reads");
        write_schema(&path, &schema).expect("schema rewrites");
        let second = std::fs::read(&path).expect("schema rereads");

        assert_eq!(first, second);
        assert!(first.ends_with(b"\n"));
        std::fs::remove_dir_all(directory).expect("temporary schema directory removes");
    }
}
