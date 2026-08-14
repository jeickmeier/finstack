//! Result structs and enums for the analytics domain.

use crate::bindings::core::dates::utils::date_to_py;
use crate::bindings::pandas_utils::{
    dates_to_pylist, dict_to_dataframe, serde_object_to_single_row_dataframe_with_schema,
    serde_rows_to_dataframe_with_schema, ColumnSchema,
};
use crate::errors::display_to_py;
use finstack_quant_analytics as fa;
use numpy::PyArray1;
use pyo3::prelude::*;
use pyo3::types::PyDict;

#[inline]
fn slice_to_pyarray<'py>(py: Python<'py>, values: &[f64]) -> Bound<'py, PyArray1<f64>> {
    PyArray1::from_slice(py, values)
}

// PeriodStats

/// Aggregated statistics for grouped periodic returns.
#[pyclass(name = "PeriodStats", module = "finstack_quant.analytics", frozen)]
pub struct PyPeriodStats {
    pub(super) inner: fa::PeriodStats,
}

#[pymethods]
impl PyPeriodStats {
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Deserialize from JSON.
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: fa::PeriodStats = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize to compact JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Best period return.
    #[getter]
    fn best(&self) -> f64 {
        self.inner.best
    }
    /// Worst period return.
    #[getter]
    fn worst(&self) -> f64 {
        self.inner.worst
    }
    /// Longest consecutive winning streak.
    #[getter]
    fn consecutive_wins(&self) -> usize {
        self.inner.consecutive_wins
    }
    /// Longest consecutive losing streak.
    #[getter]
    fn consecutive_losses(&self) -> usize {
        self.inner.consecutive_losses
    }
    /// Fraction of positive-return periods.
    #[getter]
    fn win_rate(&self) -> f64 {
        self.inner.win_rate
    }
    /// Average return across all periods.
    #[getter]
    fn avg_return(&self) -> f64 {
        self.inner.avg_return
    }
    /// Average return of positive periods.
    #[getter]
    fn avg_win(&self) -> f64 {
        self.inner.avg_win
    }
    /// Average return of negative periods.
    #[getter]
    fn avg_loss(&self) -> f64 {
        self.inner.avg_loss
    }
    /// Payoff ratio (avg win / |avg loss|).
    #[getter]
    fn payoff_ratio(&self) -> f64 {
        self.inner.payoff_ratio
    }
    /// Profit factor (gross profits / gross losses).
    #[getter]
    fn profit_factor(&self) -> f64 {
        self.inner.profit_factor
    }
    /// Common-sense ratio (CPC).
    #[getter]
    fn cpc_ratio(&self) -> f64 {
        self.inner.cpc_ratio
    }
    /// Kelly criterion optimal fraction.
    #[getter]
    fn kelly_criterion(&self) -> f64 {
        self.inner.kelly_criterion
    }

    fn __repr__(&self) -> String {
        format!(
            "PeriodStats(win_rate={:.4}, avg_return={:.6})",
            self.inner.win_rate, self.inner.avg_return
        )
    }
}

// BetaResult

/// Regression beta with confidence interval.
#[pyclass(name = "BetaResult", module = "finstack_quant.analytics", frozen)]
pub struct PyBetaResult {
    pub(super) inner: fa::BetaResult,
}

#[pymethods]
impl PyBetaResult {
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Deserialize from JSON.
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: fa::BetaResult = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize to compact JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Beta coefficient.
    #[getter]
    fn beta(&self) -> f64 {
        self.inner.beta
    }
    /// Standard error of the beta estimate.
    #[getter]
    fn std_err(&self) -> f64 {
        self.inner.std_err
    }
    /// Lower 95% confidence bound.
    #[getter]
    fn ci_lower(&self) -> f64 {
        self.inner.ci_lower
    }
    /// Upper 95% confidence bound.
    #[getter]
    fn ci_upper(&self) -> f64 {
        self.inner.ci_upper
    }

