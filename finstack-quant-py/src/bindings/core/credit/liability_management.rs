//! Python bindings for `finstack_quant_core::credit::liability_management`.

use crate::bindings::pandas_utils::serde_object_to_single_row_dataframe;
use crate::errors::core_to_py;
use finstack_quant_core::credit::liability_management::{
    self as lm, ExchangeOfferAnalysis, ExchangeType, LeverageImpact, LmeAnalysis, LmeType,
};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};

/// Hold-versus-tender economics of a distressed exchange offer.
#[pyclass(
    name = "ExchangeOfferAnalysis",
    module = "finstack_quant.core.credit.liability_management",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyExchangeOfferAnalysis {
    inner: ExchangeOfferAnalysis,
}

#[pymethods]
impl PyExchangeOfferAnalysis {
    #[getter]
    fn exchange_type(&self) -> String {
        self.inner.exchange_type.as_str().to_string()
    }

    #[getter]
    fn old_npv(&self) -> f64 {
        self.inner.old_npv
    }

    #[getter]
    fn new_npv(&self) -> f64 {
        self.inner.new_npv
    }

    #[getter]
    fn consent_fee(&self) -> f64 {
        self.inner.consent_fee
    }

    #[getter]
    fn equity_sweetener_value(&self) -> f64 {
        self.inner.equity_sweetener_value
    }

    #[getter]
    fn tender_total(&self) -> f64 {
        self.inner.tender_total
    }

    #[getter]
    fn delta_npv(&self) -> f64 {
        self.inner.delta_npv
    }

    #[getter]
    fn breakeven_recovery(&self) -> f64 {
        self.inner.breakeven_recovery
    }

    #[getter]
    fn tender_recommended(&self) -> bool {
        self.inner.tender_recommended
    }

