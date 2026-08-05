use crate::bindings::module_utils::py_to_serde;
use indexmap::IndexMap;
use pyo3::prelude::*;
use pyo3::types::PyType;
use serde::Deserialize;

use finstack_quant_portfolio::factor_model::{
    self as fm, FactorContributionDelta, PositionBudgetEntry, RiskBudget, RiskBudgetResult,
    WhatIfResult,
};
use finstack_quant_portfolio::types::PositionId;

use crate::bindings::date_utils::parse_iso_date_py;
use crate::bindings::extract::{extract_market_ref, extract_portfolio_ref};
use crate::errors::portfolio_to_py;

use super::super::json_bridge::{deserialize_json, serialize_json};
use super::contributions::PyRiskDecomposition;

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum PositionChangeSpec {
    Remove {
        position_id: String,
    },
    Resize {
        position_id: String,
        new_quantity: f64,
    },
}

fn parse_position_changes(
    py: Python<'_>,
    changes: &Bound<'_, PyAny>,
) -> PyResult<Vec<fm::PositionChange>> {
    let specs: Vec<PositionChangeSpec> = py_to_serde(py, changes, "position changes")?;
    Ok(py.detach(move || {
        specs
            .into_iter()
            .map(|spec| match spec {
                PositionChangeSpec::Remove { position_id } => fm::PositionChange::Remove {
                    position_id: PositionId::new(position_id),
                },
                PositionChangeSpec::Resize {
                    position_id,
                    new_quantity,
                } => fm::PositionChange::Resize {
                    position_id: PositionId::new(position_id),
                    new_quantity,
                },
            })
            .collect::<Vec<_>>()
    }))
}

/// Per-position budget comparison entry.
#[pyclass(
    name = "PositionBudgetEntry",
    module = "finstack_quant.portfolio",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyPositionBudgetEntry {
    pub(crate) inner: PositionBudgetEntry,
}

impl PyPositionBudgetEntry {
    fn from_inner(inner: PositionBudgetEntry) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPositionBudgetEntry {
    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        let inner: PositionBudgetEntry = deserialize_json(json_str)?;
        Ok(Self::from_inner(inner))
    }

    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serialize_json(&self.inner)
    }

    #[getter]
    fn position_id(&self) -> String {
        self.inner.position_id.as_str().to_owned()
    }

    #[getter]
    fn actual_component_var(&self) -> f64 {
        self.inner.actual_component_var
    }

    #[getter]
    fn target_component_var(&self) -> f64 {
        self.inner.target_component_var
    }

    #[getter]
    fn utilization(&self) -> f64 {
        self.inner.utilization
    }

    #[getter]
    fn excess(&self) -> f64 {
        self.inner.excess
    }

    fn __repr__(&self) -> String {
        format!(
            "PositionBudgetEntry(position_id={:?}, actual={}, target={}, utilization={}, excess={})",
            self.inner.position_id.as_str(),
            self.inner.actual_component_var,
            self.inner.target_component_var,
            self.inner.utilization,
            self.inner.excess,
        )
    }
}

/// Budget evaluation result across positions.
#[pyclass(
    name = "RiskBudgetResult",
    module = "finstack_quant.portfolio",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyRiskBudgetResult {
    pub(crate) inner: RiskBudgetResult,
}

impl PyRiskBudgetResult {
    fn from_inner(inner: RiskBudgetResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRiskBudgetResult {
    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        let inner: RiskBudgetResult = deserialize_json(json_str)?;
        Ok(Self::from_inner(inner))
    }

    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serialize_json(&self.inner)
    }

    #[getter]
    fn total_overbudget(&self) -> f64 {
        self.inner.total_overbudget
    }

    #[getter]
    fn has_breach(&self) -> bool {
        self.inner.has_breach
    }

    #[getter]
    fn positions(&self) -> Vec<PyPositionBudgetEntry> {
        self.inner
            .positions
            .iter()
            .cloned()
            .map(PyPositionBudgetEntry::from_inner)
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "RiskBudgetResult(positions={}, total_overbudget={}, has_breach={})",
            self.inner.positions.len(),
            self.inner.total_overbudget,
            self.inner.has_breach,
        )
    }
}

/// Per-factor contribution change between a baseline and a scenario.
#[pyclass(
    name = "FactorContributionDelta",
    module = "finstack_quant.portfolio",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyFactorContributionDelta {
    pub(crate) inner: FactorContributionDelta,
}

impl PyFactorContributionDelta {
    fn from_inner(inner: FactorContributionDelta) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFactorContributionDelta {
    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        let inner: FactorContributionDelta = deserialize_json(json_str)?;
        Ok(Self::from_inner(inner))
    }

    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serialize_json(&self.inner)
    }

    #[getter]
    fn factor_id(&self) -> String {
        self.inner.factor_id.as_str().to_owned()
    }

    #[getter]
    fn absolute_change(&self) -> f64 {
        self.inner.absolute_change
    }

    #[getter]
    fn relative_change(&self) -> f64 {
        self.inner.relative_change
    }

    fn __repr__(&self) -> String {
        format!(
            "FactorContributionDelta(factor_id={:?}, absolute_change={}, relative_change={})",
            self.inner.factor_id.as_str(),
            self.inner.absolute_change,
            self.inner.relative_change,
        )
    }
}

