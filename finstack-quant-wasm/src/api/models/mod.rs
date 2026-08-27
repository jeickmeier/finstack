//! WASM bindings for the `finstack-quant-models` crate.
//!
//! Split by model family:
//! - [`analytic`] — closed-form option primitives.
//! - [`fourier`] — COS-method Fourier pricers.
//! - [`sabr`] — SABR parameters, model, smile, and calibrator.
//! - [`credit`] — structural-credit model factories.
//! - [`correlation`] — copula, recovery, and joint-probability utilities.
//! - [`monte_carlo`] — stochastic option-pricing convenience functions.
//! - [`rates`] — interest-rate models and dynamic term-structure engines.

pub mod analytic;
pub mod correlation;
pub mod credit;
pub mod fourier;
pub mod monte_carlo;
pub mod rates;
pub mod sabr;
