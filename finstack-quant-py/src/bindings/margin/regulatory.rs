//! Python bindings for FRTB SBA and SA-CCR regulatory capital frameworks.
//!
//! Exposes a deliberately simplified surface: callers build sensitivity or
//! trade containers with ergonomic `add_*` methods, then invoke a free function
//! that returns the headline capital number plus a per-component breakdown.
//! Full typed access to every enum variant is intentionally omitted; where
//! complex configuration is needed, JSON round-tripping is used.

use super::sensitivity_frame::SensitivityRows;
use crate::bindings::module_utils::{parse_currency, parse_date};
use crate::bindings::pandas_utils::{
    serde_object_to_single_row_dataframe_with_schema, serde_rows_to_dataframe_with_schema,
    ColumnSchema,
};
use crate::errors::{core_to_py, display_to_py};
use finstack_quant_core::currency::Currency;
use finstack_quant_margin::regulatory::{
    frtb::{
        CorrelationScenario, FrtbRiskClass, FrtbSbaEngine, FrtbSbaResult, FrtbSensitivities,
        RraoPosition,
    },
    sa_ccr::{EadResult, SaCcrAssetClass, SaCcrEngine, SaCcrNettingSetConfig, SaCcrTrade},
};
use finstack_quant_margin::NettingSetId;
use pyo3::prelude::*;
use std::collections::BTreeMap;

fn parse_correlation_scenario(s: &str) -> PyResult<CorrelationScenario> {
    match s {
        "low" => Ok(CorrelationScenario::Low),
        "medium" => Ok(CorrelationScenario::Medium),
        "high" => Ok(CorrelationScenario::High),
        _ => Err(crate::errors::value_error(format!(
            "unknown FRTB correlation scenario '{s}' (expected 'low', 'medium', or 'high')"
        ))),
    }
}

/// Render a value as its own canonical serde wire label.
///
/// Reading the serde representation rather than re-listing variants here means
/// a new variant on a `#[non_exhaustive]` enum cannot silently collapse into a
/// shared `"unknown"` key — which, in a breakdown map, would drop a capital
/// charge as soon as two new variants collided.
fn serde_label<T: serde::Serialize + std::fmt::Debug>(value: T) -> String {
    match serde_json::to_value(&value) {
        Ok(serde_json::Value::String(label)) => label,
        _ => format!("{value:?}"),
    }
}

fn risk_class_label(rc: FrtbRiskClass) -> String {
    serde_label(rc)
}

fn scenario_label(scenario: CorrelationScenario) -> String {
    serde_label(scenario)
}

/// Render an optional bucket index as the string form used by the long-format
/// sensitivity frames.
fn bucket_label(bucket: u8) -> Option<String> {
    Some(bucket.to_string())
}

/// Render a currency pair as a single `issuer` value (e.g. `"EUR/USD"`).
fn pair_label(ccy1: Currency, ccy2: Currency) -> Option<String> {
    Some(format!("{ccy1}/{ccy2}"))
}

fn asset_class_label(ac: SaCcrAssetClass) -> String {
    serde_label(ac)
}

/// FRTB sensitivity portfolio for the Sensitivity-Based Approach.
///
/// Build up delta/vega/curvature inputs with the ``add_*`` methods, then pass
/// to :func:`frtb_sba_charge`. JSON round-tripping is available for advanced
/// use cases (e.g. loading a full portfolio produced by an upstream tool).
#[pyclass(
    name = "FrtbSensitivities",
    module = "finstack_quant.margin",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFrtbSensitivities {
    pub(super) inner: FrtbSensitivities,
}

