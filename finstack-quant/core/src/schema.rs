//! Deterministic JSON Schema assembly helpers.

use schemars::JsonSchema;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use crate::{Error, Result};

/// JSON Schema dialect used by generated Finstack contracts.
pub const JSON_SCHEMA_DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";

/// Stable base URI for shared schema definitions.
pub const COMMON_SCHEMA_BASE: &str = "https://finstack_quant.dev/schemas/common/1/";

/// Return the canonical schema URI for a shared schemars definition.
///
/// # Arguments
///
/// * `name` - Exact generated schemars definition name; unsupported names
///   return `None`.
#[must_use]
pub fn common_definition_uri(name: &str) -> Option<String> {
    let filename = match name {
        "Attributes" => "attributes.schema.json",
        "BusinessDayConvention" => "business_day_convention.schema.json",
        "Currency" => "currency.schema.json",
        "Date" | "DateWire" => "date.schema.json",
        "DayCount" => "day_count.schema.json",
        "Decimal" | "DecimalWire" => "decimal.schema.json",
        "Diagnostic" => "diagnostic.schema.json",
        "Id" => "id.schema.json",
        "Money" => "money.schema.json",
        "Tenor" => "tenor.schema.json",
        "ValidationReport" => "validation_report.schema.json",
        _ => return None,
    };
    Some(format!("{COMMON_SCHEMA_BASE}{filename}"))
}

/// Build a generated schema with stable canonical metadata.
///
/// The default `serde_json::Map` representation serializes keys in
/// lexicographic order, making pretty-printed output deterministic across
/// repeated runs. Validation assertions come directly from `T`; this helper
/// adds only stable document metadata.
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
    let generated = serde_json::to_value(schemars::schema_for!(T))
        .map_err(|error| Error::Internal(format!("serialize generated {title} schema: {error}")))?;
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
    let mut document = Value::Object(output);
    sort_json(&mut document);
    Ok(document)
}

/// One generated JSON Schema artifact owned by a Rust contract type.
///
/// The registry stores only stable metadata and a monomorphized
/// `schema_for!(T)` function. Existing checked-in JSON is never consulted when
/// generating the document.
pub struct SchemaArtifact {
    /// Path relative to the owning crate's manifest directory.
    pub relative_path: &'static str,
    /// Canonical absolute `$id` for the schema document.
    pub id: &'static str,
    /// Stable human-readable title.
    pub title: &'static str,
    /// Stable contract description.
    pub description: &'static str,
    generator: fn(&SchemaArtifact) -> Result<Value>,
    examples: fn() -> Result<Vec<Value>>,
    packager: fn(&mut Value),
}

impl SchemaArtifact {
    /// Register a schema artifact generated directly from `T`.
    ///
    /// # Arguments
    ///
    /// * `relative_path` - Destination path relative to the owning crate.
    /// * `id` - Canonical absolute JSON Schema identifier.
    /// * `title` - Stable human-readable schema title.
    /// * `description` - Stable description of the persisted contract.
    #[must_use]
    pub const fn new<T: JsonSchema>(
        relative_path: &'static str,
        id: &'static str,
        title: &'static str,
        description: &'static str,
    ) -> Self {
        Self {
            relative_path,
            id,
            title,
            description,
            generator: generate_artifact::<T>,
            examples: empty_examples,
            packager: no_op_packager,
        }
    }

    /// Attach deterministic examples serialized from Rust values.
    ///
    /// # Arguments
    ///
    /// * `examples` - Function returning stable JSON examples for this
    ///   artifact. It must not read generated output or use time, randomness,
    ///   or environment-dependent values.
    #[must_use]
    pub const fn with_examples(mut self, examples: fn() -> Result<Vec<Value>>) -> Self {
        self.examples = examples;
        self
    }

    /// Attach a deterministic packaging pass that preserves validation assertions.
    ///
    /// # Arguments
    ///
    /// * `packager` - Function that may externalize equivalent `$defs`
    ///   references and prune definitions made unreachable by that rewrite.
    #[must_use]
    pub const fn with_packager(mut self, packager: fn(&mut Value)) -> Self {
        self.packager = packager;
        self
    }

    fn generate(&self) -> Result<Value> {
        (self.generator)(self)
    }
}

fn empty_examples() -> Result<Vec<Value>> {
    Ok(Vec::new())
}

fn no_op_packager(_: &mut Value) {}

