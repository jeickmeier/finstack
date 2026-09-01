//! P&L attribution entry points and JSON helpers.

use crate::bindings::attribution::pnl_attribution::PyPnlAttribution;
use crate::bindings::attribution::return_contribution::PyReturnContributionResult;
use crate::bindings::module_utils::py_to_json_string;
use crate::errors::{core_to_py, display_to_py, serde_json_to_py};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

/// Extract a human-readable message from a caught panic payload.
fn panic_message(panic: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = panic.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = panic.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic payload".to_string()
    }
}

/// Run an attribution computation with the GIL released, converting a Rust
/// panic into a catchable `RuntimeError` instead of letting it unwind as a
/// `pyo3_runtime.PanicException` (a `BaseException` that escapes
/// ``except Exception`` handlers). Mirrors the WASM binding's
/// `catch_attribution_panic`.
fn detach_catch_attribution_panic<T: Send>(
    py: Python<'_>,
    label: &str,
    f: impl FnOnce() -> Result<T, finstack_quant_core::Error> + Send,
) -> PyResult<T> {
    match py.detach(|| std::panic::catch_unwind(std::panic::AssertUnwindSafe(f))) {
        Ok(result) => result.map_err(core_to_py),
        Err(panic) => Err(PyRuntimeError::new_err(format!(
            "internal panic in attribution ({label}): {}",
            panic_message(panic.as_ref())
        ))),
    }
}

/// Run P&L attribution for a single instrument.
///
/// This is the main entry point. It accepts the instrument, two market
/// snapshots, valuation dates, and a method descriptor — all as simple
/// Python objects — and returns a typed :class:`PnlAttribution`. Call
/// ``.to_json()`` on the result for the canonical JSON form.
///
/// Parameters
/// ----------
/// instrument_json : str
///     Canonical v1 instrument envelope
///     (``{"schema": "finstack_quant.instrument/1", "instrument": {...}}``).
/// market_t0_json : str
///     JSON-serialized ``MarketContext`` at T₀.
/// market_t1_json : str
///     JSON-serialized ``MarketContext`` at T₁.
/// as_of_t0 : datetime.date | str
///     Valuation date T₀, either a date-like object (``datetime.date``,
///     ``pandas.Timestamp``) or an ISO 8601 calendar date (``YYYY-MM-DD``).
///     A tz-aware timestamp contributes its wall-clock calendar date; for
///     time-of-day-sensitive workflows pass the start-of-day date in UTC.
/// as_of_t1 : datetime.date | str
///     Valuation date T₁, in the same forms as ``as_of_t0``.
/// method : str | dict
///     Attribution method. One of:
///
///     * ``"parallel"``
///     * ``{"waterfall": ["carry", "rates_curves", ...]}``
///     * ``"metrics_based"``
///     * ``{"taylor": {"include_gamma": true, ...}}``
/// config : dict, optional
///     Optional attribution config overrides (tolerance, metrics, bump sizes).
/// full_cross_attribution : bool, optional
///     When true, the parallel method evaluates every pairwise cross-factor
///     interaction (full cross matrix) instead of the default seven economic
///     pairs. More repricings, smaller residual.
/// model_params_t0_json : str, optional
///     Serialized opening ``ModelParamsSnapshot``. When omitted, model-
///     parameter P&L is isolated from the instrument's current snapshot.
/// credit_factor_model_json : str, optional
///     Serialized ``CreditFactorModel``. When supplied, credit-factor
///     hierarchy detail is populated on the result.
///
/// Returns
/// -------
/// PnlAttribution
///     Typed attribution result. Use ``.to_json()`` for the wire form and
///     ``.to_dataframe()`` for a pandas view.
///
/// Examples
/// --------
/// >>> attr = attribute_pnl(inst, mkt_t0, mkt_t1, "2025-01-15", "2025-01-16", "parallel")
/// >>> print(attr.explain())
/// >>> attr.to_dataframe()
#[pyfunction]
#[pyo3(signature = (instrument_json, market_t0_json, market_t1_json, as_of_t0, as_of_t1, method, config=None, full_cross_attribution=None, model_params_t0_json=None, credit_factor_model_json=None))]
#[allow(clippy::too_many_arguments)]
pub(crate) fn attribute_pnl(
    py: Python<'_>,
    instrument_json: &str,
    market_t0_json: &str,
    market_t1_json: &str,
    as_of_t0: &Bound<'_, PyAny>,
    as_of_t1: &Bound<'_, PyAny>,
    method: &Bound<'_, PyAny>,
    config: Option<&Bound<'_, PyAny>>,
    full_cross_attribution: Option<bool>,
    model_params_t0_json: Option<&str>,
    credit_factor_model_json: Option<&str>,
) -> PyResult<PyPnlAttribution> {
    let as_of_t0 = crate::bindings::date_utils::extract_date_iso(as_of_t0)?;
    let as_of_t1 = crate::bindings::date_utils::extract_date_iso(as_of_t1)?;
    let method_json = py_to_json_string(py, method, "method")?;
    let config_json = config
        .map(|value| py_to_json_string(py, value, "config"))
        .transpose()?;
    // Parsing reconstructs instruments and markets and can panic on
    // pathological payloads (e.g. `Money::new` on a non-finite amount), so it
    // is guarded like the WASM twin's `attributePnl/from_json_inputs` wrap.
    let spec = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        finstack_quant_attribution::AttributionSpec::from_json_inputs(
            finstack_quant_attribution::AttributionJsonInputs {
                instrument_json,
                market_t0_json,
                market_t1_json,
                as_of_t0: &as_of_t0,
                as_of_t1: &as_of_t1,
                method_json: &method_json,
                config_json: config_json.as_deref(),
                model_params_t0_json,
                credit_factor_model_json,
                full_cross_attribution: full_cross_attribution.unwrap_or(false),
            },
        )
    })) {
        Ok(result) => result.map_err(core_to_py)?,
        Err(panic) => {
            return Err(PyRuntimeError::new_err(format!(
                "internal panic in attribution (attribute_pnl/from_json_inputs): {}",
                panic_message(panic.as_ref())
            )))
        }
    };

    let result = detach_catch_attribution_panic(py, "attribute_pnl", || spec.execute())?;
    Ok(PyPnlAttribution {
        inner: result.attribution,
    })
}