#[pymethods]
impl PyFrtbSensitivities {
    /// Create an empty sensitivity container.
    ///
    /// ``base_currency`` is the reporting currency (e.g. ``"USD"``).
    #[new]
    #[pyo3(signature = (base_currency = "USD"))]
    fn new(base_currency: &str) -> PyResult<Self> {
        let ccy = parse_currency(base_currency)?;
        Ok(Self {
            inner: FrtbSensitivities::new(ccy),
        })
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

    /// Construct from a JSON serialization of `FrtbSensitivities`.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: FrtbSensitivities = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Add a GIRR delta sensitivity (currency P&L per 1 percentage-point move).
    #[pyo3(signature = (tenor, amount, currency = None))]
    fn add_girr_delta(&mut self, tenor: &str, amount: f64, currency: Option<&str>) -> PyResult<()> {
        let ccy = self.currency_or_base(currency)?;
        self.inner.add_girr_delta(ccy, tenor, amount);
        Ok(())
    }

    /// Add a CSR (non-securitization) delta sensitivity.
    #[pyo3(signature = (issuer, bucket, tenor, amount))]
    fn add_csr_delta(&mut self, issuer: &str, bucket: u8, tenor: &str, amount: f64) {
        self.inner
            .add_csr_nonsec_delta(issuer, bucket, tenor, amount);
    }

    /// Add an equity delta sensitivity.
    #[pyo3(signature = (underlier, bucket, amount))]
    fn add_equity_delta(&mut self, underlier: &str, bucket: u8, amount: f64) {
        self.inner.add_equity_delta(underlier, bucket, amount);
    }

    /// Add an FX delta sensitivity for the pair ``(ccy1, ccy2)``.
    #[pyo3(signature = (ccy1, ccy2, amount))]
    fn add_fx_delta(&mut self, ccy1: &str, ccy2: &str, amount: f64) -> PyResult<()> {
        let c1 = parse_currency(ccy1)?;
        let c2 = parse_currency(ccy2)?;
        self.inner.add_fx_delta(c1, c2, amount);
        Ok(())
    }

    /// Add a commodity delta sensitivity.
    #[pyo3(signature = (name, bucket, tenor, amount))]
    fn add_commodity_delta(&mut self, name: &str, bucket: u8, tenor: &str, amount: f64) {
        self.inner.add_commodity_delta(name, bucket, tenor, amount);
    }

    /// Add a GIRR vega sensitivity.
    #[pyo3(signature = (option_maturity, underlying_tenor, amount, currency = None))]
    fn add_girr_vega(
        &mut self,
        option_maturity: &str,
        underlying_tenor: &str,
        amount: f64,
        currency: Option<&str>,
    ) -> PyResult<()> {
        let ccy = self.currency_or_base(currency)?;
        self.inner
            .add_girr_vega(ccy, option_maturity, underlying_tenor, amount);
        Ok(())
    }

    /// Add an equity vega sensitivity.
    #[pyo3(signature = (underlier, bucket, maturity, amount))]
    fn add_equity_vega(&mut self, underlier: &str, bucket: u8, maturity: &str, amount: f64) {
        self.inner
            .add_equity_vega(underlier, bucket, maturity, amount);
    }

    /// Add an FX vega sensitivity.
    #[pyo3(signature = (ccy1, ccy2, maturity, amount))]
    fn add_fx_vega(&mut self, ccy1: &str, ccy2: &str, maturity: &str, amount: f64) -> PyResult<()> {
        let c1 = parse_currency(ccy1)?;
        let c2 = parse_currency(ccy2)?;
        self.inner.add_fx_vega(c1, c2, maturity, amount);
        Ok(())
    }

    /// Add a GIRR curvature sensitivity (CVR up and CVR down).
    #[pyo3(signature = (cvr_up, cvr_down, currency = None))]
    fn add_girr_curvature(
        &mut self,
        cvr_up: f64,
        cvr_down: f64,
        currency: Option<&str>,
    ) -> PyResult<()> {
        let ccy = self.currency_or_base(currency)?;
        self.inner.add_girr_curvature(ccy, cvr_up, cvr_down);
        Ok(())
    }

    /// Add an equity curvature sensitivity (CVR up and CVR down).
    #[pyo3(signature = (underlier, bucket, cvr_up, cvr_down))]
    fn add_equity_curvature(&mut self, underlier: &str, bucket: u8, cvr_up: f64, cvr_down: f64) {
        self.inner
            .add_equity_curvature(underlier, bucket, cvr_up, cvr_down);
    }

    /// Add an FX curvature sensitivity.
    #[pyo3(signature = (ccy1, ccy2, cvr_up, cvr_down))]
    fn add_fx_curvature(
        &mut self,
        ccy1: &str,
        ccy2: &str,
        cvr_up: f64,
        cvr_down: f64,
    ) -> PyResult<()> {
        let c1 = parse_currency(ccy1)?;
        let c2 = parse_currency(ccy2)?;
        self.inner.add_fx_curvature(c1, c2, cvr_up, cvr_down);
        Ok(())
    }

    /// Add an RRAO (residual risk add-on) position.
    ///
    /// Set ``is_exotic=True`` for the 1% weight (exotic underlying), leave as
    /// ``False`` for the 0.1% weight (other residual risk: gap, correlation,
    /// behavioural).
    #[pyo3(signature = (instrument_id, notional, is_exotic = false))]
    fn add_rrao_position(&mut self, instrument_id: &str, notional: f64, is_exotic: bool) {
        self.inner.rrao_exotic_notionals.push(RraoPosition {
            instrument_id: instrument_id.to_string(),
            notional,
            is_exotic,
        });
    }

    /// Base/reporting currency code.
    #[getter]
    fn base_currency(&self) -> String {
        self.inner.base_currency.to_string()
    }

    /// Export every populated sensitivity bucket as one long-format pandas
    /// ``DataFrame``.
    ///
    /// Columns: ``risk_class``, ``bucket``, ``tenor``, ``issuer``, ``kind``,
    /// ``amount``. One row per populated bucket; an empty container still
    /// carries all six columns. Long format is used deliberately — a column
    /// per bucket would give a different schema for every portfolio.
    ///
    /// ``risk_class`` uses the same labels as the ``frtb_sba_charge``
    /// breakdown (``girr``, ``csr_non_sec``, ``csr_sec_ctp``,
    /// ``csr_sec_non_ctp``, ``equity``, ``commodity``, ``fx``), plus ``drc``
    /// and ``rrao`` for the two position lists.
    ///
    /// ``kind`` is ``delta``, ``vega``, ``curvature_up``, ``curvature_down``,
    /// ``inflation_delta``, ``xccy_basis_delta``, ``jtd`` (DRC notional), or
    /// ``exotic_notional`` / ``other_notional`` (RRAO). A curvature pair is
    /// split across two rows so ``amount`` stays scalar.
    ///
    /// ``issuer`` carries the name axis: a currency code for GIRR, a
    /// ``"CCY1/CCY2"`` pair for FX, an issuer, tranche, underlier, commodity
    /// name, or instrument id elsewhere. ``bucket`` is the FRTB bucket index
    /// as a **string** (``pd.to_numeric`` if you need it numeric);
    /// ``tenor`` is the tenor or option maturity, and for GIRR vega the
    /// ``"{option_maturity}/{underlying_tenor}"`` pair. Both are ``None``
    /// where the risk class has no such axis.
    ///
    /// ``amount`` keeps each bucket's own convention: GIRR deltas are
    /// base-currency P&L per **1 percentage point** of curve shift (that is,
    /// ``100 x DV01``), DRC rows are signed JTD notionals before LGD, and
    /// RRAO rows are gross notionals.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let mut rows = SensitivityRows::default();
        let sens = &self.inner;

        for ((currency, tenor), amount) in &sens.girr_delta {
            rows.push(
                "girr",
                "delta",
                Some(currency.to_string()),
                None,
                Some(tenor.clone()),
                *amount,
            );
        }
        for (currency, amount) in &sens.girr_inflation_delta {
            rows.push(
                "girr",
                "inflation_delta",
                Some(currency.to_string()),
                None,
                None,
                *amount,
            );
        }
        for (currency, amount) in &sens.girr_xccy_basis_delta {
            rows.push(
                "girr",
                "xccy_basis_delta",
                Some(currency.to_string()),
                None,
                None,
                *amount,
            );
        }
        for ((currency, option_maturity, underlying_tenor), amount) in &sens.girr_vega {
            rows.push(
                "girr",
                "vega",
                Some(currency.to_string()),
                None,
                Some(format!("{option_maturity}/{underlying_tenor}")),
                *amount,
            );
        }
        for (currency, pair) in &sens.girr_curvature {
            rows.push_curvature("girr", Some(currency.to_string()), None, *pair);
        }

        // CSR (non-securitization and both securitization sub-types)
        for (label, delta, vega, curvature) in [
            (
                "csr_non_sec",
                &sens.csr_nonsec_delta,
                &sens.csr_nonsec_vega,
                &sens.csr_nonsec_curvature,
            ),
            (
                "csr_sec_ctp",
                &sens.csr_sec_ctp_delta,
                &sens.csr_sec_ctp_vega,
                &sens.csr_sec_ctp_curvature,
            ),
            (
                "csr_sec_non_ctp",
                &sens.csr_sec_nonctp_delta,
                &sens.csr_sec_nonctp_vega,
                &sens.csr_sec_nonctp_curvature,
            ),
        ] {
            for ((issuer, bucket, tenor), amount) in delta {
                rows.push(
                    label,
                    "delta",
                    Some(issuer.clone()),
                    bucket_label(*bucket),
                    Some(tenor.clone()),
                    *amount,
                );
            }
            for ((issuer, bucket, maturity), amount) in vega {
                rows.push(
                    label,
                    "vega",
                    Some(issuer.clone()),
                    bucket_label(*bucket),
                    Some(maturity.clone()),
                    *amount,
                );
            }
            for ((issuer, bucket), pair) in curvature {
                rows.push_curvature(label, Some(issuer.clone()), bucket_label(*bucket), *pair);
            }
        }

        for ((underlier, bucket), amount) in &sens.equity_delta {
            rows.push(
                "equity",
                "delta",
                Some(underlier.clone()),
                bucket_label(*bucket),
                None,
                *amount,
            );
        }
        for ((underlier, bucket, maturity), amount) in &sens.equity_vega {
            rows.push(
                "equity",
                "vega",
                Some(underlier.clone()),
                bucket_label(*bucket),
                Some(maturity.clone()),
                *amount,
            );
        }
        for ((underlier, bucket), pair) in &sens.equity_curvature {
            rows.push_curvature(
                "equity",
                Some(underlier.clone()),
                bucket_label(*bucket),
                *pair,
            );
        }

        for ((name, bucket, tenor), amount) in &sens.commodity_delta {
            rows.push(
                "commodity",
                "delta",
                Some(name.clone()),
                bucket_label(*bucket),
                Some(tenor.clone()),
                *amount,
            );
        }
        for ((name, bucket, maturity), amount) in &sens.commodity_vega {
            rows.push(
                "commodity",
                "vega",
                Some(name.clone()),
                bucket_label(*bucket),
                Some(maturity.clone()),
                *amount,
            );
        }
        for ((name, bucket), pair) in &sens.commodity_curvature {
            rows.push_curvature(
                "commodity",
                Some(name.clone()),
                bucket_label(*bucket),
                *pair,
            );
        }

        for ((ccy1, ccy2), amount) in &sens.fx_delta {
            rows.push("fx", "delta", pair_label(*ccy1, *ccy2), None, None, *amount);
        }
        for ((ccy1, ccy2, maturity), amount) in &sens.fx_vega {
            rows.push(
                "fx",
                "vega",
                pair_label(*ccy1, *ccy2),
                None,
                Some(maturity.clone()),
                *amount,
            );
        }
        for ((ccy1, ccy2), pair) in &sens.fx_curvature {
            rows.push_curvature("fx", pair_label(*ccy1, *ccy2), None, *pair);
        }

        // DRC and RRAO position lists
        for position in &sens.drc_positions {
            rows.push(
                "drc",
                "jtd",
                Some(position.issuer.clone()),
                bucket_label(position.rating_bucket),
                None,
                position.jtd_amount,
            );
        }
        for position in &sens.rrao_exotic_notionals {
            let kind = if position.is_exotic {
                "exotic_notional"
            } else {
                "other_notional"
            };
            rows.push(
                "rrao",
                kind,
                Some(position.instrument_id.clone()),
                None,
                None,
                position.notional,
            );
        }

        rows.into_dataframe(py)
    }