fn generate_artifact<T: JsonSchema>(artifact: &SchemaArtifact) -> Result<Value> {
    let (schema_base, filename) = artifact.id.rsplit_once('/').ok_or_else(|| {
        Error::Internal(format!(
            "schema id must contain a filename: {}",
            artifact.id
        ))
    })?;
    let mut schema = generated_schema::<T>(
        &format!("{schema_base}/"),
        filename,
        artifact.title,
        artifact.description,
    )?;
    (artifact.packager)(&mut schema);
    let examples = (artifact.examples)()?;
    if !examples.is_empty() {
        schema
            .as_object_mut()
            .ok_or_else(|| Error::Internal("generated schema must be an object".to_string()))?
            .insert("examples".to_string(), Value::Array(examples));
    }
    sort_json(&mut schema);
    Ok(schema)
}

/// Generation operation selected by a schema generator CLI.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchemaGenerationMode {
    /// Write expected artifacts and remove stale schemas in the owned root.
    Write,
    /// Compare expected artifacts without mutating the filesystem.
    Check,
    /// Print the deterministic artifact path inventory.
    List,
}

/// Parsed command for a registry-driven schema generator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaGenerationCommand {
    /// Selected generation operation.
    pub mode: SchemaGenerationMode,
    /// Optional root used instead of the crate manifest directory.
    pub output_root: Option<PathBuf>,
}

impl SchemaGenerationCommand {
    /// Parse a schema generator command from process arguments.
    ///
    /// Exactly one of `--write`, `--check`, or `--list` is required.
    /// `--output-root PATH` redirects all crate-relative artifacts, enabling
    /// clean-room reproducibility checks.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`] for missing, conflicting, or unknown
    /// arguments.
    pub fn from_env() -> Result<Self> {
        Self::parse(std::env::args().skip(1))
    }

    /// Parse a schema generator command from an argument iterator.
    ///
    /// # Arguments
    ///
    /// * `arguments` - CLI arguments excluding the executable name.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`] for missing, conflicting, or unknown
    /// arguments.
    pub fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Self> {
        let mut mode = None;
        let mut output_root = None;
        let mut arguments = arguments.into_iter();
        while let Some(argument) = arguments.next() {
            let selected = match argument.as_str() {
                "--write" => Some(SchemaGenerationMode::Write),
                "--check" => Some(SchemaGenerationMode::Check),
                "--list" => Some(SchemaGenerationMode::List),
                "--output-root" => {
                    let path = arguments.next().ok_or_else(|| {
                        Error::Validation("--output-root requires a path".to_string())
                    })?;
                    if output_root.replace(PathBuf::from(path)).is_some() {
                        return Err(Error::Validation(
                            "--output-root may be supplied only once".to_string(),
                        ));
                    }
                    None
                }
                _ => {
                    return Err(Error::Validation(format!(
                        "unknown schema generator argument {argument:?}"
                    )))
                }
            };
            if let Some(selected) = selected {
                if mode.replace(selected).is_some() {
                    return Err(Error::Validation(
                        "choose exactly one of --write, --check, or --list".to_string(),
                    ));
                }
            }
        }
        let mode = mode.ok_or_else(|| {
            Error::Validation("choose one of --write, --check, or --list".to_string())
        })?;
        Ok(Self { mode, output_root })
    }
}

