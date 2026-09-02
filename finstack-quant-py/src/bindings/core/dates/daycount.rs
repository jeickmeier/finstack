//! Python bindings for day-count conventions from [`finstack_quant_core::dates`].

use crate::bindings::core::dates::tenor::PyTenor;
use crate::bindings::date_utils::{date_to_py, py_to_date};
use crate::errors::core_to_py;
use finstack_quant_core::dates::{
    DayCount, DayCountContext, DayCountContextState, Thirty360Convention,
};
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyType};

/// Wrapper for [`DayCount`] exposed to Python as `finstack_quant.core.dates.DayCount`.
#[pyclass(
    name = "DayCount",
    module = "finstack_quant.core.dates",
    frozen,
    eq,
    hash,
    skip_from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PyDayCount {
    /// Inner day-count convention.
    pub(crate) inner: DayCount,
}

impl PyDayCount {
    /// Build from an existing Rust [`DayCount`].
    pub(crate) const fn from_inner(inner: DayCount) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyDayCount {
    /// Actual/360 (money market).
    #[classattr]
    const ACT_360: PyDayCount = PyDayCount {
        inner: DayCount::Act360,
    };
    /// Actual/365 Fixed.
    #[classattr]
    const ACT_365F: PyDayCount = PyDayCount {
        inner: DayCount::Act365F,
    };
    /// Actual/365L (ICMA Rule 251). Annual periods (or periods without a
    /// supplied frequency) use denominator 366 exactly when February 29 falls
    /// in ``(start, end]``; non-annual periods use 366 exactly when the end
    /// date's year is a leap year. Otherwise the denominator is 365. This is
    /// explicitly not ACT/ACT AFB, which uses sub-period splitting.
    #[classattr]
    const ACT_365L: PyDayCount = PyDayCount {
        inner: DayCount::Act365L,
    };
    /// 30/360 US (Bond Basis).
    #[classattr]
    const THIRTY_360: PyDayCount = PyDayCount {
        inner: DayCount::Thirty360,
    };
    /// 30E/360 (Eurobond Basis).
    #[classattr]
    const THIRTY_E_360: PyDayCount = PyDayCount {
        inner: DayCount::ThirtyE360,
    };
    /// 30E/360 ISDA.
    #[classattr]
    const THIRTY_E_360_ISDA: PyDayCount = PyDayCount {
        inner: DayCount::ThirtyE360Isda,
    };
    /// Actual/Actual (ISDA).
    #[classattr]
    const ACT_ACT: PyDayCount = PyDayCount {
        inner: DayCount::ActAct,
    };
    /// Actual/Actual (ICMA/ISMA).
    #[classattr]
    const ACT_ACT_ISMA: PyDayCount = PyDayCount {
        inner: DayCount::ActActIsma,
    };
    /// Actual/Actual AFB (Actual/Actual Euro).
    ///
    /// Walks whole years backwards from the end date (QuantLib
    /// ``ActualActual::AFB``). A year-step landing on 28 February of a leap
    /// year is bumped to 29 February. The residual uses denominator 366 if
    /// 29 February lies in ``[start, residual_end)``, else 365.
    #[classattr]
    const ACT_ACT_AFB: PyDayCount = PyDayCount {
        inner: DayCount::ActActAfb,
    };
    /// 30/360 Italian.
    ///
    /// Day 31 becomes 30, and any February day after the 27th becomes 30
    /// (QuantLib ``Thirty360::Italian``). Distinct from US SIA and 30E/360.
    #[classattr]
    const THIRTY_360_IT: PyDayCount = PyDayCount {
        inner: DayCount::Thirty360It,
    };
    /// Business/252 (Brazilian market convention).
    #[classattr]
    const BUS_252: PyDayCount = PyDayCount {
        inner: DayCount::Bus252,
    };

    /// Parse a day-count convention from its string name (e.g. ``"act_360"``).
    ///
    /// # Arguments
    ///
    /// * `name` - Canonical lowercase convention identifier such as
    ///   ``"act_360"``, ``"act_act_afb"``, or ``"30_360_it"``.
    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        name.parse::<DayCount>()
            .map(Self::from_inner)
            .map_err(crate::errors::value_error)
    }

    /// Compute the year fraction between two dates under this convention.
    ///
    /// Dates are Python ``datetime.date`` objects.
    #[pyo3(signature = (start, end, ctx=None))]
    fn year_fraction(
        &self,
        start: &Bound<'_, PyAny>,
        end: &Bound<'_, PyAny>,
        ctx: Option<&PyDayCountContext>,
    ) -> PyResult<f64> {
        let s = py_to_date(start)?;
        let e = py_to_date(end)?;
        let context = match ctx {
            Some(c) => c.to_rust_ctx()?,
            None => DayCountContext::default(),
        };
        self.inner.year_fraction(s, e, context).map_err(core_to_py)
    }

