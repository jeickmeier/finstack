//! Python binding for portfolio historical replay.

use crate::bindings::extract::extract_portfolio_ref;
use crate::bindings::pandas_utils::{
    serde_rows_to_dataframe_with_schema, serde_to_py, ColumnSchema,
};
use crate::errors::{display_to_py, portfolio_to_py};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyString};
use serde::Serialize;
use std::cell::RefCell;

thread_local! {
    /// Per-thread replay JSON scratch space.
    ///
    /// Large `serde_json::to_string` buffers are not reliably returned to the
    /// process RSS allocator between calls on macOS. Retaining only the byte
    /// capacity bounds repeated-call RSS without caching any portfolio,
    /// market, valuation, or replay-result state.
    static REPLAY_JSON_SCRATCH: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Column schema for [`PyReplayResult::to_dataframe`].
const REPLAY_STEP_COLUMNS: &[ColumnSchema<'static>] = &[
    ("date", "str"),
    ("value", "float64"),
    ("daily_pnl", "float64"),
    ("cumulative_pnl", "float64"),
];

/// One row of the replay step ladder.
#[derive(Serialize)]
struct ReplayStepRow {
    date: String,
    value: f64,
    daily_pnl: Option<f64>,
    cumulative_pnl: Option<f64>,
}

/// Full output of a historical replay run.
///
/// Returned by :func:`replay_portfolio`.
#[pyclass(
    name = "ReplayResult",
    module = "finstack_quant.portfolio",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyReplayResult {
    pub(crate) inner: finstack_quant_portfolio::replay::ReplayResult,
}

impl PyReplayResult {
    fn rows(&self) -> Vec<ReplayStepRow> {
        self.inner
            .steps
            .iter()
            .map(|step| ReplayStepRow {
                date: step.date.to_string(),
                value: step.valuation.total_base_currency.amount(),
                daily_pnl: step.daily_pnl.map(|m| m.amount()),
                cumulative_pnl: step.cumulative_pnl.map(|m| m.amount()),
            })
            .collect()
    }
}

#[pymethods]
impl PyReplayResult {
    /// Per-step output as a list of dicts (full valuations included).
    #[getter]
    fn steps<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        serde_to_py(py, &self.inner.steps)
    }

    /// Aggregate statistics across the full replay, as a dict.
    #[getter]
    fn summary<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        serde_to_py(py, &self.inner.summary)
    }

    /// Snapshots skipped in best-effort mode, as ``(date, reason)`` pairs.
    #[getter]
    fn skipped_dates<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        serde_to_py(py, &self.inner.skipped_dates)
    }

    /// Per-step value and P&L ladder as a :class:`pandas.DataFrame`.
    ///
    /// One row per replay step; ``daily_pnl`` and ``cumulative_pnl`` are
    /// null at step 0. Full per-step valuations remain available from the
    /// ``steps`` getter.
    ///
    /// Columns: ``date``, ``value``, ``daily_pnl``, ``cumulative_pnl``.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        serde_rows_to_dataframe_with_schema(py, &self.rows(), REPLAY_STEP_COLUMNS)
    }

    /// Serialize to a compact JSON string.
    fn to_json(&self, py: Python<'_>) -> PyResult<String> {
        let inner = &self.inner;
        py.detach(|| serde_json::to_string(inner))
            .map_err(display_to_py)
    }

    /// Deserialize from a JSON string.
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(py: Python<'_>, json: &str) -> PyResult<Self> {
        let json = json.to_owned();
        let inner: finstack_quant_portfolio::replay::ReplayResult = py
            .detach(move || serde_json::from_str(&json))
            .map_err(display_to_py)?;
        Ok(Self { inner })
    }

    fn __repr__(&self) -> String {
        format!(
            "ReplayResult(steps={}, start={}, end={})",
            self.inner.steps.len(),
            self.inner.summary.start_date,
            self.inner.summary.end_date,
        )
    }

    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json(py)?)
    }
}

