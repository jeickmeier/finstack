//! Python bindings for the calibration engine.
//!
//! Wraps [`finstack_quant_calibration::api::engine::execute`] behind
//! a JSON-in / rich-result-out API that matches the existing scenarios-engine
//! binding pattern.

mod schema;

use crate::bindings::core::market_data::context::PyMarketContext;
use crate::bindings::pandas_utils::dict_to_dataframe;
use crate::errors::display_to_py;
use finstack_quant_calibration::api::engine::{self, ExecuteError};
use finstack_quant_calibration::api::schema::CalibrationResultEnvelope;
use finstack_quant_calibration::api::validate as validate_api;
use finstack_quant_core::contract::LoadLimits;
use finstack_quant_core::market_data::context::MarketContext;
use numpy::PyArray1;
use pyo3::create_exception;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString};
use std::collections::HashMap;
use std::sync::OnceLock;

create_exception!(
    finstack_quant.calibration,
    CalibrationEnvelopeError,
    PyRuntimeError,
    "Raised when calibration ingestion, validation, context construction, or solving fails.\n\n\
     Carries `kind`, `stage`, `step_id`, `solver_diagnostics`, and `details` \
     attributes for programmatic handling."
);

/// Map every execution stage to the same structured Python exception contract.
fn execute_error_to_py(py: Python<'_>, err: ExecuteError) -> PyErr {
    let details = err.details();
    let details_json = err.to_json();
    let solver_diagnostics = match details.solver_diagnostics.as_ref() {
        Some(diagnostics) => match serde_json::to_string(diagnostics) {
            Ok(json) => Some(json),
            Err(serialization_err) => {
                return PyRuntimeError::new_err(format!(
                    "failed to serialize solver diagnostics for CalibrationEnvelopeError: \
                     {serialization_err}; underlying calibration error: {}",
                    details.cause
                ));
            }
        },
        None => None,
    };
    let exc = CalibrationEnvelopeError::new_err(details.cause.clone());
    let value = exc.value(py);
    let attrs: [(&str, PyResult<()>); 5] = [
        ("kind", value.setattr("kind", details.category.clone())),
        ("stage", value.setattr("stage", details.stage.as_str())),
        ("details", value.setattr("details", details_json)),
        ("step_id", value.setattr("step_id", details.step_id.clone())),
        (
            "solver_diagnostics",
            value.setattr("solver_diagnostics", solver_diagnostics),
        ),
    ];
    for (name, result) in attrs {
        if let Err(setattr_err) = result {
            return PyRuntimeError::new_err(format!(
                "failed to attach '{name}' attribute to CalibrationEnvelopeError \
                 ({}): underlying calibration error: {}",
                setattr_err.value(py),
                details.cause
            ));
        }
    }
    exc
}

/// Result of a calibration plan execution.
///
/// Provides access to the calibrated market context, per-step reports,
/// and overall success status.
#[pyclass(
    name = "CalibrationResult",
    module = "finstack_quant.calibration",
    skip_from_py_object
)]
pub struct PyCalibrationResult {
    inner: CalibrationResultEnvelope,
    cached_json: OnceLock<String>,
    cached_market_json: OnceLock<String>,
    cached_report_json: OnceLock<String>,
    cached_step_reports: OnceLock<HashMap<String, String>>,
}

impl Clone for PyCalibrationResult {
    fn clone(&self) -> Self {
        Self::new(self.inner.clone())
    }
}

impl PyCalibrationResult {
    fn new(inner: CalibrationResultEnvelope) -> Self {
        Self {
            inner,
            cached_json: OnceLock::new(),
            cached_market_json: OnceLock::new(),
            cached_report_json: OnceLock::new(),
            cached_step_reports: OnceLock::new(),
        }
    }
}

fn cached_json<'py, F>(
    py: Python<'py>,
    cache: &OnceLock<String>,
    serialize: F,
) -> PyResult<Bound<'py, PyString>>
where
    F: FnOnce() -> serde_json::Result<String>,
{
    if let Some(value) = cache.get() {
        return Ok(PyString::new(py, value));
    }
    let value = serialize().map_err(display_to_py)?;
    let py_value = PyString::new(py, &value);
    let _ = cache.set(value);
    Ok(py_value)
}

