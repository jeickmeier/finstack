//! Python bindings for `finstack_quant_core::money::fx` FX matrix and conversion policy.

use std::sync::Arc;

use finstack_quant_core::money::fx::{
    FxConversionPolicy, FxMatrix, FxQuery, FxRateResult, SimpleFxProvider,
};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};

use crate::bindings::core::currency::extract_currency;
use crate::bindings::core::dates::utils::py_to_date;
use crate::bindings::pandas_utils::serde_object_to_single_row_dataframe_with_schema;
use crate::errors::core_to_py;

// Helpers

/// Parse an [`FxConversionPolicy`] from a string.
fn parse_fx_policy(s: &str) -> PyResult<FxConversionPolicy> {
    s.parse::<FxConversionPolicy>()
        .map_err(|e| crate::errors::value_error(format!("Invalid FxConversionPolicy {s:?}: {e}")))
}

fn extract_fx_policy(value: &Bound<'_, PyAny>) -> PyResult<FxConversionPolicy> {
    if let Ok(wrapper) = value.extract::<PyRef<'_, PyFxConversionPolicy>>() {
        Ok(wrapper.inner)
    } else if let Ok(s) = value.extract::<String>() {
        parse_fx_policy(&s)
    } else {
        Err(crate::errors::value_error(
            "policy must be FxConversionPolicy or str",
        ))
    }
}

// PyFxConversionPolicy

/// FX conversion policy enum.
///
/// Wraps [`FxConversionPolicy`] from `finstack-quant-core`.
#[pyclass(
    name = "FxConversionPolicy",
    module = "finstack_quant.core.market_data.fx",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, Debug)]
pub struct PyFxConversionPolicy {
    /// Inner Rust policy.
    pub(crate) inner: FxConversionPolicy,
}

