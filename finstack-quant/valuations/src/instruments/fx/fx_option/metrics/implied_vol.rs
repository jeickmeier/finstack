//! Implied volatility metric for FX options.
//!
//! Solves for σ such that model PV(σ) equals the instrument's base PV
//! already computed in the `MetricContext`. Uses the configured pricer
//! (Hybrid solver under the hood) with log-σ parameterization.

use crate::instruments::fx::fx_option::FxOption;

/// Implied volatility metric for FX options.
pub(crate) struct ImpliedVolCalculator;

impl crate::metrics::MetricCalculator for ImpliedVolCalculator {
    fn calculate(
        &self,
        context: &mut crate::metrics::MetricContext,
    ) -> finstack_quant_core::Result<f64> {
        let option: &FxOption = context.instrument_as()?;
        let target = context.base_value.amount();
        option.implied_vol(&context.curves, context.as_of, target)
    }
}
