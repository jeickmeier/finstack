//! Python wrappers for margin metric types.

use crate::bindings::pandas_utils::dict_to_dataframe;
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Margin utilization result (ratio of posted to required margin).
#[pyclass(
    name = "MarginUtilization",
    module = "finstack_quant.margin",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyMarginUtilization {
    inner: finstack_quant_margin::metrics::MarginUtilization,
}

#[pymethods]
impl PyMarginUtilization {
    /// Create a new margin utilization result.
    #[new]
    fn new(posted_amount: f64, required_amount: f64, currency: &str) -> PyResult<Self> {
        let ccy: finstack_quant_core::currency::Currency =
            currency.parse().map_err(crate::errors::display_to_py)?;
        let posted = finstack_quant_core::money::Money::try_new(posted_amount, ccy)
            .map_err(crate::errors::core_to_py)?;
        let required = finstack_quant_core::money::Money::try_new(required_amount, ccy)
            .map_err(crate::errors::core_to_py)?;
        Ok(Self {
            inner: finstack_quant_margin::metrics::MarginUtilization::new(posted, required),
        })
    }

    /// Posted margin amount.
    #[getter]
    fn posted(&self) -> f64 {
        self.inner.posted.amount()
    }

    /// Required margin amount.
    #[getter]
    fn required(&self) -> f64 {
        self.inner.required.amount()
    }

    /// Utilization ratio (posted / required).
    #[getter]
    fn ratio(&self) -> f64 {
        self.inner.ratio
    }

    /// Whether margin is adequate (ratio >= 1.0).
    fn is_adequate(&self) -> bool {
        self.inner.is_adequate()
    }

    /// Shortfall amount (if any).
    fn shortfall(&self) -> f64 {
        self.inner.shortfall().amount()
    }

