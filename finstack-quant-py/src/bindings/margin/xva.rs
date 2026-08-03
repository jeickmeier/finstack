//! Python wrappers for XVA types (CVA/DVA/FVA configuration and results).

use crate::bindings::pandas_utils::dict_to_dataframe;
use crate::errors::{core_to_py, display_to_py};
use finstack_quant_margin::xva::types as xva;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use finstack_quant_margin::xva::cva;
use finstack_quant_margin::xva::mva;

use crate::bindings::core::market_data::curves::{PyDiscountCurve, PyHazardCurve};
use crate::bindings::margin::im::{PySimmCalculator, PySimmSensitivities};

// FundingConfig

/// Funding cost/benefit configuration for FVA and MVA calculation.
#[pyclass(
    name = "FundingConfig",
    module = "finstack_quant.margin",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFundingConfig {
    pub(super) inner: xva::FundingConfig,
}

#[pymethods]
impl PyFundingConfig {
    /// Create a new funding configuration.
    ///
    /// Parameters
    /// ----------
    /// funding_spread_bp : float
    ///     Non-negative finite funding cost spread in basis points.
    /// funding_benefit_bp : float | None
    ///     Non-negative finite funding benefit spread in bp, no greater than
    ///     ``funding_spread_bp``. If ``None``, symmetric funding.
    /// im_profile : ImProfile | None
    ///     Valid expected initial-margin profile driving MVA. If ``None``, MVA
    ///     is not computed.
    /// margin_funding_spread_bp : float | None
    ///     Non-negative finite spread applied to posted IM. If ``None``, uses
    ///     ``funding_spread_bp``.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If a spread or the optional IM profile violates these constraints.
    #[new]
    #[pyo3(signature = (
        funding_spread_bp,
        funding_benefit_bp=None,
        im_profile=None,
        margin_funding_spread_bp=None,
    ))]
    fn new(
        funding_spread_bp: f64,
        funding_benefit_bp: Option<f64>,
        im_profile: Option<&PyImProfile>,
        margin_funding_spread_bp: Option<f64>,
    ) -> PyResult<Self> {
        let inner = xva::FundingConfig {
            funding_spread_bp,
            funding_benefit_bp,
            im_profile: im_profile.map(|p| p.inner.clone()),
            margin_funding_spread_bp,
        };
        inner.validate().map_err(core_to_py)?;
        Ok(Self { inner })
    }

    /// Funding spread in basis points.
    #[getter]
    fn funding_spread_bp(&self) -> f64 {
        self.inner.funding_spread_bp
    }

    /// Funding benefit spread in basis points (or None).
    #[getter]
    fn funding_benefit_bp(&self) -> Option<f64> {
        self.inner.funding_benefit_bp
    }

    /// Expected IM profile driving MVA (or None).
    #[getter]
    fn im_profile(&self) -> Option<PyImProfile> {
        self.inner.im_profile.as_ref().map(|inner| PyImProfile {
            inner: inner.clone(),
        })
    }

    /// IM funding spread in basis points (or None).
    #[getter]
    fn margin_funding_spread_bp(&self) -> Option<f64> {
        self.inner.margin_funding_spread_bp
    }

    /// Effective funding benefit spread in basis points.
    fn effective_benefit_bp(&self) -> f64 {
        self.inner.effective_benefit_bp()
    }

    /// Effective IM funding spread in basis points (falls back to
    /// ``funding_spread_bp``).
    fn effective_margin_spread_bp(&self) -> f64 {
        self.inner.effective_margin_spread_bp()
    }

    fn __repr__(&self) -> String {
        format!(
            "FundingConfig(spread={:.1}bp, benefit={:?}bp, mva={})",
            self.inner.funding_spread_bp,
            self.inner.funding_benefit_bp,
            self.inner.im_profile.is_some(),
        )
    }
}

// XvaConfig

/// XVA calculation configuration.
#[pyclass(
    name = "XvaConfig",
    module = "finstack_quant.margin",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyXvaConfig {
    pub(super) inner: xva::XvaConfig,
}

