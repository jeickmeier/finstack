//! Python bindings for `finstack_quant_cashflows::builder::schedule`.

use finstack_quant_cashflows::aggregation::DateContext;
use finstack_quant_cashflows::builder::schedule::merge_cashflow_schedules;
use finstack_quant_cashflows::builder::{CashFlowMeta, CashFlowSchedule, PvDiscountSource};
use finstack_quant_core::dates::{DayCount, DayCountContext};
use finstack_quant_core::types::CurveId;
use indexmap::IndexMap;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyType};

use crate::bindings::cashflows::primitives::PyCashFlow;
use crate::bindings::core::dates::daycount::PyDayCount;
use crate::bindings::core::dates::periods::PyPeriod;
use crate::bindings::core::dates::utils::{date_to_py, py_to_date};
use crate::bindings::core::money::PyMoney;
use crate::errors::core_to_py;

use super::orchestrator::PyCashFlowBuilder;
use super::specs::PyNotional;

/// Wrapper for [`CashFlowMeta`]
/// (`finstack_quant.cashflows.builder.CashFlowMeta`).
#[pyclass(
    name = "CashFlowMeta",
    module = "finstack_quant.cashflows.builder",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCashFlowMeta {
    /// Inner schedule metadata.
    pub(crate) inner: CashFlowMeta,
}

#[pymethods]
impl PyCashFlowMeta {
    /// Schedule representation label (``"contractual"``, ``"projected"``, …).
    #[getter]
    fn representation(&self) -> PyResult<String> {
        serde_json::to_value(self.inner.representation)
            .map_err(crate::errors::display_to_py)?
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| crate::errors::value_error("non-string representation label"))
    }

    /// Holiday calendar identifiers used by the schedule.
    #[getter]
    fn calendar_ids(&self) -> Vec<String> {
        self.inner.calendar_ids.clone()
    }

    /// Optional facility limit / commitment.
    #[getter]
    fn facility_limit(&self) -> Option<PyMoney> {
        self.inner.facility_limit.map(PyMoney::from_inner)
    }

    /// Instrument issue date, when known.
    #[getter]
    fn issue_date<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyAny>>> {
        self.inner.issue_date.map(|d| date_to_py(py, d)).transpose()
    }

    /// Contractual maturity date, when known.
    #[getter]
    fn maturity_date<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyAny>>> {
        self.inner
            .maturity_date
            .map(|d| date_to_py(py, d))
            .transpose()
    }

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        format!("CashFlowMeta({:?})", self.inner)
    }
}

/// Wrapper for [`CashFlowSchedule`]
/// (`finstack_quant.cashflows.builder.CashFlowSchedule`).
#[pyclass(
    name = "CashFlowSchedule",
    module = "finstack_quant.cashflows.builder",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCashFlowSchedule {
    /// Inner canonical schedule.
    pub(crate) inner: CashFlowSchedule,
}

