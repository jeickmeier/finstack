//! Python bindings for `finstack_quant_models::factor`.
//!
//! The module mirrors the Rust crate boundary. Credit hierarchy bindings are
//! registered under `finstack_quant.models.factor.credit`.

use pyo3::prelude::*;
use pyo3::types::PyList;

pub(crate) mod credit;
mod schema;

/// Register the `models.factor` Python domain.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "factor")?;
    let qual = crate::bindings::module_utils::set_submodule_package_by_package(
        parent,
        &m,
        "factor",
        "finstack_quant.models",
    )?;
    m.setattr(
        "__doc__",
        "Factor-model primitives, credit calibration, and decomposition.",
    )?;

    let credit = PyModule::new(py, "credit")?;
    let credit_qual = crate::bindings::module_utils::set_submodule_package_by_package(
        &m, &credit, "credit", &qual,
    )?;
    credit.setattr(
        "__doc__",
        "Credit factor hierarchy artifacts, calibration, and decomposition.",
    )?;
    credit::register(py, &credit)?;

    let credit_all = PyList::new(
        py,
        [
            "CreditFactorModel",
            "CreditCalibrator",
            "LevelsAtDate",
            "PeriodDecomposition",
            "FactorCovarianceForecast",
            "FactorCovarianceMatrix",
            "FactorModelConfig",
            "decompose_levels",
            "decompose_period",
        ],
    )?;
    credit.setattr("__all__", credit_all)?;
    crate::bindings::module_utils::register_submodule_at(py, &m, &credit, &credit_qual)?;

    schema::register(py, &m)?;

    let all = PyList::new(py, ["credit", "schema"])?;
    m.setattr("__all__", all)?;
    crate::bindings::module_utils::register_submodule_at(py, parent, &m, &qual)?;

    Ok(())
}
