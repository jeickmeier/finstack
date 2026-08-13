//! Python bindings for `finstack_quant_cashflows::accrual`.

use finstack_quant_cashflows::accrual::{
    accrued_interest_amount, AccrualConfig, AccrualIndex, AccrualMethod, ExCouponRule,
};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyType};

use crate::bindings::cashflows::builder::schedule::PyCashFlowSchedule;
use crate::bindings::core::dates::tenor::extract_tenor;
use crate::bindings::core::dates::utils::{date_to_py, py_to_date};
use crate::errors::core_to_py;

/// Wrapper for [`AccrualMethod`] (`finstack_quant.cashflows.accrual.AccrualMethod`).
#[pyclass(
    name = "AccrualMethod",
    module = "finstack_quant.cashflows.accrual",
    frozen,
    eq,
    hash,
    skip_from_py_object
)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PyAccrualMethod {
    /// Inner accrual method.
    pub(crate) inner: AccrualMethod,
}

#[pymethods]
impl PyAccrualMethod {
    /// Linear accrual: ``Accrued = Coupon × (elapsed / period)`` (default; ICMA 251.1).
    ///
    /// ICMA Rule 251.1 prescribes linear accrual for bond accrued-interest
    /// calculations. Use this method for bond-style instruments.
    #[classattr]
    #[allow(non_snake_case)]
    fn LINEAR() -> Self {
        Self {
            inner: AccrualMethod::Linear,
        }
    }

    /// Compounded accrual: ``Accrued = N × [(1 + r)^f − 1]`` (not ICMA-style).
    ///
    /// This variant uses true exponential compounding and is **not**
    /// ICMA-compliant; do not cite it as ICMA-style accrual. It is intended
    /// for instruments that genuinely compound within a coupon period (e.g.
    /// some leveraged loans).
    #[classattr]
    #[allow(non_snake_case)]
    fn COMPOUNDED() -> Self {
        Self {
            inner: AccrualMethod::Compounded,
        }
    }

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        format!("AccrualMethod({:?})", self.inner)
    }
}

/// Wrapper for [`ExCouponRule`] (`finstack_quant.cashflows.accrual.ExCouponRule`).
#[pyclass(
    name = "ExCouponRule",
    module = "finstack_quant.cashflows.accrual",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyExCouponRule {
    /// Inner ex-coupon rule.
    pub(crate) inner: ExCouponRule,
}

#[pymethods]
impl PyExCouponRule {
    /// Ex-coupon convention applied to coupon flows.
    ///
    /// Parameters
    /// ----------
    /// days_before_coupon : int
    ///     Number of days before the coupon date that go ex (max 366).
    /// calendar_id : str, optional
    ///     Business-day calendar; when omitted, calendar days are used.
    #[new]
    #[pyo3(
        signature = (days_before_coupon, calendar_id=None),
        text_signature = "(days_before_coupon, calendar_id=None)"
    )]
    fn new(days_before_coupon: u32, calendar_id: Option<String>) -> Self {
        Self {
            inner: ExCouponRule {
                days_before_coupon,
                calendar_id,
            },
        }
    }

    /// Days before the coupon date that go ex.
    #[getter]
    fn days_before_coupon(&self) -> u32 {
        self.inner.days_before_coupon
    }

    /// Optional business-day calendar identifier.
    #[getter]
    fn calendar_id(&self) -> Option<String> {
        self.inner.calendar_id.clone()
    }

    /// Ex-coupon date for a coupon paid on ``payment_date``.
    ///
    /// Parameters
    /// ----------
    /// payment_date : datetime.date
    ///     Payment date of the coupon this ex-coupon window precedes.
    ///
    /// Returns
    /// -------
    /// datetime.date
    ///     Ex-coupon date; from this date (inclusive) until ``payment_date``
    ///     (exclusive), the instrument trades ex-coupon.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``days_before_coupon`` exceeds 366.
    /// KeyError
    ///     If the configured calendar id cannot be resolved.
    #[pyo3(text_signature = "(self, payment_date)")]
    fn ex_date<'py>(
        &self,
        py: Python<'py>,
        payment_date: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let d = self
            .inner
            .ex_date(py_to_date(payment_date)?)
            .map_err(core_to_py)?;
        date_to_py(py, d)
    }

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        format!("ExCouponRule({:?})", self.inner)
    }
}