    fn __repr__(&self) -> String {
        format!(
            "FrtbSensitivities(base={}, girr_delta={}, equity_delta={}, fx_delta={})",
            self.inner.base_currency,
            self.inner.girr_delta.len(),
            self.inner.equity_delta.len(),
            self.inner.fx_delta.len(),
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

impl PyFrtbSensitivities {
    fn currency_or_base(&self, currency: Option<&str>) -> PyResult<Currency> {
        match currency {
            Some(c) => parse_currency(c),
            None => Ok(self.inner.base_currency),
        }
    }
}

/// FRTB SBA engine.
#[pyclass(
    name = "FrtbSbaEngine",
    module = "finstack_quant.margin",
    skip_from_py_object
)]
pub struct PyFrtbSbaEngine {
    inner: FrtbSbaEngine,
}

#[pymethods]
impl PyFrtbSbaEngine {
    /// Create an FRTB SBA engine.
    #[new]
    #[pyo3(signature = (correlation_scenario = None))]
    fn new(correlation_scenario: Option<&str>) -> PyResult<Self> {
        let mut builder = FrtbSbaEngine::builder();
        if let Some(s) = correlation_scenario {
            let scenario = parse_correlation_scenario(s)?;
            builder = builder.scenarios(vec![scenario]);
        }
        Ok(Self {
            inner: builder.build().map_err(core_to_py)?,
        })
    }

