//! Python wrappers for margin calculators (VM + IM result types).

use super::types::{PyCsaSpec, PyImMethodology};
use crate::bindings::pandas_utils::dict_to_dataframe;
use crate::errors::{core_to_py, display_to_py};
use finstack_quant_core::currency::Currency;
use finstack_quant_core::money::Money;
use finstack_quant_margin as fm;
use pyo3::prelude::*;
use pyo3::types::PyDict;

// VmResult

/// Variation margin calculation result.
#[pyclass(
    name = "VmResult",
    module = "finstack_quant.margin",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyVmResult {
    pub(super) inner: fm::VmResult,
}

#[pymethods]
impl PyVmResult {
    /// Gross mark-to-market exposure amount.
    #[getter]
    fn gross_exposure(&self) -> f64 {
        self.inner.gross_exposure.amount()
    }

    /// Net exposure after threshold and independent amount.
    #[getter]
    fn net_exposure(&self) -> f64 {
        self.inner.net_exposure.amount()
    }

    /// Delivery amount (positive = we post margin).
    #[getter]
    fn delivery_amount(&self) -> f64 {
        self.inner.delivery_amount.amount()
    }

    /// Return amount (positive = we receive margin back).
    #[getter]
    fn return_amount(&self) -> f64 {
        self.inner.return_amount.amount()
    }

    /// Net margin amount (delivery - return).
    #[getter]
    fn net_margin(&self) -> f64 {
        self.inner.net_margin().amount()
    }

    /// Whether a margin call is required.
    #[getter]
    fn requires_call(&self) -> bool {
        self.inner.requires_call()
    }

    /// Export the result as a single-row pandas ``DataFrame``.
    ///
    /// Columns: ``gross_exposure``, ``net_exposure``, ``delivery_amount``,
    /// ``return_amount``, ``net_margin``, ``requires_call``, ``currency``.
    ///
    /// All amount columns are floats in the single CSA currency reported by
    /// ``currency``; positive ``delivery_amount`` means we post margin and
    /// positive ``return_amount`` means we receive margin back.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let data = PyDict::new(py);
        data.set_item("gross_exposure", vec![self.inner.gross_exposure.amount()])?;
        data.set_item("net_exposure", vec![self.inner.net_exposure.amount()])?;
        data.set_item("delivery_amount", vec![self.inner.delivery_amount.amount()])?;
        data.set_item("return_amount", vec![self.inner.return_amount.amount()])?;
        data.set_item("net_margin", vec![self.inner.net_margin().amount()])?;
        data.set_item("requires_call", vec![self.inner.requires_call()])?;
        data.set_item(
            "currency",
            vec![self.inner.gross_exposure.currency().to_string()],
        )?;
        dict_to_dataframe(py, &data, None)
    }

    fn __repr__(&self) -> String {
        format!(
            "VmResult(delivery={:.2}, return={:.2}, requires_call={})",
            self.inner.delivery_amount.amount(),
            self.inner.return_amount.amount(),
            self.inner.requires_call()
        )
    }
}

// VmCalculator

/// Variation margin calculator following ISDA CSA rules.
#[pyclass(
    name = "VmCalculator",
    module = "finstack_quant.margin",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyVmCalculator {
    inner: fm::VmCalculator,
}

#[pymethods]
impl PyVmCalculator {
    /// Create a new VM calculator with the given CSA specification.
    #[new]
    fn new(csa: &PyCsaSpec) -> Self {
        Self {
            inner: fm::VmCalculator::new(csa.inner.clone()),
        }
    }

    /// Calculate variation margin.
    #[pyo3(signature = (exposure, posted_collateral, currency, year, month, day))]
    fn calculate(
        &self,
        exposure: f64,
        posted_collateral: f64,
        currency: &str,
        year: i32,
        month: u8,
        day: u8,
    ) -> PyResult<PyVmResult> {
        let ccy: finstack_quant_core::currency::Currency =
            currency.parse().map_err(display_to_py)?;
        let exp = finstack_quant_core::money::Money::try_new(exposure, ccy)
            .map_err(crate::errors::core_to_py)?;
        let posted = finstack_quant_core::money::Money::try_new(posted_collateral, ccy)
            .map_err(crate::errors::core_to_py)?;
        let m = time::Month::try_from(month)
            .map_err(|e| crate::errors::value_error(format!("invalid month: {e}")))?;
        let as_of = finstack_quant_core::dates::Date::from_calendar_date(year, m, day)
            .map_err(|e| crate::errors::value_error(format!("invalid date: {e}")))?;

        let result = self
            .inner
            .calculate(exp, posted, as_of)
            .map_err(core_to_py)?;
        Ok(PyVmResult { inner: result })
    }
}

// ImResult

