//! Python bindings for reusable quantitative model engines.

mod analytic;
pub mod correlation;
pub(crate) mod credit;
mod fourier;
pub mod monte_carlo;
pub mod rates;
mod sabr;

use pyo3::prelude::*;
use pyo3::types::PyList;

/// Register the `finstack_quant.models` domain.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "models")?;
    let qualified_name = crate::bindings::module_utils::set_submodule_package(
        parent,
        &module,
        "models",
        crate::bindings::module_utils::ROOT_PACKAGE,
        crate::bindings::module_utils::ParentNameSource::Name,
    )?;
    module.setattr(
        "__doc__",
        "Reusable analytical, Fourier, volatility, credit, correlation, rates, and Monte Carlo models.",
    )?;

    analytic::register(py, &module)?;
    fourier::register(py, &module)?;
    sabr::register(py, &module)?;
    monte_carlo::register(py, &module)?;
    credit::register(py, &module)?;
    correlation::register(py, &module)?;
    rates::register(py, &module)?;

    let all = PyList::new(
        py,
        [
            "SabrCalibrator",
            "SabrModel",
            "SabrParameters",
            "SabrSmile",
            "asian_option_price",
            "barrier_call",
            "black76_implied_vol",
            "bs_cos_price",
            "bs_greeks",
            "bs_implied_vol",
            "bs_price",
            "correlation",
            "credit",
            "lookback_option_price",
            "merton_jump_cos_price",
            "monte_carlo",
            "quanto_option_price",
            "rates",
            "vanilla_expiry_payoff",
            "vg_cos_price",
        ],
    )?;
    module.setattr("__all__", all)?;
    crate::bindings::module_utils::register_submodule_at(py, parent, &module, &qualified_name)?;
    Ok(())
}