/// Run the canonical Rust replay engine for both entry points.
fn run_replay_portfolio(
    py: Python<'_>,
    portfolio: &Bound<'_, PyAny>,
    snapshots_json: &str,
    config_json: &str,
) -> PyResult<finstack_quant_portfolio::replay::ReplayResult> {
    let portfolio = extract_portfolio_ref(py, portfolio)?;
    let config_json = config_json.to_owned();
    let config: finstack_quant_portfolio::replay::ReplayConfig = py
        .detach(move || serde_json::from_str(&config_json))
        .map_err(display_to_py)?;
    let snapshots_json = snapshots_json.to_owned();
    let timeline = py
        .detach(move || {
            finstack_quant_portfolio::replay::ReplayTimeline::from_json_snapshots(&snapshots_json)
        })
        .map_err(display_to_py)?;
    let finstack_config = finstack_quant_core::config::FinstackConfig::default();
    let portfolio_ref: &finstack_quant_portfolio::Portfolio = &portfolio;
    py.detach(|| {
        finstack_quant_portfolio::replay::replay_portfolio(
            portfolio_ref,
            &timeline,
            &config,
            &finstack_config,
        )
    })
    .map_err(portfolio_to_py)
}

/// Replay a portfolio through dated market snapshots.
///
/// Parameters
/// ----------
/// portfolio : Portfolio | str
///     A :class:`Portfolio` object (fast path) or a JSON-serialized
///     ``PortfolioSpec`` string.
/// snapshots_json : str
///     JSON array of ``{"date": "YYYY-MM-DD", "market": {...}}`` objects.
///     Markets use the standard ``MarketContextState`` JSON format.
/// config_json : str
///     JSON-serialized ``ReplayConfig``.
///
/// Returns
/// -------
/// ReplayResult
///     Typed result with ``steps``, ``summary`` and ``skipped_dates``
///     getters plus ``to_dataframe()``. Use :func:`replay_portfolio_json`
///     for the raw wire string.
#[pyfunction]
#[pyo3(text_signature = "(portfolio, snapshots_json, config_json)")]
fn replay_portfolio(
    py: Python<'_>,
    portfolio: &Bound<'_, PyAny>,
    snapshots_json: &str,
    config_json: &str,
) -> PyResult<PyReplayResult> {
    Ok(PyReplayResult {
        inner: run_replay_portfolio(py, portfolio, snapshots_json, config_json)?,
    })
}

/// Replay a portfolio through dated market snapshots and return wire JSON.
///
/// Wire twin of :func:`replay_portfolio`; same inputs, JSON-string output.
///
/// Returns
/// -------
/// str
///     JSON-serialized ``ReplayResult``.
#[pyfunction]
#[pyo3(text_signature = "(portfolio, snapshots_json, config_json)")]
fn replay_portfolio_json<'py>(
    py: Python<'py>,
    portfolio: &Bound<'_, PyAny>,
    snapshots_json: &str,
    config_json: &str,
) -> PyResult<Bound<'py, PyString>> {
    let result = run_replay_portfolio(py, portfolio, snapshots_json, config_json)?;
    py.detach(move || {
        REPLAY_JSON_SCRATCH.with(|scratch| {
            let mut scratch = scratch.borrow_mut();
            scratch.clear();
            serde_json::to_writer(&mut *scratch, &result)
        })
    })
    .map_err(display_to_py)?;

    REPLAY_JSON_SCRATCH.with(|scratch| {
        let mut scratch = scratch.borrow_mut();
        let output = PyString::from_bytes(py, scratch.as_slice());
        scratch.clear();
        output
    })
}

/// Register replay functions on the portfolio submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyReplayResult>()?;
    m.add_function(pyo3::wrap_pyfunction!(replay_portfolio, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(replay_portfolio_json, m)?)?;
    Ok(())
}