    /// Export as a single-row pandas ``DataFrame``.
    ///
    /// Columns: ``exchange_type``, ``old_npv``, ``new_npv``, ``consent_fee``,
    /// ``equity_sweetener_value``, ``tender_total``, ``delta_npv``,
    /// ``breakeven_recovery``, ``tender_recommended``.
    ///
    /// One offer is one flat record, so a one-row frame is the right shape:
    /// ``pd.concat`` over several candidate offers gives a hold-versus-tender
    /// comparison table directly.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        // Built explicitly rather than serializing `inner` so `exchange_type`
        // lands as its canonical string label instead of the serde enum
        // representation.
        let row = serde_json::json!({
            "exchange_type": self.inner.exchange_type.as_str(),
            "old_npv": self.inner.old_npv,
            "new_npv": self.inner.new_npv,
            "consent_fee": self.inner.consent_fee,
            "equity_sweetener_value": self.inner.equity_sweetener_value,
            "tender_total": self.inner.tender_total,
            "delta_npv": self.inner.delta_npv,
            "breakeven_recovery": self.inner.breakeven_recovery,
            "tender_recommended": self.inner.tender_recommended,
        });
        serde_object_to_single_row_dataframe(py, &row)
    }

    fn __repr__(&self) -> String {
        format!(
            "ExchangeOfferAnalysis(exchange_type='{}', tender_total={}, delta_npv={}, \
             tender_recommended={})",
            self.inner.exchange_type,
            self.inner.tender_total,
            self.inner.delta_npv,
            self.inner.tender_recommended
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

/// Gross-leverage impact of a liability management exercise.
#[pyclass(
    name = "LeverageImpact",
    module = "finstack_quant.core.credit.liability_management",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyLeverageImpact {
    inner: LeverageImpact,
}

#[pymethods]
impl PyLeverageImpact {
    #[getter]
    fn pre_total_debt(&self) -> f64 {
        self.inner.pre_total_debt
    }

    #[getter]
    fn post_total_debt(&self) -> f64 {
        self.inner.post_total_debt
    }

    #[getter]
    fn pre_leverage(&self) -> f64 {
        self.inner.pre_leverage
    }

    #[getter]
    fn post_leverage(&self) -> f64 {
        self.inner.post_leverage
    }

    #[getter]
    fn leverage_reduction(&self) -> f64 {
        self.inner.leverage_reduction
    }

    fn __repr__(&self) -> String {
        format!(
            "LeverageImpact(pre_leverage={}, post_leverage={}, leverage_reduction={})",
            self.inner.pre_leverage, self.inner.post_leverage, self.inner.leverage_reduction
        )
    }
}

/// Issuer-side economics of a liability management exercise.
#[pyclass(
    name = "LmeAnalysis",
    module = "finstack_quant.core.credit.liability_management",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyLmeAnalysis {
    inner: LmeAnalysis,
}

#[pymethods]
impl PyLmeAnalysis {
    #[getter]
    fn lme_type(&self) -> String {
        self.inner.lme_type.as_str().to_string()
    }

    #[getter]
    fn cost(&self) -> f64 {
        self.inner.cost
    }

    #[getter]
    fn notional_reduction(&self) -> f64 {
        self.inner.notional_reduction
    }

    #[getter]
    fn discount_capture(&self) -> f64 {
        self.inner.discount_capture
    }

    #[getter]
    fn discount_capture_pct(&self) -> f64 {
        self.inner.discount_capture_pct
    }

    #[getter]
    fn remaining_holder_impact_pct(&self) -> f64 {
        self.inner.remaining_holder_impact_pct
    }

    #[getter]
    fn leverage_impact(&self) -> Option<PyLeverageImpact> {
        self.inner
            .leverage_impact
            .clone()
            .map(|inner| PyLeverageImpact { inner })
    }

    /// Export as a single-row pandas ``DataFrame``.
    ///
    /// Columns: ``lme_type``, ``cost``, ``notional_reduction``,
    /// ``discount_capture``, ``discount_capture_pct``,
    /// ``remaining_holder_impact_pct``, ``pre_total_debt``,
    /// ``post_total_debt``, ``pre_leverage``, ``post_leverage``,
    /// ``leverage_reduction``.
    ///
    /// One exercise is one flat record, so a one-row frame is the right shape:
    /// ``pd.concat`` over several structures gives a discount-capture
    /// comparison table directly.
    ///
    /// The five leverage columns come from :attr:`leverage_impact` and are
    /// flattened onto the same row rather than nested. They are ``None`` (and
    /// therefore ``object`` dtype) when no positive EBITDA was supplied;
    /// coerce with ``pd.to_numeric`` before aggregating a mixed set.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        // `LmeAnalysis` derives Serialize, but `leverage_impact` is a nested
        // struct that blind serde would drop into a single object-dtype cell,
        // so the row is flattened by hand.
        let leverage = self.inner.leverage_impact.as_ref();
        let row = serde_json::json!({
            "lme_type": self.inner.lme_type.as_str(),
            "cost": self.inner.cost,
            "notional_reduction": self.inner.notional_reduction,
            "discount_capture": self.inner.discount_capture,
            "discount_capture_pct": self.inner.discount_capture_pct,
            "remaining_holder_impact_pct": self.inner.remaining_holder_impact_pct,
            "pre_total_debt": leverage.map(|l| l.pre_total_debt),
            "post_total_debt": leverage.map(|l| l.post_total_debt),
            "pre_leverage": leverage.map(|l| l.pre_leverage),
            "post_leverage": leverage.map(|l| l.post_leverage),
            "leverage_reduction": leverage.map(|l| l.leverage_reduction),
        });
        serde_object_to_single_row_dataframe(py, &row)
    }

    fn __repr__(&self) -> String {
        format!(
            "LmeAnalysis(lme_type='{}', cost={}, notional_reduction={}, discount_capture={})",
            self.inner.lme_type,
            self.inner.cost,
            self.inner.notional_reduction,
            self.inner.discount_capture
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

/// Compare hold-versus-tender economics for a distressed exchange offer.
#[pyfunction]
#[pyo3(signature = (old_pv, new_pv, consent_fee=0.0, equity_sweetener_value=0.0, exchange_type="par_for_par"))]
#[pyo3(
    text_signature = "(old_pv, new_pv, consent_fee=0.0, equity_sweetener_value=0.0, exchange_type='par_for_par')"
)]
fn analyze_exchange_offer(
    old_pv: f64,
    new_pv: f64,
    consent_fee: f64,
    equity_sweetener_value: f64,
    exchange_type: &str,
) -> PyResult<PyExchangeOfferAnalysis> {
    let exchange_type: ExchangeType = exchange_type.parse().map_err(core_to_py)?;
    lm::analyze_exchange_offer(
        old_pv,
        new_pv,
        consent_fee,
        equity_sweetener_value,
        exchange_type,
    )
    .map(|inner| PyExchangeOfferAnalysis { inner })
    .map_err(core_to_py)
}

/// Compute discount capture and leverage impact for an LME transaction.
#[pyfunction]
#[pyo3(signature = (lme_type, notional, repurchase_price_pct, opt_acceptance_pct=1.0, ebitda=None))]
#[pyo3(
    text_signature = "(lme_type, notional, repurchase_price_pct, opt_acceptance_pct=1.0, ebitda=None)"
)]
fn analyze_lme(
    lme_type: &str,
    notional: f64,
    repurchase_price_pct: f64,
    opt_acceptance_pct: f64,
    ebitda: Option<f64>,
) -> PyResult<PyLmeAnalysis> {
    let lme_type: LmeType = lme_type.parse().map_err(core_to_py)?;
    lm::analyze_lme(
        lme_type,
        notional,
        repurchase_price_pct,
        opt_acceptance_pct,
        ebitda,
    )
    .map(|inner| PyLmeAnalysis { inner })
    .map_err(core_to_py)
}

/// Build the `finstack_quant.core.credit.liability_management` submodule.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "liability_management")?;
    m.setattr(
        "__doc__",
        "Distressed-exchange hold-versus-tender economics and issuer LME analytics.",
    )?;

    m.add_class::<PyExchangeOfferAnalysis>()?;
    m.add_class::<PyLeverageImpact>()?;
    m.add_class::<PyLmeAnalysis>()?;
    m.add_function(wrap_pyfunction!(analyze_exchange_offer, &m)?)?;
    m.add_function(wrap_pyfunction!(analyze_lme, &m)?)?;

    let all = PyList::new(
        py,
        [
            "ExchangeOfferAnalysis",
            "LeverageImpact",
            "LmeAnalysis",
            "analyze_exchange_offer",
            "analyze_lme",
        ],
    )?;
    m.setattr("__all__", all)?;
    crate::bindings::module_utils::register_submodule(
        py,
        parent,
        &m,
        "liability_management",
        "finstack_quant.core.credit",
        crate::bindings::module_utils::ParentNameSource::Package,
    )?;
    Ok(())
}