#[pymethods]
impl PyXvaConfig {
    /// Create a default XVA configuration (quarterly grid to 30Y, 40% recovery).
    #[new]
    #[pyo3(signature = (time_grid=None, recovery_rate=None, own_recovery_rate=None, funding=None))]
    fn new(
        time_grid: Option<Vec<f64>>,
        recovery_rate: Option<f64>,
        own_recovery_rate: Option<f64>,
        funding: Option<&PyFundingConfig>,
    ) -> Self {
        let default = xva::XvaConfig::default();
        Self {
            inner: xva::XvaConfig {
                time_grid: time_grid.unwrap_or(default.time_grid),
                recovery_rate: recovery_rate.unwrap_or(default.recovery_rate),
                own_recovery_rate,
                funding: funding.map(|f| f.inner.clone()),
            },
        }
    }

    /// Deserialize from JSON.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: xva::XvaConfig = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize to JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner).map_err(display_to_py)
    }

    /// Validate configuration parameters.
    fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(core_to_py)
    }

    /// Time grid for exposure simulation (years from today).
    #[getter]
    fn time_grid(&self) -> Vec<f64> {
        self.inner.time_grid.clone()
    }

    /// Recovery rate for counterparty default.
    #[getter]
    fn recovery_rate(&self) -> f64 {
        self.inner.recovery_rate
    }

    /// Recovery rate for own default (or None).
    #[getter]
    fn own_recovery_rate(&self) -> Option<f64> {
        self.inner.own_recovery_rate
    }

    fn __repr__(&self) -> String {
        format!(
            "XvaConfig(grid_points={}, recovery={:.0}%)",
            self.inner.time_grid.len(),
            self.inner.recovery_rate * 100.0
        )
    }
}

// ExposureDiagnostics

/// Diagnostics from exposure simulation.
#[pyclass(
    name = "ExposureDiagnostics",
    module = "finstack_quant.margin",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyExposureDiagnostics {
    pub(super) inner: xva::ExposureDiagnostics,
}

#[pymethods]
impl PyExposureDiagnostics {
    /// Number of market-roll failures.
    #[getter]
    fn market_roll_failures(&self) -> usize {
        self.inner.market_roll_failures
    }

    /// Total instrument valuation failures.
    #[getter]
    fn valuation_failures(&self) -> usize {
        self.inner.valuation_failures
    }

    /// Total time grid points evaluated.
    #[getter]
    fn total_time_points(&self) -> usize {
        self.inner.total_time_points
    }

    fn __repr__(&self) -> String {
        format!(
            "ExposureDiagnostics(market_roll_failures={}, valuation_failures={}, points={})",
            self.inner.market_roll_failures,
            self.inner.valuation_failures,
            self.inner.total_time_points
        )
    }
}

// ExposureProfile

/// Exposure profile at each time grid point.
#[pyclass(
    name = "ExposureProfile",
    module = "finstack_quant.margin",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyExposureProfile {
    pub(super) inner: xva::ExposureProfile,
}

#[pymethods]
impl PyExposureProfile {
    /// Construct from vectors.
    #[new]
    fn new(times: Vec<f64>, mtm_values: Vec<f64>, epe: Vec<f64>, ene: Vec<f64>) -> Self {
        Self {
            inner: xva::ExposureProfile {
                times,
                mtm_values,
                epe,
                ene,
                diagnostics: None,
            },
        }
    }

    /// Deserialize from JSON.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: xva::ExposureProfile = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize to JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner).map_err(display_to_py)
    }

    /// Validate internal consistency.
    fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(core_to_py)
    }

    /// Time points in years.
    #[getter]
    fn times(&self) -> Vec<f64> {
        self.inner.times.clone()
    }

    /// Portfolio MtM values at each time point.
    #[getter]
    fn mtm_values(&self) -> Vec<f64> {
        self.inner.mtm_values.clone()
    }

    /// Expected Positive Exposure at each time point.
    #[getter]
    fn epe(&self) -> Vec<f64> {
        self.inner.epe.clone()
    }

    /// Expected Negative Exposure at each time point.
    #[getter]
    fn ene(&self) -> Vec<f64> {
        self.inner.ene.clone()
    }

    /// Number of time points.
    fn __len__(&self) -> usize {
        self.inner.times.len()
    }

    /// Export as a pandas ``DataFrame`` with time (years) as index.
    ///
    /// Columns: ``mtm_values``, ``epe``, ``ene``.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let data = PyDict::new(py);
        data.set_item("mtm_values", &self.inner.mtm_values)?;
        data.set_item("epe", &self.inner.epe)?;
        data.set_item("ene", &self.inner.ene)?;
        let idx = self.inner.times.clone().into_pyobject(py)?.into_any();
        dict_to_dataframe(py, &data, Some(idx))
    }

    fn __repr__(&self) -> String {
        format!("ExposureProfile(points={})", self.inner.times.len())
    }
}

