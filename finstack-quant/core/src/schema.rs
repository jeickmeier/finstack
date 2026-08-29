//! Deterministic JSON Schema assembly helpers.

#[cfg(feature = "json-schema")]
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use crate::{Error, Result};

/// JSON Schema dialect used by generated Finstack contracts.
pub const JSON_SCHEMA_DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";

/// Stable base URI for shared schema definitions.
pub const COMMON_SCHEMA_BASE: &str = "https://finstack_quant.dev/schemas/common/1/";

/// Runtime serde contract that may own a generated JSON Schema artifact.
///
/// Requiring all three traits prevents schema-only shadow roots from entering
/// a registry: every root must be serializable, deserializable, and derivable.
pub trait SerdeSchema: Serialize + DeserializeOwned + JsonSchema {}

impl<T> SerdeSchema for T where T: Serialize + DeserializeOwned + JsonSchema {}

/// Typed common definitions eligible for assertion-checked externalization.
pub const COMMON_SCHEMA_DEFINITIONS: &[ExternalSchemaDefinition] = &[
    ExternalSchemaDefinition::new::<crate::types::Attributes>(
        "Attributes",
        "https://finstack_quant.dev/schemas/common/1/attributes.schema.json",
    ),
    ExternalSchemaDefinition::new::<crate::dates::BusinessDayConvention>(
        "BusinessDayConvention",
        "https://finstack_quant.dev/schemas/common/1/business_day_convention.schema.json",
    ),
    ExternalSchemaDefinition::new::<crate::currency::Currency>(
        "Currency",
        "https://finstack_quant.dev/schemas/common/1/currency.schema.json",
    ),
    ExternalSchemaDefinition::new::<crate::wire::DateWire>(
        "Date",
        "https://finstack_quant.dev/schemas/common/1/date.schema.json",
    ),
    ExternalSchemaDefinition::new::<crate::wire::DateWire>(
        "DateWire",
        "https://finstack_quant.dev/schemas/common/1/date.schema.json",
    ),
    ExternalSchemaDefinition::new::<crate::dates::DayCount>(
        "DayCount",
        "https://finstack_quant.dev/schemas/common/1/day_count.schema.json",
    ),
    ExternalSchemaDefinition::new::<crate::wire::DecimalWire>(
        "Decimal",
        "https://finstack_quant.dev/schemas/common/1/decimal.schema.json",
    ),
    ExternalSchemaDefinition::new::<crate::wire::DecimalWire>(
        "DecimalWire",
        "https://finstack_quant.dev/schemas/common/1/decimal.schema.json",
    ),
    ExternalSchemaDefinition::new::<crate::contract::Diagnostic>(
        "Diagnostic",
        "https://finstack_quant.dev/schemas/common/1/diagnostic.schema.json",
    ),
    ExternalSchemaDefinition::new::<crate::types::InstrumentId>(
        "Id",
        "https://finstack_quant.dev/schemas/common/1/id.schema.json",
    ),
    ExternalSchemaDefinition::new::<crate::money::Money>(
        "Money",
        "https://finstack_quant.dev/schemas/common/1/money.schema.json",
    ),
    ExternalSchemaDefinition::new::<crate::dates::Tenor>(
        "Tenor",
        "https://finstack_quant.dev/schemas/common/1/tenor.schema.json",
    ),
    ExternalSchemaDefinition::new::<crate::contract::ValidationReport>(
        "ValidationReport",
        "https://finstack_quant.dev/schemas/common/1/validation_report.schema.json",
    ),
];

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
pub fn generated_schema<T: SerdeSchema>(
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

/// Direction of travel for a generated contract.
///
/// A consumer choosing what to send or how to read a reply needs this before it
/// needs anything else, and it cannot be derived from the schema document.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SchemaKind {
    /// A root document callers author and submit.
    Input,
    /// A root document the library emits and callers read.
    Output,
    /// A reusable definition referenced by roots, not submitted on its own.
    Component,
}

impl SchemaKind {
    /// Return the stable snake_case label used in the schema index.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Output => "output",
            Self::Component => "component",
        }
    }
}

/// One generated JSON Schema artifact owned by a Rust contract type.
///
/// The registry stores only stable metadata and a monomorphized
/// `schema_for!(T)` function. Existing checked-in JSON is never consulted when
/// generating the document.
#[derive(Clone, Copy)]
pub struct SchemaArtifact {
    /// Path relative to the owning crate's manifest directory.
    pub relative_path: &'static str,
    /// Canonical absolute `$id` for the schema document.
    pub id: &'static str,
    /// Stable human-readable title.
    pub title: &'static str,
    /// Stable contract description.
    pub description: &'static str,
    /// One-line summary for the schema index, or empty to reuse `description`.
    pub summary: &'static str,
    /// Whether callers author this document, read it, or only reference it.
    pub kind: SchemaKind,
    generator: fn(&SchemaArtifact) -> Result<Value>,
    examples: fn() -> Result<Vec<Value>>,
    packager: fn(&mut Value) -> Result<()>,
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
    pub const fn new<T: SerdeSchema>(
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
            summary: "",
            kind: SchemaKind::Component,
            generator: generate_artifact::<T>,
            examples: empty_examples,
            packager: no_op_packager,
        }
    }

    /// Declare this artifact a root document callers author or read.
    ///
    /// Registration defaults to [`SchemaKind::Component`], so only roots need
    /// to say so.
    ///
    /// # Arguments
    ///
    /// * `kind` - Direction of travel for this contract.
    #[must_use]
    pub const fn with_kind(mut self, kind: SchemaKind) -> Self {
        self.kind = kind;
        self
    }

    /// Attach a one-line summary for the schema index.
    ///
    /// Titles alone do not distinguish sibling contracts — every one of the
    /// seventy instrument artifacts shares a single generated description — so
    /// the index carries a per-artifact line instead.
    ///
    /// # Arguments
    ///
    /// * `summary` - One sentence stating what this contract governs.
    #[must_use]
    pub const fn with_summary(mut self, summary: &'static str) -> Self {
        self.summary = summary;
        self
    }

    /// Return the index summary, falling back to the contract description.
    #[must_use]
    pub const fn index_summary(&self) -> &'static str {
        if self.summary.is_empty() {
            self.description
        } else {
            self.summary
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
    pub const fn with_packager(mut self, packager: fn(&mut Value) -> Result<()>) -> Self {
        self.packager = packager;
        self
    }

    /// Render this artifact exactly as the checked-in file is written.
    ///
    /// This is the single rendering path: registry metadata, the packager, the
    /// single-branch-union collapse, examples and key sorting all apply. Tests,
    /// bindings and tools must call this rather than `generated_schema`, which
    /// produces only the raw derived document and will drift from the artifact.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Internal`] if the schema or its examples cannot be
    /// built.
    pub fn generate(&self) -> Result<Value> {
        (self.generator)(self)
    }
}

fn empty_examples() -> Result<Vec<Value>> {
    Ok(Vec::new())
}

fn no_op_packager(_: &mut Value) -> Result<()> {
    Ok(())
}

/// Keywords that annotate a schema without constraining any instance.
///
/// A node carrying only these plus `oneOf` contributes no assertion of its own,
/// which is what makes collapsing its single branch safe.
const ANNOTATION_KEYWORDS: &[&str] = &[
    "$comment",
    "default",
    "deprecated",
    "description",
    "examples",
    "readOnly",
    "title",
    "writeOnly",
];

/// Replace every single-branch `oneOf` with the branch itself.
///
/// schemars emits a one-variant enum as `{"oneOf": [branch]}`. That is
/// logically identical to `branch`, but it is not equivalent in *diagnostics*:
/// a validator reports a failing `oneOf` at the union node, so an error inside
/// the branch surfaces as "no branch matched" against the whole subtree instead
/// of pointing at the offending field. Seventy single-instrument schemas wrap
/// their payload this way, which is why a bond missing `maturity` reports at
/// `/instrument` with the entire instance attached.
///
/// The rewrite only fires when the wrapper carries no assertion of its own -
/// every sibling key must be in [`ANNOTATION_KEYWORDS`] - so the collapsed node
/// asserts exactly what the branch asserted. Sibling annotations win over the
/// branch's, keeping the field-level documentation that the wrapper carries.
fn collapse_single_branch_unions(value: &mut Value) {
    match value {
        Value::Object(object) => {
            for nested in object.values_mut() {
                collapse_single_branch_unions(nested);
            }

            let collapsible = object
                .get("oneOf")
                .and_then(Value::as_array)
                .is_some_and(|branches| branches.len() == 1 && branches[0].is_object())
                && object
                    .keys()
                    .all(|key| key == "oneOf" || ANNOTATION_KEYWORDS.contains(&key.as_str()));
            if !collapsible {
                return;
            }

            let Some(Value::Array(mut branches)) = object.remove("oneOf") else {
                return;
            };
            let Some(Value::Object(branch)) = branches.pop() else {
                return;
            };
            for (key, nested) in branch {
                object.entry(key).or_insert(nested);
            }
        }
        Value::Array(items) => {
            for item in items {
                collapse_single_branch_unions(item);
            }
        }
        _ => {}
    }
}