impl PyCashFlowSchedule {
    /// Build from an existing Rust [`CashFlowSchedule`].
    pub(crate) fn from_inner(inner: CashFlowSchedule) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCashFlowSchedule {
    /// Create a new fluent cashflow builder (the only builder entry point).
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn builder() -> PyCashFlowBuilder {
        PyCashFlowBuilder::new_default()
    }

    /// Canonical ordered cashflows.
    #[pyo3(text_signature = "(self)")]
    fn get_flows(&self) -> Vec<PyCashFlow> {
        self.inner
            .get_flows()
            .iter()
            .map(|f| PyCashFlow::from_inner(*f))
            .collect()
    }

    /// Interest-like coupon cashflows (Fixed/FloatReset/InflationCoupon/Stub).
    #[pyo3(text_signature = "(self)")]
    fn coupons(&self) -> Vec<PyCashFlow> {
        self.inner
            .coupons()
            .map(|f| PyCashFlow::from_inner(*f))
            .collect()
    }

    /// Flow dates in schedule order.
    #[pyo3(text_signature = "(self)")]
    fn dates<'py>(&self, py: Python<'py>) -> PyResult<Vec<Bound<'py, PyAny>>> {
        self.inner
            .dates()
            .into_iter()
            .map(|d| date_to_py(py, d))
            .collect()
    }

    /// Representative schedule notional.
    #[pyo3(text_signature = "(self)")]
    fn get_notional(&self) -> PyNotional {
        PyNotional::from_inner(self.inner.get_notional().clone())
    }

    /// Representative day-count convention.
    #[pyo3(text_signature = "(self)")]
    fn get_day_count(&self) -> PyDayCount {
        PyDayCount::from_inner(self.inner.get_day_count())
    }

    /// Schedule-level metadata.
    #[pyo3(text_signature = "(self)")]
    fn get_meta(&self) -> PyCashFlowMeta {
        PyCashFlowMeta {
            inner: self.inner.get_meta().clone(),
        }
    }

    /// Validate all schedule-level and per-flow invariants.
    #[pyo3(text_signature = "(self)")]
    fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(core_to_py)
    }

    /// Return a new schedule with every amount scaled by ``scale``.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``scale`` is NaN or infinite.
    #[pyo3(text_signature = "(self, scale)")]
    fn scale_amounts(&self, scale: f64) -> PyResult<Self> {
        self.inner
            .clone()
            .scale_amounts(scale)
            .map(Self::from_inner)
            .map_err(core_to_py)
    }

    /// Weighted average life in years from ``as_of`` (Act/365F basis).
    #[pyo3(text_signature = "(self, as_of)")]
    fn weighted_average_life(&self, as_of: &Bound<'_, PyAny>) -> PyResult<f64> {
        self.inner
            .weighted_average_life(py_to_date(as_of)?)
            .map_err(core_to_py)
    }

    /// Outstanding balance path as ``[(date, Money), ...]``.
    #[pyo3(text_signature = "(self)")]
    fn outstanding_by_date<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Vec<(Bound<'py, PyAny>, PyMoney)>> {
        self.inner
            .outstanding_by_date()
            .map_err(core_to_py)?
            .into_iter()
            .map(|(d, m)| Ok((date_to_py(py, d)?, PyMoney::from_inner(m))))
            .collect()
    }

    /// Periodized present values resolved from a market context.
    ///
    /// Parameters
    /// ----------
    /// periods : list[Period]
    ///     Reporting periods (half-open buckets).
    /// market : MarketContext
    ///     Market context containing the required curves.
    /// disc_curve_id : str
    ///     Discount curve identifier.
    /// base : datetime.date
    ///     Valuation date for discount times.
    /// day_count : DayCount, optional
    ///     Day-count for discount times (default Act/365F).
    /// hazard_curve_id : str, optional
    ///     Hazard curve identifier for credit-adjusted PV.
    ///
    /// Returns
    /// -------
    /// dict[str, dict[str, Money]]
    ///     ``{period_id_label: {currency_code: pv}}``; empty periods omitted.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(
        signature = (periods, market, disc_curve_id, base, day_count=None, hazard_curve_id=None),
        text_signature = "(self, periods, market, disc_curve_id, base, day_count=None, hazard_curve_id=None)"
    )]
    fn pv_by_period<'py>(
        &self,
        py: Python<'py>,
        periods: Vec<PyRef<'_, PyPeriod>>,
        market: &Bound<'_, PyAny>,
        disc_curve_id: &str,
        base: &Bound<'_, PyAny>,
        day_count: Option<PyRef<'_, PyDayCount>>,
        hazard_curve_id: Option<&str>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let periods: Vec<finstack_quant_core::dates::Period> =
            periods.iter().map(|p| p.inner.clone()).collect();
        let market = crate::bindings::extract::extract_market(py, market)?;
        let base = py_to_date(base)?;
        let day_count = day_count.map_or(DayCount::Act365F, |d| d.inner);
        let disc_id = CurveId::from(disc_curve_id);
        let hazard_id = hazard_curve_id.map(CurveId::from);
        let schedule = self.inner.clone();
        let result = py
            .detach(move || {
                self_pv(
                    &periods,
                    &market,
                    &disc_id,
                    hazard_id.as_ref(),
                    base,
                    day_count,
                    &schedule,
                )
            })
            .map_err(core_to_py)?;
        let out = PyDict::new(py);
        for (pid, per_currency) in result {
            let inner = PyDict::new(py);
            for (ccy, m) in per_currency {
                inner.set_item(ccy.to_string(), PyMoney::from_inner(m))?;
            }
            out.set_item(pid.to_string(), inner)?;
        }
        Ok(out)
    }

    /// Serialize the canonical schedule to JSON.
    #[allow(clippy::wrong_self_convention)]
    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| crate::errors::serde_json_to_py(e, "failed to serialize CashFlowSchedule"))
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

    /// Deserialize a schedule from canonical JSON (strict field names).
    #[classmethod]
    #[pyo3(text_signature = "(cls, json)")]
    fn from_json(_cls: &Bound<'_, PyType>, json: &str) -> PyResult<Self> {
        serde_json::from_str::<CashFlowSchedule>(json)
            .map(Self::from_inner)
            .map_err(|e| crate::errors::serde_json_to_py(e, "invalid CashFlowSchedule JSON"))
    }

    /// Flows as a pandas DataFrame with columns ``date, kind, amount, currency``.
    #[allow(clippy::wrong_self_convention)]
    #[pyo3(text_signature = "(self)")]
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let flows = self.inner.get_flows();
        let dates: Vec<time::Date> = flows.iter().map(|f| f.date).collect();
        let kinds: Vec<String> = flows.iter().map(|f| f.kind.to_string()).collect();
        let amounts: Vec<f64> = flows.iter().map(|f| f.amount.amount()).collect();
        let currencies: Vec<String> = flows
            .iter()
            .map(|f| f.amount.currency().to_string())
            .collect();
        let columns = PyDict::new(py);
        columns.set_item(
            "date",
            crate::bindings::pandas_utils::dates_to_pylist(py, &dates)?,
        )?;
        columns.set_item("kind", kinds)?;
        columns.set_item("amount", amounts)?;
        columns.set_item("currency", currencies)?;
        crate::bindings::pandas_utils::dict_to_dataframe(py, &columns, None)
    }

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        format!("CashFlowSchedule(flows={})", self.inner.get_flows().len())
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

