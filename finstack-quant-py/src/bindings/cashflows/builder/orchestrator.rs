//! Python bindings for `CashFlowBuilder` and `PrincipalEvent`.

use finstack_quant_cashflows::builder::{CashFlowBuilder, PrincipalEvent};
use pyo3::prelude::*;

use crate::bindings::cashflows::primitives::{extract_cf_kind, PyCFKind};
use crate::bindings::core::dates::utils::{date_to_py, py_to_date};
use crate::bindings::core::money::PyMoney;
use crate::errors::core_to_py;

use super::schedule::PyCashFlowSchedule;
use super::specs::{
    PyAmortizationSpec, PyCouponType, PyFeeSpec, PyFixedCouponSpec, PyFloatingCouponSpec,
    PyStepUpCouponSpec,
};

/// Wrapper for [`PrincipalEvent`]
/// (`finstack_quant.cashflows.builder.PrincipalEvent`).
#[pyclass(
    name = "PrincipalEvent",
    module = "finstack_quant.cashflows.builder",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyPrincipalEvent {
    /// Inner principal event.
    pub(crate) inner: PrincipalEvent,
}

#[pymethods]
impl PyPrincipalEvent {
    /// Principal event applied during schedule build (draws/repays).
    ///
    /// Parameters
    /// ----------
    /// date : datetime.date
    ///     Event date.
    /// delta : Money
    ///     Outstanding delta (positive increases balance, negative repays).
    /// cash : Money
    ///     Cash leg paid/received (may differ from delta for OID/fees).
    /// kind : CFKind or str
    ///     Classification for the emitted cashflow.
    #[new]
    #[pyo3(text_signature = "(date, delta, cash, kind)")]
    fn new(
        date: &Bound<'_, PyAny>,
        delta: PyMoney,
        cash: PyMoney,
        kind: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: PrincipalEvent {
                date: py_to_date(date)?,
                delta: delta.inner,
                cash: cash.inner,
                kind: extract_cf_kind(kind)?,
            },
        })
    }

    /// Event date.
    #[getter]
    fn date<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        date_to_py(py, self.inner.date)
    }

    /// Outstanding delta.
    #[getter]
    fn delta(&self) -> PyMoney {
        PyMoney::from_inner(self.inner.delta)
    }

    /// Cash leg.
    #[getter]
    fn cash(&self) -> PyMoney {
        PyMoney::from_inner(self.inner.cash)
    }

    /// Emitted cashflow classification.
    #[getter]
    fn kind(&self) -> PyCFKind {
        PyCFKind::from_inner(self.inner.kind)
    }

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        format!(
            "PrincipalEvent(date={}, kind='{}')",
            self.inner.date, self.inner.kind
        )
    }
}

/// Wrapper for [`CashFlowBuilder`]
/// (`finstack_quant.cashflows.builder.CashFlowBuilder`).
///
/// Created via ``CashFlowSchedule.builder()`` only; fluent methods mutate in
/// place and return ``self`` for chaining, matching the Rust `&mut self` API.
#[pyclass(
    name = "CashFlowBuilder",
    module = "finstack_quant.cashflows.builder",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCashFlowBuilder {
    /// Inner fluent builder.
    pub(crate) inner: CashFlowBuilder,
}

impl PyCashFlowBuilder {
    /// Fresh default builder (used by `CashFlowSchedule.builder()`).
    pub(crate) fn new_default() -> Self {
        Self {
            inner: CashFlowBuilder::default(),
        }
    }
}