impl PyFxConversionPolicy {
    /// Build from inner.
    pub(crate) fn from_inner(inner: FxConversionPolicy) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFxConversionPolicy {
    /// Use spot/forward on the cashflow date.
    #[classattr]
    const CASHFLOW_DATE: PyFxConversionPolicy = PyFxConversionPolicy {
        inner: FxConversionPolicy::CashflowDate,
    };
    /// Use period end date.
    #[classattr]
    const PERIOD_END: PyFxConversionPolicy = PyFxConversionPolicy {
        inner: FxConversionPolicy::PeriodEnd,
    };
    /// Use an average over the period.
    #[classattr]
    const PERIOD_AVERAGE: PyFxConversionPolicy = PyFxConversionPolicy {
        inner: FxConversionPolicy::PeriodAverage,
    };
    /// Custom strategy defined by the caller.
    #[classattr]
    const CUSTOM: PyFxConversionPolicy = PyFxConversionPolicy {
        inner: FxConversionPolicy::Custom,
    };

    /// Parse from a string label (e.g. ``"cashflow_date"``).
    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    fn from_name(_cls: &Bound<'_, pyo3::types::PyType>, name: &str) -> PyResult<Self> {
        parse_fx_policy(name).map(Self::from_inner)
    }

    fn __repr__(&self) -> String {
        format!("FxConversionPolicy({})", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

// PyFxRateResult

/// Result of an FX rate query.
///
/// Wraps [`FxRateResult`] from `finstack-quant-core`.
#[pyclass(
    name = "FxRateResult",
    module = "finstack_quant.core.market_data.fx",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFxRateResult {
    /// Inner Rust result.
    inner: FxRateResult,
}

#[pymethods]
impl PyFxRateResult {
    /// The FX conversion rate.
    #[getter]
    fn rate(&self) -> f64 {
        self.inner.rate
    }

    /// Whether the rate was obtained via triangulation.
    #[getter]
    fn triangulated(&self) -> bool {
        self.inner.triangulated
    }

    /// Export as a single-row pandas ``DataFrame``.
    ///
    /// Columns: ``rate``, ``triangulated``.
    ///
    /// One lookup is one flat record, so a one-row frame is the right shape:
    /// ``pd.concat([matrix.rate(*pair, d).to_dataframe() for pair in pairs])``
    /// builds a fixing table, and the ``triangulated`` flag travels with each
    /// rate so a downstream check can refuse derived quotes.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let row = serde_json::json!({
            "rate": self.inner.rate,
            "triangulated": self.inner.triangulated,
        });
        serde_object_to_single_row_dataframe_with_schema(py, &row, &["rate", "triangulated"])
    }

    fn __repr__(&self) -> String {
        format!(
            "FxRateResult(rate={}, triangulated={})",
            self.inner.rate, self.inner.triangulated,
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

// PyFxMatrix

/// Foreign-exchange rate matrix for currency conversion.
///
/// Wraps [`FxMatrix`] from `finstack-quant-core`. Quote mutations go through
/// `FxMatrix::set_quote` (interior mutability) so that matrices obtained
/// from a `MarketContext` share state with the underlying context.
#[pyclass(
    name = "FxMatrix",
    module = "finstack_quant.core.market_data.fx",
    skip_from_py_object
)]
pub struct PyFxMatrix {
    /// The matrix used for rate lookups (includes triangulation / caching).
    pub(crate) inner: Arc<FxMatrix>,
}

#[pymethods]
impl PyFxMatrix {
    /// Create an empty FX matrix.
    #[new]
    fn new() -> Self {
        let provider = Arc::new(SimpleFxProvider::new());
        let matrix = FxMatrix::new(provider);
        Self {
            inner: Arc::new(matrix),
        }
    }

    /// Set an explicit FX quote.
    ///
    /// Parameters
    /// ----------
    /// base : Currency | str
    ///     Base (from) currency.
    /// quote : Currency | str
    ///     Quote (to) currency.
    /// rate : float
    ///     The conversion rate ``1 base = rate quote``.
    #[pyo3(text_signature = "(self, base, quote, rate)")]
    fn set_quote(
        &self,
        base: &Bound<'_, PyAny>,
        quote: &Bound<'_, PyAny>,
        rate: f64,
    ) -> PyResult<()> {
        let base_currency = extract_currency(base)?;
        let quote_currency = extract_currency(quote)?;
        self.inner
            .set_quote(base_currency, quote_currency, rate)
            .map_err(core_to_py)?;
        Ok(())
    }

    /// Set an authoritative FX quote scoped to one date and policy.
    #[pyo3(text_signature = "(self, base, quote, date, policy, rate)")]
    fn set_quote_on(
        &self,
        base: &Bound<'_, PyAny>,
        quote: &Bound<'_, PyAny>,
        date: &Bound<'_, PyAny>,
        policy: &Bound<'_, PyAny>,
        rate: f64,
    ) -> PyResult<()> {
        let base_currency = extract_currency(base)?;
        let quote_currency = extract_currency(quote)?;
        let date = py_to_date(date)?;
        let policy = extract_fx_policy(policy)?;
        self.inner
            .set_quote_on(base_currency, quote_currency, date, policy, rate)
            .map_err(core_to_py)
    }

    /// Look up an FX rate.
    ///
    /// Parameters
    /// ----------
    /// base : Currency | str
    ///     Base (from) currency.
    /// quote : Currency | str
    ///     Quote (to) currency.
    /// date : datetime.date
    ///     Applicable date for the rate.
    /// policy : str, optional
    ///     Conversion policy (default ``"cashflow_date"``).
    ///
    /// Returns
    /// -------
    /// FxRateResult
    #[pyo3(signature = (base, quote, date, policy=None))]
    #[pyo3(text_signature = "(self, base, quote, date, policy=None)")]
    fn rate(
        &self,
        base: &Bound<'_, PyAny>,
        quote: &Bound<'_, PyAny>,
        date: &Bound<'_, PyAny>,
        policy: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyFxRateResult> {
        let base_currency = extract_currency(base)?;
        let quote_currency = extract_currency(quote)?;
        let d = py_to_date(date)?;
        let pol = match policy {
            Some(p) => extract_fx_policy(p)?,
            None => FxConversionPolicy::CashflowDate,
        };

        let query = FxQuery::with_policy(base_currency, quote_currency, d, pol);

        let result = self.inner.rate(query).map_err(core_to_py)?;
        Ok(PyFxRateResult { inner: result })
    }

    fn __repr__(&self) -> String {
        "FxMatrix(...)".to_string()
    }
}

// Module registration

pub(super) const EXPORTS: &[&str] = &["FxConversionPolicy", "FxMatrix", "FxRateResult"];

/// Register the `finstack_quant.core.market_data.fx` submodule.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "fx")?;
    m.setattr(
        "__doc__",
        "FX rate matrix and conversion policy bindings (finstack-quant-core).",
    )?;

    m.add_class::<PyFxConversionPolicy>()?;
    m.add_class::<PyFxRateResult>()?;
    m.add_class::<PyFxMatrix>()?;

    let all = PyList::new(py, EXPORTS)?;
    m.setattr("__all__", all)?;

    crate::bindings::module_utils::register_submodule(
        py,
        parent,
        &m,
        "fx",
        "finstack_quant.core.market_data",
        crate::bindings::module_utils::ParentNameSource::Package,
    )?;

    Ok(())
}
