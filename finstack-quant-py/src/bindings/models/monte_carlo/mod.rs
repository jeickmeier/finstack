//! Python bindings for the `finstack-quant-models` crate.
//!
//! Exposes canonical European, Asian, LSMC, Heston, analytical, and Greek
//! workflows. Advanced Rust process, discretization, RNG, and payoff types
//! remain Rust-only.

mod analytical;
mod engine;
mod greeks;
mod pricers;
mod results;

use pyo3::prelude::*;
use pyo3::types::PyList;

/// Register the `finstack_quant.models.monte_carlo` submodule.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "monte_carlo")?;
    m.setattr(
        "__doc__",
        "Monte Carlo convenience bindings (finstack-quant-models).",
    )?;

    results::register(py, &m)?;
    engine::register(py, &m)?;
    pricers::register(py, &m)?;
    analytical::register(py, &m)?;
    greeks::register(py, &m)?;

    let all = PyList::new(
        py,
        [
            "MoneyEstimate",
            "Estimate",
            "GbmPathSummary",
            "simulate_gbm_paths",
            "heston_satisfies_feller",
            "EuropeanPricer",
            "PathDependentPricer",
            "LsmcPricer",
            "black_scholes_call",
            "black_scholes_put",
            "price_heston_call",
            "price_heston_put",
            "finite_diff_delta",
            "finite_diff_delta_crn",
            "finite_diff_gamma",
            "finite_diff_gamma_crn",
        ],
    )?;
    m.setattr("__all__", all)?;
    crate::bindings::module_utils::register_submodule(
        py,
        parent,
        &m,
        "monte_carlo",
        "finstack_quant.models",
        crate::bindings::module_utils::ParentNameSource::Package,
    )?;

    Ok(())
}
