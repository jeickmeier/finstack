//! Python bindings for the `finstack-quant-core` crate.

mod config;
mod credit;
pub(crate) mod currency;
pub mod dates;
pub mod market_data;
mod math;
pub(crate) mod money;
mod rating_scales;
mod schema;
pub(crate) mod table;
pub(crate) mod types;

use pyo3::prelude::*;
use pyo3::types::PyList;

/// Register the `core` submodule on the parent module.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "core")?;
    m.setattr("__doc__", "Bindings for the finstack-quant-core crate.")?;
    m.setattr("__package__", "finstack_quant.core")?;

    config::register(py, &m)?;
    types::register(py, &m)?;
    currency::register(py, &m)?;
    money::register(py, &m)?;
    math::register(py, &m)?;
    dates::register(py, &m)?;
    market_data::register(py, &m)?;
    credit::register(py, &m)?;
    rating_scales::register(py, &m)?;
    table::register(py, &m)?;

    // `FinstackError` is declared with `module = finstack_quant.core`, so this
    // is its canonical home. Exporting it here is what makes that declaration
    // true: without it `repr()` names an unreachable module and pickling an
    // exception instance cannot resolve the class.
    m.setattr(
        "FinstackError",
        py.get_type::<crate::errors::FinstackError>(),
    )?;

    schema::register(py, &m)?;

    let all = PyList::new(
        py,
        [
            "FinstackError",
            "config",
            "credit",
            "currency",
            "dates",
            "market_data",
            "math",
            "money",
            "rating_scales",
            "schema",
            "table",
            "types",
        ],
    )?;
    m.setattr("__all__", all)?;
    // `Name`, not `Package`: every other domain registers the compiled module
    // under the extension's own path so the pure-Python shim package owns
    // `finstack_quant.core`. Deriving from `__package__` here claimed the
    // public key first, leaving `finstack_quant/core/__init__.py` — its
    // docstring, doctest and sorted `__all__` — permanently unreachable.
    crate::bindings::module_utils::register_submodule(
        py,
        parent,
        &m,
        "core",
        crate::bindings::module_utils::ROOT_PACKAGE,
        crate::bindings::module_utils::ParentNameSource::Name,
    )?;
    Ok(())
}