fn generate_artifact<T: SerdeSchema>(artifact: &SchemaArtifact) -> Result<Value> {
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
    (artifact.packager)(&mut schema)?;
    collapse_single_branch_unions(&mut schema);
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
        let rendered = deterministic_json_bytes(&artifact.generate()?)?;
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

/// Rustdoc sections that describe the Rust API rather than the wire contract.
///
/// Kept out of the projected form: a model filling in a payload gains nothing
/// from a Rust usage example, and `# Examples` alone accounts for 346 of the
/// 1,578 headings in the corpus. Domain sections — `# References`,
/// `# Standards Reference`, `# Market Conventions` and the like — are the
/// grounding that makes the schema readable, and are deliberately kept.
const RUSTDOC_SECTIONS_TO_DROP: &[&str] = &[
    "Arguments",
    "Errors",
    "Example",
    "Examples",
    "Panics",
    "Safety",
    "See Also",
    "Thread Safety",
    "Type Parameters",
];

/// Keywords a projected schema drops because provider subsets ignore or reject them.
const NON_PORTABLE_KEYWORDS: &[&str] = &[
    "$comment",
    "default",
    "deprecated",
    "format",
    "readOnly",
    "uniqueItems",
    "writeOnly",
];

/// Which projection passes to apply.
///
/// Every pass defaults on; turn one off to isolate its effect when measuring.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LlmProfile {
    /// Pull externally referenced documents into `$defs` so nothing is fetched.
    pub inline_references: bool,
    /// Rewrite all-string-`const` unions as a flat `enum`.
    pub flatten_const_unions: bool,
    /// Strip Rust API prose from `description` text.
    pub trim_descriptions: bool,
    /// Close objects, require every declared property, and drop non-portable keywords.
    pub strict_shape: bool,
    /// Largest referenced document to inline, in compact bytes.
    ///
    /// Inlining is what makes a schema self-contained, but a reference to a
    /// large document turns a small contract into a large one — a portfolio
    /// bundle that names the instrument union inherits all seventy branches.
    /// Anything over this budget becomes a handle instead: an opaque string
    /// carrying [`RESOLVES_FROM_KEYWORD`] so the caller knows which contract to
    /// fetch and validate separately.
    pub max_inline_bytes: usize,
    /// Longest description to keep, in characters. Zero disables the cap.
    ///
    /// Rustdoc prose is written for a reader deciding *whether* to use a type;
    /// a payload author already decided and needs to know only what the field
    /// means. The leading paragraph carries that, and the elaboration after it
    /// is the single largest remaining cost once sections and code blocks are
    /// gone — measured at 53% to 65% of every projected instrument's bytes.
    pub max_description_chars: usize,
}

/// Extension keyword naming the contract a handle stands in for.
pub const RESOLVES_FROM_KEYWORD: &str = "x-finstack-resolves-from";

/// Default ceiling for inlining a referenced document, in compact bytes.
///
/// 16 KiB is not a tuning knob; it is the gap in the corpus between the two
/// kinds of shared document. The largest primitive a payload author needs in
/// front of them is `day_count` at 13.9 KB (`money` is 11.3 KB, `currency`
/// 11.2 KB). The smallest bag they almost never fill in is
/// `metric_pricing_overrides` at 20.2 KB, followed by
/// `instrument_pricing_overrides` at 55.4 KB — and those two drag in the whole
/// pricing-model configuration universe behind them.
///
/// Measured across all 109 artifacts, moving this ceiling from 64 KiB to
/// 16 KiB takes the median projected instrument from 80.4 KB to 32.4 KB and
/// the count fitting in 8k tokens from 32 to 90.
pub const DEFAULT_MAX_INLINE_BYTES: usize = 16 * 1024;

/// Default ceiling for one description, in characters.
///
/// 240 characters is about two sentences — enough for the summary line plus a
/// unit or convention note, which is what a payload author acts on. Anything
/// longer is elaboration a reader can fetch from the canonical artifact.
pub const DEFAULT_MAX_DESCRIPTION_CHARS: usize = 240;

impl Default for LlmProfile {
    fn default() -> Self {
        Self {
            inline_references: true,
            flatten_const_unions: true,
            trim_descriptions: true,
            strict_shape: true,
            max_inline_bytes: DEFAULT_MAX_INLINE_BYTES,
            max_description_chars: DEFAULT_MAX_DESCRIPTION_CHARS,
        }
    }
}

/// Project a published schema into a form a language model can consume.
///
/// The published artifacts are tuned for validation and rustdoc parity. That
/// makes them correct and unusable as tool schemas: they reference an
/// unresolvable host, spend roughly half their bytes on Rust prose, and express
/// every unit enum as a union of `const` branches. This rewrites all three,
/// plus the shape differences that provider subsets reject.
///
/// # This is not a validator
///
/// The result is simultaneously **stricter** than the runtime contract (every
/// property is required, with optionality moved into the type) and **looser**
/// (tuples become fixed-length arrays, non-portable assertions are dropped).
/// Validate against the artifact from
/// [`SchemaArtifact::generate`](SchemaArtifact::generate) — never against this.
///
/// # Arguments
///
/// * `schema` - A generated artifact, as written to disk.
/// * `resolve` - Maps an absolute `$id` to that document. Callers that own the
///   whole corpus can look up their registries; returning `None` leaves the
///   reference in place rather than failing.
/// * `profile` - Which passes to run.
///
/// # Errors
///
/// Returns [`Error::Internal`] if the document is not a JSON object.
pub fn project_llm(
    schema: &Value,
    resolve: &dyn Fn(&str) -> Option<Value>,
    profile: &LlmProfile,
) -> Result<Value> {
    if !schema.is_object() {
        return Err(Error::Internal(
            "projected schema must be a JSON object".to_string(),
        ));
    }
    let mut projected = schema.clone();

    if profile.inline_references {
        inline_external_references(&mut projected, resolve, profile.max_inline_bytes);
    }
    if profile.max_inline_bytes > 0 {
        substitute_oversized_local_definitions(&mut projected, profile.max_inline_bytes);
        prune_unreachable_definitions(&mut projected);
    }
    if profile.flatten_const_unions {
        flatten_const_unions(&mut projected);
    }
    if profile.trim_descriptions {
        trim_descriptions(&mut projected, profile.max_description_chars);
    }
    if profile.strict_shape {
        apply_strict_shape(&mut projected);
    }

    sort_json(&mut projected);
    Ok(projected)
}

/// Turn an absolute `$id` into a `$defs` key.
fn definition_name_for(id: &str) -> String {
    let stem = id
        .rsplit('/')
        .next()
        .unwrap_or(id)
        .trim_end_matches(".schema.json");
    let mut name = String::new();
    let mut capitalize = true;
    for character in stem.chars() {
        if character == '_' || character == '-' || character == '.' {
            capitalize = true;
        } else if capitalize {
            name.extend(character.to_uppercase());
            capitalize = false;
        } else {
            name.push(character);
        }
    }
    if name.is_empty() {
        "External".to_string()
    } else {
        name
    }
}

/// Collect every absolute `$ref` target in a document.
fn collect_external_refs(node: &Value, found: &mut BTreeSet<String>) {
    match node {
        Value::Object(object) => {
            if let Some(Value::String(target)) = object.get("$ref") {
                if !target.starts_with('#') {
                    found.insert(target.clone());
                }
            }
            for nested in object.values() {
                collect_external_refs(nested, found);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_external_refs(item, found);
            }
        }
        _ => {}
    }
}

/// Rewrite absolute `$ref`s to local ones, given the id-to-name mapping.
fn rewrite_refs(node: &mut Value, names: &BTreeMap<String, String>) {
    match node {
        Value::Object(object) => {
            if let Some(Value::String(target)) = object.get("$ref") {
                if let Some(name) = names.get(target) {
                    let local = format!("#/$defs/{name}");
                    object.insert("$ref".to_string(), Value::String(local));
                }
            }
            for nested in object.values_mut() {
                rewrite_refs(nested, names);
            }
        }
        Value::Array(items) => {
            for item in items {
                rewrite_refs(item, names);
            }
        }
        _ => {}
    }
}

