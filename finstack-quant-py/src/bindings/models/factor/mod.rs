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
    crate::bindings::portfolio::factor_model::register_credit_forecast(&credit)?;

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
            "VolHorizon",
            "decompose_levels",
            "decompose_period",
        ],
    )?;
    credit.setattr("__all__", credit_all)?;
    crate::bindings::module_utils::register_submodule_at(py, &m, &credit, &credit_qual)?;

    let risk = PyModule::new(py, "risk")?;
    let risk_qual =
        crate::bindings::module_utils::set_submodule_package_by_package(&m, &risk, "risk", &qual)?;
    risk.setattr(
        "__doc__",
        "Product-independent factor and position risk decomposition kernels.",
    )?;
    crate::bindings::portfolio::factor_model::register_risk(&risk)?;
    let risk_all = PyList::new(
        py,
        [
            "FactorContribution",
            "PositionFactorContribution",
            "PositionResidualContribution",
            "RiskDecomposition",
            "PositionVarContribution",
            "PositionEsContribution",
            "PositionRiskDecomposition",
            "PositionBudgetEntry",
            "RiskBudgetResult",
            "StressPositionEntry",
            "TailScenarioBreakdown",
            "StressAttribution",
            "DecompositionConfig",
            "parametric_var_decomposition",
            "parametric_es_decomposition",
            "historical_var_decomposition",
            "evaluate_risk_budget",
            "build_stress_attribution",
            "position_component_var",
        ],
    )?;
    risk.setattr("__all__", risk_all)?;
    crate::bindings::module_utils::register_submodule_at(py, &m, &risk, &risk_qual)?;

    schema::register(py, &m)?;

    let all = PyList::new(py, ["credit", "risk", "schema"])?;
    m.setattr("__all__", all)?;
    crate::bindings::module_utils::register_submodule_at(py, parent, &m, &qual)?;

    Ok(())
}
