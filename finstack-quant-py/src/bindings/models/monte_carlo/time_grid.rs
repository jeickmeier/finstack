//! TimeGrid bindings.

use crate::errors::core_to_py;
use finstack_quant_models::monte_carlo::time_grid::TimeGrid;
use pyo3::prelude::*;

/// Discretised time grid for Monte Carlo simulation.
#[pyclass(
    name = "TimeGrid",
    module = "finstack_quant.models.monte_carlo",
    frozen
)]
pub struct PyTimeGrid {
    pub(super) inner: TimeGrid,
}

#[pymethods]
impl PyTimeGrid {
    /// Create a uniform time grid.
    #[new]
    #[pyo3(signature = (t_max, num_steps))]
    fn new(t_max: f64, num_steps: usize) -> PyResult<Self> {
        TimeGrid::uniform(t_max, num_steps)
            .map(|tg| Self { inner: tg })
            .map_err(core_to_py)
    }

    /// Create a time grid from explicit time points.
    #[staticmethod]
    fn from_times(times: Vec<f64>) -> PyResult<Self> {
        TimeGrid::from_times(times)
            .map(|tg| Self { inner: tg })
            .map_err(core_to_py)
    }

    /// Create a near-uniform grid that includes required knot times exactly.
    ///
    /// Builds a uniform grid of ``max(round(t_max * steps_per_year),
    /// min_steps)`` steps over ``[0, t_max]``, then merges each
    /// ``required_times`` entry (e.g. exercise dates, barrier monitoring or
    /// cashflow dates) as an exact grid knot.
    #[staticmethod]
    #[pyo3(signature = (t_max, steps_per_year, min_steps, required_times))]
    fn uniform_with_required_times(
        t_max: f64,
        steps_per_year: f64,
        min_steps: usize,
        required_times: Vec<f64>,
    ) -> PyResult<Self> {
        TimeGrid::uniform_with_required_times(t_max, steps_per_year, min_steps, &required_times)
            .map(|tg| Self { inner: tg })
            .map_err(core_to_py)
    }

    /// Number of time steps.
    #[getter]
    fn num_steps(&self) -> usize {
        self.inner.num_steps()
    }

    /// Maximum time.
    #[getter]
    fn t_max(&self) -> f64 {
        self.inner.t_max()
    }

    /// Whether the grid is uniformly spaced.
    #[getter]
    fn is_uniform(&self) -> bool {
        self.inner.is_uniform()
    }

    /// All time points.
    #[getter]
    fn times(&self) -> Vec<f64> {
        self.inner.times().to_vec()
    }

    /// All time step sizes.
    #[getter]
    fn dts(&self) -> Vec<f64> {
        self.inner.dts().to_vec()
    }

    /// Time at a given step index.
    fn time(&self, step: usize) -> f64 {
        self.inner.time(step)
    }

    /// Step size at a given step index.
    fn dt(&self, step: usize) -> f64 {
        self.inner.dt(step)
    }

    fn __repr__(&self) -> String {
        format!(
            "TimeGrid(t_max={:.4}, steps={}, uniform={})",
            self.inner.t_max(),
            self.inner.num_steps(),
            self.inner.is_uniform()
        )
    }
}

pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyTimeGrid>()?;
    Ok(())
}
