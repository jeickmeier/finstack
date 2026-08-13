//! Typed Python wrappers for statement-analysis configs and root results.

use crate::bindings::pandas_utils::{
    serde_rows_to_dataframe_with_schema, table_to_dataframe, ColumnSchema,
};
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

/// Column schema for [`PyVarianceReport::to_dataframe`].
const VARIANCE_ROW_COLUMNS: [ColumnSchema<'static>; 6] = [
    ("period", "str"),
    ("metric", "str"),
    ("baseline", "float64"),
    ("comparison", "float64"),
    ("abs_var", "float64"),
    ("pct_var", "float64"),
];

/// Column schema for [`PyBridgeChart::to_dataframe`].
const BRIDGE_STEP_COLUMNS: [ColumnSchema<'static>; 2] =
    [("driver", "str"), ("contribution", "float64")];

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

    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Analysis mode: ``"Diagonal"``, ``"FullGrid"``, or ``"Tornado"``.
    #[getter]
    fn mode(&self) -> &'static str {
        sensitivity_mode_name(self.inner.mode)
    }

    /// Node identifiers of the statement metrics tracked across scenarios.
    #[getter]
    fn target_metrics(&self) -> Vec<String> {
        self.inner.target_metrics.clone()
    }

    /// Number of configured parameters (one `ParameterSpec` per entry).
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

    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Label for the baseline scenario (e.g. ``"management_case"``).
    #[getter]
    fn baseline_label(&self) -> &str {
        &self.inner.baseline_label
    }

    /// Label for the comparison scenario (e.g. ``"bank_case"``).
    #[getter]
    fn comparison_label(&self) -> &str {
        &self.inner.comparison_label
    }

    /// Node identifiers of the metrics compared between the two scenarios.
    #[getter]
    fn metrics(&self) -> Vec<String> {
        self.inner.metrics.clone()
    }

    /// Periods to compare, as period-id strings (e.g. ``"2025Q1"``).
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

    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Scenario names in definition (insertion) order.
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
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

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

    /// Node identifiers of the metrics tracked by the originating config.
    #[getter]
    fn target_metrics(&self) -> Vec<String> {
        self.inner.config.target_metrics.clone()
    }

    /// Export the per-scenario parameter values as a pandas ``DataFrame``.
    ///
    /// Columns: ``scenario`` (0-based scenario index), plus one column per
    /// perturbed parameter, named exactly as the Rust result keys it
    /// (``node_id@period``). One row per generated scenario; a parameter a
    /// given scenario does not perturb is ``NaN``. An empty result still
    /// carries the ``scenario`` column.
    ///
    /// Scenario *outputs* are not included — read them per node and period
    /// with ``get_value``.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let rows: Vec<serde_json::Value> = self
            .inner
            .scenarios
            .iter()
            .enumerate()
            .map(|(index, scenario)| {
                let mut row = serde_json::Map::new();
                row.insert("scenario".to_string(), serde_json::json!(index));
                for (parameter, value) in &scenario.parameter_values {
                    row.insert(parameter.clone(), serde_json::json!(value));
                }
                serde_json::Value::Object(row)
            })
            .collect();
        serde_rows_to_dataframe_with_schema(py, &rows, &[("scenario", "int64")])
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

    /// Render as an HTML table in Jupyter notebooks.
    ///
    /// Delegates to the frame from `to_dataframe`, so pandas' own row/column
    /// truncation applies and a large result stays a small repr. Returns
    /// `None` if the frame cannot be built, which makes IPython fall back to
    /// `__repr__` instead of raising from the display hook.
    fn _repr_html_(&self, py: Python<'_>) -> Option<String> {
        let frame = self.to_dataframe(py).ok()?;
        frame.call_method0("_repr_html_").ok()?.extract().ok()
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
    /// Period this row covers, as a period-id string (e.g. ``"2025Q1"``).
    #[getter]
    fn period(&self) -> String {
        self.inner.period.to_string()
    }

    /// Node identifier of the compared metric.
    #[getter]
    fn metric(&self) -> &str {
        &self.inner.metric
    }

    /// Metric value in the baseline scenario, in the metric's own units.
    #[getter]
    fn baseline(&self) -> f64 {
        self.inner.baseline
    }

    /// Metric value in the comparison scenario, in the metric's own units.
    #[getter]
    fn comparison(&self) -> f64 {
        self.inner.comparison
    }

    /// Absolute variance ``comparison - baseline``, in the metric's units.
    #[getter]
    fn abs_var(&self) -> f64 {
        self.inner.abs_var
    }

    /// Percentage variance ``abs_var / baseline`` as a decimal fraction
    /// (``0.1`` = +10%).
    ///
    /// ``None`` when the baseline is effectively zero, where a ratio would be
    /// undefined rather than zero; fall back to ``abs_var`` in that case.
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
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Label for the baseline scenario (e.g. ``"management_case"``).
    #[getter]
    fn baseline_label(&self) -> &str {
        &self.inner.baseline_label
    }

    /// Label for the comparison scenario (e.g. ``"bank_case"``).
    #[getter]
    fn comparison_label(&self) -> &str {
        &self.inner.comparison_label
    }

    /// Per-metric, per-period variance rows, in report order.
    #[getter]
    fn rows(&self) -> Vec<PyVarianceRow> {
        self.inner
            .rows
            .iter()
            .cloned()
            .map(|inner| PyVarianceRow { inner })
            .collect()
    }

    /// Export the variance rows as a pandas ``DataFrame``.
    ///
    /// Columns: ``period``, ``metric``, ``baseline``, ``comparison``,
    /// ``abs_var``, ``pct_var``. One row per (metric, period) pair, in report
    /// order; an empty report still carries the full column schema.
    ///
    /// ``baseline``, ``comparison`` and ``abs_var`` are in the metric's own
    /// units; ``pct_var`` is a decimal fraction (``0.1`` = +10%) and is
    /// ``NaN`` where the baseline is effectively zero. The scenario labels
    /// are report metadata (``baseline_label`` / ``comparison_label``) and are
    /// not repeated per row.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let rows: Vec<serde_json::Value> = self
            .inner
            .rows
            .iter()
            .map(|row| {
                serde_json::json!({
                    "period": row.period.to_string(),
                    "metric": row.metric,
                    "baseline": row.baseline,
                    "comparison": row.comparison,
                    "abs_var": row.abs_var,
                    // Emitted explicitly (not via `VarianceRow`'s serde, which
                    // skips a `None`) so the column exists even when every
                    // baseline is zero.
                    "pct_var": row.pct_var,
                })
            })
            .collect();
        serde_rows_to_dataframe_with_schema(py, &rows, &VARIANCE_ROW_COLUMNS)
    }

    /// Render as an HTML table in Jupyter notebooks.
    ///
    /// Delegates to the frame from `to_dataframe`, so pandas' own row/column
    /// truncation applies and a large result stays a small repr. Returns
    /// `None` if the frame cannot be built, which makes IPython fall back to
    /// `__repr__` instead of raising from the display hook.
    fn _repr_html_(&self, py: Python<'_>) -> Option<String> {
        let frame = self.to_dataframe(py).ok()?;
        frame.call_method0("_repr_html_").ok()?.extract().ok()
    }
}

