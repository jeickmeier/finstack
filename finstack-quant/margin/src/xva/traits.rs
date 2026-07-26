//! Traits for XVA-compatible instruments.

use finstack_quant_core::dates::Date;
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::money::Money;
use finstack_quant_core::Result;
use finstack_quant_monte_carlo::PathState;

/// Minimal trait for values consumed by XVA exposure calculations.
///
/// XVA exposure only needs to identify instruments and value them at future
/// dates, so this trait deliberately stays narrower than the full
/// `Instrument` interface from `finstack-quant-valuations`.
pub trait Valuable: Send + Sync {
    /// Returns the instrument identifier used in diagnostics.
    fn id(&self) -> &str;

    /// Computes the instrument value at the requested future date.
    fn value(&self, market: &MarketContext, as_of: Date) -> Result<Money>;
}

/// Path-consistent portfolio valuation bridge for stochastic exposure.
///
/// The margin crate cannot depend on `finstack-quant-valuations` (dependency
/// direction: valuations → margin), so instrument repricing on simulated paths
/// is abstracted behind this trait. Implementors (the valuations crate or the
/// caller) map a simulated [`PathState`] at time `t` to the **netted portfolio
/// MtM** in reporting-currency units (signed: positive = counterparty owes us).
///
/// Used by [`crate::xva::exposure::compute_stochastic_exposure_with_valuer`],
/// which handles close-out netting (`max(V, 0)`), MPOR-lagged CSA collateral,
/// and quantile PFE on top of the values this trait produces.
///
/// This trait is Rust-only (not exposed through Python/WASM bindings).
///
/// # Determinism
///
/// The exposure engine's bit-identical-for-fixed-seed guarantee holds only if
/// the implementation is a pure function of `(PathState, t)` — no hidden
/// mutable state, randomness, or I/O that could vary between calls with the
/// same inputs.
pub trait PathValuer: Send + Sync {
    /// Netted portfolio value in this path state at time `t` (years).
    ///
    /// # Errors
    ///
    /// Implementations return an error when the state lacks required variables
    /// or valuation fails; the exposure engine propagates it.
    fn value_on_path(&self, path_state: &PathState, t: f64) -> finstack_quant_core::Result<f64>;
}
