//! Python bindings for the `finstack-quant-scenarios` crate.
//!
//! Scenarios are spec-based (serde), so this module exposes typed specification
//! construction, validation, template registry discovery, and scenario engine
//! application with explicit JSON round-trips on wrapper methods.

pub(crate) mod engine;
mod horizon;
mod operation_spec;
mod schema;
mod spec;

use operation_spec::PyOperationSpec;
use pyo3::prelude::*;
use pyo3::types::PyList;
use spec::{PyScenarioSpec, PyTemplateMetadata};

fn validate_spec(spec: &finstack_quant_scenarios::ScenarioSpec) -> PyResult<()> {
    spec.validate()
        .map_err(|e| crate::errors::value_error(format!("ScenarioSpec validation failed: {e}")))
}

fn parse_spec(json_str: &str) -> PyResult<finstack_quant_scenarios::ScenarioSpec> {
    serde_json::from_str(json_str).map_err(|error| {
        crate::errors::value_error(format!("Failed to parse ScenarioSpec JSON: {error}"))
    })
}

fn builtin_registry() -> PyResult<finstack_quant_scenarios::TemplateRegistry> {
    finstack_quant_scenarios::TemplateRegistry::with_embedded_builtins()
        .map_err(|e| crate::errors::value_error(format!("Failed to load embedded templates: {e}")))
}

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
/// ScenarioSpec
///     Validated typed scenario specification.
///
/// Raises
/// ------
/// ValueError
///     If the JSON is malformed or does not match the ``ScenarioSpec`` schema.
///     Unknown fields are rejected rather than ignored.
#[pyfunction]
fn parse_scenario_spec(json_str: &str) -> PyResult<PyScenarioSpec> {
    let spec = parse_spec(json_str)?;
    validate_spec(&spec)?;
    Ok(PyScenarioSpec::from_inner(spec))
}

/// Build and validate a typed scenario specification.
///
/// Parameters
/// ----------
/// id : str
///     Stable scenario identifier written to the returned specification.
/// operations : list[OperationSpec]
///     Typed scenario operations in execution order.
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
/// ScenarioSpec
///     Validated typed scenario specification.
///
/// Raises
/// ------
/// ValueError
///     If ``resolution_mode`` is not accepted or the resulting scenario fails
///     validation.
///
/// Examples
/// --------
/// >>> from finstack_quant.scenarios import build_scenario_spec
/// >>> spec = build_scenario_spec("stress", [], resolution_mode="cumulative")
/// >>> spec.resolution_mode
/// 'cumulative'
#[pyfunction]
#[pyo3(signature = (
    id,
    operations,
    name=None,
    description=None,
    priority=0,
    resolution_mode="most_specific_wins"
))]
fn build_scenario_spec(
    id: &str,
    operations: Vec<PyOperationSpec>,
    name: Option<&str>,
    description: Option<&str>,
    priority: i32,
    resolution_mode: &str,
) -> PyResult<PyScenarioSpec> {
    PyScenarioSpec::build(id, operations, name, description, priority, resolution_mode)
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
/// specs : list[ScenarioSpec]
///     Typed scenario specifications in application order.
///
/// Returns
/// -------
/// ScenarioSpec
///     Typed composed scenario specification.
///
/// Raises
/// ------
/// ValueError
///     If the specifications cannot be composed.
///
/// Examples
/// --------
/// >>> from finstack_quant.scenarios import build_scenario_spec, compose_scenarios
/// >>> rates = build_scenario_spec("rates", [])
/// >>> credit = build_scenario_spec("credit", [])
/// >>> compose_scenarios([rates, credit]).operations
/// []
#[pyfunction]
fn compose_scenarios(specs: Vec<PyScenarioSpec>) -> PyResult<PyScenarioSpec> {
    let specs = specs.into_iter().map(|spec| spec.inner).collect();
    let composed = finstack_quant_scenarios::ScenarioEngine::new()
        .try_compose(specs)
        .map_err(|error| {
            crate::errors::value_error(format!("Scenario composition failed: {error}"))
        })?;
    Ok(PyScenarioSpec::from_inner(composed))
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
/// list[TemplateMetadata]
///     Typed metadata objects in deterministic registry order.
///
/// See Also
/// --------
/// list_builtin_templates : Just the identifiers.
#[pyfunction]
fn list_builtin_template_metadata() -> PyResult<Vec<PyTemplateMetadata>> {
    let registry = builtin_registry()?;
    Ok(registry
        .list()
        .into_iter()
        .cloned()
        .map(PyTemplateMetadata::from_inner)
        .collect())
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
/// ScenarioSpec
///     Typed scenario specification. Call :meth:`ScenarioSpec.to_json` when
///     passing it to an explicitly JSON-input API.
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
fn build_from_template(template_id: &str) -> PyResult<PyScenarioSpec> {
    let spec = builtin_registry()?
        .build(template_id)
        .map_err(|error| crate::errors::value_error(error.to_string()))?;
    Ok(PyScenarioSpec::from_inner(spec))
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
    builtin_registry()?
        .component_ids(template_id)
        .map(|ids| ids.into_iter().map(str::to_string).collect())
        .map_err(|error| crate::errors::value_error(error.to_string()))
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
/// ScenarioSpec
///     Typed scenario specification covering only that component.
///
/// Raises
/// ------
/// ValueError
///     If either identifier is unknown, or the component fails to build.
#[pyfunction]
fn build_template_component(template_id: &str, component_id: &str) -> PyResult<PyScenarioSpec> {
    let spec = builtin_registry()?
        .build_component(template_id, component_id)
        .map_err(|error| crate::errors::value_error(error.to_string()))?;
    Ok(PyScenarioSpec::from_inner(spec))
}

pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "scenarios")?;
    m.setattr(
        "__doc__",
        "Scenario specification, validation, composition, application, and built-in templates.",
    )?;

    m.add_class::<PyScenarioSpec>()?;
    m.add_class::<PyTemplateMetadata>()?;
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
            "ScenarioSpec",
            "TemplateMetadata",
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