/// GIL-free core of `pv_by_period` (kept out of the pymethod for clarity).
#[allow(clippy::too_many_arguments)]
fn self_pv(
    periods: &[finstack_quant_core::dates::Period],
    market: &finstack_quant_core::market_data::context::MarketContext,
    disc_id: &CurveId,
    hazard_id: Option<&CurveId>,
    base: finstack_quant_core::dates::Date,
    day_count: DayCount,
    schedule: &CashFlowSchedule,
) -> finstack_quant_core::Result<
    IndexMap<
        finstack_quant_core::dates::PeriodId,
        IndexMap<finstack_quant_core::currency::Currency, finstack_quant_core::money::Money>,
    >,
> {
    schedule.pv_by_period(
        periods,
        PvDiscountSource::Market {
            market,
            disc_curve_id: disc_id,
            hazard_curve_id: hazard_id,
        },
        DateContext::new(base, day_count, DayCountContext::default()),
    )
}

/// Merge multiple schedules into one deterministic composite schedule.
///
/// Parameters
/// ----------
/// schedules : list[CashFlowSchedule]
///     Schedules to combine.
/// notional : Notional
///     Representative notional stamped on the merged schedule.
/// day_count : DayCount
///     Day-count convention attached to the merged schedule.
#[pyfunction(name = "merge_cashflow_schedules")]
#[pyo3(text_signature = "(schedules, notional, day_count)")]
pub(crate) fn py_merge_cashflow_schedules(
    schedules: Vec<PyRef<'_, PyCashFlowSchedule>>,
    notional: PyRef<'_, PyNotional>,
    day_count: PyRef<'_, PyDayCount>,
) -> PyCashFlowSchedule {
    let inner = merge_cashflow_schedules(
        schedules.iter().map(|s| s.inner.clone()),
        notional.inner.clone(),
        day_count.inner,
    );
    PyCashFlowSchedule::from_inner(inner)
}