/// Pull every resolvable external reference into local `$defs`.
///
/// Definitions are added once and shared, and each newly inlined document is
/// scanned in turn, so a chain of references resolves in one pass. Unresolvable
/// targets are left untouched — a missing shared definition should surface as a
/// dangling reference, not as a silently different contract.
fn inline_external_references(
    schema: &mut Value,
    resolve: &dyn Fn(&str) -> Option<Value>,
    max_inline_bytes: usize,
) {
    let mut names: BTreeMap<String, String> = BTreeMap::new();
    let mut bodies: BTreeMap<String, Value> = BTreeMap::new();
    let mut pending = BTreeSet::new();
    collect_external_refs(schema, &mut pending);

    while let Some(id) = pending.iter().next().cloned() {
        pending.remove(&id);
        if names.contains_key(&id) {
            continue;
        }
        let Some(mut document) = resolve(&id) else {
            continue;
        };

        // Too large to carry: stand it in as a handle rather than let one
        // reference dominate the projected document.
        let measured = serde_json::to_vec(&document).map(|bytes| bytes.len());
        if measured.is_ok_and(|size| size > max_inline_bytes) {
            let summary = document
                .get("description")
                .and_then(Value::as_str)
                .map(str::to_string);
            let mut handle = Map::new();
            // Keep the target's own top-level type. Forcing a string here would
            // be wrong wherever the reference stands for an object, so the
            // handle stays shape-correct and merely permissive about contents.
            if let Some(kind) = document.get("type").cloned() {
                handle.insert("type".to_string(), kind);
            }
            if handle.get("type").is_some_and(|kind| kind == "object") {
                handle.insert("additionalProperties".to_string(), Value::Bool(true));
            }
            handle.insert(RESOLVES_FROM_KEYWORD.to_string(), Value::String(id.clone()));
            handle.insert(
                "description".to_string(),
                Value::String(match summary {
                    Some(text) => format!(
                        "{text}\n\nToo large to inline here: build this against `{id}` and \
                         validate it separately."
                    ),
                    None => format!(
                        "Value matching `{id}`; build and validate it against that contract."
                    ),
                }),
            );
            let mut candidate = definition_name_for(&id);
            while bodies.contains_key(&candidate) {
                candidate.push('_');
            }
            names.insert(id, candidate.clone());
            bodies.insert(candidate, Value::Object(handle));
            continue;
        }

        // The inlined body becomes a definition, so document-level identity and
        // its own examples are dropped; nested `$defs` are hoisted alongside.
        let mut nested_defs = Map::new();
        if let Some(object) = document.as_object_mut() {
            for key in ["$id", "$schema", "examples"] {
                object.remove(key);
            }
            if let Some(Value::Object(defs)) = object.remove("$defs") {
                nested_defs = defs;
            }
        }

        let mut candidate = definition_name_for(&id);
        while bodies.contains_key(&candidate) {
            candidate.push('_');
        }
        collect_external_refs(&document, &mut pending);
        for value in nested_defs.values() {
            collect_external_refs(value, &mut pending);
        }
        names.insert(id, candidate.clone());
        bodies.insert(candidate, document);
        for (key, value) in nested_defs {
            bodies.entry(key).or_insert(value);
        }
    }

    if bodies.is_empty() {
        return;
    }
    rewrite_refs(schema, &names);
    for body in bodies.values_mut() {
        rewrite_refs(body, &names);
    }
    let Some(object) = schema.as_object_mut() else {
        return;
    };
    let defs = object
        .entry("$defs".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if let Some(defs) = defs.as_object_mut() {
        for (name, body) in bodies {
            defs.entry(name).or_insert(body);
        }
    }
}

/// Replace locally-defined definitions that exceed the inline budget with handles.
///
/// [`inline_external_references`] can only substitute a handle for a document
/// reached by `$ref`. A container instrument — a basket, a levered equity
/// wrapper — embeds the *whole* instrument union as local `$defs` instead, so
/// the budget never applies and one contract carries all seventy branches:
/// measured at 302 definitions and 393 KB for `basket`. Standing the oversized
/// definition down to a handle makes its dependents unreachable, which
/// [`prune_unreachable_definitions`] then removes.
fn substitute_oversized_local_definitions(schema: &mut Value, max_inline_bytes: usize) {
    let Some(defs) = schema.get("$defs").and_then(Value::as_object) else {
        return;
    };
    let oversized: Vec<String> = defs
        .iter()
        .filter(|(_, body)| {
            serde_json::to_vec(body).is_ok_and(|bytes| bytes.len() > max_inline_bytes)
        })
        .map(|(name, _)| name.clone())
        .collect();
    if oversized.is_empty() {
        return;
    }

    let Some(defs) = schema.get_mut("$defs").and_then(Value::as_object_mut) else {
        return;
    };
    for name in oversized {
        let Some(body) = defs.get(&name) else {
            continue;
        };
        let summary = body
            .get("description")
            .and_then(Value::as_str)
            .map(str::to_string);
        let mut handle = Map::new();
        // Keep the definition's own top-level type where it declares one, so
        // the handle stays shape-correct and is merely permissive about
        // contents.
        if let Some(kind) = body.get("type").cloned() {
            handle.insert("type".to_string(), kind);
        }
        if handle.get("type").is_some_and(|kind| kind == "object") {
            handle.insert("additionalProperties".to_string(), Value::Bool(true));
        }
        handle.insert(
            "description".to_string(),
            Value::String(match summary {
                Some(text) => format!(
                    "{text}\n\nToo large to inline here: build this sub-document on its own \
                     and validate it against the canonical contract."
                ),
                None => format!(
                    "A `{name}` value. Too large to inline here: build this sub-document on \
                     its own and validate it against the canonical contract."
                ),
            }),
        );
        defs.insert(name, Value::Object(handle));
    }
}

/// Drop definitions no longer reachable from the document root.
///
/// Standing an oversized definition down to a handle severs the only path to
/// everything it referenced. Those bodies are then pure cost: unreachable from
/// the root, unusable by a generator, and indistinguishable from the reachable
/// ones to a reader.
fn prune_unreachable_definitions(schema: &mut Value) {
    let Some(defs) = schema.get("$defs").and_then(Value::as_object) else {
        return;
    };
    if defs.is_empty() {
        return;
    }

    let mut reachable: BTreeSet<String> = BTreeSet::new();
    let mut frontier: Vec<String> = Vec::new();
    let mut from_root = BTreeSet::new();
    for (key, value) in schema.as_object().into_iter().flatten() {
        if key != "$defs" {
            collect_local_definition_refs(value, &mut from_root);
        }
    }
    frontier.extend(from_root);

    while let Some(name) = frontier.pop() {
        if !reachable.insert(name.clone()) {
            continue;
        }
        if let Some(body) = defs.get(&name) {
            let mut nested = BTreeSet::new();
            collect_local_definition_refs(body, &mut nested);
            frontier.extend(nested);
        }
    }

    let Some(defs) = schema.get_mut("$defs").and_then(Value::as_object_mut) else {
        return;
    };
    defs.retain(|name, _| reachable.contains(name));
    if defs.is_empty() {
        if let Some(object) = schema.as_object_mut() {
            object.remove("$defs");
        }
    }
}

/// Collect every `#/$defs/<name>` target named anywhere in a node.
fn collect_local_definition_refs(node: &Value, found: &mut BTreeSet<String>) {
    match node {
        Value::Object(object) => {
            if let Some(Value::String(target)) = object.get("$ref") {
                if let Some(name) = target.strip_prefix("#/$defs/") {
                    found.insert(name.to_string());
                }
            }
            for nested in object.values() {
                collect_local_definition_refs(nested, found);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_local_definition_refs(item, found);
            }
        }
        _ => {}
    }
}

/// Rewrite `oneOf` unions of string `const`s as a flat `enum`.
///
/// schemars emits a unit enum as one branch per variant so each keeps its doc
/// comment. That is the single largest avoidable cost in the corpus, and a flat
/// `enum` is what constrained decoding handles natively. Variant documentation
/// is folded into the parent description when it is short enough to be worth
/// the tokens.
fn flatten_const_unions(node: &mut Value) {
    match node {
        Value::Object(object) => {
            for nested in object.values_mut() {
                flatten_const_unions(nested);
            }

            let Some(branches) = object.get("oneOf").and_then(Value::as_array) else {
                return;
            };
            if branches.len() < 2 {
                return;
            }
            let mut values = Vec::with_capacity(branches.len());
            let mut notes = Vec::new();
            for branch in branches {
                let Some(branch) = branch.as_object() else {
                    return;
                };
                let Some(Value::String(constant)) = branch.get("const") else {
                    return;
                };
                if branch
                    .keys()
                    .any(|key| !matches!(key.as_str(), "const" | "type" | "description" | "title"))
                {
                    return;
                }
                if let Some(Value::String(note)) = branch.get("description") {
                    if let Some(first) = note.lines().find(|line| !line.trim().is_empty()) {
                        let mut gloss = format!("`{constant}`: {}", first.trim());
                        // The variant's standards citation is the part that
                        // tells a payload author whether this is the spelling
                        // their contract actually calls for, so it rides along
                        // with the summary rather than being dropped with the
                        // rest of the body.
                        let citations = collect_citation_lines(note);
                        if !citations.is_empty() {
                            gloss.push_str(" (");
                            gloss.push_str(&citations.join("; "));
                            gloss.push(')');
                        }
                        notes.push(gloss);
                    }
                }
                values.push(Value::String(constant.clone()));
            }

            // A long variant gloss costs more than it teaches; ISO currency
            // codes are the case that made this threshold necessary. Glosses
            // carrying a standards citation are exempt: a wrong day count is a
            // priced error, and the citation is what prevents it.
            const MAX_FOLDED_NOTE_BYTES: usize = 600;
            let carries_citation = notes.iter().any(|note| mentions_a_standard(note));
            let folded = notes.join(" · ");
            object.remove("oneOf");
            object.insert("type".to_string(), Value::String("string".to_string()));
            object.insert("enum".to_string(), Value::Array(values));
            if !folded.is_empty() && (carries_citation || folded.len() <= MAX_FOLDED_NOTE_BYTES) {
                let existing = object
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let combined = if existing.is_empty() {
                    folded
                } else {
                    format!("{existing}\n\n{folded}")
                };
                object.insert("description".to_string(), Value::String(combined));
            }
        }
        Value::Array(items) => {
            for item in items {
                flatten_const_unions(item);
            }
        }
        _ => {}
    }
}

/// Pull the standards-citation lines out of a rustdoc body, stripped of markup.
///
/// Citations are written as Markdown list items under a reference heading, for
/// example `- **ISDA**: 2006 ISDA Definitions, Section 4.16(d)`. The bullet and
/// emphasis carry no meaning once the text is folded into a one-line gloss.
fn collect_citation_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| line.starts_with('-') && mentions_a_standard(line))
        .map(|line| {
            line.trim_start_matches('-')
                .trim()
                .replace("**", "")
                .replace("Also known as:", "aka")
        })
        .collect()
}

