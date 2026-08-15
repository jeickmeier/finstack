//! Python wrappers for capital-structure specs (waterfall + ECF sweep + PIK toggle).
//!
//! Mirrors `finstack_quant_statements::capital_structure::{WaterfallSpec, EcfSweepSpec,
//! PikToggleSpec, PaymentPriority}`. All classes support JSON round-trip via
//! `from_json`/`to_json` and structured keyword-argument construction.

use crate::errors::display_to_py;
use finstack_quant_statements::capital_structure::{
    EcfSweepSpec, PaymentPriority, PikToggleSpec, WaterfallSpec,
};
use pyo3::prelude::*;

fn parse_priority(s: &str) -> PyResult<PaymentPriority> {
    match s {
        "fees" => Ok(PaymentPriority::Fees),
        "interest" => Ok(PaymentPriority::Interest),
        "amortization" => Ok(PaymentPriority::Amortization),
        "mandatory_prepayment" => Ok(PaymentPriority::MandatoryPrepayment),
        "voluntary_prepayment" => Ok(PaymentPriority::VoluntaryPrepayment),
        "sweep" => Ok(PaymentPriority::Sweep),
        "equity" => Ok(PaymentPriority::Equity),
        other => Err(crate::errors::value_error(format!(
            "unknown payment priority {other:?}; expected one of: fees, interest, amortization, mandatory_prepayment, voluntary_prepayment, sweep, equity"
        ))),
    }
}

fn priority_to_str(p: PaymentPriority) -> &'static str {
    match p {
        PaymentPriority::Fees => "fees",
        PaymentPriority::Interest => "interest",
        PaymentPriority::Amortization => "amortization",
        PaymentPriority::MandatoryPrepayment => "mandatory_prepayment",
        PaymentPriority::VoluntaryPrepayment => "voluntary_prepayment",
        PaymentPriority::Sweep => "sweep",
        PaymentPriority::Equity => "equity",
    }
}

/// Excess Cash Flow (ECF) sweep specification.
///
/// Defines how to compute ECF and what fraction sweeps to debt paydown.
#[pyclass(
    name = "EcfSweepSpec",
    module = "finstack_quant.statements",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyEcfSweepSpec {
    pub(super) inner: EcfSweepSpec,
}

#[pymethods]
impl PyEcfSweepSpec {
    /// Construct an ECF sweep spec.
    ///
    /// Excess cash flow is built as
    /// ``ECF = EBITDA - taxes - capex - ΔWC - cash interest paid``, less any
    /// fees and scheduled principal that rank ahead of the prepayment
    /// priority. Every node reference is evaluated per period as a monetary
    /// amount in the model's reporting currency.
    ///
    /// Parameters
    /// ----------
    /// ebitda_node : str
    ///     Node reference or DSL formula for EBITDA.
    /// sweep_percentage : float
    ///     Fraction of ECF swept to debt paydown, as a **decimal fraction in
    ///     [0, 1]** (0.5 = 50%), not a percentage.
    /// taxes_node, capex_node, working_capital_node, cash_interest_node : str | None
    ///     Optional node references deducted from EBITDA to compute ECF. When
    ///     ``cash_interest_node`` is omitted the engine deducts contractual
    ///     cash interest from the period's debt-service magnitude instead.
    /// target_instrument_id : str | None
    ///     If set, sweep applies only to this debt instrument id; otherwise
    ///     it applies to all term loans.
    #[new]
    #[pyo3(
        signature = (
            ebitda_node,
            sweep_percentage,
            taxes_node=None,
            capex_node=None,
            working_capital_node=None,
            cash_interest_node=None,
            target_instrument_id=None,
        ),
        text_signature = "(ebitda_node, sweep_percentage, taxes_node=None, capex_node=None, working_capital_node=None, cash_interest_node=None, target_instrument_id=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn new(
        ebitda_node: String,
        sweep_percentage: f64,
        taxes_node: Option<String>,
        capex_node: Option<String>,
        working_capital_node: Option<String>,
        cash_interest_node: Option<String>,
        target_instrument_id: Option<String>,
    ) -> Self {
        Self {
            inner: EcfSweepSpec {
                ebitda_node,
                taxes_node,
                capex_node,
                working_capital_node,
                cash_interest_node,
                sweep_percentage,
                target_instrument_id,
            },
        }
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

    /// Deserialize from JSON.
    #[staticmethod]
    #[pyo3(text_signature = "(json, /)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: EcfSweepSpec = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize to JSON.
    #[pyo3(text_signature = "($self)")]
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Node reference or DSL formula supplying EBITDA, the ECF starting
    /// point.
    ///
    /// Returns
    /// -------
    /// str
    ///     A node id (``"ebitda"``) or an expression over nodes
    ///     (``"revenue - cogs - opex"``), evaluated per period as a monetary
    ///     amount.
    #[getter]
    fn ebitda_node(&self) -> &str {
        &self.inner.ebitda_node
    }

    /// Fraction of excess cash flow swept to debt paydown.
    ///
    /// Returns
    /// -------
    /// float
    ///     A **decimal fraction in [0, 1]**, not a percentage — 0.5 means a
    ///     50% sweep. Values outside the unit interval are rejected by
    ///     :meth:`WaterfallSpec.validate`.
    #[getter]
    fn sweep_percentage(&self) -> f64 {
        self.inner.sweep_percentage
    }

    /// Debt instrument the sweep pays down, if the sweep is targeted.
    ///
    /// Returns
    /// -------
    /// str | None
    ///     Instrument id, or ``None`` when the sweep applies to all term
    ///     loans in the capital structure.
    #[getter]
    fn target_instrument_id(&self) -> Option<&str> {
        self.inner.target_instrument_id.as_deref()
    }

    /// Return the debug representation with the EBITDA source, sweep
    /// fraction and target instrument.
    fn __repr__(&self) -> String {
        format!(
            "EcfSweepSpec(ebitda_node={:?}, sweep_percentage={}, target_instrument_id={:?})",
            self.inner.ebitda_node,
            self.inner.sweep_percentage,
            self.inner
                .target_instrument_id
                .as_deref()
                .unwrap_or("<all>")
        )
    }
}