    /// Export as a single-row pandas ``DataFrame``.
    ///
    /// Columns: ``beta``, ``std_err``, ``ci_lower``, ``ci_upper``.
    ///
    /// One flat record describes one regression, so a one-row frame is the
    /// right shape: ``pd.concat([r.to_dataframe() for r in results])`` stacks
    /// every ticker's beta into one comparison table without reshaping.
    ///
    /// A degenerate regression (fewer than three observations) yields
    /// non-finite estimates, which arrive as ``None`` and make the affected
    /// column ``object`` dtype; coerce with ``pd.to_numeric`` before
    /// aggregating.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let row = serde_json::json!({
            "beta": self.inner.beta,
            "std_err": self.inner.std_err,
            "ci_lower": self.inner.ci_lower,
            "ci_upper": self.inner.ci_upper,
        });
        serde_object_to_single_row_dataframe_with_schema(
            py,
            &row,
            &["beta", "std_err", "ci_lower", "ci_upper"],
        )
    }

    fn __repr__(&self) -> String {
        format!(
            "BetaResult(beta={:.4}, se={:.4}, ci=[{:.4}, {:.4}])",
            self.inner.beta, self.inner.std_err, self.inner.ci_lower, self.inner.ci_upper
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

// GreeksResult

/// Alpha, beta, and R-squared from a single-index regression.
#[pyclass(name = "GreeksResult", module = "finstack_quant.analytics", frozen)]
pub struct PyGreeksResult {
    pub(super) inner: fa::GreeksResult,
}

#[pymethods]
impl PyGreeksResult {
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Deserialize from JSON.
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: fa::GreeksResult = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize to compact JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Annualized Jensen alpha.
    #[getter]
    fn alpha(&self) -> f64 {
        self.inner.alpha
    }
    /// Beta coefficient.
    #[getter]
    fn beta(&self) -> f64 {
        self.inner.beta
    }
    /// R-squared.
    #[getter]
    fn r_squared(&self) -> f64 {
        self.inner.r_squared
    }
    /// Adjusted R-squared.
    #[getter]
    fn adjusted_r_squared(&self) -> f64 {
        self.inner.adjusted_r_squared
    }

    /// Export as a single-row pandas ``DataFrame``.
    ///
    /// Columns: ``alpha``, ``beta``, ``r_squared``, ``adjusted_r_squared``.
    ///
    /// One flat record describes one regression, so a one-row frame is the
    /// right shape: ``pd.concat([r.to_dataframe() for r in results])`` stacks
    /// every ticker's greeks into one comparison table without reshaping.
    ///
    /// Non-finite estimates from a degenerate fit arrive as ``None`` and make
    /// the affected column ``object`` dtype; coerce with ``pd.to_numeric``
    /// before aggregating.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let row = serde_json::json!({
            "alpha": self.inner.alpha,
            "beta": self.inner.beta,
            "r_squared": self.inner.r_squared,
            "adjusted_r_squared": self.inner.adjusted_r_squared,
        });
        serde_object_to_single_row_dataframe_with_schema(
            py,
            &row,
            &["alpha", "beta", "r_squared", "adjusted_r_squared"],
        )
    }

    fn __repr__(&self) -> String {
        format!(
            "GreeksResult(alpha={:.6}, beta={:.4}, r2={:.4}, adj_r2={:.4})",
            self.inner.alpha, self.inner.beta, self.inner.r_squared, self.inner.adjusted_r_squared
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

// RollingGreeks

/// Rolling alpha and beta time series.
#[pyclass(name = "RollingGreeks", module = "finstack_quant.analytics", frozen)]
pub struct PyRollingGreeks {
    pub(super) inner: fa::RollingGreeks,
}

#[pymethods]
impl PyRollingGreeks {
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Deserialize from JSON.
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: fa::RollingGreeks = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize to compact JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Date labels for each rolling window.
    #[getter]
    fn dates<'py>(&self, py: Python<'py>) -> PyResult<Vec<Bound<'py, PyAny>>> {
        self.inner
            .dates
            .iter()
            .map(|&d| date_to_py(py, d))
            .collect()
    }
    /// Rolling alpha values.
    #[getter]
    fn alphas<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        slice_to_pyarray(py, &self.inner.alphas)
    }
    /// Rolling beta values.
    #[getter]
    fn betas<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        slice_to_pyarray(py, &self.inner.betas)
    }

    /// Convert to a pandas ``DataFrame`` with date index and alpha/beta columns.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let data = PyDict::new(py);
        data.set_item("alpha", slice_to_pyarray(py, &self.inner.alphas))?;
        data.set_item("beta", slice_to_pyarray(py, &self.inner.betas))?;
        let dates = dates_to_pylist(py, &self.inner.dates)?;
        let idx = dates.into_pyobject(py)?.into_any();
        dict_to_dataframe(py, &data, Some(idx))
    }

    fn __repr__(&self) -> String {
        format!("RollingGreeks(len={})", self.inner.dates.len())
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

// MultiFactorResult

/// Column schema of `PyMultiFactorResult::to_dataframe`, kept so a
/// zero-factor regression still exports a frame with the documented columns.
const MULTI_FACTOR_COLUMNS: &[ColumnSchema<'static>] = &[
    ("factor", "str"),
    ("beta", "float64"),
    ("alpha", "float64"),
    ("r_squared", "float64"),
    ("adjusted_r_squared", "float64"),
    ("residual_vol", "float64"),
];

/// Multi-factor regression result.
#[pyclass(
    name = "MultiFactorResult",
    module = "finstack_quant.analytics",
    frozen
)]
pub struct PyMultiFactorResult {
    pub(super) inner: fa::MultiFactorResult,
}

