//! Python bindings for the comparable company analysis module.
//!
//! Exposes a function-based API for cross-sectional peer analytics:
//!
//! - Descriptive peer statistics (`peer_stats`).
//! - Percentile rank and z-score of a subject within a peer distribution.
//! - Single-factor OLS regression for fair-value estimation.
//! - Canonical valuation multiple computation on `CompanyMetrics`.
//! - Multi-dimension composite rich/cheap scoring (`score_relative_value`).
//!
//! `score_relative_value` takes the canonical serde forms of the Rust
//! `PeerSet` and `ScoringDimension` types — as JSON strings or plain
//! dicts/lists with the same shape — exactly like the WASM twin.

use finstack_quant_statements_analytics::analysis::{
    compute_multiple as core_compute_multiple, peer_stats as core_peer_stats,
    percentile_rank as core_percentile_rank, regression_fair_value as core_regression,
    score_relative_value as core_score, z_score as core_z_score, CompanyMetrics, Multiple, PeerSet,
    ScoringDimension,
};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict};

use crate::bindings::pandas_utils::serde_to_py;
use crate::errors::{core_to_py, display_to_py};

/// Deserialize a canonical serde payload from a JSON string or a plain
/// Python object (dict/list) with the same shape.
fn extract_serde<'py, T: serde::de::DeserializeOwned + Send>(
    py: Python<'py>,
    obj: &Bound<'py, PyAny>,
    label: &str,
) -> PyResult<T> {
    if let Ok(json) = obj.extract::<String>() {
        return serde_json::from_str(&json)
            .map_err(|e| crate::errors::serde_json_to_py(e, &format!("invalid {label}")));
    }
    crate::bindings::module_utils::py_to_serde(py, obj, label)
}

/// Percentile rank of ``value`` within ``peer_values`` (0-1 scale).
///
/// Uses the "fraction of values less than or equal" convention. Returns
/// ``None`` when ``peer_values`` is empty.
///
/// Arguments:
///     value: The subject value to rank.
///     peer_values: Peer distribution (need not be sorted).
///
/// Returns:
///     Percentile rank in [0, 1], or ``None`` when ``peer_values`` is empty.
#[pyfunction]
#[pyo3(text_signature = "(value, peer_values)")]
fn percentile_rank(value: f64, peer_values: Vec<f64>) -> Option<f64> {
    core_percentile_rank(&peer_values, value)
}

/// Standard (z-) score of ``value`` in the peer distribution.
///
/// Returns ``None`` if fewer than two peers are provided or the peer
/// distribution has zero variance.
///
/// Arguments:
///     value: The subject value.
///     peer_values: Peer distribution.
///
/// Returns:
///     ``(value - mean(peers)) / stddev(peers)``, or ``None`` when undefined.
#[pyfunction]
#[pyo3(text_signature = "(value, peer_values)")]
fn z_score(value: f64, peer_values: Vec<f64>) -> Option<f64> {
    core_z_score(&peer_values, value)
}

/// Descriptive statistics for a peer distribution.
///
/// Arguments:
///     peer_values: Peer distribution (need not be sorted).
///
/// Returns:
///     Dict with keys ``{"mean", "median", "q1", "q3", "iqr", "std_dev",
///     "min", "max", "count"}`` mirroring the Rust ``PeerStats`` field
///     names (serde form). Returns ``None`` when no statistics can be
///     computed (matching the WASM twin's ``undefined``).
#[pyfunction]
#[pyo3(text_signature = "(peer_values)")]
fn peer_stats<'py>(py: Python<'py>, peer_values: Vec<f64>) -> PyResult<Option<Bound<'py, PyAny>>> {
    match core_peer_stats(&peer_values) {
        Some(stats) => serde_to_py(py, &stats).map(Some),
        None => Ok(None),
    }
}

/// Single-factor OLS fit and evaluation at the subject's X.
///
/// Regresses ``y_values`` on ``x_values`` and returns the fitted value
/// and residual for the subject. Conventions:
///
/// - ``fitted_value = intercept + slope * subject_x``
/// - ``residual = subject_y - fitted_value``.
///
/// Arguments:
///     x_values: Peer X observations (independent variable).
///     y_values: Peer Y observations (dependent variable). Must be
///         the same length as ``x_values``.
///     subject_x: Subject's X value at which to evaluate the fit.
///     subject_y: Subject's observed Y value for residual computation.
///
/// Returns:
///     Dict with keys ``{"slope", "intercept", "r_squared",
///     "fitted_value", "residual", "n"}`` mirroring the Rust
///     ``RegressionResult`` serde form. Returns ``None`` if fewer than
///     three observations are available or the regression cannot be
///     computed (e.g., zero variance in X), matching the WASM twin's
///     ``undefined``.
#[pyfunction]
#[pyo3(text_signature = "(x_values, y_values, subject_x, subject_y)")]
fn regression_fair_value<'py>(
    py: Python<'py>,
    x_values: Vec<f64>,
    y_values: Vec<f64>,
    subject_x: f64,
    subject_y: f64,
) -> PyResult<Option<Bound<'py, PyAny>>> {
    match core_regression(&x_values, &y_values, subject_x, subject_y) {
        Some(reg) => serde_to_py(py, &reg).map(Some),
        None => Ok(None),
    }
}