/// Typed evaluated results for a set of named scenarios.
///
/// Named after the canonical Rust type
/// (`finstack_quant_statements_analytics::analysis::ScenarioResults`).
#[pyclass(
    name = "ScenarioResults",
    module = "finstack_quant.statements_analytics",
    from_py_object
)]
#[derive(Clone)]
pub struct PyScenarioResults {
    pub(crate) inner: ScenarioResults,
}

#[pymethods]
impl PyScenarioResults {
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

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

    /// Evaluated scenario names, in the order the scenario set defined them.
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
        let table = self.comparison_table(&metrics)?;
        crate::bindings::core::table::PyArrowTable::from_envelope(&table)
    }

    /// Export the scenario comparison as a pandas ``DataFrame``.
    ///
    /// Columns: ``period``, ``metric``, one column per scenario name holding
    /// that scenario's metric value, and one ``{scenario}_vs_{baseline}_pct``
    /// column per non-baseline scenario holding the relative change as a
    /// decimal fraction (``0.1`` = +10%, ``NaN`` on a near-zero baseline).
    /// One row per (metric, period) pair.
    ///
    /// This is the same table as ``to_comparison_table`` — both call one Rust
    /// implementation, so the two exports cannot drift apart. The baseline is
    /// the scenario named ``"base"`` when present, otherwise the first
    /// scenario.
    ///
    /// Parameters
    /// ----------
    /// metrics : list[str]
    ///     Node identifiers to include as rows.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the result set or ``metrics`` is empty.
    #[pyo3(text_signature = "(metrics)")]
    fn to_dataframe<'py>(
        &self,
        py: Python<'py>,
        metrics: Vec<String>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let table = self.comparison_table(&metrics)?;
        table_to_dataframe(py, &table)
    }
}

