//! Python bindings for FRTB SBA and SA-CCR regulatory capital frameworks.
//!
//! Exposes a deliberately simplified surface: callers build sensitivity or
//! trade containers with ergonomic `add_*` methods, then invoke a free function
//! that returns the headline capital number plus a per-component breakdown.
//! Full typed access to every enum variant is intentionally omitted; where
//! complex configuration is needed, JSON round-tripping is used.

use super::sensitivity_frame::SensitivityRows;
use crate::bindings::module_utils::{parse_currency, parse_date};
use crate::errors::{core_to_py, display_to_py};
use finstack_quant_core::currency::Currency;
use finstack_quant_margin::regulatory::{
    frtb::{CorrelationScenario, FrtbRiskClass, FrtbSbaEngine, FrtbSensitivities, RraoPosition},
    sa_ccr::{SaCcrAssetClass, SaCcrEngine, SaCcrNettingSetConfig, SaCcrTrade},
};
use finstack_quant_margin::NettingSetId;
use pyo3::prelude::*;
use pyo3::types::PyDict;

// Helpers

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

fn risk_class_label(rc: FrtbRiskClass) -> &'static str {
    match rc {
        FrtbRiskClass::Girr => "girr",
        FrtbRiskClass::CsrNonSec => "csr_non_sec",
        FrtbRiskClass::CsrSecCtp => "csr_sec_ctp",
        FrtbRiskClass::CsrSecNonCtp => "csr_sec_non_ctp",
        FrtbRiskClass::Equity => "equity",
        FrtbRiskClass::Commodity => "commodity",
        FrtbRiskClass::Fx => "fx",
        _ => "unknown",
    }
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

fn asset_class_label(ac: SaCcrAssetClass) -> &'static str {
    match ac {
        SaCcrAssetClass::InterestRate => "interest_rate",
        SaCcrAssetClass::ForeignExchange => "foreign_exchange",
        SaCcrAssetClass::Credit => "credit",
        SaCcrAssetClass::Equity => "equity",
        SaCcrAssetClass::Commodity => "commodity",
        _ => "unknown",
    }
}

// FrtbSensitivities wrapper

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
        Ok((from_json, (self.to_json()?,)))
    }

    /// Construct from a JSON serialization of `FrtbSensitivities`.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: FrtbSensitivities = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner).map_err(display_to_py)
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

        // GIRR
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

        // Equity
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

        // Commodity
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

        // FX
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
    fn calculate(
        &self,
        py: Python<'_>,
        sensitivities: &PyFrtbSensitivities,
    ) -> PyResult<(f64, Py<PyDict>)> {
        let result = py
            .detach(|| self.inner.calculate(&sensitivities.inner))
            .map_err(core_to_py)?;
        frtb_result_to_py(py, &result)
    }
}

// SaCcrTrade wrapper

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
        Ok((from_json, (self.to_json()?,)))
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
        serde_json::to_string_pretty(&self.inner).map_err(display_to_py)
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
        asset_class_label(self.inner.asset_class).to_string()
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
        Ok((from_json, (self.to_json()?,)))
    }

    /// Construct from a JSON serialization of `SaCcrNettingSetConfig`.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: SaCcrNettingSetConfig = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner).map_err(display_to_py)
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
    fn calculate_ead(
        &self,
        py: Python<'_>,
        config: &PySaCcrNettingSetConfig,
        trades: Vec<PyRef<'_, PySaCcrTrade>>,
    ) -> PyResult<Py<PyDict>> {
        let trade_vec: Vec<SaCcrTrade> = trades.iter().map(|t| t.inner.clone()).collect();
        let result = py
            .detach(|| self.inner.calculate_ead(&config.inner, &trade_vec))
            .map_err(core_to_py)?;
        ead_result_to_py(py, &result)
    }
}

// Module-level functions

fn frtb_result_to_py(
    py: Python<'_>,
    result: &finstack_quant_margin::regulatory::frtb::FrtbSbaResult,
) -> PyResult<(f64, Py<PyDict>)> {
    let dict = PyDict::new(py);

    let delta = PyDict::new(py);
    for (rc, v) in &result.delta_by_risk_class {
        delta.set_item(risk_class_label(*rc), *v)?;
    }
    dict.set_item("delta", delta)?;

    let vega = PyDict::new(py);
    for (rc, v) in &result.vega_by_risk_class {
        vega.set_item(risk_class_label(*rc), *v)?;
    }
    dict.set_item("vega", vega)?;

    let curvature = PyDict::new(py);
    for (rc, v) in &result.curvature_by_risk_class {
        curvature.set_item(risk_class_label(*rc), *v)?;
    }
    dict.set_item("curvature", curvature)?;

    dict.set_item("drc", result.drc)?;
    dict.set_item("rrao", result.rrao)?;

    let binding_scenario_name = match result.binding_scenario {
        CorrelationScenario::Low => "low",
        CorrelationScenario::Medium => "medium",
        CorrelationScenario::High => "high",
    };
    dict.set_item("binding_scenario", binding_scenario_name)?;

    let scenarios = PyDict::new(py);
    for (s, v) in &result.scenario_charges {
        let name = match s {
            CorrelationScenario::Low => "low",
            CorrelationScenario::Medium => "medium",
            CorrelationScenario::High => "high",
        };
        scenarios.set_item(name, *v)?;
    }
    dict.set_item("scenario_charges", scenarios)?;

    Ok((result.total, dict.unbind()))
}

