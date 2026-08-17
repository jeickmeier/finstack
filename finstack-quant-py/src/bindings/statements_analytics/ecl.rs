//! Python bindings for Expected Credit Loss (ECL) / IFRS 9 / CECL.
//!
//! Exposes the minimum viable workflow:
//!
//! - [`PyExposure`] — a credit exposure at a reporting date.
//! - [`classify_stage`] — IFRS 9 three-stage classification with audit trail.
//! - [`compute_ecl`] — single-scenario ECL integrating marginal PD x LGD x EAD x DF.
//! - [`compute_ecl_weighted`] — probability-weighted ECL across macro scenarios.
//!
//! PD term structures are passed as ``Vec<(time_years, cumulative_pd)>`` knots
//! (wrapped by [`finstack_quant_statements_analytics::analysis::RawPdCurve`]).

use crate::bindings::pandas_utils::dict_to_dataframe;
use crate::errors::display_to_py;
use finstack_quant_statements_analytics::analysis as rust_ecl;
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Parse a stage label (case-insensitive: "stage1"/"1", "stage2"/"2", "stage3"/"3").
fn parse_stage(s: &str) -> PyResult<rust_ecl::Stage> {
    let normalized: String = s
        .chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(|c| c.to_lowercase())
        .collect();
    match normalized.as_str() {
        "stage1" | "1" | "s1" => Ok(rust_ecl::Stage::Stage1),
        "stage2" | "2" | "s2" => Ok(rust_ecl::Stage::Stage2),
        "stage3" | "3" | "s3" => Ok(rust_ecl::Stage::Stage3),
        other => Err(crate::errors::value_error(format!(
            "unknown stage '{}' (expected one of: stage1/stage2/stage3 or 1/2/3)",
            other
        ))),
    }
}

/// Render a [`rust_ecl::StagingTrigger`] as a short human-readable reason.
///
/// Delegates to the canonical `Display` implementation in the
/// `finstack-quant-statements-analytics` crate, so the reason-string contract
/// lives in exactly one place.
fn trigger_reason(trigger: &rust_ecl::StagingTrigger) -> String {
    trigger.to_string()
}

/// A single credit exposure at a reporting date.
///
/// Parameters
/// ----------
/// id : str
///     Unique identifier for the exposure.
/// ead : float
///     Drawn outstanding balance at the reporting date, in base currency.
///     Priced EAD is ``drawn + undrawn × ccf`` (core ``ead_revolver``).
/// lgd : float
///     Loss given default in decimal (0..1).
/// eir : float
///     Effective interest rate in decimal (used as IFRS 9 discount rate).
/// remaining_maturity : float
///     Remaining maturity in years.
/// current_pd : float
///     Current lifetime PD in decimal (0..1). Used as the BBB-rated curve value.
/// origination_pd : float
///     Lifetime PD at initial recognition, in decimal.
/// dpd : int | None
///     Current days past due. If omitted, the canonical request uses zero days.
/// undrawn : float
///     Undrawn commitment in the same currency as ``ead``. Default ``0.0``
///     (fully drawn term loan). Constant across the horizon.
/// ccf : float
///     Credit-conversion factor applied to ``undrawn``, as a decimal in
///     ``[0, 1]``. Default ``0.75`` (Basel IRB revolver,
///     ``DEFAULT_REVOLVER_CCF``). Unused when ``undrawn`` is zero.
#[pyclass(
    name = "Exposure",
    module = "finstack_quant.statements_analytics",
    from_py_object
)]
#[derive(Clone)]
pub struct PyExposure {
    /// Unique identifier for the exposure.
    #[pyo3(get, set)]
    pub id: String,
    /// Drawn outstanding balance at the reporting date, in the exposure's
    /// base currency. Priced EAD is ``drawn + undrawn × ccf``.
    #[pyo3(get, set)]
    pub ead: f64,
    /// Undrawn commitment at the reporting date, in the same currency as
    /// ``ead``. Default ``0.0`` (fully drawn term loan).
    #[pyo3(get, set)]
    pub undrawn: f64,
    /// Credit-conversion factor applied to ``undrawn``, as a decimal in
    /// ``[0, 1]``. Default ``0.75`` (Basel IRB revolver).
    #[pyo3(get, set)]
    pub ccf: f64,
    /// Loss given default as a decimal fraction in ``[0, 1]`` (``0.45`` =
    /// 45% loss).
    #[pyo3(get, set)]
    pub lgd: f64,
    /// Effective interest rate as a decimal fraction (``0.06`` = 6%), used as
    /// the IFRS 9 discount rate.
    #[pyo3(get, set)]
    pub eir: f64,
    /// Remaining maturity in years.
    #[pyo3(get, set)]
    pub remaining_maturity: f64,
    /// Current lifetime probability of default as a decimal fraction in
    /// ``[0, 1]``.
    #[pyo3(get, set)]
    pub current_pd: f64,
    /// Lifetime probability of default at initial recognition, as a decimal
    /// fraction in ``[0, 1]``.
    ///
    /// The SICR test compares ``current_pd`` against this origination value.
    #[pyo3(get, set)]
    pub origination_pd: f64,
    dpd: Option<u32>,
}