/// PIK toggle specification.
///
/// Controls when interest accrues as PIK (added to principal) vs. cash, based
/// on a liquidity metric threshold with optional hysteresis.
#[pyclass(
    name = "PikToggleSpec",
    module = "finstack_quant.statements",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyPikToggleSpec {
    pub(super) inner: PikToggleSpec,
}

#[pymethods]
impl PyPikToggleSpec {
    /// Construct a PIK toggle spec.
    ///
    /// Parameters
    /// ----------
    /// liquidity_metric : str
    ///     Node reference or DSL formula for the liquidity signal (a balance
    ///     such as ``"cash_balance"`` or a ratio such as
    ///     ``"ebitda / interest_expense"``).
    /// threshold : float
    ///     PIK triggers when ``metric < threshold``. Expressed in the same
    ///     units as ``liquidity_metric`` — currency amount for a balance,
    ///     unitless for a ratio.
    /// target_instrument_ids : list[str] | None
    ///     If set, PIK toggles only these instruments; otherwise every
    ///     PIK-capable instrument. An explicitly empty list is rejected by
    ///     :meth:`WaterfallSpec.validate`.
    /// min_periods_in_pik : int
    ///     Hysteresis floor counted in **periods** on the model's own cadence
    ///     (not months): once triggered, PIK stays on for at least this many
    ///     periods. Default 0 lets PIK toggle every period.
    #[new]
    #[pyo3(
        signature = (
            liquidity_metric,
            threshold,
            target_instrument_ids=None,
            min_periods_in_pik=0,
        ),
        text_signature = "(liquidity_metric, threshold, target_instrument_ids=None, min_periods_in_pik=0)"
    )]
    fn new(
        liquidity_metric: String,
        threshold: f64,
        target_instrument_ids: Option<Vec<String>>,
        min_periods_in_pik: usize,
    ) -> Self {
        Self {
            inner: PikToggleSpec {
                liquidity_metric,
                threshold,
                target_instrument_ids,
                min_periods_in_pik,
            },
        }
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

    /// Deserialize a PIK toggle spec from JSON.
    #[staticmethod]
    #[pyo3(text_signature = "(json, /)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: PikToggleSpec = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize this PIK toggle spec to JSON.
    #[pyo3(text_signature = "($self)")]
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Node reference or DSL formula producing the liquidity signal.
    ///
    /// Returns
    /// -------
    /// str
    ///     A node id (``"cash_balance"``) or an expression
    ///     (``"ebitda / interest_expense"``). Whether the value is monetary
    ///     or a unitless ratio depends on the expression — the threshold must
    ///     use the same units.
    #[getter]
    fn liquidity_metric(&self) -> &str {
        &self.inner.liquidity_metric
    }

    /// Level below which interest accrues as PIK instead of cash.
    ///
    /// Returns
    /// -------
    /// float
    ///     Threshold in the **same units as the liquidity metric** — a
    ///     currency amount when the metric is a balance, a unitless ratio
    ///     when it is a coverage ratio. PIK triggers when
    ///     ``metric < threshold``.
    #[getter]
    fn threshold(&self) -> f64 {
        self.inner.threshold
    }

    /// Hysteresis floor: minimum time PIK stays on once triggered.
    ///
    /// Returns
    /// -------
    /// int
    ///     Count in **periods** on the model's own cadence (quarters for a
    ///     quarterly model), not months. ``0`` disables hysteresis, letting
    ///     PIK toggle every period.
    #[getter]
    fn min_periods_in_pik(&self) -> usize {
        self.inner.min_periods_in_pik
    }

    /// Return the debug representation with metric, threshold and hysteresis.
    fn __repr__(&self) -> String {
        format!(
            "PikToggleSpec(liquidity_metric={:?}, threshold={}, min_periods_in_pik={})",
            self.inner.liquidity_metric, self.inner.threshold, self.inner.min_periods_in_pik
        )
    }
}

