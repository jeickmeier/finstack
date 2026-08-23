//! Implied volatility metric for `CDSOption`.
//!
//! Computes the Black-on-spreads implied volatility that matches the
//! instrument's current PV (`context.base_value`) using the CDS option
//! pricer and core math solvers (HybridSolver).

use crate::instruments::credit_derivatives::cds_option::CDSOption;

/// Implied volatility metric for credit options on CDS spreads.
pub(crate) struct ImpliedVolCalculator;

impl crate::metrics::MetricCalculator for ImpliedVolCalculator {
    fn calculate(
        &self,
        context: &mut crate::metrics::MetricContext,
    ) -> finstack_quant_core::Result<f64> {
        let option: &CDSOption = context.instrument_as()?;
        let target = context.base_value.amount();
        option.implied_vol(&context.curves, context.as_of, target, None)
    }
}
