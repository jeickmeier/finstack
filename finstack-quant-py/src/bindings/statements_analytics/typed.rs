//! Typed Python wrappers for statement-analysis configs and root results.

use crate::bindings::statements::evaluator::PyStatementResult;
use crate::errors::display_to_py;
use finstack_quant_core::dates::PeriodId;
use finstack_quant_statements::evaluator::StatementResult;
use finstack_quant_statements_analytics::analysis::{
    BridgeChart as RustBridgeChart, ParameterSpec, ScenarioDefinition,
    ScenarioDiff as RustScenarioDiff, ScenarioResults, ScenarioSet as RustScenarioSet,
    SensitivityConfig as RustSensitivityConfig, SensitivityMode,
    SensitivityResult as RustSensitivityResult, VarianceConfig as RustVarianceConfig,
    VarianceReport as RustVarianceReport, VarianceRow as RustVarianceRow,
};
use indexmap::IndexMap;
use pyo3::exceptions::{PyIndexError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;

type SensitivityParameterInput = (String, String, f64, Vec<f64>);

fn parse_period(period: &str) -> PyResult<PeriodId> {
    period.parse().map_err(display_to_py)
}

fn parse_sensitivity_mode(mode: &str) -> PyResult<SensitivityMode> {
    match mode {
        "Diagonal" | "diagonal" => Ok(SensitivityMode::Diagonal),
        "FullGrid" | "full_grid" => Ok(SensitivityMode::FullGrid),
        "Tornado" | "tornado" => Ok(SensitivityMode::Tornado),
        _ => Err(PyValueError::new_err(format!(
            "unknown sensitivity mode '{mode}'; expected Diagonal, FullGrid, or Tornado"
        ))),
    }
}

fn sensitivity_mode_name(mode: SensitivityMode) -> &'static str {
    match mode {
        SensitivityMode::Diagonal => "Diagonal",
        SensitivityMode::FullGrid => "FullGrid",
        SensitivityMode::Tornado => "Tornado",
    }
}

fn extract_overrides(value: &Bound<'_, PyAny>) -> PyResult<IndexMap<String, f64>> {
    let values = value.cast::<PyDict>()?;
    let mut overrides = IndexMap::with_capacity(values.len());
    for (node_id, value) in values.iter() {
        overrides.insert(node_id.extract()?, value.extract()?);
    }
    Ok(overrides)
}

/// Configuration for statement sensitivity analysis.
#[pyclass(
    name = "SensitivityConfig",
    module = "finstack_quant.statements_analytics",
    from_py_object
)]
#[derive(Clone)]
pub struct PySensitivityConfig {
    pub(crate) inner: RustSensitivityConfig,
}

#[pymethods]
impl PySensitivityConfig {
    #[new]
    #[pyo3(signature = (mode, parameters=Vec::new(), target_metrics=Vec::new()))]
    fn new(
        mode: &str,
        parameters: Vec<SensitivityParameterInput>,
        target_metrics: Vec<String>,
    ) -> PyResult<Self> {
        let parameters = parameters
            .into_iter()
            .map(|(node_id, period, base_value, perturbations)| {
                Ok(ParameterSpec::new(
                    node_id,
                    parse_period(&period)?,
                    base_value,
                    perturbations,
                ))
            })
            .collect::<PyResult<Vec<_>>>()?;
        Ok(Self {
            inner: RustSensitivityConfig {
                mode: parse_sensitivity_mode(mode)?,
                parameters,
                target_metrics,
            },
        })
    }

    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    #[getter]
    fn mode(&self) -> &'static str {
        sensitivity_mode_name(self.inner.mode)
    }

    #[getter]
    fn target_metrics(&self) -> Vec<String> {
        self.inner.target_metrics.clone()
    }

    #[getter]
    fn parameter_count(&self) -> usize {
        self.inner.parameters.len()
    }
}

/// Configuration for comparing two statement results.
#[pyclass(
    name = "VarianceConfig",
    module = "finstack_quant.statements_analytics",
    from_py_object
)]
#[derive(Clone)]
pub struct PyVarianceConfig {
    pub(crate) inner: RustVarianceConfig,
}

#[pymethods]
impl PyVarianceConfig {
    #[new]
    fn new(
        baseline_label: &str,
        comparison_label: &str,
        metrics: Vec<String>,
        periods: Vec<String>,
    ) -> PyResult<Self> {
        let periods = periods
            .iter()
            .map(|period| parse_period(period))
            .collect::<PyResult<Vec<_>>>()?;
        Ok(Self {
            inner: RustVarianceConfig::new(baseline_label, comparison_label, metrics, periods),
        })
    }

    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    #[getter]
    fn baseline_label(&self) -> &str {
        &self.inner.baseline_label
    }

    #[getter]
    fn comparison_label(&self) -> &str {
        &self.inner.comparison_label
    }