/// Waterfall specification for dynamic cash flow allocation.
///
/// Configures payment priority, optional ECF sweep, and optional PIK toggle.
/// Call `validate()` before passing to a model builder to surface configuration
/// errors (e.g. `Sweep` ordered after `Equity` when a sweep is configured).
#[pyclass(
    name = "WaterfallSpec",
    module = "finstack_quant.statements",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyWaterfallSpec {
    pub(super) inner: WaterfallSpec,
}

#[pymethods]
impl PyWaterfallSpec {
    /// Construct a waterfall spec.
    ///
    /// Parameters
    /// ----------
    /// priority_of_payments : list[str] | None
    ///     Priority order (strings: fees, interest, amortization, mandatory_prepayment,
    ///     voluntary_prepayment, sweep, equity). Defaults to the standard
    ///     [fees, interest, amortization, sweep, equity] order.
    /// available_cash_node : str | None
    ///     Optional formula/node reference for cash available to allocate.
    /// ecf_sweep : EcfSweepSpec | None
    ///     Optional ECF sweep configuration.
    /// pik_toggle : PikToggleSpec | None
    ///     Optional PIK toggle configuration.
    #[new]
    #[pyo3(
        signature = (
            priority_of_payments=None,
            available_cash_node=None,
            ecf_sweep=None,
            pik_toggle=None,
        ),
        text_signature = "(priority_of_payments=None, available_cash_node=None, ecf_sweep=None, pik_toggle=None)"
    )]
    fn new(
        priority_of_payments: Option<Vec<String>>,
        available_cash_node: Option<String>,
        ecf_sweep: Option<&PyEcfSweepSpec>,
        pik_toggle: Option<&PyPikToggleSpec>,
    ) -> PyResult<Self> {
        let mut inner = WaterfallSpec {
            priority_of_payments:
                finstack_quant_statements::capital_structure::default_priority_of_payments(),
            available_cash_node: available_cash_node.clone().unwrap_or_default(),
            ecf_sweep: None,
            pik_toggle: None,
        };
        if let Some(priority) = priority_of_payments {
            inner.priority_of_payments = priority
                .into_iter()
                .map(|s| parse_priority(&s))
                .collect::<PyResult<Vec<_>>>()?;
        }
        inner.available_cash_node = available_cash_node.unwrap_or_default();
        inner.ecf_sweep = ecf_sweep.map(|p| p.inner.clone());
        inner.pik_toggle = pik_toggle.map(|p| p.inner.clone());
        Ok(Self { inner })
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

    /// Deserialize from JSON.
    #[staticmethod]
    #[pyo3(text_signature = "(json, /)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: WaterfallSpec = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize to JSON.
    #[pyo3(text_signature = "($self)")]
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Validate the spec against internal consistency rules.
    ///
    /// Raises `ValueError` if the configuration is economically inconsistent
    /// (e.g. `Sweep` ordered after `Equity` while a positive ECF sweep is set).
    #[pyo3(text_signature = "($self)")]
    fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(display_to_py)
    }

    /// Payment priority order, highest priority first.
    ///
    /// Returns
    /// -------
    /// list[str]
    ///     Snake-case priority names in allocation order — cash is applied to
    ///     the first entry before any of the next. Allocation *within* a
    ///     category is single-class pro-rata across instruments; there is no
    ///     tranche seniority, so a shortfall is shared proportionally.
    #[getter]
    fn priority_of_payments(&self) -> Vec<&'static str> {
        self.inner
            .priority_of_payments
            .iter()
            .copied()
            .map(priority_to_str)
            .collect()
    }

    /// Node reference or DSL formula for the cash pool the waterfall may
    /// spend.
    ///
    /// Returns
    /// -------
    /// str
    ///     A node id or expression evaluating to a monetary amount per
    ///     period. Required: without it the waterfall would report scheduled
    ///     cashflows as paid in full without capping them against available
    ///     cash.
    #[getter]
    fn available_cash_node(&self) -> &str {
        self.inner.available_cash_node.as_str()
    }

    /// Whether an excess-cash-flow sweep is configured.
    #[getter]
    fn has_ecf_sweep(&self) -> bool {
        self.inner.ecf_sweep.is_some()
    }

    /// Whether a PIK toggle is configured.
    #[getter]
    fn has_pik_toggle(&self) -> bool {
        self.inner.pik_toggle.is_some()
    }

    /// Return the debug representation with priority order and which optional
    /// mechanics are configured.
    fn __repr__(&self) -> String {
        format!(
            "WaterfallSpec(priority={:?}, ecf_sweep={}, pik_toggle={})",
            self.priority_of_payments(),
            self.inner.ecf_sweep.is_some(),
            self.inner.pik_toggle.is_some(),
        )
    }
}

/// Register capital-structure classes.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyEcfSweepSpec>()?;
    m.add_class::<PyPikToggleSpec>()?;
    m.add_class::<PyWaterfallSpec>()?;
    Ok(())
}