    /// Calculate the FRTB SBA charge for a sensitivity portfolio.
    ///
    /// Returns a :class:`FrtbSbaResult` carrying the total charge, the
    /// per-risk-class delta/vega/curvature breakdown, DRC, RRAO, and the
    /// per-scenario charges with the binding scenario named.
    fn calculate(
        &self,
        py: Python<'_>,
        sensitivities: &PyFrtbSensitivities,
    ) -> PyResult<PyFrtbSbaResult> {
        let result = py
            .detach(|| self.inner.calculate(&sensitivities.inner))
            .map_err(core_to_py)?;
        Ok(PyFrtbSbaResult::from_inner(result))
    }
}

/// A single derivative trade for SA-CCR EAD computation.
///
/// Construct through :meth:`from_json` so the complete canonical regulatory
/// schema, including supervisory delta and option classification, is explicit.
#[pyclass(
    name = "SaCcrTrade",
    module = "finstack_quant.margin",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PySaCcrTrade {
    pub(super) inner: SaCcrTrade,
}

#[pymethods]
impl PySaCcrTrade {
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Construct and validate the canonical `SaCcrTrade` JSON schema.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: SaCcrTrade = serde_json::from_str(json).map_err(display_to_py)?;
        inner.validate().map_err(core_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Trade identifier.
    #[getter]
    fn trade_id(&self) -> &str {
        &self.inner.trade_id
    }

    /// SA-CCR asset class label (``interest_rate``, ``foreign_exchange``,
    /// ``credit``, ``equity``, or ``commodity``).
    #[getter]
    fn asset_class(&self) -> String {
        asset_class_label(self.inner.asset_class)
    }

    /// Trade notional, in the netting set's reporting currency.
    #[getter]
    fn notional(&self) -> f64 {
        self.inner.notional
    }

    /// Current mark-to-market value, in the netting set's reporting currency.
    #[getter]
    fn mtm(&self) -> f64 {
        self.inner.mtm
    }

    fn __repr__(&self) -> String {
        format!(
            "SaCcrTrade(id={}, class={}, notional={:.0}, mtm={:.0})",
            self.inner.trade_id,
            asset_class_label(self.inner.asset_class),
            self.inner.notional,
            self.inner.mtm,
        )
    }
}

/// SA-CCR netting-set configuration.
#[pyclass(
    name = "SaCcrNettingSetConfig",
    module = "finstack_quant.margin",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PySaCcrNettingSetConfig {
    inner: SaCcrNettingSetConfig,
}

#[pymethods]
impl PySaCcrNettingSetConfig {
    /// Create an unmargined netting set configuration.
    #[staticmethod]
    #[pyo3(signature = (
        counterparty_id,
        csa_id,
        collateral,
        as_of_year,
        as_of_month,
        as_of_day,
    ))]
    fn unmargined(
        counterparty_id: &str,
        csa_id: &str,
        collateral: f64,
        as_of_year: i32,
        as_of_month: u8,
        as_of_day: u8,
    ) -> PyResult<Self> {
        let as_of = parse_date(as_of_year, as_of_month, as_of_day)?;
        Ok(Self {
            inner: SaCcrNettingSetConfig::unmargined(
                NettingSetId::bilateral(counterparty_id, csa_id),
                collateral,
                as_of,
            ),
        })
    }

    /// Create a margined netting set configuration.
    #[staticmethod]
    #[pyo3(signature = (
        counterparty_id,
        csa_id,
        collateral,
        threshold,
        mta,
        nica,
        mpor_days,
        as_of_year,
        as_of_month,
        as_of_day,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn margined(
        counterparty_id: &str,
        csa_id: &str,
        collateral: f64,
        threshold: f64,
        mta: f64,
        nica: f64,
        mpor_days: u32,
        as_of_year: i32,
        as_of_month: u8,
        as_of_day: u8,
    ) -> PyResult<Self> {
        let as_of = parse_date(as_of_year, as_of_month, as_of_day)?;
        Ok(Self {
            inner: SaCcrNettingSetConfig::margined(
                NettingSetId::bilateral(counterparty_id, csa_id),
                collateral,
                threshold,
                mta,
                nica,
                mpor_days,
                as_of,
            ),
        })
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

    /// Construct from a JSON serialization of `SaCcrNettingSetConfig`.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: SaCcrNettingSetConfig = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Whether the netting set is margined (a CSA with an MPoR applies).
    #[getter]
    fn is_margined(&self) -> bool {
        self.inner.is_margined
    }

    /// Net collateral held against the netting set, in its reporting currency.
    #[getter]
    fn collateral(&self) -> f64 {
        self.inner.collateral
    }

    /// Identify this value in notebooks and logs.
    ///
    /// Rendered from the wire representation, so the fields shown are the
    /// fields `to_json()` names. Collections are summarised by length; use
    /// `to_json()` or a DataFrame exit when the contents matter.
    fn __repr__(&self) -> String {
        crate::bindings::repr_support::repr_from_serde("SaCcrNettingSetConfig", &self.inner)
    }
}

