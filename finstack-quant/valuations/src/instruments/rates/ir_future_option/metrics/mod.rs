//! Metric registration for options on interest-rate futures.

use crate::metrics::{MetricCalculator, MetricContext, MetricRegistry};

#[derive(Debug, Clone, Default)]
struct InterestRateFutureOptionDv01;

impl MetricCalculator for InterestRateFutureOptionDv01 {
    fn calculate(&self, context: &mut MetricContext) -> finstack_quant_core::Result<f64> {
        let option: &crate::instruments::InterestRateFutureOption = context.instrument_as()?;
        option.rate_dv01(&context.curves, context.as_of)
    }
}

/// Register price Greeks and mapped rate DV01 for interest-rate futures options.
pub(crate) fn register_interest_rate_future_option_metrics(
    registry: &mut MetricRegistry,
) -> std::result::Result<(), crate::metrics::MetricRegistryError> {
    crate::register_metrics! {
        registry: registry,
        instrument: crate::pricer::InstrumentType::InterestRateFutureOption,
        metrics: [
            (Delta, crate::metrics::OptionGreekCalculator::<crate::instruments::InterestRateFutureOption>::delta()),
            (Gamma, crate::metrics::OptionGreekCalculator::<crate::instruments::InterestRateFutureOption>::gamma()),
            (Vega, crate::metrics::OptionGreekCalculator::<crate::instruments::InterestRateFutureOption>::vega()),
            (Theta, crate::metrics::OptionGreekCalculator::<crate::instruments::InterestRateFutureOption>::theta()),
            (Dv01, InterestRateFutureOptionDv01),
        ]
    }
    Ok(())
}