#[pymethods]
impl PyMultiFactorResult {
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Deserialize from JSON.
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: fa::MultiFactorResult = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize to compact JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Raw regression intercept, annualized with the supplied factor frequency.
    #[getter]
    fn alpha(&self) -> f64 {
        self.inner.alpha
    }
    /// Factor betas.
    #[getter]
    fn betas<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        slice_to_pyarray(py, &self.inner.betas)
    }
    /// R-squared.
    #[getter]
    fn r_squared(&self) -> f64 {
        self.inner.r_squared
    }
    /// Adjusted R-squared.
    #[getter]
    fn adjusted_r_squared(&self) -> f64 {
        self.inner.adjusted_r_squared
    }
    /// Residual volatility.
    #[getter]
    fn residual_vol(&self) -> f64 {
        self.inner.residual_vol
    }

    /// Export the factor loadings as a pandas ``DataFrame``, one row per factor.
    ///
    /// Columns: ``factor``, ``beta``, ``alpha``, ``r_squared``,
    /// ``adjusted_r_squared``, ``residual_vol``.
    ///
    /// The loadings are the per-row payload; the four regression-level
    /// statistics repeat on every row so a single row carries its own fit
    /// context after ``pd.concat`` across tickers or ``groupby("factor")``.
    ///
    /// Rows follow the order of the ``factor_returns`` passed to
    /// :meth:`Performance.multi_factor_greeks`, which is also the order of
    /// :attr:`betas`. There is always at least one row: the regression rejects
    /// an empty factor set.
    ///
    /// Parameters
    /// ----------
    /// factor_names : list[str], optional
    ///     Labels for the ``factor`` column, positionally aligned with
    ///     :attr:`betas`. Defaults to ``factor_0``, ``factor_1``, ... because
    ///     the regression itself carries no names.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``factor_names`` is supplied and its length differs from the
    ///     number of fitted betas.
    #[pyo3(signature = (factor_names=None))]
    #[pyo3(text_signature = "(self, factor_names=None)")]
    fn to_dataframe<'py>(
        &self,
        py: Python<'py>,
        factor_names: Option<Vec<String>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let betas = &self.inner.betas;
        let names = match factor_names {
            Some(names) if names.len() != betas.len() => {
                return Err(crate::errors::value_error(format!(
                    "factor_names has {} entries but the regression fitted {} betas",
                    names.len(),
                    betas.len()
                )));
            }
            Some(names) => names,
            None => (0..betas.len()).map(|i| format!("factor_{i}")).collect(),
        };
        let rows: Vec<serde_json::Value> = names
            .iter()
            .zip(betas.iter())
            .map(|(name, &beta)| {
                serde_json::json!({
                    "factor": name,
                    "beta": beta,
                    "alpha": self.inner.alpha,
                    "r_squared": self.inner.r_squared,
                    "adjusted_r_squared": self.inner.adjusted_r_squared,
                    "residual_vol": self.inner.residual_vol,
                })
            })
            .collect();
        serde_rows_to_dataframe_with_schema(py, &rows, MULTI_FACTOR_COLUMNS)
    }

    fn __repr__(&self) -> String {
        format!(
            "MultiFactorResult(alpha={:.6}, r2={:.4}, adj_r2={:.4})",
            self.inner.alpha, self.inner.r_squared, self.inner.adjusted_r_squared
        )
    }
}