/// Execute a complete registry-driven schema generation operation.
///
/// The expected artifact set is built in memory before any writes occur.
/// Check mode is non-mutating and reports missing, changed, and extra schema
/// files. Write mode removes extras only below `owned_root` after both the root
/// and every artifact path have passed strict relative-path validation.
///
/// # Arguments
///
/// * `manifest_dir` - Owning crate's absolute Cargo manifest directory.
/// * `owned_root` - Crate-relative schema directory exclusively owned by this
///   registry, such as `schemas/cashflow`.
/// * `artifacts` - Complete set of generated schemas for the owned root.
/// * `command` - Explicit generation mode and optional output root.
///
/// # Errors
///
/// Returns an error for invalid paths, duplicate registry entries, generation
/// failures, filesystem failures, or check-mode drift.
pub fn run_schema_generator(
    manifest_dir: &Path,
    owned_root: &Path,
    artifacts: &[SchemaArtifact],
    command: &SchemaGenerationCommand,
) -> Result<()> {
    validate_owned_root(owned_root)?;
    let base = command.output_root.as_deref().unwrap_or(manifest_dir);
    let mut expected = BTreeMap::new();
    let mut ids = BTreeSet::new();
    for artifact in artifacts {
        let relative_path = Path::new(artifact.relative_path);
        validate_artifact_path(relative_path, owned_root)?;
        if !ids.insert(artifact.id) {
            return Err(Error::Validation(format!(
                "duplicate schema id in registry: {}",
                artifact.id
            )));
        }
        let rendered = render_schema(&artifact.generate()?)?;
        if expected
            .insert(relative_path.to_path_buf(), rendered)
            .is_some()
        {
            return Err(Error::Validation(format!(
                "duplicate schema path in registry: {}",
                relative_path.display()
            )));
        }
    }

    if command.mode == SchemaGenerationMode::List {
        for path in expected.keys() {
            println!("{}", path.display());
        }
        return Ok(());
    }

    let owned_directory = base.join(owned_root);
    let actual = schema_files(&owned_directory, base)?;
    let expected_paths = expected.keys().cloned().collect::<BTreeSet<_>>();
    let extra = actual
        .difference(&expected_paths)
        .cloned()
        .collect::<Vec<_>>();

    match command.mode {
        SchemaGenerationMode::Check => {
            let mut drift = Vec::new();
            for (relative_path, rendered) in &expected {
                let path = base.join(relative_path);
                match std::fs::read(&path) {
                    Ok(actual) if actual == *rendered => {}
                    Ok(_) => drift.push(format!("changed {}", relative_path.display())),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        drift.push(format!("missing {}", relative_path.display()));
                    }
                    Err(error) => {
                        return Err(Error::Internal(format!(
                            "read schema {}: {error}",
                            path.display()
                        )))
                    }
                }
            }
            drift.extend(extra.iter().map(|path| format!("extra {}", path.display())));
            if drift.is_empty() {
                Ok(())
            } else {
                Err(Error::Validation(format!(
                    "schema artifacts are not current:\n{}",
                    drift.join("\n")
                )))
            }
        }
        SchemaGenerationMode::Write => {
            for (relative_path, rendered) in expected {
                let path = base.join(relative_path);
                if std::fs::read(&path).ok().as_deref() == Some(rendered.as_slice()) {
                    continue;
                }
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|error| {
                        Error::Internal(format!(
                            "create schema directory {}: {error}",
                            parent.display()
                        ))
                    })?;
                }
                std::fs::write(&path, rendered).map_err(|error| {
                    Error::Internal(format!("write schema {}: {error}", path.display()))
                })?;
                println!("updated {}", path.display());
            }
            for relative_path in extra {
                let path = base.join(&relative_path);
                std::fs::remove_file(&path).map_err(|error| {
                    Error::Internal(format!("remove stale schema {}: {error}", path.display()))
                })?;
                println!("removed stale {}", path.display());
            }
            remove_empty_directories(&owned_directory, &owned_directory)?;
            Ok(())
        }
        SchemaGenerationMode::List => Ok(()),
    }
}

/// Render a JSON value with recursively sorted object keys, UTF-8 encoding,
/// two-space indentation, LF line endings, and one final newline.
///
/// # Arguments
///
/// * `value` - JSON value to render deterministically.
///
/// # Errors
///
/// Returns [`Error::Internal`] if serialization fails.
pub fn deterministic_json_bytes(value: &Value) -> Result<Vec<u8>> {
    render_schema(value)
}

fn render_schema(schema: &Value) -> Result<Vec<u8>> {
    let mut schema = schema.clone();
    sort_json(&mut schema);
    let mut json = serde_json::to_vec_pretty(&schema)
        .map_err(|error| Error::Internal(format!("serialize schema: {error}")))?;
    json.push(b'\n');
    Ok(json)
}

fn sort_json(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for child in map.values_mut() {
                sort_json(child);
            }
            let sorted = std::mem::take(map).into_iter().collect::<BTreeMap<_, _>>();
            map.extend(sorted);
        }
        Value::Array(items) => {
            for item in items {
                sort_json(item);
            }
        }
        _ => {}
    }
}

fn validate_relative_path(path: &Path, label: &str) -> Result<()> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(Error::Validation(format!(
            "{label} must be a non-empty normalized relative path: {}",
            path.display()
        )));
    }
    Ok(())
}

fn validate_owned_root(owned_root: &Path) -> Result<()> {
    validate_relative_path(owned_root, "owned schema root")?;
    if owned_root.components().next() != Some(Component::Normal("schemas".as_ref()))
        || owned_root.components().count() < 2
    {
        return Err(Error::Validation(format!(
            "owned schema root must be a specific directory below schemas/: {}",
            owned_root.display()
        )));
    }
    Ok(())
}

fn validate_artifact_path(path: &Path, owned_root: &Path) -> Result<()> {
    validate_relative_path(path, "schema artifact path")?;
    if !path.starts_with(owned_root)
        || path.extension().and_then(|extension| extension.to_str()) != Some("json")
        || !path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".schema.json"))
    {
        return Err(Error::Validation(format!(
            "schema artifact must be a .schema.json file below {}: {}",
            owned_root.display(),
            path.display()
        )));
    }
    Ok(())
}