    /// Compute the signed year fraction (negative when ``start > end``).
    #[pyo3(signature = (start, end, ctx=None))]
    fn signed_year_fraction(
        &self,
        start: &Bound<'_, PyAny>,
        end: &Bound<'_, PyAny>,
        ctx: Option<&PyDayCountContext>,
    ) -> PyResult<f64> {
        let s = py_to_date(start)?;
        let e = py_to_date(end)?;
        let context = match ctx {
            Some(c) => c.to_rust_ctx()?,
            None => DayCountContext::default(),
        };
        self.inner
            .signed_year_fraction(s, e, context)
            .map_err(core_to_py)
    }

    /// Count the calendar days between two dates.
    #[staticmethod]
    #[pyo3(text_signature = "(start, end)")]
    fn calendar_days(start: &Bound<'_, PyAny>, end: &Bound<'_, PyAny>) -> PyResult<i64> {
        let s = py_to_date(start)?;
        let e = py_to_date(end)?;
        Ok(DayCount::calendar_days(s, e))
    }

    fn __repr__(&self) -> String {
        format!("DayCount('{}')", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

/// Optional context for day-count calculations.
///
/// Certain conventions require additional information:
/// - ``Bus/252`` requires a holiday calendar (resolved by ``calendar_id``).
/// - ``Act/Act (ISMA)`` requires the coupon ``frequency``.
#[pyclass(
    name = "DayCountContext",
    module = "finstack_quant.core.dates",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDayCountContext {
    /// Serializable state; the live calendar is resolved on each use.
    pub(crate) inner: DayCountContextState,
}

impl PyDayCountContext {
    /// Resolve to a runtime [`DayCountContext`] using the global calendar registry.
    ///
    /// # Errors
    ///
    /// Raises ``KeyError`` when ``calendar_id`` is set but cannot be resolved
    /// in the global calendar registry.
    fn to_rust_ctx(&self) -> PyResult<DayCountContext<'static>> {
        // Routes through the core registry error so unknown codes surface
        // "Did you mean …?" suggestions instead of a bare message.
        self.inner.to_ctx().map_err(core_to_py)
    }
}

#[pymethods]
impl PyDayCountContext {
    /// Create a day-count context.
    ///
    /// ``coupon_period`` is an optional ``(start, end)`` pair of
    /// ``datetime.date`` giving the reference coupon period for
    /// ACT/ACT (ICMA).
    #[new]
    #[pyo3(signature = (calendar_id=None, frequency=None, bus_basis=None, coupon_period=None, end_is_termination_date=false))]
    fn new(
        calendar_id: Option<String>,
        frequency: Option<PyRef<PyTenor>>,
        bus_basis: Option<u16>,
        coupon_period: Option<(Bound<'_, PyAny>, Bound<'_, PyAny>)>,
        end_is_termination_date: bool,
    ) -> PyResult<Self> {
        let coupon_period = coupon_period
            .map(|(s, e)| {
                let start = py_to_date(&s)?;
                let end = py_to_date(&e)?;
                DayCountContext::default()
                    .with_coupon_period(start, end)
                    .map(|_| (start, end))
                    .map_err(core_to_py)
            })
            .transpose()?;
        Ok(Self {
            inner: DayCountContextState {
                calendar_id,
                frequency: frequency.map(|f| f.inner),
                bus_basis,
                coupon_period,
                end_is_termination_date,
            },
        })
    }

    /// Optional calendar identifier.
    #[getter]
    fn calendar_id(&self) -> Option<&str> {
        self.inner.calendar_id.as_deref()
    }

    /// Optional coupon frequency.
    #[getter]
    fn frequency(&self) -> Option<PyTenor> {
        self.inner.frequency.map(PyTenor::from_inner)
    }

    /// Optional custom business-day divisor.
    #[getter]
    fn bus_basis(&self) -> Option<u16> {
        self.inner.bus_basis
    }

    /// Optional reference coupon period as ``(start, end)`` dates.
    #[getter]
    fn coupon_period<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Option<(Bound<'py, PyAny>, Bound<'py, PyAny>)>> {
        self.inner
            .coupon_period
            .map(|(s, e)| Ok((date_to_py(py, s)?, date_to_py(py, e)?)))
            .transpose()
    }

    /// Whether the accrual end is the instrument termination date.
    #[getter]
    fn end_is_termination_date(&self) -> bool {
        self.inner.end_is_termination_date
    }

    /// Convert to a serializable state snapshot.
    fn to_state(&self) -> PyDayCountContextState {
        PyDayCountContextState {
            inner: self.inner.clone(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "DayCountContext(calendar_id={:?}, frequency={:?}, bus_basis={:?}, coupon_period={:?}, end_is_termination_date={})",
            self.inner.calendar_id,
            self.inner.frequency,
            self.inner.bus_basis,
            self.inner.coupon_period,
            self.inner.end_is_termination_date,
        )
    }
}

/// Serializable snapshot of [`DayCountContext`] for persistence.
#[pyclass(
    name = "DayCountContextState",
    module = "finstack_quant.core.dates",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDayCountContextState {
    /// Inner serializable state.
    pub(crate) inner: DayCountContextState,
}

#[pymethods]
impl PyDayCountContextState {
    /// Create a context state.
    ///
    /// ``coupon_period`` is an optional ``(start, end)`` pair of
    /// ``datetime.date``.
    #[new]
    #[pyo3(signature = (calendar_id=None, frequency=None, bus_basis=None, coupon_period=None, end_is_termination_date=false))]
    fn new(
        calendar_id: Option<String>,
        frequency: Option<PyRef<PyTenor>>,
        bus_basis: Option<u16>,
        coupon_period: Option<(Bound<'_, PyAny>, Bound<'_, PyAny>)>,
        end_is_termination_date: bool,
    ) -> PyResult<Self> {
        let coupon_period = coupon_period
            .map(|(s, e)| Ok::<_, PyErr>((py_to_date(&s)?, py_to_date(&e)?)))
            .transpose()?;
        Ok(Self {
            inner: DayCountContextState {
                calendar_id,
                frequency: frequency.map(|f| f.inner),
                bus_basis,
                coupon_period,
                end_is_termination_date,
            },
        })
    }

    /// Reconstruct a live [`DayCountContext`] from this state.
    fn to_context(&self) -> PyDayCountContext {
        PyDayCountContext {
            inner: self.inner.clone(),
        }
    }

    /// Optional calendar identifier.
    #[getter]
    fn calendar_id(&self) -> Option<&str> {
        self.inner.calendar_id.as_deref()
    }

    /// Optional coupon frequency.
    #[getter]
    fn frequency(&self) -> Option<PyTenor> {
        self.inner.frequency.map(PyTenor::from_inner)
    }

    /// Optional custom business-day divisor.
    #[getter]
    fn bus_basis(&self) -> Option<u16> {
        self.inner.bus_basis
    }

    /// Optional reference coupon period as ``(start, end)`` dates.
    #[getter]
    fn coupon_period<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Option<(Bound<'py, PyAny>, Bound<'py, PyAny>)>> {
        self.inner
            .coupon_period
            .map(|(s, e)| Ok((date_to_py(py, s)?, date_to_py(py, e)?)))
            .transpose()
    }

    /// Whether the accrual end is the instrument termination date.
    #[getter]
    fn end_is_termination_date(&self) -> bool {
        self.inner.end_is_termination_date
    }

    fn __repr__(&self) -> String {
        format!(
            "DayCountContextState(calendar_id={:?}, frequency={:?}, bus_basis={:?}, coupon_period={:?}, end_is_termination_date={})",
            self.inner.calendar_id,
            self.inner.frequency,
            self.inner.bus_basis,
            self.inner.coupon_period,
            self.inner.end_is_termination_date,
        )
    }
}

/// 30/360 sub-convention (US vs European).
#[pyclass(
    name = "Thirty360Convention",
    module = "finstack_quant.core.dates",
    frozen,
    eq,
    hash,
    skip_from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PyThirty360Convention {
    /// Inner convention variant.
    pub(crate) inner: Thirty360Convention,
}

#[pymethods]
impl PyThirty360Convention {
    /// US 30/360 SIA / Bond Basis convention.
    #[classattr]
    const US_SIA: PyThirty360Convention = PyThirty360Convention {
        inner: Thirty360Convention::UsSia,
    };
    /// 30/360 ISDA convention.
    #[classattr]
    const ISDA: PyThirty360Convention = PyThirty360Convention {
        inner: Thirty360Convention::Isda,
    };
    /// European 30E/360 convention.
    #[classattr]
    const EUROPEAN: PyThirty360Convention = PyThirty360Convention {
        inner: Thirty360Convention::European,
    };
    /// 30/360 Italian convention.
    #[classattr]
    const ITALIAN: PyThirty360Convention = PyThirty360Convention {
        inner: Thirty360Convention::Italian,
    };

    fn __repr__(&self) -> String {
        let label = match self.inner {
            Thirty360Convention::UsSia => "US_SIA",
            Thirty360Convention::Isda => "ISDA",
            Thirty360Convention::European => "EUROPEAN",
            Thirty360Convention::Italian => "ITALIAN",
        };
        format!("Thirty360Convention.{label}")
    }

    fn __str__(&self) -> String {
        match self.inner {
            Thirty360Convention::UsSia => "us_sia".to_string(),
            Thirty360Convention::Isda => "isda".to_string(),
            Thirty360Convention::European => "european".to_string(),
            Thirty360Convention::Italian => "italian".to_string(),
        }
    }
}

/// Register day-count types on the `finstack_quant.core.dates` module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyDayCount>()?;
    m.add_class::<PyDayCountContext>()?;
    m.add_class::<PyDayCountContextState>()?;
    m.add_class::<PyThirty360Convention>()?;
    Ok(())
}

/// Names exported from this submodule.
pub const EXPORTS: &[&str] = &[
    "DayCount",
    "DayCountContext",
    "DayCountContextState",
    "Thirty360Convention",
];