impl PyScenarioResults {
    /// Build the canonical comparison table shared by `to_comparison_table`
    /// and `to_dataframe`.
    fn comparison_table(
        &self,
        metrics: &[String],
    ) -> PyResult<finstack_quant_core::table::TableEnvelope> {
        let refs: Vec<&str> = metrics.iter().map(String::as_str).collect();
        self.inner.to_comparison_table(&refs).map_err(display_to_py)
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
    /// Name of the scenario used as the baseline of the diff.
    #[getter]
    fn baseline(&self) -> &str {
        &self.inner.baseline
    }

    /// Name of the scenario compared against the baseline.
    #[getter]
    fn comparison(&self) -> &str {
        &self.inner.comparison
    }

    /// Underlying variance report between the two named scenarios.
    #[getter]
    fn variance(&self) -> PyVarianceReport {
        PyVarianceReport {
            inner: self.inner.variance.clone(),
        }
    }

    /// Export the underlying variance rows as a pandas ``DataFrame``.
    ///
    /// Columns: ``period``, ``metric``, ``baseline``, ``comparison``,
    /// ``abs_var``, ``pct_var``. One row per (metric, period) pair, in report
    /// order; an empty diff still carries the full column schema.
    ///
    /// This is the same table as ``variance.to_dataframe()`` — both call one
    /// implementation, so the two cannot drift apart. The two scenario *names*
    /// are diff metadata (the ``baseline`` / ``comparison`` getters) and are
    /// not repeated per row; the ``baseline`` and ``comparison`` columns hold
    /// the metric *values* in each scenario.
    #[pyo3(text_signature = "($self)")]
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.variance().to_dataframe(py)
    }

    /// Render as an HTML table in Jupyter notebooks.
    ///
    /// Delegates to the frame from `to_dataframe`, so pandas' own row/column
    /// truncation applies and a large result stays a small repr. Returns
    /// `None` if the frame cannot be built, which makes IPython fall back to
    /// `__repr__` instead of raising from the display hook.
    fn _repr_html_(&self, py: Python<'_>) -> Option<String> {
        let frame = self.to_dataframe(py).ok()?;
        frame.call_method0("_repr_html_").ok()?.extract().ok()
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
    /// Driver node identifier (e.g. ``"revenue"``).
    #[getter]
    fn driver(&self) -> &str {
        &self.driver
    }

    /// This driver's raw delta between the two scenarios, in the *driver's*
    /// own units.
    ///
    /// Contributions are not sensitivities of the target metric, so they
    /// generally do not sum to the target variance — see
    /// ``BridgeChart.unexplained``.
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
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Node identifier of the metric this bridge decomposes (e.g.
    /// ``"ebitda"``).
    #[getter]
    fn target_metric(&self) -> &str {
        &self.inner.target_metric
    }

    /// Period the bridge covers, as a period-id string (e.g. ``"2025Q1"``).
    #[getter]
    fn period(&self) -> String {
        self.inner.period.to_string()
    }

    /// Label for the baseline scenario (e.g. ``"management_case"``).
    #[getter]
    fn baseline_label(&self) -> &str {
        &self.inner.baseline_label
    }

    /// Label for the comparison scenario (e.g. ``"bank_case"``).
    #[getter]
    fn comparison_label(&self) -> &str {
        &self.inner.comparison_label
    }

    /// Target-metric value in the baseline scenario, in the metric's units.
    #[getter]
    fn baseline_value(&self) -> f64 {
        self.inner.baseline_value
    }

    /// Target-metric value in the comparison scenario, in the metric's units.
    #[getter]
    fn comparison_value(&self) -> f64 {
        self.inner.comparison_value
    }

    /// Ordered driver contributions making up the bridge.
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

    /// Export the driver steps as a pandas ``DataFrame``.
    ///
    /// Columns: ``driver``, ``contribution``. One row per bridge step, in
    /// decomposition order; an empty bridge still carries both columns.
    /// Contributions are raw deltas in each driver's own units.
    ///
    /// The scalar header fields (``target_metric``, ``period``,
    /// ``baseline_label``, ``comparison_label``, ``baseline_value``,
    /// ``comparison_value``, ``unexplained``) are chart metadata and are not
    /// repeated on every row.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let rows: Vec<serde_json::Value> = self
            .inner
            .steps
            .iter()
            .map(|step| {
                serde_json::json!({
                    "driver": step.driver,
                    "contribution": step.contribution,
                })
            })
            .collect();
        serde_rows_to_dataframe_with_schema(py, &rows, &BRIDGE_STEP_COLUMNS)
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

    /// Render as an HTML table in Jupyter notebooks.
    ///
    /// Delegates to the frame from `to_dataframe`, so pandas' own row/column
    /// truncation applies and a large result stays a small repr. Returns
    /// `None` if the frame cannot be built, which makes IPython fall back to
    /// `__repr__` instead of raising from the display hook.
    fn _repr_html_(&self, py: Python<'_>) -> Option<String> {
        let frame = self.to_dataframe(py).ok()?;
        frame.call_method0("_repr_html_").ok()?.extract().ok()
    }
}

pub fn register(_py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PySensitivityConfig>()?;
    module.add_class::<PyVarianceConfig>()?;
    module.add_class::<PyScenarioSet>()?;
    module.add_class::<PySensitivityResult>()?;
    module.add_class::<PyVarianceRow>()?;
    module.add_class::<PyVarianceReport>()?;
    module.add_class::<PyScenarioResults>()?;
    module.add_class::<PyScenarioDiff>()?;
    module.add_class::<PyBridgeStep>()?;
    module.add_class::<PyBridgeChart>()?;
    Ok(())
}
