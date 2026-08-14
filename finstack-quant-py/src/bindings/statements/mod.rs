//! Python bindings for the `finstack-quant-statements` crate.
//!
//! Exposes the financial model specification types, builder, evaluator,
//! DSL parser, and EBITDA normalization engine.

mod adjustments;
pub(crate) mod builder;
pub(crate) mod capital_structure;
mod checks;
mod dsl;
pub(crate) mod evaluator;
mod monte_carlo;
mod schema;
pub(crate) mod types;

use pyo3::prelude::*;
use pyo3::types::PyList;

/// Register the `statements` submodule on the parent module.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "statements")?;
    m.setattr(
        "__doc__",
        "Financial statement modeling: builders, evaluators, forecasts, DSL, adjustments.",
    )?;

    types::register(py, &m)?;
    capital_structure::register(py, &m)?;
    builder::register(py, &m)?;
    evaluator::register(py, &m)?;
    monte_carlo::register(py, &m)?;
    dsl::register(py, &m)?;
    adjustments::register(py, &m)?;
    checks::register(py, &m)?;
    schema::register(py, &m)?;

    let all = PyList::new(
        py,
        [
            "EcfSweepSpec",
            "PikToggleSpec",
            "WaterfallSpec",
            "ForecastMethod",
            "ForecastSpec",
            "NodeType",
            "NodeId",
            "NumericMode",
            "FinancialModelSpec",
            "ModelBuilder",
            "MixedNodeBuilder",
            "MetricRegistry",
            "StatementResult",
            "Evaluator",
            "MonteCarloConfig",
            "MonteCarloResults",
            "run_monte_carlo",
            "parse_formula",
            "validate_formula",
            "NormalizationConfig",
            "normalize",
            "CheckSuiteSpec",
            "CheckReport",
            "schema",
        ],
    )?;
    m.setattr("__all__", all)?;
    crate::bindings::module_utils::register_submodule(
        py,
        parent,
        &m,
        "statements",
        crate::bindings::module_utils::ROOT_PACKAGE,
        crate::bindings::module_utils::ParentNameSource::Name,
    )?;

    Ok(())
}