/// SA-CCR engine.
#[pyclass(
    name = "SaCcrEngine",
    module = "finstack_quant.margin",
    skip_from_py_object
)]
pub struct PySaCcrEngine {
    inner: SaCcrEngine,
}

#[pymethods]
impl PySaCcrEngine {
    /// Create an SA-CCR engine.
    #[new]
    #[pyo3(signature = (alpha = None))]
    fn new(alpha: Option<f64>) -> PyResult<Self> {
        let mut builder = SaCcrEngine::builder();
        if let Some(a) = alpha {
            builder = builder.alpha(a);
        }
        Ok(Self {
            inner: builder.build().map_err(core_to_py)?,
        })
    }

    /// Calculate SA-CCR EAD for a netting set and trade list.
    ///
    /// Returns an :class:`EadResult` carrying EAD, RC, PFE, the multiplier,
    /// the aggregate and per-asset-class add-ons, alpha, and the maturity
    /// factor.
    fn calculate_ead(
        &self,
        py: Python<'_>,
        config: &PySaCcrNettingSetConfig,
        trades: Vec<PyRef<'_, PySaCcrTrade>>,
    ) -> PyResult<PyEadResult> {
        let trade_vec: Vec<SaCcrTrade> = trades.iter().map(|t| t.inner.clone()).collect();
        let result = py
            .detach(|| self.inner.calculate_ead(&config.inner, &trade_vec))
            .map_err(core_to_py)?;
        Ok(PyEadResult::from_inner(result))
    }
}

/// FRTB SBA capital-charge result (BCBS d457).
///
/// Returned by :func:`frtb_sba_charge` and :meth:`FrtbSbaEngine.calculate`.
/// Carries the headline charge, the per-risk-class delta/vega/curvature
/// breakdown, the default risk charge, the residual risk add-on, and the
/// charge under each correlation scenario together with the binding one.
#[pyclass(
    name = "FrtbSbaResult",
    module = "finstack_quant.margin",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFrtbSbaResult {
    inner: FrtbSbaResult,
}

impl PyFrtbSbaResult {
    fn from_inner(inner: FrtbSbaResult) -> Self {
        Self { inner }
    }

    fn labeled(map: &BTreeMap<FrtbRiskClass, f64>) -> BTreeMap<String, f64> {
        map.iter()
            .map(|(rc, v)| (risk_class_label(*rc), *v))
            .collect()
    }
}

