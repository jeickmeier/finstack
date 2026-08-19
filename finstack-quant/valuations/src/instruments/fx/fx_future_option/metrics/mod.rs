//! Metric registration for options on FX futures.

use crate::metrics::MetricRegistry;

/// Register delta, gamma, vega, and theta for FX futures options.
pub(crate) fn register_fx_future_option_metrics(registry: &mut MetricRegistry) {
    crate::register_metrics! {
        registry: registry,
        instrument: crate::pricer::InstrumentType::FxFutureOption,
        metrics: [
            (Delta, crate::metrics::OptionGreekCalculator::<crate::instruments::FxFutureOption>::delta()),
            (Gamma, crate::metrics::OptionGreekCalculator::<crate::instruments::FxFutureOption>::gamma()),
            (Vega, crate::metrics::OptionGreekCalculator::<crate::instruments::FxFutureOption>::vega()),
            (Theta, crate::metrics::OptionGreekCalculator::<crate::instruments::FxFutureOption>::theta()),
        ]
    }
}