// DrawdownEpisode

/// A single drawdown episode with timing and depth information.
#[pyclass(name = "DrawdownEpisode", module = "finstack_quant.analytics", frozen)]
pub struct PyDrawdownEpisode {
    pub(super) inner: fa::DrawdownEpisode,
}

#[pymethods]
impl PyDrawdownEpisode {
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Deserialize from JSON.
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: fa::DrawdownEpisode = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize to compact JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Start date of the drawdown.
    #[getter]
    fn start<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        date_to_py(py, self.inner.start)
    }
    /// Date of the maximum drawdown within this episode.
    #[getter]
    fn valley<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        date_to_py(py, self.inner.valley)
    }
    /// Recovery date (``None`` if still in drawdown).
    #[getter]
    fn end<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyAny>>> {
        match self.inner.end {
            Some(d) => date_to_py(py, d).map(Some),
            None => Ok(None),
        }
    }
    /// Duration in calendar days.
    #[getter]
    fn duration_days(&self) -> i64 {
        self.inner.duration_days
    }
    /// Maximum drawdown depth (negative).
    #[getter]
    fn max_drawdown(&self) -> f64 {
        self.inner.max_drawdown
    }
    /// Near-recovery threshold.
    #[getter]
    fn near_recovery_threshold(&self) -> f64 {
        self.inner.near_recovery_threshold
    }
    /// ``True`` when the episode began before the first observation
    /// (left-censored: no prior peak observed).
    #[getter]
    fn truncated_at_start(&self) -> bool {
        self.inner.truncated_at_start
    }

    fn __repr__(&self) -> String {
        format!(
            "DrawdownEpisode(dd={:.4}, days={})",
            self.inner.max_drawdown, self.inner.duration_days
        )
    }
}

// LookbackReturns

/// Period-to-date returns for each ticker.
#[pyclass(name = "LookbackReturns", module = "finstack_quant.analytics", frozen)]
pub struct PyLookbackReturns {
    pub(super) inner: fa::LookbackReturns,
}

#[pymethods]
impl PyLookbackReturns {
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Deserialize from JSON.
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: fa::LookbackReturns = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize to compact JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Month-to-date returns per ticker.
    #[getter]
    fn mtd<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        slice_to_pyarray(py, &self.inner.mtd)
    }
    /// Quarter-to-date returns per ticker.
    #[getter]
    fn qtd<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        slice_to_pyarray(py, &self.inner.qtd)
    }
    /// Year-to-date returns per ticker.
    #[getter]
    fn ytd<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        slice_to_pyarray(py, &self.inner.ytd)
    }
    /// Fiscal-year-to-date returns (``None`` if no fiscal config).
    #[getter]
    fn fytd<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyArray1<f64>>> {
        self.inner.fytd.as_deref().map(|v| slice_to_pyarray(py, v))
    }

    /// Convert to a pandas ``DataFrame`` with ticker names as index.
    ///
    /// Columns: mtd, qtd, ytd (and fytd when available).
    fn to_dataframe<'py>(
        &self,
        py: Python<'py>,
        ticker_names: Vec<String>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let data = PyDict::new(py);
        data.set_item("mtd", slice_to_pyarray(py, &self.inner.mtd))?;
        data.set_item("qtd", slice_to_pyarray(py, &self.inner.qtd))?;
        data.set_item("ytd", slice_to_pyarray(py, &self.inner.ytd))?;
        if let Some(ref fytd) = self.inner.fytd {
            data.set_item("fytd", slice_to_pyarray(py, fytd))?;
        }
        let idx = ticker_names.into_pyobject(py)?.into_any();
        dict_to_dataframe(py, &data, Some(idx))
    }

    fn __repr__(&self) -> String {
        format!(
            "LookbackReturns(mtd_len={}, has_fytd={})",
            self.inner.mtd.len(),
            self.inner.fytd.is_some()
        )
    }
}

