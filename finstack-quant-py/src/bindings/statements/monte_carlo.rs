//! Python wrappers for statement-model Monte Carlo evaluation.

use crate::bindings::extract::extract_model_ref;
use crate::errors::display_to_py;
use finstack_quant_statements::evaluator::{
    Evaluator, MonteCarloConfig as RustMonteCarloConfig, MonteCarloResults as RustMonteCarloResults,
};
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Configuration for statement-model Monte Carlo evaluation.
#[pyclass(
    name = "MonteCarloConfig",
    module = "finstack_quant.statements",
    from_py_object
)]
#[derive(Clone)]
struct PyMonteCarloConfig {
    inner: RustMonteCarloConfig,
}

#[pymethods]
impl PyMonteCarloConfig {
    #[new]
    #[pyo3(signature = (n_paths, seed, percentiles=None, include_path_data=false))]
    fn new(
        n_paths: usize,
        seed: u64,
        percentiles: Option<Vec<f64>>,
        include_path_data: bool,
    ) -> Self {
        let mut inner = RustMonteCarloConfig::new(n_paths, seed);
        if let Some(percentiles) = percentiles {
            inner = inner.with_percentiles(percentiles);
        }
        inner = inner.with_path_data(include_path_data);
        Self { inner }
    }

    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    #[getter]
    fn n_paths(&self) -> usize {
        self.inner.n_paths
    }

    #[getter]
    fn seed(&self) -> u64 {
        self.inner.seed
    }

    #[getter]
    fn percentiles(&self) -> Vec<f64> {
        self.inner.percentiles.clone()
    }

    #[getter]
    fn include_path_data(&self) -> bool {
        self.inner.include_path_data
    }
}

/// Typed results for statement-model Monte Carlo evaluation.
#[pyclass(
    name = "MonteCarloResults",
    module = "finstack_quant.statements",
    from_py_object
)]
#[derive(Clone)]
struct PyMonteCarloResults {
    inner: RustMonteCarloResults,
}

#[pymethods]
impl PyMonteCarloResults {
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    #[getter]
    fn n_paths(&self) -> usize {
        self.inner.n_paths
    }

    #[getter]
    fn percentiles(&self) -> Vec<f64> {
        self.inner.percentiles.clone()
    }

    #[getter]
    fn forecast_periods(&self) -> Vec<String> {
        self.inner
            .forecast_periods
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    fn get_percentile_series<'py>(
        &self,
        py: Python<'py>,
        metric: &str,
        percentile: f64,
    ) -> PyResult<Option<Bound<'py, PyDict>>> {
        let Some(values) = self.inner.get_percentile_series(metric, percentile) else {
            return Ok(None);
        };
        let series = PyDict::new(py);
        for (period, value) in values {
            series.set_item(period.to_string(), value)?;
        }
        Ok(Some(series))
    }
}

fn extract_config(value: &Bound<'_, PyAny>) -> PyResult<RustMonteCarloConfig> {
    if let Ok(config) = value.extract::<PyRef<'_, PyMonteCarloConfig>>() {
        return Ok(config.inner.clone());
    }
    serde_json::from_str(value.extract::<&str>()?).map_err(display_to_py)
}

/// Run Monte Carlo simulation on a financial model.
///
/// Parameters
/// ----------
/// model : FinancialModelSpec | str
///     A typed model or its JSON serialization.
/// config : MonteCarloConfig | str
///     Typed configuration or JSON with ``n_paths``, ``seed``, optional
///     ``percentiles``, and optional ``include_path_data``.
///
/// Returns
/// -------
/// MonteCarloResults
///     Typed Monte Carlo results with JSON serialization support.
#[pyfunction]
fn run_monte_carlo(
    py: Python<'_>,
    model: &Bound<'_, PyAny>,
    config: &Bound<'_, PyAny>,
) -> PyResult<PyMonteCarloResults> {
    let model = extract_model_ref(model)?.into_owned();
    let config = extract_config(config)?;
    py.detach(move || {
        let mut evaluator = Evaluator::new();
        let inner = evaluator
            .evaluate_monte_carlo(&model, &config)
            .map_err(display_to_py)?;
        Ok(PyMonteCarloResults { inner })
    })
}

pub fn register(_py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyMonteCarloConfig>()?;
    module.add_class::<PyMonteCarloResults>()?;
    module.add_function(pyo3::wrap_pyfunction!(run_monte_carlo, module)?)?;
    Ok(())
}
