//! Registry-backed schema access shared by every `finstack_quant.*.schema` namespace.
//!
//! The per-crate accessors elsewhere in these bindings return one named schema
//! each, which requires the caller to already know what exists. The three
//! functions generated here are the discovery surface instead: `index` lists
//! every contract the crate publishes, `get` fetches one by path, and
//! `validate` checks a payload and reports where it failed.
//!
//! Schemas are rendered from the crate's `ARTIFACTS` registry on demand rather
//! than embedded, so nothing can drift from the installed wheel and the
//! extension module carries no copy of the six megabytes of checked-in JSON.

use serde_json::Value;

use finstack_quant_core::schema::SchemaArtifact;

/// Render every artifact in a registry as one index document.
///
/// Mirrors the checked-in `schemas/index.json` row shape: `$id`, `path`,
/// `title`, `kind`, `summary` and rendered `bytes`.
///
/// # Errors
///
/// Returns an error string if an artifact cannot be rendered.
pub(crate) fn registry_index(artifacts: &[SchemaArtifact]) -> Result<Value, String> {
    let mut rows = Vec::with_capacity(artifacts.len());
    for artifact in artifacts {
        let rendered = artifact
            .generate()
            .map_err(|error| format!("render {}: {error}", artifact.relative_path))?;
        let bytes = serde_json::to_vec(&rendered)
            .map_err(|error| format!("measure {}: {error}", artifact.relative_path))?
            .len();
        rows.push(serde_json::json!({
            "$id": artifact.id,
            "bytes": bytes,
            "kind": artifact.kind.as_str(),
            "path": artifact.relative_path,
            "summary": artifact.index_summary(),
            "title": artifact.title,
        }));
    }
    rows.sort_by(|left, right| left["path"].as_str().cmp(&right["path"].as_str()));
    Ok(serde_json::json!({"artifacts": rows, "schema_index_version": 1}))
}

/// Look up one artifact by `$id`, full path, or trailing filename.
///
/// # Errors
///
/// Returns an error string naming the available paths when nothing matches.
pub(crate) fn find_artifact<'a>(
    artifacts: &'a [SchemaArtifact],
    selector: &str,
) -> Result<&'a SchemaArtifact, String> {
    artifacts
        .iter()
        .find(|artifact| {
            artifact.id == selector
                || artifact.relative_path == selector
                || artifact.relative_path.ends_with(selector)
        })
        .ok_or_else(|| {
            let available: Vec<&str> = artifacts
                .iter()
                .map(|artifact| artifact.relative_path)
                .collect();
            format!("no schema matches {selector:?}; available: {}", available.join(", "))
        })
}

/// Every artifact published by the workspace, keyed by `$id`.
///
/// Published schemas reference each other by absolute `$id` on a host that does
/// not resolve, so a validator built from one document alone fails the moment it
/// meets a `$ref`. The binding crate depends on all of them, so it registers the
/// whole corpus as in-process resources and nothing is ever fetched.
fn corpus() -> &'static std::collections::BTreeMap<String, Value> {
    static CACHE: std::sync::OnceLock<std::collections::BTreeMap<String, Value>> =
        std::sync::OnceLock::new();
    CACHE.get_or_init(|| finstack_quant::schema::documents_by_id().unwrap_or_default())
}

fn resolvable_resources() -> &'static [(String, jsonschema::Resource)] {
    static CACHE: std::sync::OnceLock<Vec<(String, jsonschema::Resource)>> =
        std::sync::OnceLock::new();
    CACHE.get_or_init(|| {
        corpus()
            .iter()
            .filter_map(|(id, schema)| {
                jsonschema::Resource::from_contents(schema.clone())
                    .ok()
                    .map(|resource| (id.clone(), resource))
            })
            .collect()
    })
}

/// Render one artifact in the requested profile.
///
/// `"canonical"` is the published contract and the only form that may be used
/// to validate. `"llm"` is the projection: self-contained, flat enums, Rust
/// prose stripped, shaped for provider subsets - and deliberately not a
/// validator.
pub(crate) fn render_profile(artifact: &SchemaArtifact, profile: &str) -> Result<Value, String> {
    let canonical = artifact
        .generate()
        .map_err(|error| format!("render {}: {error}", artifact.relative_path))?;
    match profile {
        "canonical" => Ok(canonical),
        "llm" => {
            let resolve = |id: &str| corpus().get(id).cloned();
            finstack_quant_core::schema::project_llm(
                &canonical,
                &resolve,
                &finstack_quant_core::schema::LlmProfile::default(),
            )
            .map_err(|error| format!("project {}: {error}", artifact.relative_path))
        }
        other => Err(format!(
            "unknown schema profile {other:?}; expected \"canonical\" or \"llm\""
        )),
    }
}

/// Validate a payload against one artifact, returning failures as pointer/message pairs.
///
/// # Errors
///
/// Returns an error string if the artifact cannot be rendered or compiled.
pub(crate) fn validate_against(
    artifact: &SchemaArtifact,
    payload: &Value,
) -> Result<Vec<Value>, String> {
    let schema = artifact
        .generate()
        .map_err(|error| format!("render {}: {error}", artifact.relative_path))?;
    let validator = jsonschema::options()
        .with_resources(
            resolvable_resources()
                .iter()
                .map(|(id, resource)| (id.as_str(), resource.clone())),
        )
        .build(&schema)
        .map_err(|error| format!("compile {}: {error}", artifact.relative_path))?;
    Ok(validator
        .iter_errors(payload)
        .map(|error| {
            serde_json::json!({
                "pointer": error.instance_path.to_string(),
                "message": error.to_string(),
            })
        })
        .collect())
}