#[pymethods]
impl PyCashFlowBuilder {
    /// Set principal details and instrument horizon.
    #[pyo3(text_signature = "(self, initial, issue_date, maturity)")]
    fn principal<'py>(
        mut slf: PyRefMut<'py, Self>,
        initial: PyMoney,
        issue_date: &Bound<'py, PyAny>,
        maturity: &Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let issue = py_to_date(issue_date)?;
        let maturity = py_to_date(maturity)?;
        let _ = slf.inner.principal(initial.inner, issue, maturity);
        Ok(slf)
    }

    /// Configure amortization for the instrument notional.
    #[pyo3(text_signature = "(self, spec)")]
    fn amortization<'py>(
        mut slf: PyRefMut<'py, Self>,
        spec: PyRef<'py, PyAmortizationSpec>,
    ) -> PyRefMut<'py, Self> {
        let inner_spec = spec.inner.clone();
        let _ = slf.inner.amortization(inner_spec);
        slf
    }

    /// Add a single principal event (draw/repay).
    #[pyo3(
        signature = (date, delta, kind, cash=None),
        text_signature = "(self, date, delta, kind, cash=None)"
    )]
    fn add_principal_event<'py>(
        mut slf: PyRefMut<'py, Self>,
        date: &Bound<'py, PyAny>,
        delta: PyMoney,
        kind: &Bound<'py, PyAny>,
        cash: Option<PyMoney>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let date = py_to_date(date)?;
        let kind = extract_cf_kind(kind)?;
        let _ = slf
            .inner
            .add_principal_event(date, delta.inner, cash.map(|c| c.inner), kind);
        Ok(slf)
    }

    /// Add a full-horizon fixed coupon leg.
    #[pyo3(text_signature = "(self, spec)")]
    fn fixed_cf<'py>(
        mut slf: PyRefMut<'py, Self>,
        spec: PyRef<'py, PyFixedCouponSpec>,
    ) -> PyRefMut<'py, Self> {
        let inner_spec = spec.inner.clone();
        let _ = slf.inner.fixed_cf(inner_spec);
        slf
    }

    /// Add a full-horizon floating coupon leg.
    #[pyo3(text_signature = "(self, spec)")]
    fn floating_cf<'py>(
        mut slf: PyRefMut<'py, Self>,
        spec: PyRef<'py, PyFloatingCouponSpec>,
    ) -> PyRefMut<'py, Self> {
        let inner_spec = spec.inner.clone();
        let _ = slf.inner.floating_cf(inner_spec);
        slf
    }

    /// Add a full-horizon step-up coupon leg.
    #[pyo3(text_signature = "(self, spec)")]
    fn step_up_cf<'py>(
        mut slf: PyRefMut<'py, Self>,
        spec: PyRef<'py, PyStepUpCouponSpec>,
    ) -> PyRefMut<'py, Self> {
        let inner_spec = spec.inner.clone();
        let _ = slf.inner.step_up_cf(inner_spec);
        slf
    }

    /// Add a fee specification (fixed or periodic bp).
    #[pyo3(text_signature = "(self, spec)")]
    fn fee<'py>(mut slf: PyRefMut<'py, Self>, spec: PyRef<'py, PyFeeSpec>) -> PyRefMut<'py, Self> {
        let inner_spec = spec.inner.clone();
        let _ = slf.inner.fee(inner_spec);
        slf
    }

    /// Add a fixed coupon over the half-open window ``[start, end)``.
    #[pyo3(text_signature = "(self, start, end, spec)")]
    fn add_fixed_window<'py>(
        mut slf: PyRefMut<'py, Self>,
        start: &Bound<'py, PyAny>,
        end: &Bound<'py, PyAny>,
        spec: PyRef<'py, PyFixedCouponSpec>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let (start, end) = (py_to_date(start)?, py_to_date(end)?);
        let inner_spec = spec.inner.clone();
        let _ = slf.inner.add_fixed_window(start, end, inner_spec);
        Ok(slf)
    }

    /// Add a floating coupon over the half-open window ``[start, end)``.
    #[pyo3(text_signature = "(self, start, end, spec)")]
    fn add_floating_window<'py>(
        mut slf: PyRefMut<'py, Self>,
        start: &Bound<'py, PyAny>,
        end: &Bound<'py, PyAny>,
        spec: PyRef<'py, PyFloatingCouponSpec>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let (start, end) = (py_to_date(start)?, py_to_date(end)?);
        let inner_spec = spec.inner.clone();
        let _ = slf.inner.add_floating_window(start, end, inner_spec);
        Ok(slf)
    }

    /// Set the payment split (Cash/PIK/Split) over ``[start, end)``.
    #[pyo3(text_signature = "(self, start, end, split)")]
    fn add_payment_window<'py>(
        mut slf: PyRefMut<'py, Self>,
        start: &Bound<'py, PyAny>,
        end: &Bound<'py, PyAny>,
        split: PyRef<'py, PyCouponType>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let (start, end) = (py_to_date(start)?, py_to_date(end)?);
        let split = split.inner;
        let _ = slf.inner.add_payment_window(start, end, split);
        Ok(slf)
    }

    /// Payment split program from boundary dates (PIK toggle windows).
    #[pyo3(text_signature = "(self, steps)")]
    fn payment_split_program<'py>(
        mut slf: PyRefMut<'py, Self>,
        steps: Vec<(Bound<'py, PyAny>, PyRef<'py, PyCouponType>)>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let steps: Vec<(finstack_quant_core::dates::Date, _)> = steps
            .iter()
            .map(|(d, ct)| Ok((py_to_date(d)?, ct.inner)))
            .collect::<PyResult<_>>()?;
        let _ = slf.inner.payment_split_program(&steps);
        Ok(slf)
    }

    /// Build the cashflow schedule.
    ///
    /// Parameters
    /// ----------
    /// market : MarketContext, optional
    ///     Market context for floating-rate projection. Fixed coupons and
    ///     deterministic fees do not require one.
    ///
    /// Raises
    /// ------
    /// KeyError
    ///     If required inputs (principal) or market data are missing.
    /// ValueError
    ///     If a spec or date validation fails (including deferred fluent errors).
    #[pyo3(signature = (market=None), text_signature = "(self, market=None)")]
    fn build(
        &self,
        py: Python<'_>,
        market: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyCashFlowSchedule> {
        let market = crate::bindings::extract::extract_market_opt(py, market)?;
        let builder = self.inner.clone();
        py.detach(move || builder.build(market.as_ref()))
            .map(PyCashFlowSchedule::from_inner)
            .map_err(core_to_py)
    }

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        "CashFlowBuilder(...)".to_string()
    }
}