#[pymethods]
impl PyExposure {
    #[new]
    #[pyo3(signature = (
        id,
        ead,
        lgd,
        eir,
        remaining_maturity,
        current_pd,
        origination_pd,
        dpd=None,
        undrawn=0.0,
        ccf=rust_ecl::DEFAULT_REVOLVER_CCF,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        id: String,
        ead: f64,
        lgd: f64,
        eir: f64,
        remaining_maturity: f64,
        current_pd: f64,
        origination_pd: f64,
        dpd: Option<u32>,
        undrawn: f64,
        ccf: f64,
    ) -> Self {
        Self {
            id,
            ead,
            undrawn,
            ccf,
            lgd,
            eir,
            remaining_maturity,
            current_pd,
            origination_pd,
            dpd,
        }
    }

    /// Days past due as a whole number of days.
    ///
    /// Reads back the value the staging rules actually use: when no explicit
    /// value was supplied at construction the canonical request resolves to
    /// zero days, so this getter returns ``0`` rather than ``None``.
    #[getter]
    fn dpd(&self) -> u32 {
        self.stage_request(None, None, None)
            .resolved_days_past_due()
    }

    #[setter]
    fn set_dpd(&mut self, dpd: u32) {
        self.dpd = Some(dpd);
    }

    /// Export the exposure as a single-row pandas ``DataFrame``.
    ///
    /// Columns: ``id``, ``ead``, ``undrawn``, ``ccf``, ``lgd``, ``eir``,
    /// ``remaining_maturity``, ``current_pd``, ``origination_pd``, ``dpd``.
    ///
    /// ``ead`` and ``undrawn`` are in the exposure's base currency; ``ccf``,
    /// ``lgd``, ``current_pd`` and ``origination_pd`` are decimal fractions
    /// in ``[0, 1]``; ``eir`` is a decimal annual rate;
    /// ``remaining_maturity`` is in years; ``dpd`` is a whole number of days
    /// past due.
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let data = PyDict::new(py);
        data.set_item("id", vec![self.id.clone()])?;
        data.set_item("ead", vec![self.ead])?;
        data.set_item("undrawn", vec![self.undrawn])?;
        data.set_item("ccf", vec![self.ccf])?;
        data.set_item("lgd", vec![self.lgd])?;
        data.set_item("eir", vec![self.eir])?;
        data.set_item("remaining_maturity", vec![self.remaining_maturity])?;
        data.set_item("current_pd", vec![self.current_pd])?;
        data.set_item("origination_pd", vec![self.origination_pd])?;
        data.set_item("dpd", vec![self.dpd()])?;
        dict_to_dataframe(py, &data, None)
    }

    fn __repr__(&self) -> String {
        format!(
            "Exposure(id='{}', ead={:.2}, undrawn={:.2}, ccf={:.2}, lgd={:.4}, \
             eir={:.4}, maturity={:.2}y, current_pd={:.4}, origination_pd={:.4}, dpd={})",
            self.id,
            self.ead,
            self.undrawn,
            self.ccf,
            self.lgd,
            self.eir,
            self.remaining_maturity,
            self.current_pd,
            self.origination_pd,
            self.dpd(),
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

impl PyExposure {
    fn stage_request(
        &self,
        pd_delta_absolute: Option<f64>,
        dpd_stage2_trigger: Option<bool>,
        dpd_stage3_trigger: Option<bool>,
    ) -> rust_ecl::EclStageRequest {
        rust_ecl::EclStageRequest {
            exposure_id: self.id.clone(),
            remaining_maturity_years: self.remaining_maturity,
            current_pd: self.current_pd,
            origination_pd: self.origination_pd,
            days_past_due: self.dpd,
            pd_delta_absolute,
            dpd_stage2_trigger,
            dpd_stage3_trigger,
        }
    }
}

/// Classify an exposure into an IFRS 9 stage.
///
/// Parameters
/// ----------
/// exposure : Exposure
///     The credit exposure.
/// pd_delta_stage2 : float
///     Absolute PD increase threshold (e.g. ``0.01`` = 1pp) for SICR.
/// dpd_30_trigger : bool
///     When ``True``, ``days_past_due >= 30`` is used as a Stage 2 backstop
///     (bank / CECL alignment). Display contract:
///     ``dpd_stage2 (dpd=30 >= 30)``.
/// dpd_90_trigger : bool
///     When ``True``, ``days_past_due >= 90`` forces Stage 3 (non-rebuttable
///     backstop). Display contract: ``dpd_stage3 (dpd=90 >= 90)``.
///
/// Returns
/// -------
/// tuple[str, list[str]]
///     ``(stage, trigger_reasons)``. Stage is one of ``"Stage 1"``,
///     ``"Stage 2"``, ``"Stage 3"``. The trigger reasons list the full
///     ordered audit trail of triggers that fired (``["no_trigger"]`` for a
///     clean Stage 1), each rendered by the canonical Rust
///     ``StagingTrigger`` display format.
#[pyfunction]
#[pyo3(signature = (exposure, pd_delta_stage2=None, dpd_30_trigger=None, dpd_90_trigger=None))]
fn classify_stage(
    exposure: &PyExposure,
    pd_delta_stage2: Option<f64>,
    dpd_30_trigger: Option<bool>,
    dpd_90_trigger: Option<bool>,
) -> PyResult<(String, Vec<String>)> {
    let result = exposure
        .stage_request(pd_delta_stage2, dpd_30_trigger, dpd_90_trigger)
        .classify()
        .map_err(display_to_py)?;

    let reasons: Vec<String> = result.triggers.iter().map(trigger_reason).collect();

    Ok((result.stage.to_string(), reasons))
}

/// Compute single-scenario ECL for one exposure.
///
/// Parameters
/// ----------
/// ead : float
///     Priced exposure at default (``drawn + undrawn × ccf``). Term loans
///     pass the drawn balance; revolvers should pre-apply core
///     ``ead_revolver``.
/// pd_schedule : list[tuple[float, float]]
///     Cumulative PD curve as ``[(time_years, cumulative_pd), ...]``,
///     sorted ascending in time and monotonically non-decreasing in PD.
///     A ``(0.0, 0.0)`` knot is inserted automatically if not present.
/// lgd : float
///     Loss given default (decimal).
/// eir : float
///     Effective interest rate (decimal). Used for discounting.
/// max_horizon_years : float
///     Remaining maturity cap for the integration.
/// bucket_width_years : float | None
///     Width of each time bucket (e.g. ``0.25`` for quarterly). ``None``
///     uses the canonical IFRS 9 policy default.
/// stage : str
///     ``"stage1"`` (12-month ECL) or ``"stage2"``/``"stage3"`` (lifetime ECL).
/// ead_schedule : list[tuple[float, float]] | None
///     Optional EAD amortization profile as ``[(time_years, ead), ...]``
///     knots; linear interpolation with flat extrapolation.
/// stage3_time_to_recovery_years : float | None
///     Stage 3 discounting horizon to expected recovery, in years.
///
/// Returns
/// -------
/// float
///     ECL amount in the exposure's base currency.
///
/// Raises
/// ------
/// ValueError
///     If ``stage`` is unknown, a PD or EAD schedule is invalid, or an ECL
///     input is outside its accepted range.
#[pyfunction]
#[pyo3(signature = (ead, pd_schedule, lgd, eir, max_horizon_years, bucket_width_years=None, stage="stage1", ead_schedule=None, stage3_time_to_recovery_years=None))]
#[allow(clippy::too_many_arguments)]
fn compute_ecl(
    ead: f64,
    pd_schedule: Vec<(f64, f64)>,
    lgd: f64,
    eir: f64,
    max_horizon_years: f64,
    bucket_width_years: Option<f64>,
    stage: &str,
    ead_schedule: Option<Vec<(f64, f64)>>,
    stage3_time_to_recovery_years: Option<f64>,
) -> PyResult<f64> {
    let request = rust_ecl::EclRequest {
        exposure_id: "single".to_string(),
        ead,
        lgd,
        eir,
        remaining_maturity_years: max_horizon_years,
        stage: parse_stage(stage)?,
        scenarios: vec![(1.0, pd_schedule)],
        bucket_width_years,
        ead_schedule,
        stage3_time_to_recovery_years,
    };
    request
        .compute()
        .map(|result| result.ecl)
        .map_err(display_to_py)
}

/// Compute probability-weighted ECL across macro scenarios.
///
/// Parameters
/// ----------
/// ead : float
///     Priced exposure at default (``drawn + undrawn × ccf``). Term loans
///     pass the drawn balance; revolvers should pre-apply core
///     ``ead_revolver``.
/// scenarios : list[tuple[float, list[tuple[float, float]]]]
///     List of ``(weight, pd_schedule)`` pairs. Weights must sum to 1.0.
///     A ``(0.0, 0.0)`` knot is inserted automatically into each schedule
///     if not present (same convention as ``compute_ecl``).
/// lgd : float
///     Loss given default (decimal).
/// eir : float
///     Effective interest rate (decimal).
/// max_horizon_years : float
///     Remaining maturity cap for the integration.
/// bucket_width_years : float | None
///     Width of each time bucket (e.g. ``0.25`` for quarterly). ``None``
///     uses the canonical IFRS 9 policy default (same convention as
///     ``compute_ecl``).
/// stage : str
///     ``"stage1"``, ``"stage2"``, or ``"stage3"``.
/// ead_schedule : list[tuple[float, float]] | None
///     Optional EAD amortization profile as ``[(time_years, ead), ...]``
///     knots; linear interpolation with flat extrapolation.
/// stage3_time_to_recovery_years : float | None
///     Stage 3 discounting horizon to expected recovery, in years.
///
/// Returns
/// -------
/// float
///     Probability-weighted ECL in the exposure's base currency.
///
/// Raises
/// ------
/// ValueError
///     If ``stage`` is unknown, scenario weights do not sum to 1.0, a PD or
///     EAD schedule is invalid, or an ECL input is outside its accepted range.
#[pyfunction]
#[pyo3(signature = (ead, scenarios, lgd, eir, max_horizon_years, bucket_width_years=None, stage="stage1", ead_schedule=None, stage3_time_to_recovery_years=None))]
#[allow(clippy::too_many_arguments)]
fn compute_ecl_weighted(
    ead: f64,
    scenarios: Vec<(f64, Vec<(f64, f64)>)>,
    lgd: f64,
    eir: f64,
    max_horizon_years: f64,
    bucket_width_years: Option<f64>,
    stage: &str,
    ead_schedule: Option<Vec<(f64, f64)>>,
    stage3_time_to_recovery_years: Option<f64>,
) -> PyResult<f64> {
    let request = rust_ecl::EclRequest {
        exposure_id: "weighted".to_string(),
        ead,
        lgd,
        eir,
        remaining_maturity_years: max_horizon_years,
        stage: parse_stage(stage)?,
        scenarios,
        bucket_width_years,
        ead_schedule,
        stage3_time_to_recovery_years,
    };
    request
        .compute()
        .map(|result| result.ecl)
        .map_err(display_to_py)
}

/// Register ECL types and functions on the `statements_analytics` submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyExposure>()?;
    m.add_function(pyo3::wrap_pyfunction!(classify_stage, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(compute_ecl, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(compute_ecl_weighted, m)?)?;
    Ok(())
}