/// Initial margin calculation result.
#[pyclass(
    name = "ImResult",
    module = "finstack_quant.margin",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyImResult {
    pub(super) inner: fm::ImResult,
}

impl PyImResult {
    pub(super) fn from_inner(inner: fm::ImResult) -> Self {
        Self { inner }
    }
}

pub(super) fn money_from_amount(amount: f64, currency: Currency) -> PyResult<Money> {
    Money::try_new(amount, currency).map_err(core_to_py)
}

#[pymethods]
impl PyImResult {
    /// Calculated initial margin amount.
    #[getter]
    fn amount(&self) -> f64 {
        self.inner.amount.amount()
    }

    /// Currency of the IM amount.
    #[getter]
    fn currency(&self) -> String {
        self.inner.amount.currency().to_string()
    }

    /// Methodology used for calculation.
    #[getter]
    fn methodology(&self) -> PyImMethodology {
        PyImMethodology {
            inner: self.inner.methodology,
        }
    }

    /// Margin Period of Risk (days).
    #[getter]
    fn mpor_days(&self) -> u32 {
        self.inner.mpor_days
    }

    /// Whether the amount is a conservative approximation (proxy) rather than
    /// an exact computation under the named methodology.
    #[getter]
    fn approximation(&self) -> bool {
        self.inner.approximation
    }

    /// Calculation date as an ISO 8601 string.
    #[getter]
    fn as_of(&self) -> String {
        self.inner.as_of.to_string()
    }

    /// Risk-class breakdown keys (if available).
    fn breakdown_keys(&self) -> Vec<String> {
        self.inner.breakdown.keys().cloned().collect()
    }

    /// Get breakdown amount for a risk class.
    fn breakdown_amount(&self, key: &str) -> Option<f64> {
        self.inner.breakdown.get(key).map(|m| m.amount())
    }

    /// Export the headline result as a single-row pandas ``DataFrame``.
    ///
    /// Columns: ``amount``, ``currency``, ``methodology``, ``mpor_days``,
    /// ``as_of``, ``approximation``.
    ///
    /// ``amount`` is a float in ``currency``; ``mpor_days`` is the margin
    /// period of risk in calendar days; ``as_of`` is an ISO 8601 date string.
    /// ``approximation`` is ``True`` when the amount is a conservative proxy
    /// rather than an exact computation under the named methodology — do not
    /// reconcile an approximated figure against an actual margin call.
    /// Per-risk-class detail lives in ``to_breakdown_dataframe``.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let data = PyDict::new(py);
        data.set_item("amount", vec![self.inner.amount.amount()])?;
        data.set_item("currency", vec![self.inner.amount.currency().to_string()])?;
        data.set_item("methodology", vec![self.inner.methodology.to_string()])?;
        data.set_item("mpor_days", vec![self.inner.mpor_days])?;
        data.set_item("as_of", vec![self.inner.as_of.to_string()])?;
        data.set_item("approximation", vec![self.inner.approximation])?;
        dict_to_dataframe(py, &data, None)
    }

    /// Export the per-risk-class breakdown as a pandas ``DataFrame``.
    ///
    /// Columns: ``risk_class``, ``amount``, ``currency``. One row per risk
    /// class (e.g. ``"interest_rate"``, ``"credit"``, ``"equity"``), sorted
    /// by ``risk_class`` so repeated runs are byte-identical; the underlying
    /// map is unordered. Methodologies that publish no breakdown yield a
    /// zero-row frame that still carries all three columns.
    ///
    /// Breakdown components do not generally sum to ``amount``: SIMM and
    /// other methodologies aggregate risk classes with correlations.
    fn to_breakdown_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let mut entries: Vec<(&String, &Money)> = self.inner.breakdown.iter().collect();
        entries.sort_by(|(left, _), (right, _)| left.cmp(right));

        let risk_classes: Vec<String> = entries.iter().map(|(key, _)| (*key).clone()).collect();
        let amounts: Vec<f64> = entries.iter().map(|(_, money)| money.amount()).collect();
        let currencies: Vec<String> = entries
            .iter()
            .map(|(_, money)| money.currency().to_string())
            .collect();

        let data = PyDict::new(py);
        data.set_item("risk_class", risk_classes)?;
        data.set_item("amount", amounts)?;
        data.set_item("currency", currencies)?;
        dict_to_dataframe(py, &data, None)
    }

    fn __repr__(&self) -> String {
        format!(
            "ImResult(amount={:.2}, methodology={}, mpor={}d, approximation={})",
            self.inner.amount.amount(),
            self.inner.methodology,
            self.inner.mpor_days,
            self.inner.approximation
        )
    }
}

/// Register calculator classes.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyVmResult>()?;
    m.add_class::<PyVmCalculator>()?;
    m.add_class::<PyImResult>()?;
    Ok(())
}
