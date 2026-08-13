use pyo3::exceptions::PyKeyError;
use pyo3::prelude::*;

use finstack_quant_portfolio::factor_model::{
    DecompositionConfig, HistoricalPositionDecomposer, ParametricPositionDecomposer,
};

use crate::errors::core_to_py;

use super::super::matrix_input::{extract_position_pnls, extract_square_matrix};
use super::contributions::PyPositionRiskDecomposition;
use super::to_position_ids;

/// Decompose portfolio VaR/ES into position contributions via parametric
/// Euler allocation, returning a typed :class:`PositionRiskDecomposition`.
#[pyfunction]
#[pyo3(signature = (position_ids, weights, covariance, confidence = 0.95, compute_incremental = false))]
pub(super) fn parametric_var_decomposition(
    py: Python<'_>,
    position_ids: Vec<String>,
    weights: Vec<f64>,
    covariance: &Bound<'_, PyAny>,
    confidence: f64,
    compute_incremental: bool,
) -> PyResult<PyPositionRiskDecomposition> {
    let n = weights.len();
    let cov_flat = extract_square_matrix(py, covariance, n, "covariance")?;

    let mut config = DecompositionConfig::parametric(confidence);
    if compute_incremental {
        config = config.with_incremental();
    }

    let result = py
        .detach(move || {
            let ids = to_position_ids(position_ids);
            ParametricPositionDecomposer.decompose_positions(&weights, &cov_flat, &ids, &config)
        })
        .map_err(core_to_py)?;

    Ok(PyPositionRiskDecomposition::from_inner(result))
}

/// Decompose portfolio expected shortfall through the same parametric Euler
/// allocation and return the complete typed risk decomposition.
#[pyfunction]
#[pyo3(signature = (position_ids, weights, covariance, confidence = 0.95))]
pub(super) fn parametric_es_decomposition(
    py: Python<'_>,
    position_ids: Vec<String>,
    weights: Vec<f64>,
    covariance: &Bound<'_, PyAny>,
    confidence: f64,
) -> PyResult<PyPositionRiskDecomposition> {
    let n = weights.len();
    let cov_flat = extract_square_matrix(py, covariance, n, "covariance")?;
    let config = DecompositionConfig::parametric(confidence);

    let result = py
        .detach(move || {
            let ids = to_position_ids(position_ids);
            ParametricPositionDecomposer.decompose_positions(&weights, &cov_flat, &ids, &config)
        })
        .map_err(core_to_py)?;

    Ok(PyPositionRiskDecomposition::from_inner(result))
}

/// Decompose portfolio VaR and ES from per-position scenario P&Ls via
/// historical simulation, returning a typed :class:`PositionRiskDecomposition`.
#[pyfunction]
#[pyo3(signature = (position_ids, position_pnls, confidence = 0.95))]
pub(super) fn historical_var_decomposition(
    py: Python<'_>,
    position_ids: Vec<String>,
    position_pnls: &Bound<'_, PyAny>,
    confidence: f64,
) -> PyResult<PyPositionRiskDecomposition> {
    let n = position_ids.len();
    let position_pnls = extract_position_pnls(py, position_pnls, n)?;
    let n_scenarios = position_pnls.n_scenarios();

    let config = DecompositionConfig::historical(confidence);
    let result = py
        .detach(move || {
            let flat = position_pnls.into_scenario_major(n);
            let ids = to_position_ids(position_ids);
            HistoricalPositionDecomposer.decompose_from_pnls(&flat, &ids, n_scenarios, &config)
        })
        .map_err(core_to_py)?;

    Ok(PyPositionRiskDecomposition::from_inner(result))
}

/// Look up a position by id inside a :class:`PositionRiskDecomposition` and
/// return its component VaR. Raises ``KeyError`` if absent.
#[pyfunction]
#[pyo3(signature = (decomp, position_id))]
pub(super) fn position_component_var(
    decomp: &PyPositionRiskDecomposition,
    position_id: &str,
) -> PyResult<f64> {
    decomp
        .inner
        .var_contributions
        .iter()
        .find(|c| c.position_id.as_str() == position_id)
        .map(|c| c.component_var)
        .ok_or_else(|| {
            PyKeyError::new_err(format!("position '{position_id}' not in decomposition"))
        })
}