/// Wrapper for [`AccrualConfig`] (`finstack_quant.cashflows.accrual.AccrualConfig`).
#[pyclass(
    name = "AccrualConfig",
    module = "finstack_quant.cashflows.accrual",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyAccrualConfig {
    /// Inner accrual configuration.
    pub(crate) inner: AccrualConfig,
}

#[pymethods]
impl PyAccrualConfig {
    /// Generic configuration for schedule-driven interest accrual.
    ///
    /// Parameters
    /// ----------
    /// method : AccrualMethod, optional
    ///     Accrual method. When omitted, follows the Rust
    ///     `AccrualConfig::default().method` (currently
    ///     :attr:`AccrualMethod.LINEAR`, ICMA 251.1; illustrative only —
    ///     see :class:`AccrualMethod` and the Rust `Default` impl for the
    ///     authoritative value). The alternative, compounded accrual, is
    ///     not ICMA-compliant.
    /// ex_coupon : ExCouponRule, optional
    ///     Ex-coupon window rule.
    /// include_pik : bool, optional
    ///     Whether to include PIK interest in the accrued amount. When
    ///     omitted, follows the Rust `AccrualConfig::default().include_pik`
    ///     value (currently ``True``; illustrative only — see the Rust
    ///     `Default` impl for the authoritative value).
    /// frequency : Tenor or str, optional
    ///     Coupon frequency — required for ACT/ACT ISMA day count.
    #[new]
    #[pyo3(
        // NOTE: `include_pik`'s default in `signature` is derived from
        // `AccrualConfig::default()` so it always tracks the Rust default at
        // call time. `text_signature` is a static string PyO3 requires for
        // `help()`/introspection and cannot reference a Rust expression;
        // keep the literal below in sync with `AccrualConfig::default()`
        // whenever that default changes (it currently evaluates to `true`).
        signature = (method=None, ex_coupon=None, include_pik=AccrualConfig::default().include_pik, frequency=None),
        text_signature = "(method=None, ex_coupon=None, include_pik=True, frequency=None)"
    )]
    fn new(
        method: Option<PyRef<'_, PyAccrualMethod>>,
        ex_coupon: Option<PyRef<'_, PyExCouponRule>>,
        include_pik: bool,
        frequency: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        // Fall back to `AccrualConfig::default()` rather than re-encoding
        // Rust's defaults here, so a future change to the Rust default
        // flows through automatically.
        let default = AccrualConfig::default();
        Ok(Self {
            inner: AccrualConfig {
                method: method.map_or(default.method, |m| m.inner.clone()),
                ex_coupon: ex_coupon.map(|r| r.inner.clone()),
                include_pik,
                frequency: frequency.map(extract_tenor).transpose()?,
            },
        })
    }

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        format!("AccrualConfig({:?})", self.inner)
    }
}

/// Wrapper for [`AccrualIndex`] (`finstack_quant.cashflows.accrual.AccrualIndex`).
#[pyclass(
    name = "AccrualIndex",
    module = "finstack_quant.cashflows.accrual",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyAccrualIndex {
    /// Inner prebuilt accrual state.
    pub(crate) inner: AccrualIndex,
}

