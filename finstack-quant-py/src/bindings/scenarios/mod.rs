//! Python bindings for the `finstack-quant-scenarios` crate.
//!
//! Scenarios are spec-based (serde), so this module exposes JSON round-trip
//! functions for [`ScenarioSpec`] construction, validation, template
//! registry discovery, and scenario engine application.

pub(crate) mod engine;
mod horizon;
mod operation_spec;
mod schema;

use pyo3::prelude::*;
use pyo3::types::PyList;
use serde::de::DeserializeOwned;
use serde::Serialize;

fn parse_json<T: DeserializeOwned>(json: &str, context: &str) -> PyResult<T> {
    serde_json::from_str(json).map_err(|e| crate::errors::value_error(format!("{context}: {e}")))
}

fn to_json<T: Serialize>(value: &T, context: &str) -> PyResult<String> {
    serde_json::to_string(value).map_err(|e| crate::errors::value_error(format!("{context}: {e}")))
}

fn validate_spec(spec: &finstack_quant_scenarios::ScenarioSpec) -> PyResult<()> {
    spec.validate()
        .map_err(|e| crate::errors::value_error(format!("ScenarioSpec validation failed: {e}")))
}

fn parse_spec(json_str: &str) -> PyResult<finstack_quant_scenarios::ScenarioSpec> {
    parse_json(json_str, "Failed to parse ScenarioSpec JSON")
}

fn builtin_registry() -> PyResult<finstack_quant_scenarios::TemplateRegistry> {
    finstack_quant_scenarios::TemplateRegistry::with_embedded_builtins()
        .map_err(|e| crate::errors::value_error(format!("Failed to load embedded templates: {e}")))
}

fn template_entry<'a>(
    registry: &'a finstack_quant_scenarios::TemplateRegistry,
    template_id: &str,
) -> PyResult<&'a finstack_quant_scenarios::RegisteredTemplate> {
    registry
        .get(template_id)
        .ok_or_else(|| crate::errors::value_error(format!("Unknown template: '{template_id}'")))
}

// ScenarioSpec JSON round-trip

/// Parse a scenario specification and re-emit it in canonical form.
///
/// Round-tripping through the Rust type normalizes field order and fills
/// defaults, so the output is the exact spec the engine will execute. Use it to
/// diff a hand-written spec against what actually runs.
///
/// Parameters
/// ----------
/// json_str : str
///     JSON-serialized ``ScenarioSpec``.
///
/// Returns
/// -------
/// str
///     Canonical JSON-serialized ``ScenarioSpec``.
///
/// Raises
/// ------
/// ValueError
///     If the JSON is malformed or does not match the ``ScenarioSpec`` schema.
///     Unknown fields are rejected rather than ignored.
#[pyfunction]
fn parse_scenario_spec(json_str: &str) -> PyResult<String> {
    let spec = parse_spec(json_str)?;
    validate_spec(&spec)?;
    to_json(&spec, "Failed to serialize ScenarioSpec")
}

/// Build and validate a scenario specification as JSON.
///
/// Parameters
/// ----------
/// id : str
///     Stable scenario identifier written to the returned specification.
/// operations_json : str
///     JSON array of scenario operations in execution order.
/// name : str, optional
///     Optional human-readable scenario name.
/// description : str, optional
///     Optional human-readable explanation of the scenario.
/// priority : int, default 0
///     Composition priority; lower values execute first.
/// resolution_mode : str, default "most_specific_wins"
///     Hierarchy conflict policy. Accepted values are
///     ``"most_specific_wins"`` and ``"cumulative"``.
///
/// Returns
/// -------
/// str
///     Validated serialized ``ScenarioSpec`` JSON.
///
/// Raises
/// ------
/// ValueError
///     If ``operations_json`` is malformed, ``resolution_mode`` is not one of
///     the accepted values, the resulting scenario fails validation, or the
///     specification cannot be serialized.
///
/// Examples
/// --------
/// >>> from finstack_quant.scenarios import build_scenario_spec
/// >>> import json
/// >>> spec = build_scenario_spec("stress", "[]", resolution_mode="cumulative")
/// >>> json.loads(spec)["resolution_mode"]
/// 'cumulative'
#[pyfunction]
#[pyo3(signature = (
    id,
    operations_json,
    name=None,
    description=None,
    priority=0,
    resolution_mode="most_specific_wins"
))]
fn build_scenario_spec(
    id: &str,
    operations_json: &str,
    name: Option<&str>,
    description: Option<&str>,
    priority: i32,
    resolution_mode: &str,
) -> PyResult<String> {
    let operations: Vec<finstack_quant_scenarios::OperationSpec> =
        parse_json(operations_json, "Failed to parse operations JSON")?;
    let resolution_mode = serde_json::from_value(serde_json::Value::String(
        resolution_mode.to_string(),
    ))
    .map_err(|error| {
        crate::errors::value_error(format!("Invalid scenario resolution_mode: {error}"))
    })?;
    let spec = finstack_quant_scenarios::ScenarioSpec {
        id: id.to_string(),
        name: name.map(str::to_string),
        description: description.map(str::to_string),
        operations,
        priority,
        resolution_mode,
    };
    validate_spec(&spec)?;
    to_json(&spec, "Failed to serialize ScenarioSpec")
}

