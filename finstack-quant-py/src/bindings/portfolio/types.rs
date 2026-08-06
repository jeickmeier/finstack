//! Typed `#[pyclass]` wrappers for portfolio runtime objects.
//!
//! These wrappers let callers hold a built `Portfolio`, `PortfolioValuation`,
//! or `PortfolioResult` in Python and pass it back into pipeline functions
//! without paying the JSON round-trip cost on every call. Pipeline functions
//! accept either the typed object or a JSON string via the `*Access` helpers
//! in [`crate::bindings::extract`].

use std::sync::Arc;

use pyo3::exceptions::PyKeyError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};

use crate::bindings::pandas_utils::{dict_to_dataframe, table_to_dataframe};
use crate::errors::{core_to_py, display_to_py, portfolio_to_py};

// PyPortfolio

/// Python wrapper around a built [`finstack_quant_portfolio::Portfolio`].
///
/// Cheap to clone (wraps `Arc<Portfolio>`); construction from a spec pays
/// the full `Portfolio::from_spec` cost once and the result can be reused
/// across multiple pipeline calls (value, cashflows, metrics, scenarios).
#[pyclass(
    name = "Portfolio",
    module = "finstack_quant.portfolio",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyPortfolio {
    pub(crate) inner: Arc<finstack_quant_portfolio::Portfolio>,
}

impl PyPortfolio {
    pub(crate) fn from_inner(inner: finstack_quant_portfolio::Portfolio) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pymethods]
impl PyPortfolio {
    /// Build a portfolio from a JSON ``PortfolioSpec``.
    ///
    /// This performs position materialization, rebuilds the position and
    /// dependency indices, and validates the result. Hold the returned
    /// object and pass it directly to pipeline functions to avoid repeating
    /// this work.
    #[staticmethod]
    #[pyo3(text_signature = "(spec_json)")]
    fn from_spec(py: Python<'_>, spec_json: &str) -> PyResult<Self> {
        let spec_json = spec_json.to_owned();
        let spec: finstack_quant_portfolio::portfolio::PortfolioSpec = py
            .detach(move || serde_json::from_str(&spec_json))
            .map_err(display_to_py)?;
        let inner = py
            .detach(move || finstack_quant_portfolio::Portfolio::from_spec(spec))
            .map_err(portfolio_to_py)?;
        Ok(Self::from_inner(inner))
    }

    /// Portfolio identifier.
    #[getter]
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    /// Valuation date (ISO 8601).
    #[getter]
    fn as_of(&self) -> String {
        self.inner.as_of.to_string()
    }

    /// Base currency code.
    #[getter]
    fn base_currency(&self) -> String {
        self.inner.base_currency.to_string()
    }

    /// Number of positions in the portfolio.
    fn __len__(&self) -> usize {
        self.inner.positions().len()
    }

    /// Round-trip the portfolio back to its JSON spec form.
    #[pyo3(text_signature = "(self)")]
    fn to_spec_json(&self, py: Python<'_>) -> PyResult<String> {
        let portfolio = self.inner.as_ref();
        py.detach(|| serde_json::to_string(&portfolio.to_spec()))
            .map_err(display_to_py)
    }

    fn __repr__(&self) -> String {
        format!(
            "Portfolio(id=\"{}\", as_of={}, base_currency={}, positions={})",
            self.inner.id,
            self.inner.as_of,
            self.inner.base_currency,
            self.inner.positions().len()
        )
    }
}

// PyPortfolioValuation

/// Python wrapper around a [`finstack_quant_portfolio::valuation::PortfolioValuation`].
///
/// Avoids re-parsing the (potentially large) valuation JSON every time a
/// downstream function (``aggregate_metrics``, ``portfolio_result_*``) needs
/// to read from it.
#[pyclass(
    name = "PortfolioValuation",
    module = "finstack_quant.portfolio",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyPortfolioValuation {
    pub(crate) inner: finstack_quant_portfolio::valuation::PortfolioValuation,
}

impl PyPortfolioValuation {
    pub(crate) fn from_inner(
        inner: finstack_quant_portfolio::valuation::PortfolioValuation,
    ) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPortfolioValuation {
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let payload = self.to_json(py)?;
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        Ok((from_json, (payload,)))
    }