// XvaResult

/// Result of XVA calculations (CVA, DVA, FVA, MVA, exposure profiles).
#[pyclass(
    name = "XvaResult",
    module = "finstack_quant.margin",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyXvaResult {
    pub(super) inner: xva::XvaResult,
}

#[pymethods]
impl PyXvaResult {
    /// Deserialize from JSON.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: xva::XvaResult = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize to JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner).map_err(display_to_py)
    }

    /// Unilateral CVA (positive = cost).
    #[getter]
    fn cva(&self) -> f64 {
        self.inner.cva
    }

    /// DVA (own-default benefit, or None).
    #[getter]
    fn dva(&self) -> Option<f64> {
        self.inner.dva
    }

    /// FVA (net funding cost/benefit, or None).
    #[getter]
    fn fva(&self) -> Option<f64> {
        self.inner.fva
    }

    /// MVA (funding cost of posted initial margin, or None).
    #[getter]
    fn mva(&self) -> Option<f64> {
        self.inner.mva
    }

    /// All-in adjustment = CVA - DVA + FVA + MVA.
    ///
    /// Uncomputed legs contribute zero. This is the quantity subtracted from
    /// the risk-free value of the netting set.
    #[getter]
    fn total_xva(&self) -> f64 {
        self.inner.total_xva
    }

    /// Maximum PFE across the profile.
    #[getter]
    fn max_pfe(&self) -> f64 {
        self.inner.max_pfe
    }

    /// Effective EPE (time-weighted average, regulatory metric).
    #[getter]
    fn effective_epe(&self) -> f64 {
        self.inner.effective_epe
    }

    /// EPE profile as list of (time, value) tuples.
    #[getter]
    fn epe_profile(&self) -> Vec<(f64, f64)> {
        self.inner.epe_profile.clone()
    }

    /// ENE profile as list of (time, value) tuples.
    #[getter]
    fn ene_profile(&self) -> Vec<(f64, f64)> {
        self.inner.ene_profile.clone()
    }

    /// PFE profile as list of (time, value) tuples.
    #[getter]
    fn pfe_profile(&self) -> Vec<(f64, f64)> {
        self.inner.pfe_profile.clone()
    }

    /// Effective EPE profile as list of (time, value) tuples.
    #[getter]
    fn effective_epe_profile(&self) -> Vec<(f64, f64)> {
        self.inner.effective_epe_profile.clone()
    }

    /// Export exposure profiles as a pandas ``DataFrame``.
    ///
    /// Columns: ``epe``, ``ene``, ``pfe``, ``effective_epe`` — indexed by
    /// time in years.  Time values are taken from the EPE profile.
    fn profiles_to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let data = PyDict::new(py);
        let (times, epe_vals): (Vec<f64>, Vec<f64>) =
            self.inner.epe_profile.iter().copied().unzip();
        let (_, ene_vals): (Vec<f64>, Vec<f64>) = self.inner.ene_profile.iter().copied().unzip();
        let (_, pfe_vals): (Vec<f64>, Vec<f64>) = self.inner.pfe_profile.iter().copied().unzip();
        let (_, eff_epe_vals): (Vec<f64>, Vec<f64>) =
            self.inner.effective_epe_profile.iter().copied().unzip();

        data.set_item("epe", epe_vals)?;
        data.set_item("ene", ene_vals)?;
        data.set_item("pfe", pfe_vals)?;
        data.set_item("effective_epe", eff_epe_vals)?;
        let idx = times.into_pyobject(py)?.into_any();
        dict_to_dataframe(py, &data, Some(idx))
    }

    fn __repr__(&self) -> String {
        format!(
            "XvaResult(cva={:.4}, dva={:?}, fva={:?}, mva={:?}, total_xva={:.4}, max_pfe={:.2})",
            self.inner.cva,
            self.inner.dva,
            self.inner.fva,
            self.inner.mva,
            self.inner.total_xva,
            self.inner.max_pfe
        )
    }
}

// CsaTerms (XVA)

/// Credit Support Annex terms for XVA collateralization.
#[pyclass(
    name = "CsaTerms",
    module = "finstack_quant.margin",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyCsaTerms {
    pub(super) inner: xva::CsaTerms,
}