/// Reduce one rustdoc description to the prose a payload author needs.
fn project_description(text: &str) -> String {
    let mut output: Vec<String> = Vec::new();
    let mut in_code_block = false;
    let mut dropping_section = false;

    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }
        if let Some(heading) = trimmed.strip_prefix("# ") {
            dropping_section = RUSTDOC_SECTIONS_TO_DROP.contains(&heading.trim());
            if dropping_section {
                continue;
            }
        }
        if dropping_section {
            continue;
        }
        output.push(line.to_string());
    }

    let joined = output.join("\n");
    // `[`Type`]` and `[`mod::fn`]` are rustdoc link syntax, not emphasis.
    let mut cleaned = String::with_capacity(joined.len());
    let mut rest = joined.as_str();
    while let Some(start) = rest.find("[`") {
        cleaned.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find("`]") {
            Some(end) => {
                cleaned.push('`');
                cleaned.push_str(&after[..end]);
                cleaned.push('`');
                rest = &after[end + 2..];
            }
            None => {
                cleaned.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    cleaned.push_str(rest);

    cleaned
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Apply [`project_description`] to every `description` in the document.
fn trim_descriptions(node: &mut Value, max_chars: usize) {
    match node {
        Value::Object(object) => {
            if let Some(Value::String(text)) = object.get("description") {
                let projected = shorten_description(&project_description(text), max_chars);
                if projected.is_empty() {
                    object.remove("description");
                } else {
                    object.insert("description".to_string(), Value::String(projected));
                }
            }
            for (key, nested) in object.iter_mut() {
                if key != "examples" {
                    trim_descriptions(nested, max_chars);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                trim_descriptions(item, max_chars);
            }
        }
        _ => {}
    }
}

/// Reduce one description to its leading paragraph, within a character budget.
///
/// Falls back through three steps: keep the text if it already fits, else keep
/// the leading paragraph, else keep whole sentences from that paragraph while
/// they fit. A description that cannot be cut at a sentence boundary is left
/// whole rather than truncated mid-clause — a half-sentence about a market
/// convention is worse than a long one.
fn shorten_description(text: &str, max_chars: usize) -> String {
    if max_chars == 0 || text.chars().count() <= max_chars {
        return text.to_string();
    }

    let (lead, sections) = split_leading_prose(text);
    let mut kept = shorten_prose(lead, max_chars);
    for section in sections {
        // A citation earns its place by being compact. The discriminating
        // case — which ISDA section each accepted spelling implements — is
        // folded into the lead prose by `flatten_const_unions` and preserved
        // there; a long reference block hanging off a single field is
        // background the caller can read in the canonical artifact.
        if mentions_a_standard(section) && section.chars().count() <= max_chars {
            kept.push_str("\n\n");
            kept.push_str(section.trim_end());
        }
    }
    kept
}

/// Split a description into its lead prose and its `# Heading` sections.
fn split_leading_prose(text: &str) -> (&str, Vec<&str>) {
    let mut boundaries: Vec<usize> = Vec::new();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        if line.starts_with("# ") {
            boundaries.push(offset);
        }
        offset += line.len();
    }
    let Some(&first) = boundaries.first() else {
        return (text, Vec::new());
    };
    let mut sections = Vec::with_capacity(boundaries.len());
    for (index, &start) in boundaries.iter().enumerate() {
        let end = boundaries.get(index + 1).copied().unwrap_or(text.len());
        sections.push(&text[start..end]);
    }
    (&text[..first], sections)
}

/// Whether a section cites a standards body or market convention.
///
/// These citations are what make a wire contract auditable — which ISDA
/// section a day count implements, which ISO code a value corresponds to — so
/// they are never treated as elaboration and never dropped for length.
fn mentions_a_standard(section: &str) -> bool {
    const BODIES: &[&str] = &[
        "ISDA", "ISO", "ICMA", "ISMA", "IFRS", "FpML", "SIFMA", "Basel", "AFB", "FINRA", "IMM",
        "GAAP", "EMIR", "CFTC", "BCBS", "IOSCO",
    ];
    BODIES.iter().any(|body| section.contains(body))
}

/// Reduce free prose to its opening paragraph, plus any paragraph that cites a
/// standard, within a character budget.
///
/// The citation carve-out matters here as much as at section level: folded
/// enum glosses arrive as a paragraph, not under a heading, and they are where
/// a flattened unit enum keeps its ISDA and ISO references.
fn shorten_prose(text: &str, max_chars: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let mut paragraphs = text.split("\n\n");
    let leading = paragraphs.next().unwrap_or(text).trim();
    let cited: Vec<&str> = paragraphs
        .map(str::trim)
        .filter(|paragraph| !paragraph.is_empty() && mentions_a_standard(paragraph))
        .collect();

    let head = shorten_paragraph(leading, max_chars);
    if cited.is_empty() {
        return head;
    }
    let mut kept = head;
    for paragraph in cited {
        kept.push_str("\n\n");
        kept.push_str(paragraph);
    }
    kept
}

/// Reduce one paragraph to whole sentences within a character budget.
fn shorten_paragraph(leading: &str, max_chars: usize) -> String {
    if leading.chars().count() <= max_chars {
        return leading.to_string();
    }

    let mut kept = String::new();
    let mut rest = leading;
    while let Some(end) = sentence_end(rest) {
        let (sentence, remainder) = rest.split_at(end);
        if kept.chars().count() + sentence.chars().count() > max_chars {
            break;
        }
        kept.push_str(sentence);
        rest = remainder;
    }

    let kept = kept.trim();
    if kept.is_empty() {
        leading.to_string()
    } else {
        kept.to_string()
    }
}

/// Byte offset just past the first sentence terminator followed by a space.
///
/// Abbreviations inside these descriptions are written without a following
/// space (`e.g.` appears mid-clause), so requiring whitespace after the period
/// avoids cutting one in half.
fn sentence_end(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    for (offset, character) in text.char_indices() {
        if !matches!(character, '.' | '!' | '?') {
            continue;
        }
        let next = offset + character.len_utf8();
        match bytes.get(next) {
            None => return Some(text.len()),
            Some(following) if following.is_ascii_whitespace() => return Some(next + 1),
            _ => {}
        }
    }
    None
}

/// Close objects, require every property, and drop non-portable keywords.
///
/// Optionality moves from "absent from `required`" into the type as a nullable
/// union, which is what strict structured-output modes expect. Tuples become
/// fixed-length arrays because `prefixItems` is outside every provider subset;
/// the positional meaning survives in the description.
fn apply_strict_shape(node: &mut Value) {
    match node {
        Value::Object(object) => {
            for keyword in NON_PORTABLE_KEYWORDS {
                if let Some(default) = object.remove(*keyword) {
                    if *keyword == "default" {
                        let note = format!("Defaults to `{default}` when omitted upstream.");
                        let existing = object
                            .get("description")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string();
                        let combined = if existing.is_empty() {
                            note
                        } else {
                            format!("{existing}\n\n{note}")
                        };
                        object.insert("description".to_string(), Value::String(combined));
                    }
                }
            }

            // `type: [T, "null"]` and `anyOf: [T, null]` mean the same thing;
            // one spelling is easier for a generator to follow.
            if let Some(Value::Array(types)) = object.get("type") {
                if types.len() == 2 && types.iter().any(|entry| entry == "null") {
                    if let Some(concrete) = types.iter().find(|entry| *entry != "null").cloned() {
                        object.insert("type".to_string(), concrete);
                    }
                }
            }

            if let Some(Value::Array(prefix_items)) = object.remove("prefixItems") {
                let count = prefix_items.len();
                // Draft 2020-12 has no array form of `items`, so a tuple becomes
                // a length-pinned array whose element schema admits every
                // position. Positional typing is lost; the description keeps it
                // readable.
                let element = match prefix_items.first() {
                    Some(first) if prefix_items.iter().all(|item| item == first) => first.clone(),
                    Some(_) => {
                        let mut union = Map::new();
                        union.insert("anyOf".to_string(), Value::Array(prefix_items));
                        Value::Object(union)
                    }
                    None => Value::Object(Map::new()),
                };
                object.insert("items".to_string(), element);
                object.insert(
                    "minItems".to_string(),
                    Value::Number(serde_json::Number::from(count)),
                );
                object.insert(
                    "maxItems".to_string(),
                    Value::Number(serde_json::Number::from(count)),
                );
                let note = format!("Fixed-length array of {count} positional values.");
                let existing = object
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let combined = if existing.is_empty() {
                    note
                } else {
                    format!("{existing}\n\n{note}")
                };
                object.insert("description".to_string(), Value::String(combined));
            }

            if let Some(Value::Object(properties)) = object.get("properties") {
                let names: Vec<Value> = properties.keys().cloned().map(Value::String).collect();
                object.insert("additionalProperties".to_string(), Value::Bool(false));
                object.insert("required".to_string(), Value::Array(names));
            }

            for nested in object.values_mut() {
                apply_strict_shape(nested);
            }
        }
        Value::Array(items) => {
            for item in items {
                apply_strict_shape(item);
            }
        }
        _ => {}
    }
}

/// Version stamped into every generated schema index.
pub const SCHEMA_INDEX_VERSION: u64 = 1;

/// Build the schema index document for one crate's registry.
///
/// Rows are sorted by artifact path, so the document is stable regardless of
/// registration order.
///
/// # Arguments
///
/// * `artifacts` - Complete registry for the owning crate.
///
/// # Errors
///
/// Returns [`Error::Internal`] if an artifact cannot be generated or rendered.
fn build_schema_index(artifacts: &[SchemaArtifact]) -> Result<Value> {
    let mut rows = BTreeMap::new();
    for artifact in artifacts {
        let rendered = deterministic_json_bytes(&artifact.generate()?)?;
        let mut row = Map::new();
        row.insert("$id".to_string(), Value::String(artifact.id.to_string()));
        row.insert(
            "bytes".to_string(),
            Value::Number(serde_json::Number::from(rendered.len())),
        );
        row.insert(
            "kind".to_string(),
            Value::String(artifact.kind.as_str().to_string()),
        );
        row.insert(
            "path".to_string(),
            Value::String(artifact.relative_path.to_string()),
        );
        row.insert(
            "summary".to_string(),
            Value::String(artifact.index_summary().to_string()),
        );
        row.insert(
            "title".to_string(),
            Value::String(artifact.title.to_string()),
        );
        if rows
            .insert(artifact.relative_path, Value::Object(row))
            .is_some()
        {
            return Err(Error::Validation(format!(
                "duplicate schema path in index: {}",
                artifact.relative_path
            )));
        }
    }

    let mut document = Map::new();
    document.insert(
        "artifacts".to_string(),
        Value::Array(rows.into_values().collect()),
    );
    document.insert(
        "schema_index_version".to_string(),
        Value::Number(serde_json::Number::from(SCHEMA_INDEX_VERSION)),
    );
    Ok(Value::Object(document))
}

/// Write or verify the schema index for one crate's registry.
///
/// The index is what a non-Rust consumer reads first: it names every contract
/// the crate publishes, whether the caller authors or reads it, and how large
/// the document is. Call this once per generator with the crate's complete
/// artifact list, even when the artifacts span several owned roots.
///
/// `--write` writes the index and `--check` byte-compares it. `--list`
/// deliberately does **not** name it: that listing is the *schema artifact*
/// inventory and is required to be sorted, and one index per crate cannot sit
/// in sorted position relative to every crate's root directory name. The index
/// is instead reconstructed from each generator's crate by
/// `scripts/check_schema_generation.py`, which is what compares the generated
/// tree against the registry.
///
/// # Arguments
///
/// * `manifest_dir` - Owning crate's manifest directory.
/// * `index_relative_path` - Index destination relative to the crate.
/// * `artifacts` - Complete registry for the owning crate.
/// * `command` - Parsed generator command selecting the mode and output root.
///
/// # Errors
///
/// Returns [`Error::Validation`] when the path escapes the crate or the
/// checked-in index has drifted, and [`Error::Internal`] on IO failure.
pub fn run_schema_index_generator(
    manifest_dir: &Path,
    index_relative_path: &Path,
    artifacts: &[SchemaArtifact],
    command: &SchemaGenerationCommand,
) -> Result<()> {
    validate_relative_path(index_relative_path, "schema index path")?;
    if command.mode == SchemaGenerationMode::List {
        return Ok(());
    }

    let rendered = deterministic_json_bytes(&build_schema_index(artifacts)?)?;
    let base = command.output_root.as_deref().unwrap_or(manifest_dir);
    let path = base.join(index_relative_path);

    match command.mode {
        SchemaGenerationMode::Check => match std::fs::read(&path) {
            Ok(actual) if actual == rendered => Ok(()),
            Ok(_) => Err(Error::Validation(format!(
                "schema index is not current: changed {}",
                index_relative_path.display()
            ))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Err(Error::Validation(format!(
                    "schema index is not current: missing {}",
                    index_relative_path.display()
                )))
            }
            Err(error) => Err(Error::Internal(format!(
                "read schema index {}: {error}",
                path.display()
            ))),
        },
        SchemaGenerationMode::Write => {
            if std::fs::read(&path).ok().as_deref() == Some(rendered.as_slice()) {
                return Ok(());
            }
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|error| {
                    Error::Internal(format!(
                        "create schema index directory {}: {error}",
                        parent.display()
                    ))
                })?;
            }
            std::fs::write(&path, rendered).map_err(|error| {
                Error::Internal(format!("write schema index {}: {error}", path.display()))
            })?;
            println!("updated {}", path.display());
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
    let mut value = value.clone();
    sort_json(&mut value);
    let mut json = serde_json::to_vec_pretty(&value)
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

/// Typed external definition that a packaging pass may reference.
///
/// The generated schema for `T` is compared with the local `$defs` subgraph
/// before any reference is rewritten, so a reused definition name cannot
/// silently substitute a different validation contract.
#[derive(Clone, Copy)]
pub struct ExternalSchemaDefinition {
    name: &'static str,
    uri: &'static str,
    generator: fn() -> Result<Value>,
}

impl ExternalSchemaDefinition {
    /// Declare one typed external schema definition.
    ///
    /// # Arguments
    ///
    /// * `name` - Exact local Schemars `$defs` name to externalize.
    /// * `uri` - Canonical absolute `$id` of the equivalent artifact.
    #[must_use]
    pub const fn new<T: SerdeSchema>(name: &'static str, uri: &'static str) -> Self {
        Self {
            name,
            uri,
            generator: raw_schema::<T>,
        }
    }
}

fn raw_schema<T: SerdeSchema>() -> Result<Value> {
    serde_json::to_value(schemars::schema_for!(T))
        .map_err(|error| Error::Internal(format!("serialize derived schema: {error}")))
}

/// Externalize only definitions proven equivalent to typed serde contracts.
///
/// The comparison recursively resolves local `$defs` references and removes
/// annotation-only keywords before comparing validation assertions. A name
/// collision or shape mismatch fails generation before the schema is changed.
/// Definitions that are not present in this particular derived document are
/// ignored, allowing one canonical definition list to serve a whole registry.
///
/// # Arguments
///
/// * `schema` - Mutable schema derived from a registered serde root.
/// * `definitions` - Typed external definitions eligible for replacement.
///
/// # Errors
///
/// Returns [`Error::Validation`] for duplicate names, unresolved references,
/// or assertion mismatches. Recursive definitions are compared as typed schema
/// graphs, including their cycle edges. Returns [`Error::Internal`] if a typed
/// comparison schema cannot be serialized.
pub fn externalize_schema_definitions(
    schema: &mut Value,
    definitions: &[ExternalSchemaDefinition],
) -> Result<()> {
    let local_defs = schema
        .get("$defs")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut external_refs = BTreeMap::new();
    for definition in definitions {
        if external_refs.contains_key(definition.name) {
            return Err(Error::Validation(format!(
                "duplicate external schema definition: {}",
                definition.name
            )));
        }
        let Some(local) = local_defs.get(definition.name) else {
            continue;
        };
        let mut local_stack = vec![definition.name.to_string()];
        let mut local_validation = validation_view(local, &local_defs, &mut local_stack)?;

        let external = (definition.generator)()?;
        let external_defs = external
            .get("$defs")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let mut external_validation = validation_view(&external, &external_defs, &mut Vec::new())?;
        sort_json(&mut local_validation);
        sort_json(&mut external_validation);
        if local_validation != external_validation {
            return Err(Error::Validation(format!(
                "local $defs/{} is not assertion-equivalent to {}",
                definition.name, definition.uri
            )));
        }
        external_refs.insert(definition.name.to_string(), definition.uri.to_string());
    }

    externalize_refs(schema, &external_refs);
    prune_unreachable_defs(schema);
    Ok(())
}

fn validation_view(
    value: &Value,
    definitions: &Map<String, Value>,
    stack: &mut Vec<String>,
) -> Result<Value> {
    match value {
        Value::Object(map) => {
            let mut assertions = Map::new();
            for (key, child) in map {
                if matches!(
                    key.as_str(),
                    "$schema"
                        | "$id"
                        | "$defs"
                        | "$comment"
                        | "title"
                        | "description"
                        | "default"
                        | "deprecated"
                        | "examples"
                        | "readOnly"
                        | "writeOnly"
                ) {
                    continue;
                }
                assertions.insert(key.clone(), child.clone());
            }

            if let Some(reference) = assertions
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(local_definition_ref)
                .map(|(name, suffix)| (name, suffix.map(str::to_string)))
            {
                let (name, suffix) = reference;
                if stack.iter().any(|entry| entry == &name) {
                    assertions.remove("$ref");
                    let recursive_reference = match suffix.as_deref() {
                        Some(suffix) => format!("#/$defs/{name}/{suffix}"),
                        None => format!("#/$defs/{name}"),
                    };
                    let expanded = serde_json::json!({
                        "$recursiveRef": recursive_reference
                    });
                    if assertions.is_empty() {
                        return Ok(expanded);
                    }
                    let siblings = validation_view(&Value::Object(assertions), definitions, stack)?;
                    return Ok(serde_json::json!({ "allOf": [expanded, siblings] }));
                }
                let target = definitions.get(&name).ok_or_else(|| {
                    Error::Validation(format!("unresolved local schema definition {name}"))
                })?;
                let target = match suffix.as_deref() {
                    Some(suffix) => target.pointer(&format!("/{suffix}")).ok_or_else(|| {
                        Error::Validation(format!(
                            "unresolved schema pointer #/$defs/{name}/{suffix}"
                        ))
                    })?,
                    None => target,
                };
                stack.push(name);
                let expanded = validation_view(target, definitions, stack)?;
                stack.pop();
                assertions.remove("$ref");
                if assertions.is_empty() {
                    return Ok(expanded);
                }
                let siblings = validation_view(&Value::Object(assertions), definitions, stack)?;
                return Ok(serde_json::json!({ "allOf": [expanded, siblings] }));
            }

            assertions
                .into_iter()
                .map(|(key, child)| {
                    validation_view(&child, definitions, stack).map(|child| (key, child))
                })
                .collect::<Result<Map<_, _>>>()
                .map(Value::Object)
        }
        Value::Array(items) => items
            .iter()
            .map(|item| validation_view(item, definitions, stack))
            .collect::<Result<Vec<_>>>()
            .map(Value::Array),
        _ => Ok(value.clone()),
    }
}

fn externalize_refs(value: &mut Value, external_refs: &BTreeMap<String, String>) {
    match value {
        Value::Object(map) => {
            let replacement = map
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(local_definition_ref)
                .and_then(|(name, suffix)| {
                    external_refs.get(&name).map(|reference| match suffix {
                        Some(suffix) => format!("{reference}#/{suffix}"),
                        None => reference.clone(),
                    })
                });
            if let Some(reference) = replacement {
                map.insert("$ref".to_string(), Value::String(reference));
            }
            for child in map.values_mut() {
                externalize_refs(child, external_refs);
            }
        }
        Value::Array(items) => {
            for item in items {
                externalize_refs(item, external_refs);
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

/// A valid but empty market snapshot.
///
/// Deliberately minimal: its job is to show the required-key shape, including
/// the mandatory `hierarchy` key whose value may be an explicit `null`.
fn market_context_state_examples() -> Result<Vec<Value>> {
    let state = crate::market_data::context::MarketContextState::from(
        &crate::market_data::context::MarketContext::new(),
    );
    let value = serde_json::to_value(&state)
        .map_err(|error| Error::Internal(format!("serialize market context example: {error}")))?;
    Ok(vec![value])
}

/// The core crate's schema registry.
///
/// This lives beside the emitter rather than in the generator binary, so the
/// generator, the contract tests and the bindings all render from one
/// definition. Render an entry with [`SchemaArtifact::generate`].
pub const ARTIFACTS: &[SchemaArtifact] = &[SchemaArtifact::new::<
    crate::market_data::context::MarketContextState,
>(
    "schemas/market_data/1/market_context_state.schema.json",
    "https://finstack_quant.dev/schemas/market_data/1/market_context_state.schema.json",
    "Market Context State",
    "Canonical v1 persisted snapshot of a complete market-data context.",
)
.with_kind(SchemaKind::Input)
.with_summary(
    "Curves, surfaces, prices, series and FX for one valuation date; the market input to \
             every pricing, scenario and attribution call.",
)
.with_examples(market_context_state_examples)];

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[allow(dead_code)]
    #[derive(serde::Serialize, serde::Deserialize)]
    #[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
    #[serde(transparent)]
    #[cfg_attr(feature = "json-schema", schemars(transparent))]
    struct ExternalText(String);

    #[allow(dead_code)]
    #[derive(serde::Serialize, serde::Deserialize)]
    #[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
    struct SuffixProbe {
        value: String,
    }

    #[allow(dead_code)]
    #[derive(serde::Serialize, serde::Deserialize)]
    #[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
    struct RecursiveProbe {
        value: String,
        child: Option<Box<RecursiveProbe>>,
    }

    #[allow(dead_code)]
    #[derive(serde::Serialize, serde::Deserialize)]
    #[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
    struct RecursiveEnvelope {
        probe: RecursiveProbe,
    }

    #[allow(dead_code)]
    #[derive(serde::Serialize, serde::Deserialize)]
    #[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
    struct RecursiveContainer {
        envelope: RecursiveEnvelope,
    }

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
                    "type": "string"
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

        externalize_schema_definitions(
            &mut schema,
            &[ExternalSchemaDefinition::new::<ExternalText>(
                "Money",
                "https://example.test/money.schema.json",
            )],
        )
        .expect("equivalent definition externalizes");

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
                    "required": ["value"],
                    "type": "object"
                }
            }
        });

        externalize_schema_definitions(
            &mut schema,
            &[ExternalSchemaDefinition::new::<SuffixProbe>(
                "Foo/Bar~Baz",
                "https://example.test/foo.schema.json",
            )],
        )
        .expect("equivalent escaped definition externalizes");

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

        externalize_schema_definitions(&mut schema, &[])
            .expect("empty external definition list is valid");

        assert!(schema["$defs"].get("Foo/Bar~Baz").is_some());
        assert!(schema["$defs"].get("Unused").is_none());
        assert_eq!(
            schema["properties"]["nested"]["$ref"],
            "#/$defs/Foo~1Bar~0Baz/properties/value"
        );
    }

    #[test]
    fn packaging_rejects_name_only_shape_substitution() {
        let mut schema = json!({
            "$ref": "#/$defs/Money",
            "$defs": {
                "Money": { "type": "object" }
            }
        });
        let original = schema.clone();
        let error = externalize_schema_definitions(
            &mut schema,
            &[ExternalSchemaDefinition::new::<ExternalText>(
                "Money",
                "https://example.test/money.schema.json",
            )],
        )
        .expect_err("same definition name with different assertions must fail");
        assert!(error.to_string().contains("not assertion-equivalent"));
        assert_eq!(
            schema, original,
            "failed packaging must not mutate the schema"
        );
    }

    #[test]
    fn packaging_compares_recursive_typed_definition_graphs() {
        let mut schema = serde_json::to_value(schemars::schema_for!(RecursiveContainer))
            .expect("recursive derived schema serializes");

        externalize_schema_definitions(
            &mut schema,
            &[ExternalSchemaDefinition::new::<RecursiveEnvelope>(
                "RecursiveEnvelope",
                "https://example.test/recursive_envelope.schema.json",
            )],
        )
        .expect("equivalent recursive definition externalizes");

        assert!(
            schema
                .to_string()
                .contains("https://example.test/recursive_envelope.schema.json"),
            "the recursive edge must use the typed external schema"
        );
        assert!(schema.get("$defs").is_none());
    }

    #[test]
    fn generated_schema_preserves_derived_assertions() {
        #[allow(dead_code)]
        #[derive(serde::Serialize, serde::Deserialize)]
        #[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
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

    const INDEX_PROBE_ARTIFACTS: &[SchemaArtifact] = &[
        SchemaArtifact::new::<SuffixProbe>(
            "schemas/probe/1/second.schema.json",
            "https://finstack_quant.dev/schemas/probe/1/second.schema.json",
            "Second",
            "Registered second but sorts first by path.",
        )
        .with_kind(SchemaKind::Output),
        SchemaArtifact::new::<SuffixProbe>(
            "schemas/probe/1/first.schema.json",
            "https://finstack_quant.dev/schemas/probe/1/first.schema.json",
            "First",
            "Falls back to this description because no summary is set.",
        )
        .with_kind(SchemaKind::Input)
        .with_summary("An explicit one-line summary."),
    ];

    #[test]
    fn schema_index_is_sorted_by_path_and_carries_kind_and_summary() {
        let index = build_schema_index(INDEX_PROBE_ARTIFACTS).expect("index builds");
        let rows = index["artifacts"].as_array().expect("artifacts array");

        assert_eq!(index["schema_index_version"], json!(SCHEMA_INDEX_VERSION));
        // Registration order is second-then-first; the index must not depend on it.
        assert_eq!(rows[0]["path"], json!("schemas/probe/1/first.schema.json"));
        assert_eq!(rows[1]["path"], json!("schemas/probe/1/second.schema.json"));

        assert_eq!(rows[0]["kind"], json!("input"));
        assert_eq!(rows[0]["summary"], json!("An explicit one-line summary."));
        assert_eq!(rows[1]["kind"], json!("output"));
        assert_eq!(
            rows[1]["summary"],
            json!("Registered second but sorts first by path."),
            "an artifact without a summary must fall back to its description"
        );
        assert!(
            rows[0]["bytes"].as_u64().is_some_and(|bytes| bytes > 0),
            "each row reports the rendered artifact size"
        );
    }

    #[test]
    fn single_branch_union_collapses_and_keeps_the_wrapper_annotation() {
        let mut schema = json!({
            "description": "Field-level documentation.",
            "oneOf": [{
                "type": "object",
                "description": "Branch documentation.",
                "properties": {"spec": {"type": "string"}},
                "required": ["spec"],
                "additionalProperties": false
            }]
        });
        collapse_single_branch_unions(&mut schema);

        assert!(schema.get("oneOf").is_none(), "the wrapper is removed");
        assert_eq!(schema["type"], json!("object"));
        assert_eq!(schema["required"], json!(["spec"]));
        assert_eq!(schema["additionalProperties"], json!(false));
        assert_eq!(
            schema["description"],
            json!("Field-level documentation."),
            "the wrapper's annotation wins over the branch's"
        );
    }

    #[test]
    fn single_branch_union_is_kept_when_the_wrapper_asserts_anything() {
        // `type` is an assertion, so dropping the wrapper could change meaning.
        let mut schema = json!({
            "type": "object",
            "oneOf": [{"required": ["spec"]}]
        });
        let before = schema.clone();
        collapse_single_branch_unions(&mut schema);
        assert_eq!(schema, before);
    }

    #[test]
    fn multi_branch_unions_are_untouched() {
        let mut schema = json!({"oneOf": [{"const": "a"}, {"const": "b"}]});
        let before = schema.clone();
        collapse_single_branch_unions(&mut schema);
        assert_eq!(schema, before);
    }

    #[test]
    fn single_branch_unions_collapse_at_every_depth() {
        let mut schema = json!({
            "properties": {
                "outer": {"oneOf": [{"properties": {"inner": {"oneOf": [{"type": "integer"}]}}}]}
            }
        });
        collapse_single_branch_unions(&mut schema);
        assert_eq!(
            schema["properties"]["outer"]["properties"]["inner"]["type"],
            json!("integer")
        );
    }

    fn no_resolver(_: &str) -> Option<Value> {
        None
    }

    #[test]
    fn projection_flattens_a_const_union_and_folds_short_variant_notes() {
        let schema = json!({
            "type": "object",
            "properties": {
                "stub": {"oneOf": [
                    {"const": "short_front", "type": "string", "description": "Short first period."},
                    {"const": "long_back", "type": "string", "description": "Long final period."}
                ]}
            }
        });
        let projected =
            project_llm(&schema, &no_resolver, &LlmProfile::default()).expect("projects");
        let stub = &projected["properties"]["stub"];

        assert_eq!(stub["type"], json!("string"));
        assert_eq!(stub["enum"], json!(["short_front", "long_back"]));
        assert!(stub.get("oneOf").is_none());
        let described = stub["description"].as_str().expect("folded notes");
        assert!(
            described.contains("`short_front`: Short first period."),
            "{described}"
        );
    }

    #[test]
    fn projection_keeps_a_union_whose_branches_carry_structure() {
        // Tagged unions are the contract, not an enum; only all-`const` unions
        // may collapse.
        let schema = json!({"oneOf": [
            {"properties": {"fixed": {"type": "number"}}, "required": ["fixed"]},
            {"properties": {"floating": {"type": "number"}}, "required": ["floating"]}
        ]});
        let projected =
            project_llm(&schema, &no_resolver, &LlmProfile::default()).expect("projects");
        assert!(projected["oneOf"].is_array());
        assert!(projected.get("enum").is_none());
    }

    #[test]
    fn projection_strips_rust_prose_but_keeps_domain_references() {
        let description = concat!(
            "Act/360 day count.\n\n",
            "# Examples\n```rust\nlet convention = DayCount::Act360;\n```\n\n",
            "# Standards Reference\nISDA 2006 Definitions, Section 4.16(d).\n\n",
            "See [`DayCount`] for the full set."
        );
        let schema = json!({"type": "string", "description": description});
        let projected =
            project_llm(&schema, &no_resolver, &LlmProfile::default()).expect("projects");
        let text = projected["description"]
            .as_str()
            .expect("description survives");

        assert!(text.contains("Act/360 day count."));
        assert!(
            text.contains("ISDA 2006 Definitions"),
            "domain grounding is kept: {text}"
        );
        assert!(!text.contains("```"), "code fences are dropped: {text}");
        assert!(
            !text.contains("let convention"),
            "Rust examples are dropped: {text}"
        );
        assert!(
            text.contains("`DayCount`") && !text.contains("[`DayCount`]"),
            "links flatten: {text}"
        );
    }

    #[test]
    fn oversized_local_definition_becomes_a_handle_and_its_dependents_are_pruned() {
        // A container instrument embeds the whole instrument union locally, so
        // the union definition is the one over budget and everything it
        // references exists only to serve it.
        let union_branches: Vec<Value> = (0..200)
            .map(|index| json!({"$ref": format!("#/$defs/Leaf{index}")}))
            .collect();
        let mut defs = Map::new();
        defs.insert(
            "InstrumentJson".to_string(),
            json!({
                "description": "Any instrument payload.",
                "type": "object",
                "oneOf": union_branches,
            }),
        );
        for index in 0..200 {
            defs.insert(
                format!("Leaf{index}"),
                json!({"type": "object", "properties": {"id": {"type": "string"}}}),
            );
        }
        defs.insert(
            "Kept".to_string(),
            json!({"type": "string", "description": "Reachable from the root."}),
        );
        let schema = json!({
            "type": "object",
            "properties": {
                "holding": {"$ref": "#/$defs/InstrumentJson"},
                "label": {"$ref": "#/$defs/Kept"},
            },
            "$defs": Value::Object(defs),
        });

        // Pinned rather than inherited from the default so the test states the
        // budget it depends on.
        let profile = LlmProfile {
            max_inline_bytes: 2048,
            ..LlmProfile::default()
        };
        let projected = project_llm(&schema, &no_resolver, &profile).expect("projects");
        let projected_defs = projected["$defs"].as_object().expect("defs survive");

        assert!(
            projected_defs.contains_key("Kept"),
            "definitions reachable from the root survive"
        );
        assert!(
            !projected_defs.contains_key("Leaf0"),
            "definitions reachable only through the handle are pruned"
        );
        let handle = &projected_defs["InstrumentJson"];
        assert!(
            handle.get("oneOf").is_none(),
            "the oversized union is stood down to a handle"
        );
        assert!(
            handle["description"]
                .as_str()
                .is_some_and(|text| text.contains("Any instrument payload.")),
            "the handle keeps the original summary: {handle}"
        );
        assert!(
            serde_json::to_vec(&projected).expect("serializes").len()
                < serde_json::to_vec(&schema).expect("serializes").len() / 4,
            "the projection is dramatically smaller than the source"
        );
    }

    #[test]
    fn definitions_reachable_only_through_other_definitions_are_kept() {
        let schema = json!({
            "type": "object",
            "properties": {"outer": {"$ref": "#/$defs/Outer"}},
            "$defs": {
                "Outer": {"type": "object", "properties": {"inner": {"$ref": "#/$defs/Inner"}}},
                "Inner": {"type": "string", "description": "Two hops from the root."},
                "Orphan": {"type": "string", "description": "Referenced by nothing."},
            },
        });

        let projected =
            project_llm(&schema, &no_resolver, &LlmProfile::default()).expect("projects");
        let defs = projected["$defs"].as_object().expect("defs survive");

        assert!(defs.contains_key("Outer"), "direct reference survives");
        assert!(defs.contains_key("Inner"), "transitive reference survives");
        assert!(
            !defs.contains_key("Orphan"),
            "unreferenced definition is pruned"
        );
    }

    #[test]
    fn short_descriptions_survive_the_character_budget_untouched() {
        let schema = json!({
            "type": "string",
            "description": "Annualized coupon rate as a decimal fraction, not a percentage.",
        });
        let projected =
            project_llm(&schema, &no_resolver, &LlmProfile::default()).expect("projects");

        assert_eq!(
            projected["description"],
            json!("Annualized coupon rate as a decimal fraction, not a percentage.")
        );
    }

    #[test]
    fn long_description_keeps_its_leading_paragraph_and_drops_elaboration() {
        let description = concat!(
            "Notional amount the schedule accrues on.\n\n",
            "The remaining paragraphs elaborate at length on amortization interactions, ",
            "sinking-fund mechanics, and the treatment of partial prepayments, none of which ",
            "a payload author needs in order to fill in a single number for this field, and ",
            "all of which cost tokens in every projected document that references it.",
        );
        let schema = json!({"type": "string", "description": description});
        let projected =
            project_llm(&schema, &no_resolver, &LlmProfile::default()).expect("projects");
        let text = projected["description"]
            .as_str()
            .expect("description survives");

        assert_eq!(text, "Notional amount the schedule accrues on.");
    }

    #[test]
    fn overlong_leading_paragraph_is_cut_on_a_sentence_boundary() {
        let sentence = "Rates are decimal fractions rather than percentages. ";
        let description = sentence.repeat(12);
        let schema = json!({"type": "string", "description": description});
        let projected =
            project_llm(&schema, &no_resolver, &LlmProfile::default()).expect("projects");
        let text = projected["description"]
            .as_str()
            .expect("description survives");

        assert!(
            text.chars().count() <= DEFAULT_MAX_DESCRIPTION_CHARS,
            "budget respected, got {} chars",
            text.chars().count()
        );
        assert!(
            text.ends_with("percentages."),
            "cut lands on a sentence boundary: {text}"
        );
    }

    #[test]
    fn a_single_overlong_sentence_is_kept_whole_rather_than_cut_mid_clause() {
        let description = format!("Day-count basis {} and nothing else", "x".repeat(400));
        let schema = json!({"type": "string", "description": description});
        let projected =
            project_llm(&schema, &no_resolver, &LlmProfile::default()).expect("projects");

        assert_eq!(
            projected["description"].as_str().expect("survives"),
            description,
            "no sentence boundary exists, so the text is left intact"
        );
    }

    #[test]
    fn standards_citations_survive_the_character_budget() {
        // Which ISDA section a convention implements is contract, not prose: a
        // payload author cannot confirm they picked the right day count without
        // it, so length alone must never drop it.
        let description = concat!(
            "Actual/360 day count convention.\n\n",
            "Year fraction = (actual days between dates) / 360\n\n",
            "# Standards Reference\n\n",
            "- **ISDA**: 2006 ISDA Definitions, Section 4.16(d)\n",
            "- **ISO 20022**: Day Count Fraction Code \"Actual/360\" (A004)\n\n",
            "# Usage\n\n",
            "Standard for USD money market deposits, EUR money market instruments, ",
            "short-term rate derivatives such as SOFR and \u{20ac}STR, and FX swaps and ",
            "forwards, across a range of desks that each have their own further ",
            "conventions layered on top of the basic accrual rule described above.",
        );
        let schema = json!({"type": "string", "description": description});
        let projected =
            project_llm(&schema, &no_resolver, &LlmProfile::default()).expect("projects");
        let text = projected["description"]
            .as_str()
            .expect("description survives");

        assert!(text.contains("Actual/360 day count convention."));
        assert!(text.contains("ISDA"), "standards body survives: {text}");
        assert!(text.contains("4.16(d)"), "section number survives: {text}");
        assert!(text.contains("A004"), "ISO code survives: {text}");
        assert!(
            !text.contains("# Usage"),
            "elaboration without a citation is dropped: {text}"
        );
    }

    #[test]
    fn zero_budget_disables_description_shortening() {
        let description = concat!(
            "Leading summary sentence.\n\n",
            "Elaboration that a zero budget must preserve verbatim for callers that want ",
            "the full projected prose rather than the summary alone.",
        );
        let schema = json!({"type": "string", "description": description});
        let profile = LlmProfile {
            max_description_chars: 0,
            ..LlmProfile::default()
        };
        let projected = project_llm(&schema, &no_resolver, &profile).expect("projects");

        assert!(
            projected["description"]
                .as_str()
                .expect("survives")
                .contains("Elaboration that a zero budget"),
            "zero disables the cap"
        );
    }

    #[test]
    fn projection_closes_objects_and_requires_every_property() {
        let schema = json!({
            "type": "object",
            "properties": {"id": {"type": "string"}, "note": {"type": "string"}},
            "required": ["id"]
        });
        let projected =
            project_llm(&schema, &no_resolver, &LlmProfile::default()).expect("projects");

        assert_eq!(projected["additionalProperties"], json!(false));
        assert_eq!(projected["required"], json!(["id", "note"]));
    }

    #[test]
    fn projection_rewrites_tuples_and_drops_non_portable_keywords() {
        let schema = json!({
            "type": "array",
            "prefixItems": [{"type": "string"}, {"type": "number"}],
            "uniqueItems": true,
            "format": "double"
        });
        let projected =
            project_llm(&schema, &no_resolver, &LlmProfile::default()).expect("projects");

        assert!(projected.get("prefixItems").is_none());
        // `items` must stay a schema: draft 2020-12 has no array form.
        assert!(projected["items"].is_object(), "{}", projected["items"]);
        assert!(
            projected["items"]["anyOf"].is_array(),
            "differing positions become a union"
        );
        assert_eq!(projected["minItems"], json!(2));
        assert_eq!(projected["maxItems"], json!(2));
        assert!(projected.get("uniqueItems").is_none());
        assert!(projected.get("format").is_none());
    }

    #[test]
    fn projection_inlines_a_resolvable_reference() {
        let schema = json!({
            "type": "object",
            "properties": {"amount": {"$ref": "https://finstack_quant.dev/schemas/common/1/decimal.schema.json"}}
        });
        let resolve = |id: &str| {
            (id == "https://finstack_quant.dev/schemas/common/1/decimal.schema.json")
                .then(|| json!({"$id": id, "type": "string", "pattern": "^-?\\d+$"}))
        };
        let projected = project_llm(&schema, &resolve, &LlmProfile::default()).expect("projects");

        assert_eq!(
            projected["properties"]["amount"]["$ref"],
            json!("#/$defs/Decimal")
        );
        assert_eq!(projected["$defs"]["Decimal"]["pattern"], json!("^-?\\d+$"));
        assert!(
            projected["$defs"]["Decimal"].get("$id").is_none(),
            "identity is not carried into a definition"
        );
    }

    #[test]
    fn projection_leaves_an_unresolvable_reference_alone() {
        // A missing shared definition must surface as a dangling reference, not
        // as a silently different contract.
        let schema =
            json!({"$ref": "https://finstack_quant.dev/schemas/common/1/gone.schema.json"});
        let projected =
            project_llm(&schema, &no_resolver, &LlmProfile::default()).expect("projects");
        assert_eq!(
            projected["$ref"],
            json!("https://finstack_quant.dev/schemas/common/1/gone.schema.json")
        );
    }

    #[test]
    fn projection_substitutes_a_handle_for_an_oversized_reference() {
        let big = json!({
            "$id": "https://finstack_quant.dev/schemas/instrument/1/instrument.schema.json",
            "type": "object",
            "description": "Every supported instrument.",
            "properties": {"filler": {"type": "string", "description": "x".repeat(4096)}}
        });
        let schema = json!({
            "type": "object",
            "properties": {"instrument": {"$ref": "https://finstack_quant.dev/schemas/instrument/1/instrument.schema.json"}}
        });
        let resolve = |id: &str| (id.ends_with("instrument.schema.json")).then(|| big.clone());
        let profile = LlmProfile {
            max_inline_bytes: 512,
            ..LlmProfile::default()
        };
        let projected = project_llm(&schema, &resolve, &profile).expect("projects");

        let handle = &projected["$defs"]["Instrument"];
        assert_eq!(
            handle[RESOLVES_FROM_KEYWORD],
            json!("https://finstack_quant.dev/schemas/instrument/1/instrument.schema.json")
        );
        assert_eq!(
            handle["type"],
            json!("object"),
            "a handle keeps the target's own type"
        );
        assert!(
            handle.get("properties").is_none(),
            "the target's internals are not carried"
        );
    }

    #[test]
    fn projection_rejects_a_non_object_document() {
        assert!(project_llm(&json!([1, 2]), &no_resolver, &LlmProfile::default()).is_err());
    }

    #[test]
    fn schema_index_defaults_to_component() {
        const COMPONENT: &[SchemaArtifact] = &[SchemaArtifact::new::<SuffixProbe>(
            "schemas/probe/1/component.schema.json",
            "https://finstack_quant.dev/schemas/probe/1/component.schema.json",
            "Component",
            "Referenced by roots, never submitted alone.",
        )];
        let index = build_schema_index(COMPONENT).expect("index builds");
        assert_eq!(index["artifacts"][0]["kind"], json!("component"));
    }
}
