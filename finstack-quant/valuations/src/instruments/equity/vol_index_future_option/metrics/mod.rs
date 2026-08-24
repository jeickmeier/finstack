//! Metric registration for options on volatility-index futures.

use crate::metrics::MetricRegistry;

/// Register delta, gamma, vega, and theta for volatility-index futures options.
pub(crate) fn register_vol_index_future_option_metrics(
    registry: &mut MetricRegistry,
) -> std::result::Result<(), crate::metrics::MetricRegistryError> {
    crate::register_metrics! {
        registry: registry,
        instrument: crate::pricer::InstrumentType::VolatilityIndexFutureOption,
        metrics: [
            (Delta, crate::metrics::OptionGreekCalculator::<crate::instruments::VolatilityIndexFutureOption>::delta()),
            (Gamma, crate::metrics::OptionGreekCalculator::<crate::instruments::VolatilityIndexFutureOption>::gamma()),
            (Vega, crate::metrics::OptionGreekCalculator::<crate::instruments::VolatilityIndexFutureOption>::vega()),
            (Theta, crate::metrics::OptionGreekCalculator::<crate::instruments::VolatilityIndexFutureOption>::theta()),
        ]
    }
    Ok(())
}