    /// Export the result as a single-row pandas ``DataFrame``.
    ///
    /// Columns: ``posted``, ``required``, ``ratio``, ``shortfall``,
    /// ``is_adequate``, ``currency``.
    ///
    /// ``posted``, ``required`` and ``shortfall`` are floats in ``currency``.
    /// ``ratio`` is ``posted / required`` as a decimal fraction (``1.0`` =
    /// fully covered); it is ``inf`` when nothing is required but margin is
    /// posted, and ``1.0`` when neither is.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let data = PyDict::new(py);
        data.set_item("posted", vec![self.inner.posted.amount()])?;
        data.set_item("required", vec![self.inner.required.amount()])?;
        data.set_item("ratio", vec![self.inner.ratio])?;
        data.set_item("shortfall", vec![self.inner.shortfall().amount()])?;
        data.set_item("is_adequate", vec![self.inner.is_adequate()])?;
        data.set_item("currency", vec![self.inner.posted.currency().to_string()])?;
        dict_to_dataframe(py, &data, None)
    }

    fn __repr__(&self) -> String {
        format!(
            "MarginUtilization(ratio={:.2}, adequate={})",
            self.inner.ratio,
            self.inner.is_adequate()
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

/// Excess collateral result.
#[pyclass(
    name = "ExcessCollateral",
    module = "finstack_quant.margin",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyExcessCollateral {
    inner: finstack_quant_margin::metrics::ExcessCollateral,
}

#[pymethods]
impl PyExcessCollateral {
    /// Create a new excess collateral result.
    #[new]
    fn new(collateral_value: f64, required_value: f64, currency: &str) -> PyResult<Self> {
        let ccy: finstack_quant_core::currency::Currency =
            currency.parse().map_err(crate::errors::display_to_py)?;
        let collateral = finstack_quant_core::money::Money::try_new(collateral_value, ccy)
            .map_err(crate::errors::core_to_py)?;
        let required = finstack_quant_core::money::Money::try_new(required_value, ccy)
            .map_err(crate::errors::core_to_py)?;
        Ok(Self {
            inner: finstack_quant_margin::metrics::ExcessCollateral::new(collateral, required),
        })
    }

    /// Collateral value.
    #[getter]
    fn collateral_value(&self) -> f64 {
        self.inner.collateral_value.amount()
    }

    /// Required value.
    #[getter]
    fn required_value(&self) -> f64 {
        self.inner.required_value.amount()
    }

    /// Excess amount (positive) or shortfall (negative).
    #[getter]
    fn excess(&self) -> f64 {
        self.inner.excess.amount()
    }

    /// Whether there is excess collateral.
    fn has_excess(&self) -> bool {
        self.inner.has_excess()
    }

    /// Whether there is a shortfall.
    fn has_shortfall(&self) -> bool {
        self.inner.has_shortfall()
    }

    /// Excess as a percentage of required.
    fn excess_percentage(&self) -> f64 {
        self.inner.excess_percentage()
    }

    /// Export the result as a single-row pandas ``DataFrame``.
    ///
    /// Columns: ``collateral_value``, ``required_value``, ``excess``,
    /// ``excess_percentage``, ``has_excess``, ``has_shortfall``,
    /// ``currency``.
    ///
    /// The three amount columns are floats in ``currency``; ``excess`` is
    /// ``collateral_value - required_value`` and is negative on a shortfall.
    /// ``excess_percentage`` is a decimal fraction of ``required_value``
    /// (``0.1`` = 10% over-collateralised).
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let data = PyDict::new(py);
        data.set_item(
            "collateral_value",
            vec![self.inner.collateral_value.amount()],
        )?;
        data.set_item("required_value", vec![self.inner.required_value.amount()])?;
        data.set_item("excess", vec![self.inner.excess.amount()])?;
        data.set_item("excess_percentage", vec![self.inner.excess_percentage()])?;
        data.set_item("has_excess", vec![self.inner.has_excess()])?;
        data.set_item("has_shortfall", vec![self.inner.has_shortfall()])?;
        data.set_item(
            "currency",
            vec![self.inner.collateral_value.currency().to_string()],
        )?;
        dict_to_dataframe(py, &data, None)
    }

    fn __repr__(&self) -> String {
        format!(
            "ExcessCollateral(excess={:.2}, pct={:.2}%)",
            self.inner.excess.amount(),
            self.inner.excess_percentage() * 100.0
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

/// Margin funding cost result.
#[pyclass(
    name = "MarginFundingCost",
    module = "finstack_quant.margin",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyMarginFundingCost {
    inner: finstack_quant_margin::metrics::MarginFundingCost,
}

#[pymethods]
impl PyMarginFundingCost {
    /// Calculate margin funding cost.
    #[new]
    fn new(
        margin_posted: f64,
        funding_rate: f64,
        collateral_rate: f64,
        currency: &str,
    ) -> PyResult<Self> {
        let ccy: finstack_quant_core::currency::Currency =
            currency.parse().map_err(crate::errors::display_to_py)?;
        let margin = finstack_quant_core::money::Money::try_new(margin_posted, ccy)
            .map_err(crate::errors::core_to_py)?;
        Ok(Self {
            inner: finstack_quant_margin::metrics::MarginFundingCost::calculate(
                margin,
                funding_rate,
                collateral_rate,
            ),
        })
    }

    /// Posted margin amount.
    #[getter]
    fn margin_posted(&self) -> f64 {
        self.inner.margin_posted.amount()
    }

    /// Funding rate (annualized).
    #[getter]
    fn funding_rate(&self) -> f64 {
        self.inner.funding_rate
    }

    /// Collateral return rate.
    #[getter]
    fn collateral_rate(&self) -> f64 {
        self.inner.collateral_rate
    }

    /// Annualized funding cost.
    #[getter]
    fn annual_cost(&self) -> f64 {
        self.inner.annual_cost.amount()
    }

    /// Funding spread (funding rate - collateral rate).
    fn spread(&self) -> f64 {
        self.inner.spread()
    }

    /// Cost for a specific period.
    fn cost_for_period(&self, year_fraction: f64) -> f64 {
        self.inner.cost_for_period(year_fraction).amount()
    }

    /// Export the result as a single-row pandas ``DataFrame``.
    ///
    /// Columns: ``margin_posted``, ``funding_rate``, ``collateral_rate``,
    /// ``spread``, ``annual_cost``, ``currency``.
    ///
    /// ``margin_posted`` and ``annual_cost`` are floats in ``currency``; the
    /// three rate columns are annualized decimal fractions (``0.03`` = 3%).
    /// ``annual_cost`` is ``margin_posted * spread``, so a collateral rate
    /// above the funding rate makes it negative (a funding benefit).
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let data = PyDict::new(py);
        data.set_item("margin_posted", vec![self.inner.margin_posted.amount()])?;
        data.set_item("funding_rate", vec![self.inner.funding_rate])?;
        data.set_item("collateral_rate", vec![self.inner.collateral_rate])?;
        data.set_item("spread", vec![self.inner.spread()])?;
        data.set_item("annual_cost", vec![self.inner.annual_cost.amount()])?;
        data.set_item(
            "currency",
            vec![self.inner.margin_posted.currency().to_string()],
        )?;
        dict_to_dataframe(py, &data, None)
    }

    fn __repr__(&self) -> String {
        format!(
            "MarginFundingCost(margin={:.2}, spread={:.4}, annual_cost={:.2})",
            self.inner.margin_posted.amount(),
            self.inner.spread(),
            self.inner.annual_cost.amount()
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

/// Haircut sensitivity: PV change for +1bp haircut change.
#[pyclass(
    name = "Haircut01",
    module = "finstack_quant.margin",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyHaircut01 {
    inner: finstack_quant_margin::metrics::Haircut01,
}

#[pymethods]
impl PyHaircut01 {
    /// Calculate Haircut01.
    #[new]
    fn new(collateral_value: f64, current_haircut: f64, currency: &str) -> PyResult<Self> {
        let ccy: finstack_quant_core::currency::Currency =
            currency.parse().map_err(crate::errors::display_to_py)?;
        let collateral = finstack_quant_core::money::Money::try_new(collateral_value, ccy)
            .map_err(crate::errors::core_to_py)?;
        Ok(Self {
            inner: finstack_quant_margin::metrics::Haircut01::calculate(
                collateral,
                current_haircut,
            ),
        })
    }

    /// Collateral value.
    #[getter]
    fn collateral_value(&self) -> f64 {
        self.inner.collateral_value.amount()
    }

    /// Current haircut (decimal).
    #[getter]
    fn current_haircut(&self) -> f64 {
        self.inner.current_haircut
    }

    /// PV change for +1bp haircut.
    #[getter]
    fn pv_change(&self) -> f64 {
        self.inner.pv_change.amount()
    }

    /// Current haircut in basis points.
    fn haircut_bp(&self) -> f64 {
        self.inner.haircut_bp()
    }

    fn __repr__(&self) -> String {
        format!(
            "Haircut01(collateral={:.2}, haircut={:.4}, pv_change={:.2})",
            self.inner.collateral_value.amount(),
            self.inner.current_haircut,
            self.inner.pv_change.amount()
        )
    }
}

/// Register metric classes.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyMarginUtilization>()?;
    m.add_class::<PyExcessCollateral>()?;
    m.add_class::<PyMarginFundingCost>()?;
    m.add_class::<PyHaircut01>()?;
    Ok(())
}
