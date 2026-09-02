//! Python binding for `finstack_quant_core::math::consecutive`.

use finstack_quant_core::math::consecutive;
use pyo3::prelude::*;

/// Length of the longest run of strictly positive values.
#[pyfunction]
pub fn longest_positive_run(values: Vec<f64>) -> usize {
    consecutive::longest_positive_run(&values)
}

/// Register consecutive helpers on the parent math module.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(longest_positive_run, m)?)?;
    Ok(())
}