/// Result of a position what-if scenario.
#[pyclass(
    name = "WhatIfResult",
    module = "finstack_quant.portfolio",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyWhatIfResult {
    pub(crate) inner: WhatIfResult,
}

impl PyWhatIfResult {
    fn from_inner(inner: WhatIfResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyWhatIfResult {
    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        let inner: WhatIfResult = deserialize_json(json_str)?;
        Ok(Self::from_inner(inner))
    }

    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serialize_json(&self.inner)
    }

    #[getter]
    fn before(&self) -> PyRiskDecomposition {
        PyRiskDecomposition::from_inner(self.inner.before.clone())
    }

    #[getter]
    fn after(&self) -> PyRiskDecomposition {
        PyRiskDecomposition::from_inner(self.inner.after.clone())
    }

    #[getter]
    fn delta(&self) -> Vec<PyFactorContributionDelta> {
        self.inner
            .delta
            .iter()
            .cloned()
            .map(PyFactorContributionDelta::from_inner)
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "WhatIfResult(before_total={}, after_total={}, delta_entries={})",
            self.inner.before.total_risk,
            self.inner.after.total_risk,
            self.inner.delta.len(),
        )
    }
}

/// Evaluate a per-position risk budget against actual component VaRs,
/// returning a typed :class:`RiskBudgetResult`.
#[pyfunction]
#[pyo3(signature = (position_ids, actual_var, target_var_pct, portfolio_var, utilization_threshold = 1.20))]
pub(super) fn evaluate_risk_budget(
    py: Python<'_>,
    position_ids: Vec<String>,
    actual_var: Vec<f64>,
    target_var_pct: Vec<f64>,
    portfolio_var: f64,
    utilization_threshold: f64,
) -> PyResult<PyRiskBudgetResult> {
    let n = position_ids.len();
    if actual_var.len() != n {
        return Err(crate::errors::value_error(format!(
            "actual_var length ({}) must match position_ids length ({n})",
            actual_var.len()
        )));
    }
    if target_var_pct.len() != n {
        return Err(crate::errors::value_error(format!(
            "target_var_pct length ({}) must match position_ids length ({n})",
            target_var_pct.len()
        )));
    }

    let (shared_ids, budget, actual_var) = py
        .detach(move || {
            let shared_ids: Vec<PositionId> =
                position_ids.into_iter().map(PositionId::new).collect();
            let mut targets: IndexMap<PositionId, f64> = IndexMap::with_capacity(n);
            for (id, &pct) in shared_ids.iter().zip(target_var_pct.iter()) {
                if targets.insert(id.clone(), pct).is_some() {
                    return Err(format!(
                        "duplicate position_id '{}' in position_ids",
                        id.as_str()
                    ));
                }
            }
            Ok((
                shared_ids,
                RiskBudget::new(targets).with_threshold(utilization_threshold),
                actual_var,
            ))
        })
        .map_err(crate::errors::value_error)?;
    let result = py
        .detach(move || {
            budget.evaluate_components(
                shared_ids.iter().zip(actual_var.iter().copied()),
                portfolio_var,
            )
        })
        .map_err(crate::errors::core_to_py)?;

    Ok(PyRiskBudgetResult::from_inner(result))
}

/// Run position remove/resize what-if analysis from a factor-model config.
#[pyfunction]
#[pyo3(signature = (portfolio, market, factor_model_config_json, as_of, changes))]
pub(super) fn position_what_if(
    py: Python<'_>,
    portfolio: &Bound<'_, PyAny>,
    market: &Bound<'_, PyAny>,
    factor_model_config_json: &str,
    as_of: &str,
    changes: &Bound<'_, PyAny>,
) -> PyResult<PyWhatIfResult> {
    let portfolio = extract_portfolio_ref(py, portfolio)?;
    let market = extract_market_ref(py, market)?;
    let as_of = parse_iso_date_py(as_of)?;
    let config_json = factor_model_config_json.to_owned();
    let config: finstack_quant_factor_model::FactorModelConfig = py
        .detach(move || serde_json::from_str(&config_json))
        .map_err(crate::errors::display_to_py)?;
    let changes = parse_position_changes(py, changes)?;

    let portfolio_ref: &finstack_quant_portfolio::Portfolio = &portfolio;
    let market_ref: &finstack_quant_core::market_data::context::MarketContext = &market;
    let result = py
        .detach(move || {
            let model = fm::FactorModelBuilder::new().config(config).build()?;
            let (base, sensitivities) =
                model.analyze_with_sensitivities(portfolio_ref, market_ref, as_of)?;
            model
                .what_if(&base, &sensitivities, portfolio_ref, market_ref, as_of)
                .position_what_if(&changes)
        })
        .map_err(portfolio_to_py)?;

    Ok(PyWhatIfResult::from_inner(result))
}