#[pymethods]
impl PyCalibrationResult {
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<(Bound<'py, PyAny>, (Bound<'py, PyString>,))> {
        let payload = self.to_json(py)?;
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        Ok((from_json, (payload,)))
    }

    /// Deserialize from a JSON string.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let (inner, _report) =
            CalibrationResultEnvelope::from_slice_strict(json.as_bytes(), &LoadLimits::default())
                .map_err(display_to_py)?;
        Ok(Self::new(inner))
    }

    /// Serialize to a compact JSON string.
    ///
    /// Returns a cached Python `str`: the JSON is rendered once and reused on
    /// subsequent calls, so repeated access is allocation-free.
    fn to_json<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyString>> {
        cached_json(py, &self.cached_json, || serde_json::to_string(&self.inner))
    }

    /// Whether the overall calibration succeeded (all steps passed fitting and validation).
    #[getter]
    fn success(&self) -> bool {
        self.inner.result.report.success
    }

    /// The calibrated ``MarketContext`` containing all produced curves and surfaces.
    #[getter]
    fn market(&self) -> PyResult<PyMarketContext> {
        let ctx = MarketContext::try_from(self.inner.result.final_market.clone())
            .map_err(display_to_py)?;
        Ok(PyMarketContext::from_inner(ctx))
    }

    /// The calibrated market serialized as a JSON string.
    ///
    /// Validates the state through the same ``MarketContext`` conversion as
    /// the ``market`` getter before serializing, so both representations
    /// fail identically on an invalid state instead of the JSON form
    /// silently diverging from the validated object form.
    #[getter]
    fn market_json<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyString>> {
        MarketContext::try_from(self.inner.result.final_market.clone()).map_err(display_to_py)?;
        cached_json(py, &self.cached_market_json, || {
            serde_json::to_string(&self.inner.result.final_market)
        })
    }

    /// The aggregated calibration report as a JSON string.
    #[getter]
    fn report_json<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyString>> {
        cached_json(py, &self.cached_report_json, || {
            serde_json::to_string(&self.inner.result.report)
        })
    }

    /// List of step identifiers in lexicographic step-ID order.
    #[getter]
    fn step_ids(&self) -> Vec<String> {
        self.inner.result.step_reports.keys().cloned().collect()
    }

    /// Number of solver iterations across all steps.
    #[getter]
    fn iterations(&self) -> usize {
        self.inner.result.report.iterations
    }

    /// Maximum absolute `|residual| / step_tolerance` ratio across all steps.
    #[getter]
    fn max_residual(&self) -> f64 {
        self.inner.result.report.max_residual
    }

    /// Root mean square `|residual| / step_tolerance` ratio across all steps.
    #[getter]
    fn rmse(&self) -> f64 {
        self.inner.result.report.rmse
    }

    /// Per-step calibration report as a JSON string.
    ///
    /// Parameters
    /// ----------
    /// step_id : str
    ///     Identifier of the calibration step.
    ///
    /// Returns
    /// -------
    /// str
    ///     JSON-serialized calibration report for the step.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If no step with the given *step_id* exists.
    fn step_report_json<'py>(
        &self,
        py: Python<'py>,
        step_id: &str,
    ) -> PyResult<Bound<'py, PyString>> {
        if self.cached_step_reports.get().is_none() {
            let mut reports = HashMap::with_capacity(self.inner.result.step_reports.len());
            for (id, report) in &self.inner.result.step_reports {
                reports.insert(
                    id.clone(),
                    serde_json::to_string(report).map_err(display_to_py)?,
                );
            }
            let _ = self.cached_step_reports.set(reports);
        }

        self.cached_step_reports
            .get()
            .and_then(|reports| reports.get(step_id))
            .map(|report| PyString::new(py, report))
            .ok_or_else(|| crate::errors::value_error(format!("No step report for '{step_id}'")))
    }

    /// Export the per-step summary as a pandas ``DataFrame``.
    ///
    /// Columns: ``step_id``, ``success``, ``iterations``, ``max_residual``,
    /// ``rmse``, ``convergence_reason``. Rows are ordered lexicographically by
    /// step ID because the result contract stores reports in a ``BTreeMap``.
    /// The plan-level roll-ups (``success``, ``iterations``, ``max_residual``,
    /// ``rmse``) are getters on the result and are not repeated per row.
    #[pyo3(text_signature = "($self)")]
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let n = self.inner.result.step_reports.len();
        let mut ids: Vec<String> = Vec::with_capacity(n);
        let mut successes: Vec<bool> = Vec::with_capacity(n);
        let mut iters: Vec<usize> = Vec::with_capacity(n);
        let mut max_res: Vec<f64> = Vec::with_capacity(n);
        let mut rmses: Vec<f64> = Vec::with_capacity(n);
        let mut reasons: Vec<String> = Vec::with_capacity(n);

        for (id, report) in &self.inner.result.step_reports {
            ids.push(id.clone());
            successes.push(report.success);
            iters.push(report.iterations);
            max_res.push(report.max_residual);
            rmses.push(report.rmse);
            reasons.push(report.convergence_reason.clone());
        }

        let data = PyDict::new(py);
        data.set_item("step_id", ids)?;
        data.set_item("success", successes)?;
        data.set_item("iterations", iters)?;
        data.set_item("max_residual", PyArray1::from_vec(py, max_res).into_any())?;
        data.set_item("rmse", PyArray1::from_vec(py, rmses).into_any())?;
        data.set_item("convergence_reason", reasons)?;
        dict_to_dataframe(py, &data, None)
    }

    fn __repr__(&self) -> String {
        let n = self.inner.result.step_reports.len();
        format!(
            "CalibrationResult(success={}, steps={n}, iterations={}, max_residual={:.2e})",
            self.inner.result.report.success,
            self.inner.result.report.iterations,
            self.inner.result.report.max_residual,
        )
    }
}