#[pymethods]
impl PyCsaTerms {
    /// Create new CSA terms.
    #[new]
    fn new(threshold: f64, mta: f64, mpor_days: u32, independent_amount: f64) -> Self {
        Self {
            inner: xva::CsaTerms {
                threshold,
                mta,
                mpor_days,
                independent_amount,
            },
        }
    }

    /// Threshold below which no collateral is required.
    #[getter]
    fn threshold(&self) -> f64 {
        self.inner.threshold
    }

    /// Minimum transfer amount.
    #[getter]
    fn mta(&self) -> f64 {
        self.inner.mta
    }

    /// Margin period of risk in calendar days.
    #[getter]
    fn mpor_days(&self) -> u32 {
        self.inner.mpor_days
    }

    /// Independent amount (initial margin).
    #[getter]
    fn independent_amount(&self) -> f64 {
        self.inner.independent_amount
    }

    fn __repr__(&self) -> String {
        format!(
            "CsaTerms(threshold={:.0}, mta={:.0}, mpor={}d, ia={:.0})",
            self.inner.threshold,
            self.inner.mta,
            self.inner.mpor_days,
            self.inner.independent_amount
        )
    }
}

// NettingSet (XVA)

/// XVA netting set: collection of trades under a single ISDA master agreement.
#[pyclass(
    name = "XvaNettingSet",
    module = "finstack_quant.margin",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyXvaNettingSet {
    pub(super) inner: xva::XvaNettingSet,
}

#[pymethods]
impl PyXvaNettingSet {
    /// Create a new XVA netting set.
    #[new]
    #[pyo3(signature = (id, counterparty_id, csa=None, reporting_currency=None))]
    fn new(
        id: &str,
        counterparty_id: &str,
        csa: Option<&PyCsaTerms>,
        reporting_currency: Option<&str>,
    ) -> PyResult<Self> {
        let ccy = match reporting_currency {
            Some(s) => {
                let c: finstack_quant_core::currency::Currency =
                    s.parse().map_err(display_to_py)?;
                Some(c)
            }
            None => None,
        };
        Ok(Self {
            inner: xva::XvaNettingSet {
                id: id.to_string(),
                counterparty_id: counterparty_id.to_string(),
                csa: csa.map(|c| c.inner.clone()),
                reporting_currency: ccy,
            },
        })
    }

    /// Netting set identifier.
    #[getter]
    fn id(&self) -> &str {
        &self.inner.id
    }

    /// Counterparty identifier.
    #[getter]
    fn counterparty_id(&self) -> &str {
        &self.inner.counterparty_id
    }

    /// Whether this netting set is collateralized.
    #[getter]
    fn is_collateralized(&self) -> bool {
        self.inner.csa.is_some()
    }

    fn __repr__(&self) -> String {
        format!(
            "XvaNettingSet(id={:?}, cpty={:?}, collateralized={})",
            self.inner.id,
            self.inner.counterparty_id,
            self.inner.csa.is_some()
        )
    }
}

// ImDecayProfile

/// Deterministic IM decay profile for MVA (Green 2015, ch. 10).
#[pyclass(
    name = "ImDecayProfile",
    module = "finstack_quant.margin",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyImDecayProfile {
    pub(super) inner: mva::ImDecayProfile,
}

#[pymethods]
impl PyImDecayProfile {
    /// IM stays at today's level for the whole horizon.
    #[staticmethod]
    fn constant() -> Self {
        Self {
            inner: mva::ImDecayProfile::Constant,
        }
    }

    /// IM decays linearly to zero at ``maturity_years``.
    #[staticmethod]
    fn linear_to_maturity(maturity_years: f64) -> PyResult<Self> {
        let inner = mva::ImDecayProfile::LinearToMaturity { maturity_years };
        inner.validate().map_err(core_to_py)?;
        Ok(Self { inner })
    }

    /// IM decays like sqrt of remaining time to ``maturity_years``.
    #[staticmethod]
    fn sqrt_time(maturity_years: f64) -> PyResult<Self> {
        let inner = mva::ImDecayProfile::SqrtTime { maturity_years };
        inner.validate().map_err(core_to_py)?;
        Ok(Self { inner })
    }

    /// Decay factor at time ``t`` (years); always in ``[0, 1]``.
    fn factor(&self, t: f64) -> f64 {
        self.inner.factor(t)
    }