/// Compose several scenario specifications into one.
///
/// Later specs layer on top of earlier ones. Where two specs touch the same
/// target, the composed spec resolves the conflict using each operation's
/// ``resolution_mode`` (see :func:`build_scenario_spec`). Composition fails
/// rather than silently dropping an operation when the modes disagree.
///
/// Parameters
/// ----------
/// specs_json : str
///     JSON array of ``ScenarioSpec`` objects, in application order.
///
/// Returns
/// -------
/// str
///     JSON-serialized composed ``ScenarioSpec``.
///
/// Raises
/// ------
/// ValueError
///     If the JSON is malformed or the specs cannot be composed.
///
/// Examples
/// --------
/// >>> import json
/// >>> from finstack_quant.scenarios import build_scenario_spec, compose_scenarios
/// >>> rates = build_scenario_spec("rates", json.dumps([]))
/// >>> credit = build_scenario_spec("credit", json.dumps([]))
/// >>> composed = compose_scenarios(json.dumps([json.loads(rates), json.loads(credit)]))
/// >>> json.loads(composed)["id"] is not None
/// True
#[pyfunction]
fn compose_scenarios(specs_json: &str) -> PyResult<String> {
    let specs: Vec<finstack_quant_scenarios::ScenarioSpec> =
        parse_json(specs_json, "Failed to parse specs JSON")?;
    let engine = finstack_quant_scenarios::ScenarioEngine::new();
    let composed = engine
        .try_compose(specs)
        .map_err(|e| crate::errors::value_error(format!("Scenario composition failed: {e}")))?;
    to_json(&composed, "Failed to serialize composed spec")
}

/// Validate a scenario specification without applying it.
///
/// Parameters
/// ----------
/// json_str : str
///     JSON-serialized ``ScenarioSpec``.
///
/// Returns
/// -------
/// None
///     Returns nothing on success. An invalid spec raises instead, so
///     ``if validate_scenario_spec(s):`` is not a validity check — call it
///     bare and catch ``ValueError``.
///
/// Raises
/// ------
/// ValueError
///     If the JSON is malformed or the spec fails validation.
#[pyfunction]
fn validate_scenario_spec(json_str: &str) -> PyResult<()> {
    let spec = parse_spec(json_str)?;
    validate_spec(&spec)?;
    Ok(())
}

// Template registry

/// List the identifiers of every built-in scenario template.
///
/// Templates are the quickest way into the scenarios domain: pick an id here,
/// pass it to :func:`build_from_template`, and apply the resulting spec — no
/// hand-written operation JSON required.
///
/// Returns
/// -------
/// list[str]
///     Template identifiers, e.g. ``"rates_parallel_up_100bp"``.
///
/// See Also
/// --------
/// list_builtin_template_metadata : Names and descriptions for these ids.
/// build_from_template : Turn an id into a runnable spec.
///
/// Examples
/// --------
/// >>> from finstack_quant.scenarios import list_builtin_templates
/// >>> isinstance(list_builtin_templates(), list)
/// True
#[pyfunction]
fn list_builtin_templates() -> PyResult<Vec<String>> {
    let registry = builtin_registry()?;
    Ok(registry.list().iter().map(|m| m.id.clone()).collect())
}

/// Describe every built-in scenario template.
///
/// Returns
/// -------
/// str
///     JSON array of template metadata objects, each carrying at least ``id``,
///     ``name`` and ``description``. Parse with ``json.loads`` — or load
///     straight into pandas with ``pd.read_json(...)`` — to browse the catalog.
///
/// See Also
/// --------
/// list_builtin_templates : Just the identifiers.
#[pyfunction]
fn list_builtin_template_metadata() -> PyResult<String> {
    let registry = builtin_registry()?;
    let metadata: Vec<&finstack_quant_scenarios::TemplateMetadata> = registry.list();
    to_json(&metadata, "Failed to serialize template metadata")
}

