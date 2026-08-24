//! Metric registration for options on equity futures.

use crate::metrics::MetricRegistry;

/// Register delta, gamma, vega, and theta for equity futures options.
pub(crate) fn register_equity_future_option_metrics(
    registry: &mut MetricRegistry,
) -> std::result::Result<(), crate::metrics::MetricRegistryError> {
    crate::register_metrics! {
        registry: registry,
        instrument: crate::pricer::InstrumentType::EquityFutureOption,
        metrics: [
            (Delta, crate::metrics::OptionGreekCalculator::<crate::instruments::EquityFutureOption>::delta()),
            (Gamma, crate::metrics::OptionGreekCalculator::<crate::instruments::EquityFutureOption>::gamma()),
            (Vega, crate::metrics::OptionGreekCalculator::<crate::instruments::EquityFutureOption>::vega()),
            (Theta, crate::metrics::OptionGreekCalculator::<crate::instruments::EquityFutureOption>::theta()),
        ]
    }
    Ok(())
}