    /// Deserialize from JSON.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: mva::ImDecayProfile = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize to JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner).map_err(display_to_py)
    }

    fn __repr__(&self) -> String {
        format!("ImDecayProfile({:?})", self.inner)
    }
}

// ImProfile

/// Expected initial-margin profile E[IM(t)] on a time grid.
#[pyclass(
    name = "ImProfile",
    module = "finstack_quant.margin",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyImProfile {
    pub(super) inner: mva::ImProfile,
}

#[pymethods]
impl PyImProfile {
    /// Construct from time and IM vectors.
    ///
    /// Values are stored as given; call ``validate()`` explicitly or rely
    /// on downstream functions (``compute_mva``, ``im_profile_from_simm``)
    /// to reject an inconsistent profile.
    #[new]
    fn new(times: Vec<f64>, im_values: Vec<f64>) -> Self {
        Self {
            inner: mva::ImProfile { times, im_values },
        }
    }

    /// Deserialize from JSON.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: mva::ImProfile = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize to JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner).map_err(display_to_py)
    }

    /// Validate internal consistency.
    fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(core_to_py)
    }

    /// Time points in years.
    #[getter]
    fn times(&self) -> Vec<f64> {
        self.inner.times.clone()
    }

    /// Expected IM at each time point.
    #[getter]
    fn im_values(&self) -> Vec<f64> {
        self.inner.im_values.clone()
    }

    /// Number of time points.
    fn __len__(&self) -> usize {
        self.inner.times.len()
    }

    /// Export as a pandas ``DataFrame`` with time (years) as index.
    ///
    /// Columns: ``im``.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let data = PyDict::new(py);
        data.set_item("im", &self.inner.im_values)?;
        let idx = self.inner.times.clone().into_pyobject(py)?.into_any();
        dict_to_dataframe(py, &data, Some(idx))
    }

    fn __repr__(&self) -> String {
        format!("ImProfile(points={})", self.inner.times.len())
    }
}

// MvaResult

/// Result of an MVA computation.
#[pyclass(
    name = "MvaResult",
    module = "finstack_quant.margin",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyMvaResult {
    pub(super) inner: mva::MvaResult,
}

#[pymethods]
impl PyMvaResult {
    /// Deserialize from JSON.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: mva::MvaResult = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize to JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner).map_err(display_to_py)
    }

    /// MVA (positive = lifetime funding cost of posting IM).
    #[getter]
    fn mva(&self) -> f64 {
        self.inner.mva
    }

    /// Time-weighted average IM over the profile horizon.
    #[getter]
    fn average_im(&self) -> f64 {
        self.inner.average_im
    }

    /// IM profile used, as ``(time, value)`` tuples.
    #[getter]
    fn im_profile(&self) -> Vec<(f64, f64)> {
        self.inner.im_profile.clone()
    }

    /// Export the IM profile as a pandas ``DataFrame`` (column ``im``,
    /// indexed by time in years).
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let data = PyDict::new(py);
        let (times, im_vals): (Vec<f64>, Vec<f64>) = self.inner.im_profile.iter().copied().unzip();
        data.set_item("im", im_vals)?;
        let idx = times.into_pyobject(py)?.into_any();
        dict_to_dataframe(py, &data, Some(idx))
    }

    fn __repr__(&self) -> String {
        format!(
            "MvaResult(mva={:.4}, average_im={:.2})",
            self.inner.mva, self.inner.average_im
        )
    }
}

// MVA functions

/// Build a deterministic IM profile from SIMM sensitivities:
/// ``IM(t) = SIMM(sensitivities) * decay(t)``.
#[pyfunction]
#[pyo3(signature = (calculator, sensitivities, currency, decay, time_grid))]
fn im_profile_from_simm(
    calculator: &PySimmCalculator,
    sensitivities: &PySimmSensitivities,
    currency: &str,
    decay: &PyImDecayProfile,
    time_grid: Vec<f64>,
) -> PyResult<PyImProfile> {
    let ccy: finstack_quant_core::currency::Currency = currency.parse().map_err(display_to_py)?;
    let inner = mva::im_profile_from_simm(
        &calculator.inner,
        &sensitivities.inner,
        ccy,
        &decay.inner,
        &time_grid,
    )
    .map_err(core_to_py)?;
    Ok(PyImProfile { inner })
}

