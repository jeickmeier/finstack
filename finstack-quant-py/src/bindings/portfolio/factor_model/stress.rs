use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use finstack_quant_models::factor::risk::{
    self as model_risk, StressAttribution, StressPositionEntry, TailScenarioBreakdown,
};
use finstack_quant_portfolio::factor_model::{self as fm, StressResult};

use crate::bindings::extract::{extract_market_ref, extract_portfolio_ref};
use crate::bindings::pandas_utils::dict_to_dataframe;
use crate::errors::{core_to_py, display_to_py, portfolio_to_py, value_error};

use super::super::json_bridge::{deserialize_json, serialize_json};
use super::super::matrix_input::extract_position_pnls;
use super::contributions::PyRiskDecomposition;

/// Result of a factor-stress scenario.
#[pyclass(
    name = "StressResult",
    module = "finstack_quant.portfolio",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyStressResult {
    pub(crate) inner: StressResult,
}

impl PyStressResult {
    fn from_inner(inner: StressResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyStressResult {
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Parse from a JSON string.
    #[staticmethod]
    #[pyo3(text_signature = "(json_str)")]
    fn from_json(json_str: &str) -> PyResult<Self> {
        let inner: StressResult = deserialize_json(json_str)?;
        Ok(Self::from_inner(inner))
    }

    /// Serialize to JSON.
    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serialize_json(&self.inner)
    }

    /// Total portfolio P&L under the stressed market.
    ///
    /// Returns
    /// -------
    /// float
    ///     Portfolio-currency amount; a loss is **negative**.
    #[getter]
    fn total_pnl(&self) -> f64 {
        self.inner.total_pnl
    }

    /// Per-position ``(position_id, pnl)`` entries.
    #[getter]
    fn position_pnl(&self) -> Vec<(String, f64)> {
        self.inner
            .position_pnl
            .iter()
            .map(|(id, pnl)| (id.as_str().to_owned(), *pnl))
            .collect()
    }

    /// Risk decomposition recomputed under the stressed market.
    #[getter]
    fn stressed_decomposition(&self) -> PyRiskDecomposition {
        PyRiskDecomposition::from_inner(self.inner.stressed_decomposition.clone())
    }

    /// Export the per-position stressed P&L as a pandas ``DataFrame``.
    ///
    /// One row per entry of :attr:`position_pnl`. The scenario totals stay on
    /// :attr:`total_pnl` and :attr:`stressed_decomposition`.
    ///
    /// Columns: ``position_id``, ``pnl`` (portfolio-currency amount; a loss is
    /// negative).
    #[pyo3(text_signature = "(self)")]
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let rows = &self.inner.position_pnl;
        let position_ids: Vec<&str> = rows.iter().map(|(id, _)| id.as_str()).collect();
        let pnl: Vec<f64> = rows.iter().map(|(_, pnl)| *pnl).collect();
        let data = PyDict::new(py);
        data.set_item("position_id", position_ids)?;
        data.set_item("pnl", pnl)?;
        dict_to_dataframe(py, &data, None)
    }