fn ead_result_to_py(
    py: Python<'_>,
    result: &finstack_quant_margin::regulatory::sa_ccr::EadResult,
) -> PyResult<Py<PyDict>> {
    let dict = PyDict::new(py);
    dict.set_item("ead", result.ead)?;
    dict.set_item("rc", result.rc)?;
    dict.set_item("pfe", result.pfe)?;
    dict.set_item("multiplier", result.multiplier)?;
    dict.set_item("add_on_aggregate", result.add_on_aggregate)?;
    dict.set_item("alpha", result.alpha)?;
    dict.set_item("maturity_factor", result.maturity_factor)?;

    let by_class = PyDict::new(py);
    for (asset_class, value) in &result.add_on_by_asset_class {
        by_class.set_item(asset_class_label(*asset_class), *value)?;
    }
    dict.set_item("add_on_by_asset_class", by_class)?;
    Ok(dict.unbind())
}

/// Compute the FRTB SBA capital charge.
///
/// Returns ``(total_charge, breakdown)`` where ``breakdown`` is a dict with:
/// - ``delta``: ``{risk_class: charge}``
/// - ``vega``: ``{risk_class: charge}``
/// - ``curvature``: ``{risk_class: charge}``
/// - ``drc``: default risk charge (float)
/// - ``rrao``: residual risk add-on (float)
/// - ``binding_scenario``: scenario name selected as binding
/// - ``scenario_charges``: ``{scenario_name: sba_charge}``
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
) -> PyResult<(f64, Py<PyDict>)> {
    let mut builder = FrtbSbaEngine::builder();
    if let Some(s) = correlation_scenario {
        let scenario = parse_correlation_scenario(s)?;
        builder = builder.scenarios(vec![scenario]);
    }
    let engine = builder.build().map_err(core_to_py)?;
    let result = py
        .detach(|| engine.calculate(&sensitivities.inner))
        .map_err(core_to_py)?;

    frtb_result_to_py(py, &result)
}

/// Compute SA-CCR Exposure at Default for a set of trades.
///
/// Returns ``(rc, pfe, ead)`` per BCBS 279:
/// - ``rc``: replacement cost
/// - ``pfe``: potential future exposure (multiplier × aggregate add-on)
/// - ``ead``: exposure at default = α × (RC + PFE), with α = 1.4
///
/// The netting set uses default terms: zero collateral / threshold / MTA /
/// NICA, and 10-day MPoR when margined. For bespoke collateral terms,
/// build via :meth:`SaCcrEngine.calculate_from_json` (not yet exposed).
#[pyfunction]
#[pyo3(signature = (trades, margined = false, collateral = 0.0))]
pub fn saccr_ead(
    py: Python<'_>,
    trades: Vec<PyRef<'_, PySaCcrTrade>>,
    margined: bool,
    collateral: f64,
) -> PyResult<(f64, f64, f64)> {
    let engine = SaCcrEngine::builder().build().map_err(core_to_py)?;
    let netting_id = NettingSetId::bilateral("CPTY", "CSA");
    let trade_vec: Vec<SaCcrTrade> = trades.iter().map(|t| t.inner.clone()).collect();
    let as_of = if let Some(date) = trade_vec.iter().map(|t| t.start_date).min() {
        date
    } else {
        parse_date(1970, 1, 1)?
    };
    let config = if margined {
        SaCcrNettingSetConfig::margined(netting_id, collateral, 0.0, 0.0, 0.0, 10, as_of)
    } else {
        SaCcrNettingSetConfig::unmargined(netting_id, collateral, as_of)
    };
    let result = py
        .detach(|| engine.calculate_ead(&config, &trade_vec))
        .map_err(core_to_py)?;
    Ok((result.rc, result.pfe, result.ead))
}

/// Register FRTB / SA-CCR classes and functions on the margin module.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyFrtbSensitivities>()?;
    m.add_class::<PyFrtbSbaEngine>()?;
    m.add_class::<PySaCcrTrade>()?;
    m.add_class::<PySaCcrNettingSetConfig>()?;
    m.add_class::<PySaCcrEngine>()?;
    m.add_function(pyo3::wrap_pyfunction!(frtb_sba_charge, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(saccr_ead, m)?)?;
    Ok(())
}