#[pymethods]
impl PyFrtbSbaResult {
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Deserialize from the JSON produced by ``to_json``.
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: FrtbSbaResult = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize back to the same JSON shape ``from_json`` accepts.
    #[pyo3(text_signature = "($self)")]
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Total capital charge, in the sensitivity portfolio's base currency.
    #[getter]
    fn total(&self) -> f64 {
        self.inner.total
    }

    /// Default Risk Charge (credit + equity jump-to-default).
    #[getter]
    fn drc(&self) -> f64 {
        self.inner.drc
    }

    /// Residual Risk Add-On.
    #[getter]
    fn rrao(&self) -> f64 {
        self.inner.rrao
    }

    /// Correlation scenario that produced the binding charge
    /// (``"low"``, ``"medium"``, or ``"high"``).
    #[getter]
    fn binding_scenario(&self) -> String {
        scenario_label(self.inner.binding_scenario)
    }

    /// Delta risk charge keyed by risk-class wire label (e.g. ``"girr"``).
    #[getter]
    fn delta_by_risk_class(&self) -> BTreeMap<String, f64> {
        Self::labeled(&self.inner.delta_by_risk_class)
    }

    /// Vega risk charge keyed by risk-class wire label.
    #[getter]
    fn vega_by_risk_class(&self) -> BTreeMap<String, f64> {
        Self::labeled(&self.inner.vega_by_risk_class)
    }

    /// Curvature risk charge keyed by risk-class wire label.
    #[getter]
    fn curvature_by_risk_class(&self) -> BTreeMap<String, f64> {
        Self::labeled(&self.inner.curvature_by_risk_class)
    }

    /// Delta+vega+curvature charge under each evaluated correlation scenario.
    #[getter]
    fn scenario_charges(&self) -> BTreeMap<String, f64> {
        self.inner
            .scenario_charges
            .iter()
            .map(|(s, v)| (scenario_label(*s), *v))
            .collect()
    }