/// Validate a calibration plan JSON and return the canonical (pretty-printed) form.
///
/// Parameters
/// ----------
/// json : str
///     JSON-serialized ``CalibrationEnvelope``.
///
/// Returns
/// -------
/// str
///     Canonical pretty-printed JSON.
///
/// Raises
/// ------
/// CalibrationEnvelopeError
///     If strict loading or static validation rejects the calibration envelope.
///     Static validation is fail-fast (first error); use ``dry_run`` to list
///     every static error.
#[pyfunction]
fn validate_calibration_json(py: Python<'_>, json: &str) -> PyResult<String> {
    validate_api::validate_calibration_json(json)
        .map_err(ExecuteError::from)
        .map_err(|error| execute_error_to_py(py, error))
}

/// Pre-flight envelope validation without invoking the solver.
///
/// Parameters
/// ----------
/// json : str
///     JSON-serialized ``CalibrationEnvelope``.
///
/// Returns
/// -------
/// str
///     Pretty-printed JSON ``CalibrationValidationReport`` with all errors found in a
///     single pass plus the dependency graph.
///
/// Raises
/// ------
/// CalibrationEnvelopeError
///     If the envelope JSON is malformed.
#[pyfunction]
fn dry_run(py: Python<'_>, json: &str) -> PyResult<String> {
    validate_api::dry_run(json)
        .map_err(ExecuteError::from)
        .map_err(|error| execute_error_to_py(py, error))
}

/// Execute a calibration plan and return the full result.
///
/// Parameters
/// ----------
/// json : str
///     JSON-serialized ``CalibrationEnvelope`` containing the plan,
///     quote sets, and optional initial market state.
///
/// Returns
/// -------
/// CalibrationResult
///     The calibration result with calibrated market, reports, and diagnostics.
///
/// Raises
/// ------
/// CalibrationEnvelopeError
///     If ingestion, validation, context construction, target construction,
///     solving, or final fit acceptance fails. Static validation is fail-fast
///     (first error); use ``dry_run`` to list every static error.
#[pyfunction]
fn calibrate(py: Python<'_>, json: &str) -> PyResult<PyCalibrationResult> {
    let envelope = validate_api::parse_envelope(json)
        .map_err(ExecuteError::from)
        .map_err(|error| execute_error_to_py(py, error))?;
    // Release the GIL for the duration of the solver: calibration can run for seconds.
    // The error is boxed inside the closure: `ExecuteError` is a large enum, and
    // an un-boxed large `Err` variant on the `detach` closure trips
    // `clippy::result_large_err`.
    let result = py
        .detach(|| engine::execute(&envelope).map_err(Box::new))
        .map_err(|e| execute_error_to_py(py, *e))?;
    Ok(PyCalibrationResult::new(result))
}

/// Calibrate the explicit Bermudan LMM loading scale from the market surface.
///
/// Parameters
/// ----------
/// instrument_json : str
///     Canonical Bermudan swaption instrument envelope.
/// market : MarketContext | str
///     Market carrying the required discount and swaption-volatility inputs.
/// as_of : datetime.date | str
///     Valuation date used for tenor and expiry construction.
///
/// Returns
/// -------
/// float
///     Positive finite LMM base volatility to place in
///     ``model_config.lmm_base_vol``.
///
/// Raises
/// ------
/// ValueError
///     If the instrument is not a Bermudan swaption or inputs are invalid.
/// RuntimeError
///     If market lookup or calibration fails.
#[pyfunction]
fn calibrate_bermudan_lmm_base_vol(
    py: Python<'_>,
    instrument_json: &str,
    market: &Bound<'_, PyAny>,
    as_of: &Bound<'_, PyAny>,
) -> PyResult<f64> {
    let instrument_json = instrument_json.to_owned();
    let market = crate::bindings::extract::extract_market(py, market)?;
    let as_of = crate::bindings::date_utils::extract_date(as_of)?;
    py.detach(move || {
        finstack_quant_calibration::calibrate_bermudan_lmm_base_vol_from_json(
            &instrument_json,
            &market,
            as_of,
        )
    })
    .map_err(crate::errors::core_to_py)
}

/// Register the root-level calibration submodule.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "calibration")?;
    m.add_class::<PyCalibrationResult>()?;
    m.add(
        "CalibrationEnvelopeError",
        py.get_type::<CalibrationEnvelopeError>(),
    )?;
    m.add_function(pyo3::wrap_pyfunction!(validate_calibration_json, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(calibrate, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(dry_run, &m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(calibrate_bermudan_lmm_base_vol, &m)?)?;
    schema::register(py, &m)?;
    m.setattr(
        "__all__",
        PyList::new(
            py,
            [
                "CalibrationEnvelopeError",
                "CalibrationResult",
                "calibrate",
                "validate_calibration_json",
                "dry_run",
                "calibrate_bermudan_lmm_base_vol",
                "schema",
            ],
        )?,
    )?;
    crate::bindings::module_utils::register_submodule(
        py,
        parent,
        &m,
        "calibration",
        crate::bindings::module_utils::ROOT_PACKAGE,
        crate::bindings::module_utils::ParentNameSource::Name,
    )?;
    Ok(())
}