/// Compute a canonical valuation multiple for one company.
///
/// ``company_metrics`` is a Python dict matching the Rust
/// ``CompanyMetrics`` shape; only the fields needed for the chosen
/// multiple must be populated.
///
/// Arguments:
///     company_metrics: Dict of company metrics keyed by canonical field name.
///     multiple: Canonical multiple selector such as ``"EvEbitda"`` or ``"Pe"``.
///
/// Returns:
///     Multiple value, or ``None`` when required inputs are missing or invalid.
#[pyfunction]
#[pyo3(text_signature = "(company_metrics, multiple)")]
fn compute_multiple(company_metrics: &Bound<'_, PyDict>, multiple: &str) -> PyResult<Option<f64>> {
    let metrics = dict_to_company_metrics("subject", company_metrics)?;
    let multiple: Multiple = multiple.parse().map_err(display_to_py)?;
    Ok(core_compute_multiple(&metrics, multiple))
}

/// Convert a ``{metric_name: value}`` dict into a `CompanyMetrics`.
///
/// Known field names (e.g. ``"leverage"``, ``"oas_bp"``, ``"ebitda"``)
/// are mapped onto their dedicated optional fields; everything else is
/// stored in the `custom` map. ``None`` values are treated as missing;
/// any other non-numeric value raises ``ValueError`` naming the key.
fn dict_to_company_metrics(id: &str, d: &Bound<'_, PyDict>) -> PyResult<CompanyMetrics> {
    let mut values = Vec::with_capacity(d.len());
    for (key, val) in d.iter() {
        let name: String = key.extract()?;
        if val.is_none() {
            continue;
        }
        let Ok(v) = val.extract::<f64>() else {
            return Err(crate::errors::value_error(format!(
                "metric '{name}' for company '{id}' must be a number or None, got {}",
                val.get_type().name().map_or_else(
                    |_| "unknown".to_string(),
                    |t| t.to_string_lossy().into_owned()
                )
            )));
        };
        values.push((name, v));
    }
    Ok(CompanyMetrics::from_flat_metrics(id, values))
}

/// Score a subject against its peers across multiple weighted dimensions.
///
/// Takes the canonical serde forms of the Rust ``PeerSet`` and
/// ``ScoringDimension`` types, exactly like the WASM ``scoreRelativeValue``
/// twin. The composite is the weighted average where positive = cheap,
/// negative = rich.
///
/// Arguments:
///     peer_set: Canonical ``PeerSet`` payload — a JSON string or a dict of
///         the same shape: ``{"subject": CompanyMetrics, "peers":
///         [CompanyMetrics, ...], "period_basis": "ltm" | "ntm" |
///         {"custom": str}}``.
///     dimensions: Canonical ``ScoringDimension`` list — a JSON string or a
///         list of dicts, each ``{"label": str, "y_extractor":
///         MetricExtractor, "x_extractors": [MetricExtractor, ...],
///         "weight": float, "direction": "higher_is_cheap" |
///         "higher_is_rich"}``.
///
/// Returns:
///     Dict mirroring the Rust ``RelativeValueResult`` serde form:
///     ``{"company_id", "composite_score", "dimensions", "confidence",
///     "peer_count"}``, where ``dimensions`` is a list of
///     ``DimensionScore`` dicts.
#[pyfunction]
#[pyo3(text_signature = "(peer_set, dimensions)")]
fn score_relative_value<'py>(
    py: Python<'py>,
    peer_set: &Bound<'py, PyAny>,
    dimensions: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let peer_set: PeerSet = extract_serde(py, peer_set, "peer_set")?;
    let dims: Vec<ScoringDimension> = extract_serde(py, dimensions, "dimensions")?;
    let result = core_score(&peer_set, &dims).map_err(core_to_py)?;
    serde_to_py(py, &result)
}

/// Register comps bindings on the analytics submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(percentile_rank, m)?)?;
    m.add_function(wrap_pyfunction!(z_score, m)?)?;
    m.add_function(wrap_pyfunction!(peer_stats, m)?)?;
    m.add_function(wrap_pyfunction!(regression_fair_value, m)?)?;
    m.add_function(wrap_pyfunction!(compute_multiple, m)?)?;
    m.add_function(wrap_pyfunction!(score_relative_value, m)?)?;
    Ok(())
}