    /// Export the headline charge as a single-row pandas ``DataFrame``.
    ///
    /// Columns: ``total``, ``drc``, ``rrao`` (floats in the portfolio's base
    /// currency) and ``binding_scenario`` (string). Per-risk-class detail lives
    /// in ``to_breakdown_dataframe``.
    ///
    /// One row, so a book of desks stacks with
    /// ``pd.concat([r.to_dataframe() for r in results])``.
    #[pyo3(text_signature = "($self)")]
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let row = serde_json::json!({
            "total": self.inner.total,
            "drc": self.inner.drc,
            "rrao": self.inner.rrao,
            "binding_scenario": self.binding_scenario(),
        });
        serde_object_to_single_row_dataframe_with_schema(
            py,
            &row,
            &["total", "drc", "rrao", "binding_scenario"],
        )
    }

    /// Export the per-risk-class breakdown as a long-format pandas ``DataFrame``.
    ///
    /// Columns: ``component`` (``"delta"``, ``"vega"``, or ``"curvature"``),
    /// ``risk_class`` (wire label, e.g. ``"girr"``), and ``charge`` (float in
    /// the portfolio's base currency). Rows are emitted component-major and
    /// risk-class sorted so repeated runs are byte-identical.
    ///
    /// The components do not sum to ``total``: SBA aggregates them with
    /// prescribed correlations, and ``drc``/``rrao`` sit outside this frame.
    #[pyo3(text_signature = "($self)")]
    fn to_breakdown_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        const COLUMNS: &[ColumnSchema<'_>] = &[
            ("component", "str"),
            ("risk_class", "str"),
            ("charge", "float64"),
        ];
        let mut rows: Vec<serde_json::Value> = Vec::new();
        for (component, map) in [
            ("delta", &self.inner.delta_by_risk_class),
            ("vega", &self.inner.vega_by_risk_class),
            ("curvature", &self.inner.curvature_by_risk_class),
        ] {
            for (rc, charge) in map {
                rows.push(serde_json::json!({
                    "component": component,
                    "risk_class": risk_class_label(*rc),
                    "charge": charge,
                }));
            }
        }
        serde_rows_to_dataframe_with_schema(py, &rows, COLUMNS)
    }

    /// Return ``repr(self)``.
    fn __repr__(&self) -> String {
        format!(
            "FrtbSbaResult(total={:.2}, drc={:.2}, rrao={:.2}, binding_scenario={})",
            self.inner.total,
            self.inner.drc,
            self.inner.rrao,
            self.binding_scenario()
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

/// SA-CCR Exposure at Default result (BCBS 279).
///
/// Returned by :func:`saccr_ead` and :meth:`SaCcrEngine.calculate_ead`.
/// ``ead == alpha * (rc + pfe)``; ``pfe == multiplier * add_on_aggregate``.
#[pyclass(
    name = "EadResult",
    module = "finstack_quant.margin",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyEadResult {
    inner: EadResult,
}

impl PyEadResult {
    fn from_inner(inner: EadResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyEadResult {
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Deserialize from the JSON produced by ``to_json``.
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: EadResult = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize back to the same JSON shape ``from_json`` accepts.
    #[pyo3(text_signature = "($self)")]
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Exposure at Default: ``alpha * (rc + pfe)``, in the netting set's
    /// reporting currency.
    #[getter]
    fn ead(&self) -> f64 {
        self.inner.ead
    }

    /// Replacement cost component.
    #[getter]
    fn rc(&self) -> f64 {
        self.inner.rc
    }

    /// Potential future exposure: ``multiplier * add_on_aggregate``.
    #[getter]
    fn pfe(&self) -> f64 {
        self.inner.pfe
    }

    /// PFE multiplier, which recognises over-collateralization and negative
    /// mark-to-market (1.0 when neither applies, floored at 0.05).
    #[getter]
    fn multiplier(&self) -> f64 {
        self.inner.multiplier
    }

    /// Aggregate add-on across asset classes, before the multiplier.
    #[getter]
    fn add_on_aggregate(&self) -> f64 {
        self.inner.add_on_aggregate
    }

    /// Alpha multiplier (1.4 per BCBS 279 unless overridden on the engine).
    #[getter]
    fn alpha(&self) -> f64 {
        self.inner.alpha
    }

    /// Maturity factor applied to the netting set.
    #[getter]
    fn maturity_factor(&self) -> f64 {
        self.inner.maturity_factor
    }

    /// Add-on keyed by asset-class wire label (e.g. ``"interest_rate"``).
    #[getter]
    fn add_on_by_asset_class(&self) -> BTreeMap<String, f64> {
        self.inner
            .add_on_by_asset_class
            .iter()
            .map(|(ac, v)| (asset_class_label(*ac), *v))
            .collect()
    }

    /// Export the headline exposure as a single-row pandas ``DataFrame``.
    ///
    /// Columns: ``ead``, ``rc``, ``pfe``, ``multiplier``, ``add_on_aggregate``
    /// (floats in the netting set's reporting currency), ``alpha`` and
    /// ``maturity_factor`` (dimensionless floats). Per-asset-class add-on
    /// detail lives in ``to_add_on_dataframe``.
    ///
    /// One row, so a book of netting sets stacks with
    /// ``pd.concat([r.to_dataframe() for r in results])``.
    #[pyo3(text_signature = "($self)")]
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let row = serde_json::json!({
            "ead": self.inner.ead,
            "rc": self.inner.rc,
            "pfe": self.inner.pfe,
            "multiplier": self.inner.multiplier,
            "add_on_aggregate": self.inner.add_on_aggregate,
            "alpha": self.inner.alpha,
            "maturity_factor": self.inner.maturity_factor,
        });
        serde_object_to_single_row_dataframe_with_schema(
            py,
            &row,
            &[
                "ead",
                "rc",
                "pfe",
                "multiplier",
                "add_on_aggregate",
                "alpha",
                "maturity_factor",
            ],
        )
    }

    /// Export the per-asset-class add-on as a pandas ``DataFrame``.
    ///
    /// Columns: ``asset_class`` (wire label) and ``add_on`` (float in the
    /// netting set's reporting currency). One row per asset class present,
    /// sorted by ``asset_class`` so repeated runs are byte-identical. A netting
    /// set with no trades yields a zero-row frame that still carries both
    /// columns with their real dtypes.
    #[pyo3(text_signature = "($self)")]
    fn to_add_on_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        const COLUMNS: &[ColumnSchema<'_>] = &[("asset_class", "str"), ("add_on", "float64")];
        let rows: Vec<serde_json::Value> = self
            .inner
            .add_on_by_asset_class
            .iter()
            .map(|(ac, add_on)| {
                serde_json::json!({
                    "asset_class": asset_class_label(*ac),
                    "add_on": add_on,
                })
            })
            .collect();
        serde_rows_to_dataframe_with_schema(py, &rows, COLUMNS)
    }

    /// Return ``repr(self)``.
    fn __repr__(&self) -> String {
        format!(
            "EadResult(ead={:.2}, rc={:.2}, pfe={:.2}, alpha={:.2})",
            self.inner.ead, self.inner.rc, self.inner.pfe, self.inner.alpha
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

/// Compute the FRTB SBA capital charge.
///
/// Returns a :class:`FrtbSbaResult` exposing ``total``, ``drc``, ``rrao``,
/// ``binding_scenario``, ``scenario_charges``, and the per-risk-class
/// ``delta_by_risk_class`` / ``vega_by_risk_class`` /
/// ``curvature_by_risk_class`` breakdowns, plus ``to_dataframe`` /
/// ``to_breakdown_dataframe`` exits.
///
/// If ``correlation_scenario`` is provided (``"low"``, ``"medium"``, ``"high"``),
/// only that scenario is evaluated. Otherwise all three are run and the
/// max is taken per BCBS d457.
#[pyfunction]
#[pyo3(signature = (sensitivities, correlation_scenario = None))]
pub fn frtb_sba_charge(
    py: Python<'_>,
    sensitivities: &PyFrtbSensitivities,
    correlation_scenario: Option<&str>,
) -> PyResult<PyFrtbSbaResult> {
    let mut builder = FrtbSbaEngine::builder();
    if let Some(s) = correlation_scenario {
        let scenario = parse_correlation_scenario(s)?;
        builder = builder.scenarios(vec![scenario]);
    }
    let engine = builder.build().map_err(core_to_py)?;
    let result = py
        .detach(|| engine.calculate(&sensitivities.inner))
        .map_err(core_to_py)?;

    Ok(PyFrtbSbaResult::from_inner(result))
}

/// Compute SA-CCR Exposure at Default for a set of trades.
///
/// Returns an :class:`EadResult` per BCBS 279, carrying ``rc`` (replacement
/// cost), ``pfe`` (potential future exposure = multiplier × aggregate add-on),
/// ``ead`` (= α × (RC + PFE), with α = 1.4), plus ``multiplier``,
/// ``add_on_aggregate``, ``add_on_by_asset_class``, ``alpha``, and
/// ``maturity_factor``.
///
/// ``as_of_year`` / ``as_of_month`` / ``as_of_day`` set the valuation date
/// used for forward-start and remaining-maturity calculations; maturity
/// factors and PFE depend directly on it, so it must be supplied explicitly.
///
/// The netting set defaults to the Rust
/// ``SaCcrNettingSetConfig::unmargined`` terms (zero threshold / MTA / NICA,
/// 10-day MPoR). When ``margined=True``, ``threshold``, ``mta``, ``nica``,
/// and ``mpor_days`` override those constructor defaults where supplied.
/// ``counterparty_id`` / ``csa_id`` name the bilateral netting set and
/// default to the placeholder pair ``"CPTY"`` / ``"CSA"`` — supply real
/// identifiers for anything beyond ad-hoc analysis. For full control build a
/// :class:`SaCcrNettingSetConfig` and use :meth:`SaCcrEngine.calculate_ead`.
///
/// Raises ``ValueError`` if ``trades`` is empty, the date is invalid, or the
/// netting-set terms fail validation.
#[pyfunction]
#[pyo3(signature = (
    trades,
    as_of_year,
    as_of_month,
    as_of_day,
    margined = false,
    collateral = 0.0,
    threshold = None,
    mta = None,
    nica = None,
    mpor_days = None,
    counterparty_id = "CPTY",
    csa_id = "CSA",
))]
#[allow(clippy::too_many_arguments)]
pub fn saccr_ead(
    py: Python<'_>,
    trades: Vec<PyRef<'_, PySaCcrTrade>>,
    as_of_year: i32,
    as_of_month: u8,
    as_of_day: u8,
    margined: bool,
    collateral: f64,
    threshold: Option<f64>,
    mta: Option<f64>,
    nica: Option<f64>,
    mpor_days: Option<u32>,
    counterparty_id: &str,
    csa_id: &str,
) -> PyResult<PyEadResult> {
    if trades.is_empty() {
        return Err(crate::errors::value_error(
            "saccr_ead requires at least one trade",
        ));
    }
    let engine = SaCcrEngine::builder().build().map_err(core_to_py)?;
    let netting_id = NettingSetId::bilateral(counterparty_id, csa_id);
    let trade_vec: Vec<SaCcrTrade> = trades.iter().map(|t| t.inner.clone()).collect();
    let as_of = parse_date(as_of_year, as_of_month, as_of_day)?;
    // Start from the Rust unmargined constructor so threshold/MTA/NICA/MPoR
    // defaults are owned by the margin crate, then apply margined overrides.
    let mut config = SaCcrNettingSetConfig::unmargined(netting_id, collateral, as_of);
    if !margined && (threshold.is_some() || mta.is_some() || nica.is_some() || mpor_days.is_some())
    {
        // Refuse silently inert configuration: these terms only enter the
        // margined RC/PFE formulas.
        return Err(crate::errors::value_error(
            "threshold/mta/nica/mpor_days require margined=True",
        ));
    }
    if margined {
        config.is_margined = true;
        if let Some(threshold) = threshold {
            config.threshold = threshold;
        }
        if let Some(mta) = mta {
            config.mta = mta;
        }
        if let Some(nica) = nica {
            config.nica = nica;
        }
        if let Some(mpor_days) = mpor_days {
            config.mpor_days = mpor_days;
        }
    }
    config.validate().map_err(core_to_py)?;
    let result = py
        .detach(|| engine.calculate_ead(&config, &trade_vec))
        .map_err(core_to_py)?;
    Ok(PyEadResult::from_inner(result))
}

/// Register FRTB / SA-CCR classes and functions on the margin module.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyFrtbSensitivities>()?;
    m.add_class::<PyFrtbSbaEngine>()?;
    m.add_class::<PyFrtbSbaResult>()?;
    m.add_class::<PyEadResult>()?;
    m.add_class::<PySaCcrTrade>()?;
    m.add_class::<PySaCcrNettingSetConfig>()?;
    m.add_class::<PySaCcrEngine>()?;
    m.add_function(pyo3::wrap_pyfunction!(frtb_sba_charge, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(saccr_ead, m)?)?;
    Ok(())
}
