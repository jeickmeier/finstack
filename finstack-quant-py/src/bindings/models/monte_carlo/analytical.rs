//! Analytical closed-form pricing formulas.

use pyo3::prelude::*;

/// Black-Scholes European call present value under GBM.
///
/// Argument order is ``(spot, strike, rate, div_yield, vol, expiry)``. Internally
/// re-ordered to the Rust crate's ``(spot, strike, expiry, rate, q, vol)`` layout.
/// Rates and dividend yield are continuously compounded decimals. Non-finite
/// inputs return ``NaN``; this helper does not raise.
///
/// Parameters
/// ----------
/// spot : float
///     Spot price.
/// strike : float
///     Strike price.
/// rate : float
///     Risk-free rate (continuously compounded decimal).
/// div_yield : float
///     Dividend yield (continuously compounded decimal).
/// vol : float
///     Volatility (decimal).
/// expiry : float
///     Time to maturity in years.
///
/// Returns
/// -------
/// float
///     Present value of the European call.
///
/// Sources
/// -------
/// - Black-Scholes (1973): see docs/REFERENCES.md#black-scholes-1973
/// - Merton (1973): see docs/REFERENCES.md#merton-1973
#[pyfunction]
#[pyo3(signature = (spot, strike, rate, div_yield, vol, expiry))]
fn black_scholes_call(
    spot: f64,
    strike: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    expiry: f64,
) -> f64 {
    finstack_quant_models::monte_carlo::variance_reduction::control_variate::black_scholes_call(
        spot, strike, expiry, rate, div_yield, vol,
    )
}

/// Black-Scholes European put present value under GBM.
///
/// Argument order is ``(spot, strike, rate, div_yield, vol, expiry)``. Internally
/// re-ordered to the Rust crate's ``(spot, strike, expiry, rate, q, vol)`` layout.
/// Rates and dividend yield are continuously compounded decimals. Non-finite
/// inputs return ``NaN``; this helper does not raise.
///
/// Parameters
/// ----------
/// spot : float
///     Spot price.
/// strike : float
///     Strike price.
/// rate : float
///     Risk-free rate (continuously compounded decimal).
/// div_yield : float
///     Dividend yield (continuously compounded decimal).
/// vol : float
///     Volatility (decimal).
/// expiry : float
///     Time to maturity in years.
///
/// Returns
/// -------
/// float
///     Present value of the European put.
///
/// Sources
/// -------
/// - Black-Scholes (1973): see docs/REFERENCES.md#black-scholes-1973
/// - Merton (1973): see docs/REFERENCES.md#merton-1973
#[pyfunction]
#[pyo3(signature = (spot, strike, rate, div_yield, vol, expiry))]
fn black_scholes_put(
    spot: f64,
    strike: f64,
    rate: f64,
    div_yield: f64,
    vol: f64,
    expiry: f64,
) -> f64 {
    finstack_quant_models::monte_carlo::variance_reduction::control_variate::black_scholes_put(
        spot, strike, expiry, rate, div_yield, vol,
    )
}

pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(black_scholes_call, m)?)?;
    m.add_function(wrap_pyfunction!(black_scholes_put, m)?)?;
    Ok(())
}