    #[getter]
    fn metrics(&self) -> Vec<String> {
        self.inner.metrics.clone()
    }

    #[getter]
    fn periods(&self) -> Vec<String> {
        self.inner.periods.iter().map(ToString::to_string).collect()
    }
}

/// Named scenario definitions for statement-model evaluation.
#[pyclass(
    name = "ScenarioSet",
    module = "finstack_quant.statements_analytics",
    from_py_object
)]
#[derive(Clone)]
pub struct PyScenarioSet {
    pub(crate) inner: RustScenarioSet,
}

#[pymethods]
impl PyScenarioSet {
    #[new]
    #[pyo3(signature = (scenarios, parents=None))]
    fn new(scenarios: &Bound<'_, PyDict>, parents: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let mut definitions = IndexMap::with_capacity(scenarios.len());
        for (name, overrides) in scenarios.iter() {
            let name = name.extract::<String>()?;
            let overrides = extract_overrides(&overrides)?;
            let parent = parents
                .and_then(|items| items.get_item(&name).transpose())
                .transpose()?
                .map(|value| value.extract::<String>())
                .transpose()?;
            definitions.insert(name, ScenarioDefinition { parent, overrides });
        }
        Ok(Self {
            inner: RustScenarioSet {
                scenarios: definitions,
            },
        })
    }

    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    #[getter]
    fn names(&self) -> Vec<String> {
        self.inner.scenarios.keys().cloned().collect()
    }

    /// Resolve a scenario's inheritance lineage, root-first.
    ///
    /// Parameters
    /// ----------
    /// scenario : str
    ///     Name of the scenario to trace.
    ///
    /// Returns
    /// -------
    /// list[str]
    ///     Scenario names from the root ancestor through to `scenario`.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the scenario is unknown or its parent chain contains a cycle.
    #[pyo3(text_signature = "(scenario)")]
    fn trace(&self, scenario: &str) -> PyResult<Vec<String>> {
        self.inner.trace(scenario).map_err(display_to_py)
    }
}

/// Typed root result for statement sensitivity analysis.
#[pyclass(
    name = "SensitivityResult",
    module = "finstack_quant.statements_analytics",
    from_py_object
)]
#[derive(Clone)]
pub struct PySensitivityResult {
    pub(crate) inner: RustSensitivityResult,
}

#[pymethods]
impl PySensitivityResult {
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    #[getter]
    fn target_metrics(&self) -> Vec<String> {
        self.inner.config.target_metrics.clone()
    }

    fn get_parameter_value(&self, scenario_index: usize, parameter: &str) -> PyResult<Option<f64>> {
        let scenario = self
            .inner
            .scenarios
            .get(scenario_index)
            .ok_or_else(|| PyIndexError::new_err("scenario index out of range"))?;
        Ok(scenario.parameter_values.get(parameter).copied())
    }

    fn get_value(
        &self,
        scenario_index: usize,
        node_id: &str,
        period: &str,
    ) -> PyResult<Option<f64>> {
        let scenario = self
            .inner
            .scenarios
            .get(scenario_index)
            .ok_or_else(|| PyIndexError::new_err("scenario index out of range"))?;
        Ok(scenario.results.get(node_id, &parse_period(period)?))
    }
}

/// One typed variance-report row.
#[pyclass(
    name = "VarianceRow",
    module = "finstack_quant.statements_analytics",
    from_py_object
)]
#[derive(Clone)]
pub struct PyVarianceRow {
    inner: RustVarianceRow,
}

#[pymethods]
impl PyVarianceRow {
    #[getter]
    fn period(&self) -> String {
        self.inner.period.to_string()
    }

    #[getter]
    fn metric(&self) -> &str {
        &self.inner.metric
    }

    #[getter]
    fn baseline(&self) -> f64 {
        self.inner.baseline
    }

    #[getter]
    fn comparison(&self) -> f64 {
        self.inner.comparison
    }

    #[getter]
    fn abs_var(&self) -> f64 {
        self.inner.abs_var
    }

    #[getter]
    fn pct_var(&self) -> Option<f64> {
        self.inner.pct_var
    }
}

/// Typed root variance report.
#[pyclass(
    name = "VarianceReport",
    module = "finstack_quant.statements_analytics",
    from_py_object
)]
#[derive(Clone)]
pub struct PyVarianceReport {
    pub(crate) inner: RustVarianceReport,
}

#[pymethods]
impl PyVarianceReport {
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    #[getter]
    fn baseline_label(&self) -> &str {
        &self.inner.baseline_label
    }

    #[getter]
    fn comparison_label(&self) -> &str {
        &self.inner.comparison_label
    }

    #[getter]
    fn rows(&self) -> Vec<PyVarianceRow> {
        self.inner
            .rows
            .iter()
            .cloned()
            .map(|inner| PyVarianceRow { inner })
            .collect()
    }
}

