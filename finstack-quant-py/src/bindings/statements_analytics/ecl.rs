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

use crate::errors::display_to_py;
use finstack_quant_statements_analytics::analysis as rust_ecl;
use pyo3::prelude::*;

// Helpers

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
fn trigger_reason(trigger: &rust_ecl::StagingTrigger) -> String {
    match trigger {
        rust_ecl::StagingTrigger::DpdStage3 { dpd, threshold } => {
            format!("dpd_stage3 (dpd={} > {})", dpd, threshold)
        }
        rust_ecl::StagingTrigger::DpdStage2 { dpd, threshold } => {
            format!("dpd_stage2 (dpd={} > {})", dpd, threshold)
        }
        rust_ecl::StagingTrigger::PdDeltaAbsolute { delta, threshold } => {
            format!("pd_delta_absolute (delta={:.4} > {:.4})", delta, threshold)
        }
        rust_ecl::StagingTrigger::PdDeltaRelative { ratio, threshold } => {
            format!(
                "pd_delta_relative (ratio={:.2}x > {:.2}x)",
                ratio, threshold
            )
        }
        rust_ecl::StagingTrigger::RatingDowngrade { notches, threshold } => {
            format!("rating_downgrade ({} >= {} notches)", notches, threshold)
        }
        rust_ecl::StagingTrigger::Qualitative { flag } => format!("qualitative:{}", flag),
        rust_ecl::StagingTrigger::Stage3Qualitative { flag } => {
            format!("stage3_qualitative:{}", flag)
        }
        rust_ecl::StagingTrigger::NoTrigger => "no_trigger".to_string(),
    }
}

// PyExposure

/// A single credit exposure at a reporting date.
///
/// Parameters
/// ----------
/// id : str
///     Unique identifier for the exposure.
/// ead : float
///     Exposure at default (drawn balance), in base currency.
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
#[pyclass(
    name = "Exposure",
    module = "finstack_quant.statements_analytics",
    from_py_object
)]
#[derive(Clone)]
pub struct PyExposure {
    #[pyo3(get, set)]
    pub id: String,
    #[pyo3(get, set)]
    pub ead: f64,
    #[pyo3(get, set)]
    pub lgd: f64,
    #[pyo3(get, set)]
    pub eir: f64,
    #[pyo3(get, set)]
    pub remaining_maturity: f64,
    #[pyo3(get, set)]
    pub current_pd: f64,
    #[pyo3(get, set)]
    pub origination_pd: f64,
    dpd: Option<u32>,
}

#[pymethods]
impl PyExposure {
    #[new]
    #[pyo3(signature = (id, ead, lgd, eir, remaining_maturity, current_pd, origination_pd, dpd=None))]
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
    ) -> Self {
        Self {
            id,
            ead,
            lgd,
            eir,
            remaining_maturity,
            current_pd,
            origination_pd,
            dpd,
        }
    }

    #[getter]
    fn dpd(&self) -> u32 {
        self.stage_request(None, None, None)
            .resolved_days_past_due()
    }

    #[setter]
    fn set_dpd(&mut self, dpd: u32) {
        self.dpd = Some(dpd);
    }

    fn __repr__(&self) -> String {
        format!(
            "Exposure(id='{}', ead={:.2}, lgd={:.4}, eir={:.4}, maturity={:.2}y, \
             current_pd={:.4}, origination_pd={:.4}, dpd={})",
            self.id,
            self.ead,
            self.lgd,
            self.eir,
            self.remaining_maturity,
            self.current_pd,
            self.origination_pd,
            self.dpd(),
        )
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

// Staging

/// Classify an exposure into an IFRS 9 stage.
///
/// Parameters
/// ----------
/// exposure : Exposure
///     The credit exposure.
/// pd_delta_stage2 : float
///     Absolute PD increase threshold (e.g. ``0.01`` = 1pp) for SICR.
/// dpd_30_trigger : bool
///     When ``True``, DPD > 30 is used as a Stage 2 backstop (IFRS 9 B5.5.19).
/// dpd_90_trigger : bool
///     When ``True``, DPD > 90 forces Stage 3 (non-rebuttable backstop).
///
/// Returns
/// -------
/// tuple[str, str]
///     ``(stage, trigger_reason)``. Stage is one of ``"Stage 1"``,
///     ``"Stage 2"``, ``"Stage 3"``. The trigger reason describes the first
///     trigger that fired (or ``"no_trigger"`` for a clean Stage 1).
#[pyfunction]
#[pyo3(signature = (exposure, pd_delta_stage2=None, dpd_30_trigger=None, dpd_90_trigger=None))]
fn classify_stage(
    exposure: &PyExposure,
    pd_delta_stage2: Option<f64>,
    dpd_30_trigger: Option<bool>,
    dpd_90_trigger: Option<bool>,
) -> PyResult<(String, String)> {
    let result = exposure
        .stage_request(pd_delta_stage2, dpd_30_trigger, dpd_90_trigger)
        .classify()
        .map_err(display_to_py)?;

    let reason = result
        .triggers
        .first()
        .map(trigger_reason)
        .unwrap_or_else(|| "no_trigger".to_string());

    Ok((result.stage.to_string(), reason))
}

// ECL computation

/// Compute single-scenario ECL for one exposure.
///
/// Parameters
/// ----------
/// ead : float
///     Exposure at default.
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
/// bucket_width_years : float
///     Width of each time bucket (e.g. ``0.25`` for quarterly).
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
///     Exposure at default.
/// scenarios : list[tuple[float, list[tuple[float, float]]]]
///     List of ``(weight, pd_schedule)`` pairs. Weights must sum to 1.0.
///     A ``(0.0, 0.0)`` knot is inserted automatically into each schedule
///     if not present (same convention as ``compute_ecl``).
/// lgd : float
///     Loss given default (decimal).
/// eir : float
///     Effective interest rate (decimal).
/// max_horizon : float
///     Remaining maturity cap.
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
#[pyfunction]
#[pyo3(signature = (ead, scenarios, lgd, eir, max_horizon, stage="stage1", ead_schedule=None, stage3_time_to_recovery_years=None))]
#[allow(clippy::too_many_arguments)]
fn compute_ecl_weighted(
    ead: f64,
    scenarios: Vec<(f64, Vec<(f64, f64)>)>,
    lgd: f64,
    eir: f64,
    max_horizon: f64,
    stage: &str,
    ead_schedule: Option<Vec<(f64, f64)>>,
    stage3_time_to_recovery_years: Option<f64>,
) -> PyResult<f64> {
    let request = rust_ecl::EclRequest {
        exposure_id: "weighted".to_string(),
        ead,
        lgd,
        eir,
        remaining_maturity_years: max_horizon,
        stage: parse_stage(stage)?,
        scenarios,
        bucket_width_years: None,
        ead_schedule,
        stage3_time_to_recovery_years,
    };
    request
        .compute()
        .map(|result| result.ecl)
        .map_err(display_to_py)
}

// Registration

/// Register ECL types and functions on the `statements_analytics` submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyExposure>()?;
    m.add_function(pyo3::wrap_pyfunction!(classify_stage, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(compute_ecl, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(compute_ecl_weighted, m)?)?;
    Ok(())
}