    /// Parse a valuation from a JSON string.
    #[staticmethod]
    #[pyo3(text_signature = "(valuation_json)")]
    fn from_json(py: Python<'_>, valuation_json: &str) -> PyResult<Self> {
        let valuation_json = valuation_json.to_owned();
        let inner: finstack_quant_portfolio::valuation::PortfolioValuation = py
            .detach(move || serde_json::from_str(&valuation_json))
            .map_err(display_to_py)?;
        Ok(Self::from_inner(inner))
    }

    /// Serialize back to JSON.
    #[pyo3(text_signature = "(self)")]
    fn to_json(&self, py: Python<'_>) -> PyResult<String> {
        let valuation = &self.inner;
        py.detach(|| serde_json::to_string(valuation))
            .map_err(display_to_py)
    }

    /// Total portfolio value in the base currency (amount).
    #[getter]
    fn total_value(&self) -> f64 {
        self.inner.total_base_currency.amount()
    }

    /// Base currency of the total.
    #[getter]
    fn base_currency(&self) -> String {
        self.inner.total_base_currency.currency().to_string()
    }

    /// Valuation date (ISO 8601).
    #[getter]
    fn as_of(&self) -> String {
        self.inner.as_of.to_string()
    }

    /// Export per-position values via Arrow (zero-copy for consumers).
    ///
    /// Columns: ``position_id``, ``entity_id``, ``value_native``,
    /// ``value_base``, ``currency_native``, ``currency_base`` (see
    /// ``finstack_quant_portfolio::positions_to_table``). Returns an
    /// :class:`finstack_quant.core.table.ArrowTable`.
    #[pyo3(text_signature = "($self)")]
    fn to_arrow_positions(&self) -> PyResult<crate::bindings::core::table::PyArrowTable> {
        let table = finstack_quant_portfolio::positions_to_table(&self.inner)
            .map_err(crate::errors::core_to_py)?;
        crate::bindings::core::table::PyArrowTable::from_envelope(&table)
    }

