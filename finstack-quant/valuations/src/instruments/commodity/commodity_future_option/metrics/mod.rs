//! Metric registration for options on commodity futures.

use crate::metrics::MetricRegistry;

/// Register delta, gamma, vega, and theta for commodity futures options.
pub(crate) fn register_commodity_future_option_metrics(
    registry: &mut MetricRegistry,
) -> std::result::Result<(), crate::metrics::MetricRegistryError> {
    crate::register_metrics! {
        registry: registry,
        instrument: crate::pricer::InstrumentType::CommodityFutureOption,
        metrics: [
            (Delta, crate::metrics::OptionGreekCalculator::<crate::instruments::CommodityFutureOption>::delta()),
            (Gamma, crate::metrics::OptionGreekCalculator::<crate::instruments::CommodityFutureOption>::gamma()),
            (Vega, crate::metrics::OptionGreekCalculator::<crate::instruments::CommodityFutureOption>::vega()),
            (Theta, crate::metrics::OptionGreekCalculator::<crate::instruments::CommodityFutureOption>::theta()),
        ]
    }
    Ok(())
}
