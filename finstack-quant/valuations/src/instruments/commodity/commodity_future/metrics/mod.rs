//! Metric registration for exchange-listed commodity futures.

use crate::metrics::{MetricCalculator, MetricContext, MetricRegistry};

struct FuturesPrice;

impl MetricCalculator for FuturesPrice {
    fn calculate(&self, context: &mut MetricContext) -> finstack_quant_core::Result<f64> {
        let future: &crate::instruments::CommodityFuture = context.instrument_as()?;
        future.mark_price(&context.curves, context.as_of)
    }
}

struct Delta;

impl MetricCalculator for Delta {
    fn calculate(&self, context: &mut MetricContext) -> finstack_quant_core::Result<f64> {
        let future: &crate::instruments::CommodityFuture = context.instrument_as()?;
        future.price_curve_delta(context.as_of)
    }
}

/// Register price and projected-curve delta for listed commodity futures.
pub(crate) fn register_commodity_future_metrics(registry: &mut MetricRegistry) {
    crate::register_metrics! {
        registry: registry,
        instrument: crate::pricer::InstrumentType::CommodityFuture,
        metrics: [
            (FuturesPrice, FuturesPrice),
            (Delta, Delta),
        ]
    }
}