// DatedSeries

/// Date-indexed numeric series returned by the rolling-window analytics.
///
/// Rolling-window analytics share this carrier and attach a metric-specific
/// DataFrame column name.
#[pyclass(name = "DatedSeries", module = "finstack_quant.analytics", frozen)]
pub struct PyDatedSeries {
    pub(super) inner: fa::DatedSeries,
    pub(super) value_column: String,
}

impl PyDatedSeries {
    /// Construct from an analytics `DatedSeries` plus the desired column label.
    pub(crate) fn new(inner: fa::DatedSeries, value_column: impl Into<String>) -> Self {
        Self {
            inner,
            value_column: value_column.into(),
        }
    }
}

/// Wire shape for [`PyDatedSeries`].
///
/// The analytics `DatedSeries` carries only `values`/`dates`; the metric label
/// lives on the Python wrapper. Serializing it alongside keeps `from_json` (and
/// therefore `__reduce__`) exact — otherwise a round-trip would silently
/// relabel a rolling-Sharpe series as something else.
#[derive(serde::Serialize, serde::Deserialize)]
struct DatedSeriesWire {
    #[serde(flatten)]
    series: fa::DatedSeries,
    value_column: String,
}

#[pymethods]
impl PyDatedSeries {
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Deserialize from JSON.
    ///
    /// Expects the `values` / `dates` / `value_column` shape emitted by
    /// [`to_json`](Self::to_json).
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let wire: DatedSeriesWire = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self {
            inner: wire.series,
            value_column: wire.value_column,
        })
    }

    /// Serialize to compact JSON.
    ///
    /// Emits the analytics `DatedSeries` fields (`values`, `dates`) plus the
    /// `value_column` label this wrapper adds.
    fn to_json(&self) -> PyResult<String> {
        let wire = DatedSeriesWire {
            series: self.inner.clone(),
            value_column: self.value_column.clone(),
        };
        serde_json::to_string(&wire).map_err(display_to_py)
    }

    /// Numeric values, one per window.
    #[getter]
    fn values<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        slice_to_pyarray(py, &self.inner.values)
    }
    /// Window-end dates aligned 1:1 with [`values`](Self::values).
    #[getter]
    fn dates<'py>(&self, py: Python<'py>) -> PyResult<Vec<Bound<'py, PyAny>>> {
        self.inner
            .dates
            .iter()
            .map(|&d| date_to_py(py, d))
            .collect()
    }
    /// Column name used by [`to_dataframe`](Self::to_dataframe).
    #[getter]
    fn value_column(&self) -> &str {
        &self.value_column
    }

    /// Convert to a pandas ``DataFrame`` with the date column as index and a
    /// single value column named after [`value_column`](Self::value_column).
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let data = PyDict::new(py);
        data.set_item(
            self.value_column.as_str(),
            slice_to_pyarray(py, &self.inner.values),
        )?;
        let dates = dates_to_pylist(py, &self.inner.dates)?;
        let idx = dates.into_pyobject(py)?.into_any();
        dict_to_dataframe(py, &data, Some(idx))
    }

    fn __repr__(&self) -> String {
        format!(
            "DatedSeries(name={:?}, len={})",
            self.value_column,
            self.inner.values.len()
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

pub fn register(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPeriodStats>()?;
    m.add_class::<PyBetaResult>()?;
    m.add_class::<PyGreeksResult>()?;
    m.add_class::<PyRollingGreeks>()?;
    m.add_class::<PyMultiFactorResult>()?;
    m.add_class::<PyDrawdownEpisode>()?;
    m.add_class::<PyLookbackReturns>()?;
    m.add_class::<PyDatedSeries>()?;
    let _ = py;
    Ok(())
}