/// Compute MVA: ``∫ spread(t) · IM(t) · DF(t) · S(t) dt`` (trapezoid).
///
/// Parameters
/// ----------
/// im_profile : ImProfile
///     Expected IM profile.
/// funding_spread_curve : list[tuple[float, float]]
///     ``(time_years, spread_bp)`` pairs; a single pair means a flat spread.
/// discount_curve : DiscountCurve
///     Risk-free discount curve.
/// survival_curve : HazardCurve | None
///     Optional bank (own) hazard curve; ``None`` means no survival weighting.
#[pyfunction]
#[pyo3(signature = (im_profile, funding_spread_curve, discount_curve, survival_curve=None))]
fn compute_mva(
    im_profile: &PyImProfile,
    funding_spread_curve: Vec<(f64, f64)>,
    discount_curve: &PyDiscountCurve,
    survival_curve: Option<&PyHazardCurve>,
) -> PyResult<PyMvaResult> {
    let inner = mva::compute_mva(
        &im_profile.inner,
        &funding_spread_curve,
        &discount_curve.inner,
        survival_curve.map(|c| c.inner.as_ref()),
    )
    .map_err(core_to_py)?;
    Ok(PyMvaResult { inner })
}

/// Compute bilateral XVA: CVA, DVA, FVA, MVA, and the all-in adjustment.
///
/// All legs are weighted by joint (first-to-default) survival. MVA is computed
/// only when ``funding`` carries an ``im_profile``; that posted IM also reduces
/// ENE for bilateral DVA.
///
/// The result reports ``total_xva = CVA - DVA + FVA + MVA``. Uncomputed
/// optional legs contribute zero.
///
/// Parameters
/// ----------
/// exposure_profile : ExposureProfile
///     EPE/ENE profile from exposure simulation.
/// counterparty_hazard_curve : HazardCurve
///     Hazard curve for the counterparty's credit.
/// own_hazard_curve : HazardCurve
///     Hazard curve for the institution's own credit.
/// discount_curve : DiscountCurve
///     Risk-free discount curve.
/// counterparty_recovery_rate : float
///     Recovery rate on counterparty default, in ``[0, 1]``.
/// own_recovery_rate : float
///     Recovery rate on own default, in ``[0, 1]``.
/// funding : FundingConfig | None
///     Funding configuration driving FVA and, when it carries an
///     ``im_profile``, MVA. ``None`` computes credit legs only.
///
/// Returns
/// -------
/// XvaResult
///     Bilateral credit, funding, margin, and exposure results.
///
/// Raises
/// ------
/// ValueError
///     If a profile, recovery rate, funding input, or curve evaluation is
///     invalid, including mismatched IM and exposure horizons.
#[pyfunction]
#[pyo3(signature = (
    exposure_profile,
    counterparty_hazard_curve,
    own_hazard_curve,
    discount_curve,
    counterparty_recovery_rate,
    own_recovery_rate,
    funding=None,
))]
fn compute_bilateral_xva(
    exposure_profile: &PyExposureProfile,
    counterparty_hazard_curve: &PyHazardCurve,
    own_hazard_curve: &PyHazardCurve,
    discount_curve: &PyDiscountCurve,
    counterparty_recovery_rate: f64,
    own_recovery_rate: f64,
    funding: Option<&PyFundingConfig>,
) -> PyResult<PyXvaResult> {
    let inner = cva::compute_bilateral_xva(
        &exposure_profile.inner,
        counterparty_hazard_curve.inner.as_ref(),
        own_hazard_curve.inner.as_ref(),
        &discount_curve.inner,
        counterparty_recovery_rate,
        own_recovery_rate,
        funding.map(|f| &f.inner),
    )
    .map_err(core_to_py)?;
    Ok(PyXvaResult { inner })
}

/// Register XVA classes.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyFundingConfig>()?;
    m.add_class::<PyXvaConfig>()?;
    m.add_class::<PyExposureDiagnostics>()?;
    m.add_class::<PyExposureProfile>()?;
    m.add_class::<PyXvaResult>()?;
    m.add_class::<PyCsaTerms>()?;
    m.add_class::<PyXvaNettingSet>()?;
    m.add_class::<PyImDecayProfile>()?;
    m.add_class::<PyImProfile>()?;
    m.add_class::<PyMvaResult>()?;
    m.add_function(pyo3::wrap_pyfunction!(im_profile_from_simm, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(compute_mva, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(compute_bilateral_xva, m)?)?;
    Ok(())
}