/// Build a complete scenario specification from a built-in template.
///
/// This is the shortest path from nothing to a runnable scenario.
///
/// Parameters
/// ----------
/// template_id : str
///     Identifier from :func:`list_builtin_templates`.
///
/// Returns
/// -------
/// str
///     JSON-serialized ``ScenarioSpec``, ready for
///     :func:`apply_scenario_to_market` or :func:`compute_horizon_return`.
///
/// Raises
/// ------
/// ValueError
///     If ``template_id`` is not a built-in template, or the template fails to
///     build.
///
/// See Also
/// --------
/// list_template_components : Build only part of a template.
#[pyfunction]
fn build_from_template(template_id: &str) -> PyResult<String> {
    let registry = builtin_registry()?;
    let entry = template_entry(&registry, template_id)?;
    let spec = entry
        .builder()
        .build()
        .map_err(|e| crate::errors::value_error(format!("Failed to build template spec: {e}")))?;
    to_json(&spec, "Failed to serialize spec")
}

/// List the component identifiers within a built-in template.
///
/// Multi-part templates (e.g. a stress that shocks rates, credit and vol) are
/// decomposable, so a single leg can be applied on its own.
///
/// Parameters
/// ----------
/// template_id : str
///     Identifier from :func:`list_builtin_templates`.
///
/// Returns
/// -------
/// list[str]
///     Component identifiers accepted by :func:`build_template_component`.
///
/// Raises
/// ------
/// ValueError
///     If ``template_id`` is not a built-in template.
#[pyfunction]
fn list_template_components(template_id: &str) -> PyResult<Vec<String>> {
    let registry = builtin_registry()?;
    let entry = template_entry(&registry, template_id)?;
    Ok(entry
        .component_ids()
        .into_iter()
        .map(str::to_string)
        .collect())
}

/// Build a scenario specification from one component of a built-in template.
///
/// Parameters
/// ----------
/// template_id : str
///     Identifier from :func:`list_builtin_templates`.
/// component_id : str
///     Identifier from :func:`list_template_components`.
///
/// Returns
/// -------
/// str
///     JSON-serialized ``ScenarioSpec`` covering only that component.
///
/// Raises
/// ------
/// ValueError
///     If either identifier is unknown, or the component fails to build.
#[pyfunction]
fn build_template_component(template_id: &str, component_id: &str) -> PyResult<String> {
    let registry = builtin_registry()?;
    let entry = template_entry(&registry, template_id)?;
    let builder = entry.component(component_id).ok_or_else(|| {
        crate::errors::value_error(format!(
            "Unknown component '{component_id}' in template '{template_id}'"
        ))
    })?;
    let spec = builder
        .build()
        .map_err(|e| crate::errors::value_error(format!("Failed to build component spec: {e}")))?;
    to_json(&spec, "Failed to serialize component spec")
}

// Module registration

pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "scenarios")?;
    m.setattr(
        "__doc__",
        "Scenario specification, validation, composition, application, and built-in templates.",
    )?;

    m.add_function(wrap_pyfunction!(parse_scenario_spec, &m)?)?;
    m.add_function(wrap_pyfunction!(build_scenario_spec, &m)?)?;
    m.add_function(wrap_pyfunction!(compose_scenarios, &m)?)?;
    m.add_function(wrap_pyfunction!(validate_scenario_spec, &m)?)?;
    m.add_function(wrap_pyfunction!(list_builtin_templates, &m)?)?;
    m.add_function(wrap_pyfunction!(list_builtin_template_metadata, &m)?)?;
    m.add_function(wrap_pyfunction!(build_from_template, &m)?)?;
    m.add_function(wrap_pyfunction!(list_template_components, &m)?)?;
    m.add_function(wrap_pyfunction!(build_template_component, &m)?)?;
    engine::register(py, &m)?;
    horizon::register(py, &m)?;
    operation_spec::register(py, &m)?;

    schema::register(py, &m)?;
    // Sorted; must agree with the pure-Python shim and the `.pyi` stub.
    let all = PyList::new(
        py,
        [
            "ApplicationReport",
            "ApplicationResult",
            "Compounding",
            "CurveKind",
            "HorizonResult",
            "OperationSpec",
            "RateBindingSpec",
            "TenorMatchMode",
            "TimeRollMode",
            "apply_scenario",
            "apply_scenario_to_market",
            "build_from_template",
            "build_scenario_spec",
            "build_template_component",
            "compose_scenarios",
            "compute_horizon_return",
            "list_builtin_template_metadata",
            "list_builtin_templates",
            "list_template_components",
            "parse_scenario_spec",
            "schema",
            "validate_scenario_spec",
        ],
    )?;
    m.setattr("__all__", all)?;
    crate::bindings::module_utils::register_submodule(
        py,
        parent,
        &m,
        "scenarios",
        crate::bindings::module_utils::ROOT_PACKAGE,
        crate::bindings::module_utils::ParentNameSource::Name,
    )?;

    Ok(())
}