    fn __repr__(&self) -> String {
        format!(
            "StressResult(total_pnl={}, positions={}, stressed_total_risk={})",
            self.inner.total_pnl,
            self.inner.position_pnl.len(),
            self.inner.stressed_decomposition.total_risk,
        )
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

/// Single position's contribution to tail stress.
#[pyclass(
    name = "StressPositionEntry",
    module = "finstack_quant.models.factor.risk",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyStressPositionEntry {
    pub(crate) inner: StressPositionEntry,
}

impl PyStressPositionEntry {
    fn from_inner(inner: StressPositionEntry) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyStressPositionEntry {
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Parse from a JSON string.
    #[staticmethod]
    #[pyo3(text_signature = "(json_str)")]
    fn from_json(json_str: &str) -> PyResult<Self> {
        let inner: StressPositionEntry = deserialize_json(json_str)?;
        Ok(Self::from_inner(inner))
    }

    /// Serialize to JSON.
    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serialize_json(&self.inner)
    }

    /// Portfolio position identifier.
    #[getter]
    fn position_id(&self) -> String {
        self.inner.position_id.as_str().to_owned()
    }

    /// Average P&L contribution across the tail scenarios; a loss is negative.
    #[getter]
    fn avg_tail_pnl(&self) -> f64 {
        self.inner.avg_tail_pnl
    }

    /// Share of total portfolio tail loss attributable to this position, as a
    /// **fraction** (not a percentage) despite the ``pct_`` name.
    #[getter]
    fn pct_of_tail_loss(&self) -> f64 {
        self.inner.pct_of_tail_loss
    }

    /// Worst single-scenario P&L for this position; a loss is negative.
    #[getter]
    fn worst_scenario_pnl(&self) -> f64 {
        self.inner.worst_scenario_pnl
    }

    fn __repr__(&self) -> String {
        format!(
            "StressPositionEntry(position_id={:?}, avg_tail_pnl={}, pct_of_tail_loss={}, worst_scenario_pnl={})",
            self.inner.position_id.as_str(),
            self.inner.avg_tail_pnl,
            self.inner.pct_of_tail_loss,
            self.inner.worst_scenario_pnl,
        )
    }
}

/// Breakdown of a single tail scenario.
#[pyclass(
    name = "TailScenarioBreakdown",
    module = "finstack_quant.models.factor.risk",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyTailScenarioBreakdown {
    pub(crate) inner: TailScenarioBreakdown,
}

impl PyTailScenarioBreakdown {
    fn from_inner(inner: TailScenarioBreakdown) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyTailScenarioBreakdown {
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Parse from a JSON string.
    #[staticmethod]
    #[pyo3(text_signature = "(json_str)")]
    fn from_json(json_str: &str) -> PyResult<Self> {
        let inner: TailScenarioBreakdown = deserialize_json(json_str)?;
        Ok(Self::from_inner(inner))
    }

    /// Serialize to JSON.
    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serialize_json(&self.inner)
    }

    /// Zero-based index of this scenario in the original P&L history.
    #[getter]
    fn scenario_index(&self) -> usize {
        self.inner.scenario_index
    }

    /// Total portfolio P&L for this scenario; a loss is negative.
    #[getter]
    fn portfolio_pnl(&self) -> f64 {
        self.inner.portfolio_pnl
    }

    /// Per-position P&L for this scenario, index-aligned to
    /// :attr:`StressAttribution.position_ids` (entry ``i`` is the P&L for
    /// ``position_ids[i]``).
    #[getter]
    fn position_pnls(&self) -> Vec<f64> {
        self.inner.position_pnls.clone()
    }

    /// Export this scenario's per-position P&L as a pandas ``DataFrame``.
    ///
    /// The breakdown carries no identifiers of its own — they live once on the
    /// parent :attr:`StressAttribution.position_ids` — so they must be supplied
    /// here, exactly as for :meth:`FactorPnlProfile.to_dataframe`.
    ///
    /// Columns: ``position_id``, ``pnl`` (portfolio-currency amount; a loss is
    /// negative).
    ///
    /// Parameters
    /// ----------
    /// position_ids : list[str]
    ///     Position identifiers, normally ``attribution.position_ids``. Must
    ///     match the number of entries in :attr:`position_pnls`.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``len(position_ids)`` does not match ``len(position_pnls)``.
    #[pyo3(text_signature = "(self, position_ids)")]
    fn to_dataframe<'py>(
        &self,
        py: Python<'py>,
        position_ids: Vec<String>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let expected = self.inner.position_pnls.len();
        if position_ids.len() != expected {
            return Err(value_error(format!(
                "position_ids length ({}) does not match position_pnls length ({expected})",
                position_ids.len(),
            )));
        }
        let data = PyDict::new(py);
        data.set_item("position_id", position_ids)?;
        data.set_item("pnl", self.inner.position_pnls.clone())?;
        dict_to_dataframe(py, &data, None)
    }

    fn __repr__(&self) -> String {
        format!(
            "TailScenarioBreakdown(scenario_index={}, portfolio_pnl={}, positions={})",
            self.inner.scenario_index,
            self.inner.portfolio_pnl,
            self.inner.position_pnls.len(),
        )
    }
}

/// Per-position attribution of portfolio losses in tail scenarios.
#[pyclass(
    name = "StressAttribution",
    module = "finstack_quant.models.factor.risk",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyStressAttribution {
    pub(crate) inner: StressAttribution,
}

impl PyStressAttribution {
    fn from_inner(inner: StressAttribution) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyStressAttribution {
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Parse from a JSON string.
    #[staticmethod]
    #[pyo3(text_signature = "(json_str)")]
    fn from_json(json_str: &str) -> PyResult<Self> {
        let inner: StressAttribution = deserialize_json(json_str)?;
        Ok(Self::from_inner(inner))
    }

    /// Serialize to JSON.
    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serialize_json(&self.inner)
    }