/// Generate `index`, `get` and `validate` for one crate's schema namespace.
///
/// # Arguments
///
/// * `$registry` - Expression yielding the crate's `ARTIFACTS` slice.
/// * `$python_path` - Dotted Python path of the namespace, for `__module__`.
macro_rules! schema_registry_functions {
    ($registry:expr, $python_path:literal) => {
        /// List every JSON Schema this crate publishes.
        ///
        /// Returns
        /// -------
        /// str
        ///     Pretty-printed JSON with an ``artifacts`` array. Each row carries
        ///     ``path``, ``$id``, ``title``, ``summary``, ``bytes`` and ``kind``
        ///     (``input`` for documents you author, ``output`` for documents the
        ///     library emits, ``component`` for shared definitions).
        ///
        /// Raises
        /// ------
        /// ValueError
        ///     If an artifact cannot be rendered.
        #[pyfunction]
        #[pyo3(text_signature = "()")]
        fn index() -> PyResult<String> {
            let value = $crate::bindings::schema_registry::registry_index($registry)
                .map_err(pyo3::exceptions::PyValueError::new_err)?;
            serde_json::to_string_pretty(&value).map_err(|error| {
                pyo3::exceptions::PyValueError::new_err(format!("serialize index: {error}"))
            })
        }

        /// Fetch one JSON Schema by path, ``$id``, or filename.
        ///
        /// Parameters
        /// ----------
        /// selector : str
        ///     A ``path`` or ``$id`` from :func:`index`, or just the trailing
        ///     filename.
        ///
        /// profile : str, optional
        ///     ``"canonical"`` (default) returns the published contract, which
        ///     is what :func:`validate` checks against. ``"llm"`` returns the
        ///     projection: self-contained, unit enums flattened, Rust prose
        ///     stripped, and shaped for structured-output subsets. The
        ///     projection is deliberately **not** a validator.
        ///
        /// Returns
        /// -------
        /// str
        ///     Pretty-printed JSON Schema text.
        ///
        /// Raises
        /// ------
        /// KeyError
        ///     If no artifact matches ``selector``.
        /// ValueError
        ///     If ``profile`` is unknown, or the artifact cannot be rendered.
        #[pyfunction]
        #[pyo3(signature = (selector, profile = "canonical"), text_signature = "(selector, profile='canonical')")]
        fn get(selector: &str, profile: &str) -> PyResult<String> {
            let artifact =
                $crate::bindings::schema_registry::find_artifact($registry, selector)
                    .map_err(pyo3::exceptions::PyKeyError::new_err)?;
            let value =
                $crate::bindings::schema_registry::render_profile(artifact, profile)
                    .map_err(pyo3::exceptions::PyValueError::new_err)?;
            serde_json::to_string_pretty(&value).map_err(|error| {
                pyo3::exceptions::PyValueError::new_err(format!("serialize schema: {error}"))
            })
        }

        /// Validate a JSON payload against one published schema.
        ///
        /// Parameters
        /// ----------
        /// selector : str
        ///     A ``path`` or ``$id`` from :func:`index`, or just the trailing
        ///     filename.
        /// payload : str
        ///     JSON text to check.
        ///
        /// Returns
        /// -------
        /// str
        ///     Pretty-printed JSON array of failures, each with ``pointer`` (a
        ///     JSON Pointer into the payload) and ``message``. An empty array
        ///     means the payload validates.
        ///
        /// Raises
        /// ------
        /// KeyError
        ///     If no artifact matches ``selector``.
        /// ValueError
        ///     If ``payload`` is not valid JSON, or the schema cannot be built.
        #[pyfunction]
        #[pyo3(text_signature = "(selector, payload)")]
        fn validate(selector: &str, payload: &str) -> PyResult<String> {
            let artifact =
                $crate::bindings::schema_registry::find_artifact($registry, selector)
                    .map_err(pyo3::exceptions::PyKeyError::new_err)?;
            let parsed: serde_json::Value = serde_json::from_str(payload).map_err(|error| {
                pyo3::exceptions::PyValueError::new_err(format!("payload is not JSON: {error}"))
            })?;
            let failures =
                $crate::bindings::schema_registry::validate_against(artifact, &parsed)
                    .map_err(pyo3::exceptions::PyValueError::new_err)?;
            serde_json::to_string_pretty(&failures).map_err(|error| {
                pyo3::exceptions::PyValueError::new_err(format!("serialize report: {error}"))
            })
        }

        /// Add `index`, `get` and `validate` to a schema namespace module.
        ///
        /// # Errors
        ///
        /// Returns a `PyErr` if any function cannot be registered.
        fn add_registry_functions(m: &Bound<'_, pyo3::types::PyModule>) -> PyResult<()> {
            m.add_function(wrap_pyfunction!(index, m)?)?;
            m.add_function(wrap_pyfunction!(get, m)?)?;
            m.add_function(wrap_pyfunction!(validate, m)?)?;
            for name in ["index", "get", "validate"] {
                m.getattr(name)?.setattr("__module__", $python_path)?;
            }
            Ok(())
        }
    };
}

pub(crate) use schema_registry_functions;
