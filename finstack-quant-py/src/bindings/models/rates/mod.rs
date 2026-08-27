//! Python bindings for product-independent interest-rate models.

pub mod dtsm;

use pyo3::prelude::*;
use pyo3::types::PyList;

/// Register the `finstack_quant.models.rates` submodule.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "rates")?;
    module.setattr(
        "__doc__",
        "Product-independent interest-rate models and statistical term-structure engines.",
    )?;
    let qualified_name = crate::bindings::module_utils::set_submodule_package_by_package(
        parent,
        &module,
        "rates",
        "finstack_quant.models",
    )?;

    dtsm::register(py, &module)?;
    module.setattr("__all__", PyList::new(py, ["dtsm"])?)?;

    crate::bindings::module_utils::register_submodule_at(py, parent, &module, &qualified_name)?;

    Ok(())
}