    /// Portfolio VaR threshold separating tail scenarios from the rest.
    ///
    /// Returns
    /// -------
    /// float
    ///     Loss convention: reported as a **negative** number. Scenarios whose
    ///     portfolio P&L is at or below this value are tail events.
    #[getter]
    fn var_threshold(&self) -> f64 {
        self.inner.var_threshold
    }

    /// Number of scenarios classified as tail events.
    #[getter]
    fn n_tail_scenarios(&self) -> usize {
        self.inner.n_tail_scenarios
    }

    /// Canonical position ordering shared by every entry of
    /// :attr:`tail_scenarios`. ``tail_scenarios[k].position_pnls[i]`` is the
    /// P&L for ``position_ids[i]``.
    #[getter]
    fn position_ids(&self) -> Vec<String> {
        self.inner
            .position_ids
            .iter()
            .map(|id| id.as_str().to_owned())
            .collect()
    }

    /// Per-position average tail-loss contributions, sorted by absolute
    /// contribution (largest risk driver first).
    #[getter]
    fn position_contributions(&self) -> Vec<PyStressPositionEntry> {
        self.inner
            .position_contributions
            .iter()
            .cloned()
            .map(PyStressPositionEntry::from_inner)
            .collect()
    }

    /// Per-scenario breakdowns for every tail event.
    #[getter]
    fn tail_scenarios(&self) -> Vec<PyTailScenarioBreakdown> {
        self.inner
            .tail_scenarios
            .iter()
            .cloned()
            .map(PyTailScenarioBreakdown::from_inner)
            .collect()
    }

    /// Export the per-position tail-loss contributions as a pandas ``DataFrame``.
    ///
    /// One row per entry of :attr:`position_contributions`, in the same
    /// (largest-driver-first) order.
    ///
    /// Columns: ``position_id``, ``avg_tail_pnl`` (portfolio-currency average
    /// across tail scenarios; a loss is negative), ``pct_of_tail_loss``
    /// (**fraction**, not percentage, of total portfolio tail loss),
    /// ``worst_scenario_pnl``.
    #[pyo3(text_signature = "(self)")]
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let rows = &self.inner.position_contributions;
        let position_ids: Vec<&str> = rows.iter().map(|e| e.position_id.as_str()).collect();
        let avg_tail_pnl: Vec<f64> = rows.iter().map(|e| e.avg_tail_pnl).collect();
        let pct_of_tail_loss: Vec<f64> = rows.iter().map(|e| e.pct_of_tail_loss).collect();
        let worst_scenario_pnl: Vec<f64> = rows.iter().map(|e| e.worst_scenario_pnl).collect();
        let data = PyDict::new(py);
        data.set_item("position_id", position_ids)?;
        data.set_item("avg_tail_pnl", avg_tail_pnl)?;
        data.set_item("pct_of_tail_loss", pct_of_tail_loss)?;
        data.set_item("worst_scenario_pnl", worst_scenario_pnl)?;
        dict_to_dataframe(py, &data, None)
    }