#[pymethods]
impl PyAccrualIndex {
    /// Build reusable accrual state for repeated ``accrued_at`` queries.
    ///
    /// Parameters
    /// ----------
    /// schedule : CashFlowSchedule
    ///     Canonical cashflow schedule containing coupon, PIK, and notional
    ///     flows.
    /// config : AccrualConfig, optional
    ///     Accrual method and ex-coupon configuration bound into the index
    ///     (default linear, PIK included). Build a separate index to accrue
    ///     under a different config.
    ///
    /// Returns
    /// -------
    /// AccrualIndex
    ///     Prebuilt accrual state; call :meth:`accrued_at` for repeated
    ///     queries against the same schedule and config.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the schedule fails validation, mixes currencies across coupon
    ///     flows, or carries a non-finite accrual factor.
    #[classmethod]
    #[pyo3(
        signature = (schedule, config=None),
        text_signature = "(cls, schedule, config=None)"
    )]
    fn build(
        _cls: &Bound<'_, PyType>,
        py: Python<'_>,
        schedule: PyRef<'_, PyCashFlowSchedule>,
        config: Option<PyRef<'_, PyAccrualConfig>>,
    ) -> PyResult<Self> {
        let schedule = schedule.inner.clone();
        let cfg = config.map_or_else(AccrualConfig::default, |c| c.inner.clone());
        py.detach(move || AccrualIndex::build(&schedule, &cfg))
            .map(|inner| Self { inner })
            .map_err(core_to_py)
    }

    /// Accrued interest as of ``as_of`` using the prebuilt periods.
    ///
    /// Parameters
    /// ----------
    /// as_of : datetime.date
    ///     Accrual cut-off date; dates outside all coupon periods return 0.0.
    ///
    /// Returns
    /// -------
    /// float
    ///     Accrued interest in the schedule's currency space; negative
    ///     inside an active ex-coupon window.
    ///
    /// Raises
    /// ------
    /// KeyError
    ///     If a configured ex-coupon calendar id cannot be resolved.
    #[pyo3(text_signature = "(self, as_of)")]
    fn accrued_at(&self, as_of: &Bound<'_, PyAny>) -> PyResult<f64> {
        self.inner
            .accrued_at(py_to_date(as_of)?)
            .map_err(core_to_py)
    }

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        "AccrualIndex(...)".to_string()
    }
}

/// Compute accrued interest for a schedule as of ``as_of``.
///
/// Parameters
/// ----------
/// schedule : CashFlowSchedule
///     Canonical cashflow schedule.
/// as_of : datetime.date
///     Accrual cut-off date; dates outside all coupon periods return 0.0.
/// config : AccrualConfig, optional
///     Accrual method and ex-coupon configuration (default linear, PIK included).
///
/// Returns
/// -------
/// float
///     Accrued interest in the schedule's currency space; negative inside an
///     active ex-coupon window.
///
/// Raises
/// ------
/// ValueError
///     If the schedule fails validation, mixes currencies across coupon
///     flows, or carries a non-finite accrual factor.
/// KeyError
///     If a configured ex-coupon calendar id cannot be resolved.
#[pyfunction(name = "accrued_interest_amount")]
#[pyo3(
    signature = (schedule, as_of, config=None),
    text_signature = "(schedule, as_of, config=None)"
)]
fn py_accrued_interest_amount(
    py: Python<'_>,
    schedule: PyRef<'_, PyCashFlowSchedule>,
    as_of: &Bound<'_, PyAny>,
    config: Option<PyRef<'_, PyAccrualConfig>>,
) -> PyResult<f64> {
    let as_of = py_to_date(as_of)?;
    let schedule = schedule.inner.clone();
    let cfg = config.map_or_else(AccrualConfig::default, |c| c.inner.clone());
    py.detach(move || accrued_interest_amount(&schedule, as_of, &cfg))
        .map_err(core_to_py)
}

/// Register the `finstack_quant.cashflows.accrual` submodule.
pub(crate) fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "accrual")?;
    module.setattr(
        "__doc__",
        "Schedule-driven accrued interest: methods, ex-coupon rules, accrual index.",
    )?;
    module.add_class::<PyAccrualMethod>()?;
    module.add_class::<PyExCouponRule>()?;
    module.add_class::<PyAccrualConfig>()?;
    module.add_class::<PyAccrualIndex>()?;
    module.add_function(wrap_pyfunction!(py_accrued_interest_amount, &module)?)?;

    let all = PyList::new(
        py,
        [
            "AccrualConfig",
            "AccrualIndex",
            "AccrualMethod",
            "ExCouponRule",
            "accrued_interest_amount",
        ],
    )?;
    module.setattr("__all__", all)?;

    crate::bindings::module_utils::register_submodule(
        py,
        parent,
        &module,
        "accrual",
        "finstack_quant.cashflows",
        crate::bindings::module_utils::ParentNameSource::Package,
    )?;
    Ok(())
}