/// Typed evaluated results for a set of named scenarios.
#[pyclass(
    name = "ScenarioResultSet",
    module = "finstack_quant.statements_analytics",
    from_py_object
)]
#[derive(Clone)]
pub struct PyScenarioResultSet {
    pub(crate) inner: ScenarioResults,
}

#[pymethods]
impl PyScenarioResultSet {
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let scenarios: IndexMap<String, StatementResult> =
            serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self {
            inner: ScenarioResults { scenarios },
        })
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner.scenarios).map_err(display_to_py)
    }

    #[getter]
    fn names(&self) -> Vec<String> {
        self.inner.scenarios.keys().cloned().collect()
    }

    fn get(&self, name: &str) -> Option<PyStatementResult> {
        self.inner
            .scenarios
            .get(name)
            .cloned()
            .map(|inner| PyStatementResult { inner })
    }

    /// Build a side-by-side comparison table across every evaluated scenario.
    ///
    /// Parameters
    /// ----------
    /// metrics : list[str]
    ///     Node identifiers to include as rows.
    ///
    /// Returns
    /// -------
    /// ArrowTable
    ///     One column per scenario, one row per (metric, period).
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the result set or `metrics` is empty.
    #[pyo3(text_signature = "(metrics)")]
    fn to_comparison_table(
        &self,
        metrics: Vec<String>,
    ) -> PyResult<crate::bindings::core::table::PyArrowTable> {
        let refs: Vec<&str> = metrics.iter().map(String::as_str).collect();
        let table = self
            .inner
            .to_comparison_table(&refs)
            .map_err(display_to_py)?;
        crate::bindings::core::table::PyArrowTable::from_envelope(&table)
    }
}

/// Variance between two named scenarios in an evaluated scenario set.
#[pyclass(
    name = "ScenarioDiff",
    module = "finstack_quant.statements_analytics",
    from_py_object
)]
#[derive(Clone)]
pub struct PyScenarioDiff {
    pub(crate) inner: RustScenarioDiff,
}

#[pymethods]
impl PyScenarioDiff {
    #[getter]
    fn baseline(&self) -> &str {
        &self.inner.baseline
    }

    #[getter]
    fn comparison(&self) -> &str {
        &self.inner.comparison
    }

    #[getter]
    fn variance(&self) -> PyVarianceReport {
        PyVarianceReport {
            inner: self.inner.variance.clone(),
        }
    }
}

/// One driver step in a bridge decomposition.
#[pyclass(
    name = "BridgeStep",
    module = "finstack_quant.statements_analytics",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyBridgeStep {
    driver: String,
    contribution: f64,
}

#[pymethods]
impl PyBridgeStep {
    #[getter]
    fn driver(&self) -> &str {
        &self.driver
    }

    #[getter]
    fn contribution(&self) -> f64 {
        self.contribution
    }
}

/// Bridge decomposition of a metric's variance across named drivers.
#[pyclass(
    name = "BridgeChart",
    module = "finstack_quant.statements_analytics",
    from_py_object
)]
#[derive(Clone)]
pub struct PyBridgeChart {
    pub(crate) inner: RustBridgeChart,
}

#[pymethods]
impl PyBridgeChart {
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    #[getter]
    fn target_metric(&self) -> &str {
        &self.inner.target_metric
    }

    #[getter]
    fn period(&self) -> String {
        self.inner.period.to_string()
    }

    #[getter]
    fn baseline_label(&self) -> &str {
        &self.inner.baseline_label
    }

    #[getter]
    fn comparison_label(&self) -> &str {
        &self.inner.comparison_label
    }

    #[getter]
    fn baseline_value(&self) -> f64 {
        self.inner.baseline_value
    }

    #[getter]
    fn comparison_value(&self) -> f64 {
        self.inner.comparison_value
    }

    #[getter]
    fn steps(&self) -> Vec<PyBridgeStep> {
        self.inner
            .steps
            .iter()
            .map(|step| PyBridgeStep {
                driver: step.driver.clone(),
                contribution: step.contribution,
            })
            .collect()
    }

    /// Residual variance not explained by the driver deltas.
    ///
    /// Driver contributions are raw deltas in driver units rather than
    /// sensitivities of the target metric, so they generally do not sum to
    /// the target variance. This term makes that gap explicit.
    #[getter]
    fn unexplained(&self) -> f64 {
        self.inner.unexplained
    }
}

pub fn register(_py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PySensitivityConfig>()?;
    module.add_class::<PyVarianceConfig>()?;
    module.add_class::<PyScenarioSet>()?;
    module.add_class::<PySensitivityResult>()?;
    module.add_class::<PyVarianceRow>()?;
    module.add_class::<PyVarianceReport>()?;
    module.add_class::<PyScenarioResultSet>()?;
    module.add_class::<PyScenarioDiff>()?;
    module.add_class::<PyBridgeStep>()?;
    module.add_class::<PyBridgeChart>()?;
    Ok(())
}