    /// Export the tail scenario × position P&L matrix as a pandas ``DataFrame``.
    ///
    /// Rows are tail scenarios indexed by
    /// :attr:`TailScenarioBreakdown.scenario_index`; columns are the position
    /// identifiers from :attr:`position_ids`, in that order. Every cell is a
    /// portfolio-currency P&L (a loss is negative).
    ///
    /// Columns: one per entry of :attr:`position_ids`.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If any tail scenario's ``position_pnls`` width disagrees with
    ///     ``len(position_ids)`` (only reachable through a hand-built
    ///     :meth:`from_json` payload).
    #[pyo3(text_signature = "(self)")]
    fn to_scenario_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let n_positions = self.inner.position_ids.len();
        for scenario in &self.inner.tail_scenarios {
            if scenario.position_pnls.len() != n_positions {
                return Err(value_error(format!(
                    "tail scenario {} has {} position P&Ls but {n_positions} position ids",
                    scenario.scenario_index,
                    scenario.position_pnls.len(),
                )));
            }
        }
        let data = PyDict::new(py);
        for (i, position_id) in self.inner.position_ids.iter().enumerate() {
            let column: Vec<f64> = self
                .inner
                .tail_scenarios
                .iter()
                .map(|scenario| scenario.position_pnls[i])
                .collect();
            data.set_item(position_id.as_str(), column)?;
        }
        let index: Vec<usize> = self
            .inner
            .tail_scenarios
            .iter()
            .map(|scenario| scenario.scenario_index)
            .collect();
        let index = PyList::new(py, index)?;
        dict_to_dataframe(py, &data, Some(index.into_any()))
    }

    fn __repr__(&self) -> String {
        format!(
            "StressAttribution(var_threshold={}, n_tail_scenarios={}, position_contributions={}, tail_scenarios={})",
            self.inner.var_threshold,
            self.inner.n_tail_scenarios,
            self.inner.position_contributions.len(),
            self.inner.tail_scenarios.len(),
        )
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

/// Run a factor-stress scenario and revalue the portfolio under the stressed market.
///
/// ``as_of`` accepts either a date-like object (``datetime.date``,
/// ``pandas.Timestamp``) or an ISO 8601 string.
#[pyfunction]
#[pyo3(signature = (portfolio, market, factor_model_config_json, as_of, stresses))]
pub(super) fn factor_stress(
    py: Python<'_>,
    portfolio: &Bound<'_, PyAny>,
    market: &Bound<'_, PyAny>,
    factor_model_config_json: &str,
    as_of: &Bound<'_, PyAny>,
    stresses: Vec<(String, f64)>,
) -> PyResult<PyStressResult> {
    let portfolio = extract_portfolio_ref(py, portfolio)?;
    let market = extract_market_ref(py, market)?;
    let as_of = crate::bindings::date_utils::extract_date(as_of)?;
    let config_json = factor_model_config_json.to_owned();
    let config: finstack_quant_models::factor::FactorModelConfig = py
        .detach(move || serde_json::from_str(&config_json))
        .map_err(display_to_py)?;

    let portfolio_ref: &finstack_quant_portfolio::Portfolio = &portfolio;
    let market_ref: &finstack_quant_core::market_data::context::MarketContext = &market;
    let result = py
        .detach(move || {
            let stresses = stresses
                .into_iter()
                .map(|(factor_id, shift)| {
                    (
                        finstack_quant_models::factor::FactorId::new(factor_id),
                        shift,
                    )
                })
                .collect::<Vec<_>>();
            let model = fm::FactorModelBuilder::new().config(config).build()?;
            model.factor_stress(portfolio_ref, market_ref, as_of, &stresses)
        })
        .map_err(portfolio_to_py)?;

    Ok(PyStressResult::from_inner(result))
}

/// Build tail-scenario stress attribution from position x scenario P&Ls.
///
/// Python input is one row per position, where every row contains that
/// position's P&L across all scenarios. The binding transposes that ergonomic
/// shape into Rust's row-major scenario x position buffer.
#[pyfunction]
#[pyo3(signature = (position_ids, position_pnls, confidence = 0.95))]
pub(super) fn build_stress_attribution(
    py: Python<'_>,
    position_ids: Vec<String>,
    position_pnls: &Bound<'_, PyAny>,
    confidence: f64,
) -> PyResult<PyStressAttribution> {
    let n_positions = position_ids.len();
    let position_pnls = extract_position_pnls(py, position_pnls, n_positions)?;
    let n_scenarios = position_pnls.n_scenarios();
    let result = py
        .detach(move || {
            let flat = position_pnls.into_scenario_major(n_positions);
            model_risk::build_stress_attribution(&position_ids, &flat, n_scenarios, confidence)
        })
        .map_err(core_to_py)?;
    Ok(PyStressAttribution::from_inner(result))
}