fn schema_files(directory: &Path, base: &Path) -> Result<BTreeSet<PathBuf>> {
    let mut files = BTreeSet::new();
    if !directory.exists() {
        return Ok(files);
    }
    collect_schema_files(directory, base, &mut files)?;
    Ok(files)
}

fn collect_schema_files(
    directory: &Path,
    base: &Path,
    files: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    let entries = std::fs::read_dir(directory).map_err(|error| {
        Error::Internal(format!(
            "read schema directory {}: {error}",
            directory.display()
        ))
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            Error::Internal(format!(
                "read schema entry in {}: {error}",
                directory.display()
            ))
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_schema_files(&path, base, files)?;
        } else if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".schema.json"))
        {
            let relative = path.strip_prefix(base).map_err(|error| {
                Error::Internal(format!("resolve schema path {}: {error}", path.display()))
            })?;
            files.insert(relative.to_path_buf());
        }
    }
    Ok(())
}

fn remove_empty_directories(directory: &Path, owned_root: &Path) -> Result<bool> {
    if !directory.exists() {
        return Ok(true);
    }
    for entry in std::fs::read_dir(directory).map_err(|error| {
        Error::Internal(format!(
            "read schema directory {}: {error}",
            directory.display()
        ))
    })? {
        let path = entry
            .map_err(|error| {
                Error::Internal(format!(
                    "read schema entry in {}: {error}",
                    directory.display()
                ))
            })?
            .path();
        if path.is_dir() {
            remove_empty_directories(&path, owned_root)?;
        }
    }
    let empty = std::fs::read_dir(directory)
        .map_err(|error| {
            Error::Internal(format!(
                "read schema directory {}: {error}",
                directory.display()
            ))
        })?
        .next()
        .is_none();
    if empty && directory != owned_root {
        std::fs::remove_dir(directory).map_err(|error| {
            Error::Internal(format!(
                "remove empty schema directory {}: {error}",
                directory.display()
            ))
        })?;
    }
    Ok(empty)
}

/// Externalize selected local definitions without changing validation assertions.
///
/// The packaging traversal replaces selected local definition references and
/// removes definitions that are no longer reachable from the document root.
///
/// # Arguments
///
/// * `schema` - Mutable derived schema document to package.
/// * `external_ref` - Resolver called with each local `$defs` name. Returning
///   a URI externalizes that definition and preserves any nested JSON Pointer
///   suffix as a fragment; returning `None` keeps it local. Definition names
///   are JSON Pointer-decoded before the resolver is called.
pub fn externalize_schema_definitions(
    schema: &mut Value,
    external_ref: impl Fn(&str) -> Option<String>,
) {
    externalize_refs(schema, &external_ref);
    prune_unreachable_defs(schema);
}

fn externalize_refs(value: &mut Value, external_ref: &impl Fn(&str) -> Option<String>) {
    match value {
        Value::Object(map) => {
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
                externalize_refs(child, external_ref);
            }
        }
        Value::Array(items) => {
            for item in items {
                externalize_refs(item, external_ref);
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
    fn packaging_externalizes_refs_and_prunes_defs() {
        let mut schema = json!({
            "properties": {
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

        externalize_schema_definitions(&mut schema, |name| {
            (name == "Money").then(|| "https://example.test/money.schema.json".to_string())
        });

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
    fn packaging_decodes_nested_definition_name_and_preserves_suffix() {
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

        externalize_schema_definitions(&mut schema, |name| {
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
    fn packaging_keeps_escaped_nested_local_definition_reachable() {
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

        externalize_schema_definitions(&mut schema, |_| None);

        assert!(schema["$defs"].get("Foo/Bar~Baz").is_some());
        assert!(schema["$defs"].get("Unused").is_none());
        assert_eq!(
            schema["properties"]["nested"]["$ref"],
            "#/$defs/Foo~1Bar~0Baz/properties/value"
        );
    }

    #[test]
    fn generated_schema_preserves_derived_assertions() {
        #[allow(dead_code)]
        #[derive(JsonSchema)]
        struct DerivedProbe {
            value: String,
        }

        let raw = serde_json::to_value(schemars::schema_for!(DerivedProbe))
            .expect("derived schema serializes");
        let schema = generated_schema::<DerivedProbe>(
            "https://example.test/schema/",
            "probe.schema.json",
            "Derived probe",
            "Exercises assertion-preserving schema metadata.",
        )
        .expect("probe schema generates");

        assert_eq!(schema["type"], raw["type"]);
        assert_eq!(schema["properties"], raw["properties"]);
        assert_eq!(schema["required"], raw["required"]);
    }
}
