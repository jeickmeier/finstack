//! Python bindings for the `finstack-quant-scenarios` crate.
//!
//! Scenarios are spec-based (serde), so this module exposes JSON round-trip
//! functions for [`ScenarioSpec`] construction, validation, template
//! registry discovery, and scenario engine application.

mod engine;
mod horizon;
mod operation_spec;

use crate::bindings::date_utils::parse_iso_date_py as parse_date;
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

#[pyfunction]
fn validate_scenario_spec(json_str: &str) -> PyResult<bool> {
    let spec = parse_spec(json_str)?;
    validate_spec(&spec)?;
    Ok(true)
}

// Template registry

#[pyfunction]
fn list_builtin_templates() -> PyResult<Vec<String>> {
    let registry = builtin_registry()?;
    Ok(registry.list().iter().map(|m| m.id.clone()).collect())
}

#[pyfunction]
fn list_builtin_template_metadata() -> PyResult<String> {
    let registry = builtin_registry()?;
    let metadata: Vec<&finstack_quant_scenarios::TemplateMetadata> = registry.list();
    to_json(&metadata, "Failed to serialize template metadata")
}

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

    let all = PyList::new(
        py,
        [
            "parse_scenario_spec",
            "build_scenario_spec",
            "compose_scenarios",
            "validate_scenario_spec",
            "list_builtin_templates",
            "list_builtin_template_metadata",
            "build_from_template",
            "list_template_components",
            "build_template_component",
            "apply_scenario",
            "apply_scenario_to_market",
            "compute_horizon_return",
            "HorizonResult",
            "OperationSpec",
            "RateBindingSpec",
            "CurveKind",
            "TenorMatchMode",
            "TimeRollMode",
            "Compounding",
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