    /// Export per-position values as a pandas ``DataFrame``.
    ///
    /// One row per entry of ``position_values``. Built from the same
    /// ``positions_to_table`` envelope that backs :meth:`to_arrow_positions`,
    /// so the two exits cannot drift apart.
    ///
    /// Columns: ``position_id``, ``entity_id``, ``value_native``,
    /// ``value_base``, ``currency_native``, ``currency_base``. Values are
    /// floats and the currency codes are strings; use
    /// :meth:`to_arrow_positions` when a zero-copy handoff matters more than
    /// pandas ergonomics.
    #[pyo3(text_signature = "($self)")]
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let table =
            finstack_quant_portfolio::positions_to_table(&self.inner).map_err(core_to_py)?;
        table_to_dataframe(py, &table)
    }

    /// Number of position valuations in the result.
    fn __len__(&self) -> usize {
        self.inner.position_values.len()
    }

    fn __repr__(&self) -> String {
        format!(
            "PortfolioValuation(as_of={}, total={} {}, positions={})",
            self.inner.as_of,
            self.inner.total_base_currency.amount(),
            self.inner.total_base_currency.currency(),
            self.inner.position_values.len()
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

// PyPortfolioResult

/// Python wrapper around a [`finstack_quant_portfolio::results::PortfolioResult`].
///
/// Exposes cheap scalar accessors (``total_value``, ``get_metric``) that
/// avoid the full JSON re-parse previously required by the JSON-only API.
#[pyclass(
    name = "PortfolioResult",
    module = "finstack_quant.portfolio",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyPortfolioResult {
    pub(crate) inner: finstack_quant_portfolio::results::PortfolioResult,
}

impl PyPortfolioResult {
    pub(crate) fn from_inner(inner: finstack_quant_portfolio::results::PortfolioResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPortfolioResult {
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let payload = self.to_json(py)?;
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        Ok((from_json, (payload,)))
    }

    /// Parse a result from a JSON string.
    #[staticmethod]
    #[pyo3(text_signature = "(result_json)")]
    fn from_json(py: Python<'_>, result_json: &str) -> PyResult<Self> {
        let result_json = result_json.to_owned();
        let inner: finstack_quant_portfolio::results::PortfolioResult = py
            .detach(move || serde_json::from_str(&result_json))
            .map_err(display_to_py)?;
        Ok(Self::from_inner(inner))
    }

    /// Serialize back to JSON.
    #[pyo3(text_signature = "(self)")]
    fn to_json(&self, py: Python<'_>) -> PyResult<String> {
        let result = &self.inner;
        py.detach(|| serde_json::to_string(result))
            .map_err(display_to_py)
    }

    /// Total portfolio value in base currency.
    #[getter]
    fn total_value(&self) -> f64 {
        self.inner.total_value().amount()
    }

    /// Retrieve an aggregated metric by id. Returns ``None`` if absent.
    #[pyo3(text_signature = "(self, metric_id)")]
    fn get_metric(&self, metric_id: &str) -> Option<f64> {
        self.inner.get_metric(metric_id)
    }

    /// Retrieve a metric and raise ``KeyError`` if it is missing.
    #[pyo3(text_signature = "(self, metric_id)")]
    fn require_metric(&self, metric_id: &str) -> PyResult<f64> {
        self.inner
            .get_metric(metric_id)
            .ok_or_else(|| PyKeyError::new_err(format!("metric '{metric_id}' not present")))
    }

    fn __repr__(&self) -> String {
        let total = self.inner.total_value();
        format!(
            "PortfolioResult(total={} {})",
            total.amount(),
            total.currency(),
        )
    }
}

// PyPortfolioMetrics

type PyMetricSeriesEntry = (Vec<String>, f64, Py<PyDict>);

/// Python wrapper around Rust-aggregated portfolio metrics.
#[pyclass(
    name = "PortfolioMetrics",
    module = "finstack_quant.portfolio",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyPortfolioMetrics {
    inner: finstack_quant_portfolio::metrics::PortfolioMetrics,
}

#[pymethods]
impl PyPortfolioMetrics {
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let payload = self.to_json(py)?;
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        Ok((from_json, (payload,)))
    }

    /// Parse aggregate portfolio metrics from canonical JSON.
    #[staticmethod]
    fn from_json(py: Python<'_>, metrics_json: &str) -> PyResult<Self> {
        let metrics_json = metrics_json.to_owned();
        let inner = py
            .detach(move || serde_json::from_str(&metrics_json))
            .map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize aggregate portfolio metrics to canonical JSON.
    fn to_json(&self, py: Python<'_>) -> PyResult<String> {
        let metrics = &self.inner;
        py.detach(|| serde_json::to_string(metrics))
            .map_err(display_to_py)
    }

    /// Return decoded components, total, and ordered entity breakdown by base metric.
    fn metric_series(&self, py: Python<'_>, base: &str) -> PyResult<Vec<PyMetricSeriesEntry>> {
        let base = finstack_quant_valuations::metrics::MetricId::custom(base);
        self.inner
            .metric_series(&base)
            .into_iter()
            .map(|(components, aggregate)| {
                let by_entity = PyDict::new(py);
                for (entity, value) in &aggregate.by_entity {
                    by_entity.set_item(entity.to_string(), value)?;
                }
                Ok((components, aggregate.total, by_entity.unbind()))
            })
            .collect()
    }

    /// Export the portfolio-wide aggregated metrics as a pandas ``DataFrame``.
    ///
    /// One row per entry of the ``aggregated`` map, in canonical Rust
    /// ``IndexMap`` insertion order. The per-entity breakdown is not flattened
    /// here — reach it through :meth:`metric_series`.
    ///
    /// Columns: ``metric_id``, ``total`` (sum across positions; only summable
    /// metrics are aggregated).
    #[pyo3(text_signature = "(self)")]
    fn to_aggregated_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let metric_ids: Vec<&str> = self
            .inner
            .aggregated
            .values()
            .map(|metric| metric.metric_id.as_str())
            .collect();
        let totals: Vec<f64> = self
            .inner
            .aggregated
            .values()
            .map(|metric| metric.total)
            .collect();
        let data = PyDict::new(py);
        data.set_item("metric_id", metric_ids)?;
        data.set_item("total", totals)?;
        dict_to_dataframe(py, &data, None)
    }

    /// Export the raw per-position metric values as a long-format pandas
    /// ``DataFrame``.
    ///
    /// One row per ``(position, metric)`` pair — the row count is the total
    /// number of metric values across positions, not the number of positions.
    /// Pivot with ``df.pivot(index="position_id", columns="metric_id",
    /// values="value")`` for a wide view.
    ///
    /// Columns: ``position_id``, ``currency`` (the position's native currency;
    /// non-summable metrics are quoted in it), ``metric_id``, ``value``.
    #[pyo3(text_signature = "(self)")]
    fn to_position_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let mut position_ids: Vec<&str> = Vec::new();
        let mut currencies: Vec<String> = Vec::new();
        let mut metric_ids: Vec<&str> = Vec::new();
        let mut values: Vec<f64> = Vec::new();
        for (position_id, position_metrics) in &self.inner.by_position {
            for (metric_id, value) in &position_metrics.metrics {
                position_ids.push(position_id.as_str());
                currencies.push(position_metrics.currency.to_string());
                metric_ids.push(metric_id.as_str());
                values.push(*value);
            }
        }
        let data = PyDict::new(py);
        data.set_item("position_id", position_ids)?;
        data.set_item("currency", currencies)?;
        data.set_item("metric_id", metric_ids)?;
        data.set_item("value", values)?;
        dict_to_dataframe(py, &data, None)
    }

    fn __repr__(&self) -> String {
        format!(
            "PortfolioMetrics(aggregated={}, positions={})",
            self.inner.aggregated.len(),
            self.inner.by_position.len(),
        )
    }
}

// PyPortfolioCashflows

/// Python wrapper around a
/// [`finstack_quant_portfolio::cashflows::PortfolioCashflows`] ladder.
///
/// Returning a typed wrapper lets callers drill into `events`, `by_date`, and
/// `issues` without re-parsing the aggregated JSON payload on every access.
/// Typed accessors return JSON for now (structured access can be added
/// incrementally); `to_json()` round-trips the full structure.
#[pyclass(
    name = "PortfolioCashflows",
    module = "finstack_quant.portfolio",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyPortfolioCashflows {
    pub(crate) inner: finstack_quant_portfolio::cashflows::PortfolioCashflows,
}

impl PyPortfolioCashflows {
    pub(crate) fn from_inner(
        inner: finstack_quant_portfolio::cashflows::PortfolioCashflows,
    ) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPortfolioCashflows {
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let payload = self.to_json(py)?;
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        Ok((from_json, (payload,)))
    }

    /// Parse a cashflow ladder from a JSON string.
    #[staticmethod]
    #[pyo3(text_signature = "(cashflows_json)")]
    fn from_json(py: Python<'_>, cashflows_json: &str) -> PyResult<Self> {
        let cashflows_json = cashflows_json.to_owned();
        let inner: finstack_quant_portfolio::cashflows::PortfolioCashflows = py
            .detach(move || serde_json::from_str(&cashflows_json))
            .map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize the full ladder back to JSON.
    #[pyo3(text_signature = "(self)")]
    fn to_json(&self, py: Python<'_>) -> PyResult<String> {
        let cashflows = &self.inner;
        py.detach(|| serde_json::to_string(cashflows))
            .map_err(display_to_py)
    }

    /// Number of dated cashflow events.
    fn __len__(&self) -> usize {
        self.inner.events.len()
    }

    /// Number of positions represented in the ladder (contributing events or
    /// recorded as issues).
    fn num_positions(&self) -> usize {
        self.inner.by_position.len()
    }

    /// Number of extraction issues recorded during aggregation.
    fn num_issues(&self) -> usize {
        self.inner.issues.len()
    }

    /// JSON for the flat ``events`` vector only.
    #[pyo3(text_signature = "(self)")]
    fn events_json(&self, py: Python<'_>) -> PyResult<String> {
        let events = &self.inner.events;
        py.detach(|| serde_json::to_string(events))
            .map_err(display_to_py)
    }

    /// JSON for the ``by_date`` currency/kind totals only.
    #[pyo3(text_signature = "(self)")]
    fn by_date_json(&self, py: Python<'_>) -> PyResult<String> {
        let by_date = &self.inner.by_date;
        py.detach(|| serde_json::to_string(by_date))
            .map_err(display_to_py)
    }

    /// Export the flat ``events`` ladder as a pandas ``DataFrame``.
    ///
    /// One row per dated cashflow event, in the ladder's canonical
    /// payment-date order. ``amount`` is flattened to a float column plus a
    /// ``currency`` column rather than a nested dict, so a multi-currency
    /// ladder stays currency-safe: group by ``currency`` before summing.
    ///
    /// Columns: ``position_id``, ``instrument_id``, ``instrument_type``,
    /// ``date`` (ISO 8601 string), ``amount`` (position-scaled), ``currency``,
    /// ``kind`` (``"fixed"``, ``"float_reset"``, ``"notional"``, ...),
    /// ``reset_date`` (ISO 8601 string, ``None`` outside floating coupons),
    /// ``accrual_factor``, ``rate`` (``None`` when the event carries no rate).
    #[pyo3(text_signature = "(self)")]
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let events = &self.inner.events;
        let position_ids: Vec<&str> = events.iter().map(|e| e.position_id.as_str()).collect();
        let instrument_ids: Vec<&str> = events.iter().map(|e| e.instrument_id.as_str()).collect();
        let instrument_types: Vec<String> = events
            .iter()
            .map(|e| e.instrument_type.to_string())
            .collect();
        let dates: Vec<String> = events.iter().map(|e| e.date.to_string()).collect();
        let amounts: Vec<f64> = events.iter().map(|e| e.amount.amount()).collect();
        let currencies: Vec<String> = events
            .iter()
            .map(|e| e.amount.currency().to_string())
            .collect();
        let kinds: Vec<String> = events.iter().map(|e| e.kind.to_string()).collect();
        let reset_dates: Vec<Option<String>> = events
            .iter()
            .map(|e| e.reset_date.map(|d| d.to_string()))
            .collect();
        let accrual_factors: Vec<f64> = events.iter().map(|e| e.accrual_factor).collect();
        let rates: Vec<Option<f64>> = events.iter().map(|e| e.rate).collect();

        let data = PyDict::new(py);
        data.set_item("position_id", position_ids)?;
        data.set_item("instrument_id", instrument_ids)?;
        data.set_item("instrument_type", instrument_types)?;
        data.set_item("date", dates)?;
        data.set_item("amount", amounts)?;
        data.set_item("currency", currencies)?;
        data.set_item("kind", kinds)?;
        data.set_item("reset_date", reset_dates)?;
        data.set_item("accrual_factor", accrual_factors)?;
        data.set_item("rate", rates)?;
        dict_to_dataframe(py, &data, None)
    }

    /// JSON for the ``issues`` vector only.
    #[pyo3(text_signature = "(self)")]
    fn issues_json(&self, py: Python<'_>) -> PyResult<String> {
        let issues = &self.inner.issues;
        py.detach(|| serde_json::to_string(issues))
            .map_err(display_to_py)
    }

    /// Collapse multi-currency flows into a single base-currency
    /// ``(date, CFKind) → Money`` ladder using **spot-equivalent** FX at each
    /// payment date.
    ///
    /// See :func:`finstack_quant_portfolio::cashflows::PortfolioCashflows::collapse_to_base_by_date_kind`
    /// for the exact convention. Returns JSON.
    #[pyo3(text_signature = "(self, market, base_currency, as_of)")]
    fn collapse_to_base_by_date_kind(
        &self,
        py: Python<'_>,
        market: &Bound<'_, PyAny>,
        base_currency: &str,
        as_of: &str,
    ) -> PyResult<String> {
        let market = crate::bindings::extract::extract_market_ref(py, market)?;
        let ccy: finstack_quant_core::currency::Currency =
            base_currency.parse().map_err(display_to_py)?;
        let as_of_date = super::parse_date(as_of)?;
        let market_ref: &finstack_quant_core::market_data::context::MarketContext = &market;
        let cashflows = &self.inner;
        let collapsed = py
            .detach(|| cashflows.collapse_to_base_by_date_kind(market_ref, ccy, as_of_date))
            .map_err(portfolio_to_py)?;
        py.detach(move || serde_json::to_string(&collapsed))
            .map_err(display_to_py)
    }

    fn __repr__(&self) -> String {
        format!(
            "PortfolioCashflows(events={}, positions={}, issues={})",
            self.inner.events.len(),
            self.inner.by_position.len(),
            self.inner.issues.len(),
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

// Registration

pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPortfolio>()?;
    m.add_class::<PyPortfolioValuation>()?;
    m.add_class::<PyPortfolioResult>()?;
    m.add_class::<PyPortfolioMetrics>()?;
    m.add_class::<PyPortfolioCashflows>()?;
    Ok(())
}