/// Run attribution from a full JSON ``AttributionEnvelope`` and return JSON.
///
/// This is the raw JSON round-trip variant. Most users should prefer
/// :func:`attribute_pnl`, which accepts separate arguments and returns a
/// typed :class:`PnlAttribution`.
///
/// Parameters
/// ----------
/// spec_json : str
///     JSON-serialized ``AttributionEnvelope``.
///
/// Returns
/// -------
/// str
///     JSON-serialized ``AttributionResultEnvelope``.
#[pyfunction]
pub(crate) fn attribute_pnl_envelope_json(py: Python<'_>, spec_json: &str) -> PyResult<String> {
    use finstack_quant_attribution::AttributionEnvelope;

    let envelope: AttributionEnvelope = serde_json::from_str(spec_json)
        .map_err(|e| serde_json_to_py(e, "invalid attribution envelope JSON"))?;
    let result_envelope =
        detach_catch_attribution_panic(py, "attribute_pnl_envelope_json", || envelope.execute())?;
    serde_json::to_string(&result_envelope).map_err(display_to_py)
}

/// Compute single-period return contribution attribution from JSON.
///
/// Parameters
/// ----------
/// spec_json : str
///     JSON-serialized return contribution specification.
///
/// Returns
/// -------
/// ReturnContributionResult
///     Typed result. Use ``.to_json()`` for the wire form,
///     ``.to_dataframe()`` for per-instrument rows, and ``.to_series()`` for
///     contributions indexed by instrument id.
#[pyfunction]
pub(crate) fn attribute_return_contribution(
    spec_json: &str,
) -> PyResult<PyReturnContributionResult> {
    let spec: finstack_quant_attribution::ReturnContributionSpec = serde_json::from_str(spec_json)
        .map_err(|err| serde_json_to_py(err, "invalid return contribution JSON"))?;
    let inner =
        finstack_quant_attribution::attribute_return_contribution(&spec).map_err(core_to_py)?;
    Ok(PyReturnContributionResult { inner })
}

/// Validate an attribution specification JSON.
///
/// Deserializes the input against the ``AttributionEnvelope`` schema,
/// checks the ``schema`` version tag (the same gate ``execute`` applies, so
/// a payload that validates here cannot later be rejected at execution),
/// and returns the canonical (re-serialized) JSON.
///
/// Parameters
/// ----------
/// json : str
///     JSON-serialized ``AttributionEnvelope``.
///
/// Returns
/// -------
/// str
///     Canonical compact JSON.
#[pyfunction]
pub(crate) fn validate_attribution_json(json: &str) -> PyResult<String> {
    finstack_quant_attribution::validate_attribution_json(json).map_err(core_to_py)
}

/// Validate a return contribution specification JSON.
///
/// Parameters
/// ----------
/// spec_json : str
///     JSON-serialized return contribution specification.
///
/// Returns
/// -------
/// str
///     Canonical compact JSON.
#[pyfunction]
pub(crate) fn validate_return_contribution_json(spec_json: &str) -> PyResult<String> {
    finstack_quant_attribution::validate_return_contribution_json(spec_json).map_err(core_to_py)
}

/// Return the default waterfall factor ordering.
///
/// Returns
/// -------
/// list[str]
///     Canonical snake-case factor names in the default waterfall order.
#[pyfunction]
pub(crate) fn default_waterfall_order() -> Vec<String> {
    finstack_quant_attribution::default_waterfall_order()
        .into_iter()
        .map(|factor| factor.as_str().to_owned())
        .collect()
}

/// Return the default metric IDs used by metrics-based attribution.
///
/// Returns
/// -------
/// list[str]
///     Metric identifier strings.
#[pyfunction]
pub(crate) fn default_attribution_metrics() -> Vec<String> {
    finstack_quant_attribution::default_attribution_metrics()
        .into_iter()
        .map(|m| m.to_string())
        .collect()
}
