//! Attribution spec execution dispatch.

use super::parallel::attribute_pnl_parallel_with_credit_model;
use super::spec::{default_attribution_metrics, AttributionResult, AttributionSpec};
use super::waterfall::attribute_pnl_waterfall_with_credit_model;
use super::{attribute_pnl_metrics_based, AttributionMethod};
use finstack_quant_calibration::recalibration::CachedRecalibrationProvider;
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::Result;
use finstack_quant_core::{currency::Currency, dates::Date, money::Money};
use finstack_quant_valuations::instruments::model_params::ModelParamsSnapshot;
use finstack_quant_valuations::instruments::Instrument;
use finstack_quant_valuations::metrics::MetricId;

#[derive(Debug)]
struct RoundingCurrencyProbe {
    currency: Option<Currency>,
    value: Option<Money>,
}

impl RoundingCurrencyProbe {
    fn successful_valuations(&self) -> usize {
        usize::from(self.value.is_some())
    }
}

/// Resolves the rounding currency and retains a successful opening valuation.
fn probe_rounding_currency(
    instrument: &dyn Instrument,
    market_t0: &MarketContext,
    as_of_t0: Date,
    rounding_scale: Option<u32>,
) -> RoundingCurrencyProbe {
    if rounding_scale.is_none() {
        return RoundingCurrencyProbe {
            currency: None,
            value: None,
        };
    }

    match instrument.value(market_t0, as_of_t0) {
        Ok(value) => RoundingCurrencyProbe {
            currency: Some(value.currency()),
            value: Some(value),
        },
        Err(_) => RoundingCurrencyProbe {
            currency: instrument.notional().map(|notional| notional.currency()),
            value: None,
        },
    }
}

/// Returns the configured target only when the completed attribution needs translation.
fn target_currency_requiring_translation(
    target_currency: Option<Currency>,
    attribution: &crate::PnlAttribution,
) -> Option<Currency> {
    target_currency.filter(|target| *target != attribution.total_pnl.currency())
}

/// Resolves the opening value used by target-currency translation.
fn translation_t0_value(
    instrument: &std::sync::Arc<dyn Instrument>,
    market_t0: &MarketContext,
    as_of_t0: Date,
    model_params_t0: Option<&ModelParamsSnapshot>,
    rounding_probe_value: Option<Money>,
    notes: &mut Vec<String>,
    num_repricings: &mut usize,
) -> Result<Money> {
    if model_params_t0.is_none() {
        if let Some(value) = rounding_probe_value {
            return Ok(value);
        }
    }

    let t0_instrument = match model_params_t0 {
        Some(params) => match crate::model_params::with_model_params(instrument, params) {
            Ok(instrument) => instrument,
            Err(error) => {
                notes.push(format!(
                    "target_currency translation: T0 model-parameter application \
                     failed ({error}); using T1-parameter instrument for val_t0"
                ));
                std::sync::Arc::clone(instrument)
            }
        },
        None => std::sync::Arc::clone(instrument),
    };
    let value = t0_instrument.value(market_t0, as_of_t0)?;
    *num_repricings += 1;
    Ok(value)
}

impl AttributionSpec {
    /// Execute the attribution specification.
    ///
    /// Returns a complete result with the P&L attribution and metadata.
    ///
    /// # Errors
    ///
    /// Propagates instrument and market reconstruction, configured rounding,
    /// pricing, FX conversion, and method-specific attribution errors. For the
    /// metrics-based method, unknown configured metric names are rejected
    /// before valuation.
    pub fn execute(&self) -> Result<AttributionResult> {
        let instrument = self.instrument.clone().into_boxed()?;
        let instrument_arc: std::sync::Arc<dyn Instrument> = std::sync::Arc::from(instrument);

        let market_t0 = MarketContext::try_from(self.market_t0.clone())?;
        let market_t1 = MarketContext::try_from(self.market_t1.clone())?;

        let rounding_scale = self
            .config
            .as_ref()
            .and_then(|config| config.rounding_scale);
        let rounding_probe = probe_rounding_currency(
            instrument_arc.as_ref(),
            &market_t0,
            self.as_of_t0,
            rounding_scale,
        );
        let config = self.build_finstack_config(rounding_probe.currency)?;

        let strict_validation = self
            .config
            .as_ref()
            .and_then(|c| c.strict_validation)
            .unwrap_or(super::spec::DEFAULT_STRICT_VALIDATION);
        let execution_policy = self
            .config
            .as_ref()
            .and_then(|c| c.execution_policy)
            .unwrap_or_default();

        // Resolve optional credit-factor model for waterfall/parallel cascade.
        // Borrow the boxed model directly — the cascade entry points take
        // `Option<&CreditFactorModel>`, so deref-borrowing avoids deep-cloning
        // the entire model (issuer betas, covariance, hierarchy, diagnostics)
        // on every spec execution.
        let resolved_credit_model = self.credit_factor_model.as_deref();

        let mut attribution = match &self.method {
            AttributionMethod::Parallel => attribute_pnl_parallel_with_credit_model(
                &instrument_arc,
                &market_t0,
                &market_t1,
                self.as_of_t0,
                self.as_of_t1,
                &config,
                self.model_params_t0.as_ref(),
                resolved_credit_model,
                &self.credit_factor_detail_options,
                self.full_cross_attribution,
                execution_policy,
            )?,

            AttributionMethod::Waterfall(order) => attribute_pnl_waterfall_with_credit_model(
                &instrument_arc,
                &market_t0,
                &market_t1,
                self.as_of_t0,
                self.as_of_t1,
                &config,
                order.clone(),
                strict_validation,
                self.model_params_t0.as_ref(),
                resolved_credit_model,
                &self.credit_factor_detail_options,
            )?,

            AttributionMethod::Taylor(ref taylor_config) => {
                crate::taylor::attribute_pnl_taylor_with_model_params(
                    &instrument_arc,
                    &market_t0,
                    &market_t1,
                    self.as_of_t0,
                    self.as_of_t1,
                    taylor_config,
                    execution_policy,
                    self.model_params_t0.as_ref(),
                )?
            }

            AttributionMethod::MetricsBased => {
                let metrics = if let Some(ref cfg) = self.config {
                    if let Some(ref metric_names) = cfg.metrics {
                        let mut parsed = Vec::new();
                        let mut unknown = Vec::new();

                        for name in metric_names {
                            match MetricId::parse_strict(name) {
                                Ok(id) => parsed.push(id),
                                Err(_) => unknown.push(name.clone()),
                            }
                        }

                        if !unknown.is_empty() {
                            return Err(finstack_quant_core::Error::Validation(format!(
                                "Unknown metric names: {}",
                                unknown.join(", ")
                            )));
                        }

                        parsed
                    } else {
                        default_attribution_metrics()
                    }
                } else {
                    default_attribution_metrics()
                };

                // Compute valuations with metrics. The FinstackConfig built
                // above carries the `valuations.sensitivities.v1` extension
                // (e.g. `rate_bump_bp` from `AttributionConfig`) — it must be
                // attached to the pricing request or the sensitivity
                // calculators fall back to defaults and the config knob is
                // silently inert (audit finding: spec.rs wrote the extension
                // but pricing ran with `PricingOptions::default()`).
                let pricing_options =
                    finstack_quant_valuations::instruments::PricingOptions::default()
                        .with_config(&config)
                        .with_recalibration_provider(std::sync::Arc::new(
                            CachedRecalibrationProvider::new(),
                        ));
                let val_t0 = instrument_arc.price_with_metrics(
                    &market_t0,
                    self.as_of_t0,
                    &metrics,
                    pricing_options.clone(),
                )?;
                let val_t1 = instrument_arc.price_with_metrics(
                    &market_t1,
                    self.as_of_t1,
                    &metrics,
                    pricing_options,
                )?;

                attribute_pnl_metrics_based(
                    &instrument_arc,
                    &market_t0,
                    &market_t1,
                    &val_t0,
                    &val_t1,
                    self.as_of_t0,
                    self.as_of_t1,
                )?
            }
        };
        attribution.meta.num_repricings += rounding_probe.successful_valuations();

        if let Some(ref cfg) = self.config {
            if let Some(tol_abs) = cfg.tolerance_abs {
                attribution.meta.tolerance_abs = tol_abs;
            }
            if let Some(tol_pct) = cfg.tolerance_pct {
                attribution.meta.tolerance_pct = tol_pct;
            }
        }

        // Optional: credit-factor hierarchy decomposition of credit_curves_pnl.
        // Wired for MetricsBased and Taylor (PR-7). Other methods leave the
        // field None; they will be wired in PR-8a/b. The existing
        // `credit_curves_pnl` field is unchanged numerically — this is purely
        // additive detail.
        if let Some(model_ref) = &self.credit_factor_model {
            let linear_path = matches!(
                self.method,
                AttributionMethod::MetricsBased | AttributionMethod::Taylor(_)
            );
            // PR-8a: Parallel and Waterfall now populate `credit_factor_detail`
            // internally via the per-step credit cascade. The linear methods
            // (PR-7) still go through the back-solve in
            // `compute_credit_factor_detail`.
            if linear_path {
                let mut detail_notes: Vec<String> = Vec::new();
                match self.compute_credit_factor_detail(
                    model_ref,
                    &instrument_arc,
                    &market_t0,
                    &market_t1,
                    &attribution,
                    &mut detail_notes,
                ) {
                    Ok(Some(detail)) => {
                        attribution.credit_factor_detail = Some(detail);
                        // The detail back-solve performs 2 CS01 repricings.
                        attribution.meta.num_repricings += 2;
                    }
                    Ok(None) => {
                        if detail_notes.is_empty() {
                            attribution.meta.notes.push(
                                "credit_factor_model supplied but no resolvable issuer/CS01 \
                                 on instrument; credit_factor_detail omitted"
                                    .into(),
                            );
                        }
                    }
                    Err(e) => {
                        attribution
                            .meta
                            .notes
                            .push(format!("credit_factor_detail computation failed: {e}"));
                    }
                }
                attribution.meta.notes.extend(detail_notes);
            }
            // For Parallel / Waterfall methods, the detail (if any) is already
            // populated inside the method itself.

            // PR-8b: split coupon_income / roll_down into rates / credit parts
            // and emit `credit_carry_decomposition` (the second lens, §7.2).
            // Best-effort: failures fall back to leaving the existing scalar
            // CarryDetail untouched and append a diagnostic note.
            //
            // All four methods populate `carry_detail` (parallel / waterfall /
            // Taylor via `apply_total_return_carry`; metrics-based from the
            // carry decomposition metrics), so the split is attempted on every
            // path — the decomposition logic is method-agnostic.
            match self.compute_carry_credit_split_and_decomposition(
                model_ref,
                &instrument_arc,
                &market_t0,
                &mut attribution,
            ) {
                Ok(()) => {}
                Err(e) => attribution.meta.notes.push(format!(
                    "credit_carry_decomposition computation failed: {e}"
                )),
            }
        }

        // Item 2: optional target-currency translation. Runs as a final
        // post-processing step so direct callers of the per-method functions
        // keep their existing native-currency behavior; only the JSON-spec
        // pipeline (used by the bindings) picks up `target_currency`.
        let configured_target = self
            .config
            .as_ref()
            .and_then(|config| config.target_currency);
        if let Some(target_currency) =
            target_currency_requiring_translation(configured_target, &attribution)
        {
            match translation_t0_value(
                &instrument_arc,
                &market_t0,
                self.as_of_t0,
                self.model_params_t0.as_ref(),
                rounding_probe.value,
                &mut attribution.meta.notes,
                &mut attribution.meta.num_repricings,
            ) {
                Ok(val_t0_native) => {
                    match crate::translate_to_target_currency(
                        &mut attribution,
                        val_t0_native,
                        target_currency,
                        &market_t0,
                        &market_t1,
                        self.as_of_t0,
                        self.as_of_t1,
                    ) {
                        Ok(()) => {}
                        Err(e) => attribution
                            .meta
                            .notes
                            .push(format!("target_currency translation failed: {e}")),
                    }
                }
                Err(e) => attribution.meta.notes.push(format!(
                    "target_currency translation skipped: T0 reprice failed - {e}"
                )),
            }
        }

        let results_meta = finstack_quant_core::config::results_meta(&config);

        Ok(AttributionResult {
            attribution,
            results_meta,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AttributionConfig;
    use finstack_quant_cashflows::builder::{
        CashFlowMeta, CashFlowSchedule, CashflowRepresentation,
    };
    use finstack_quant_cashflows::{
        schedule_from_classified_flows, CashflowScheduleSource, ScheduleBuildOpts,
    };
    use finstack_quant_core::currency::Currency;
    use finstack_quant_core::dates::{Date, DayCount};
    use finstack_quant_core::money::Money;
    use finstack_quant_core::types::Attributes;
    use finstack_quant_valuations::instruments::model_params::ModelParamsSnapshot;
    use finstack_quant_valuations::instruments::{InstrumentJson, MarketDependencies};
    use finstack_quant_valuations::pricer::InstrumentType;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use time::macros::date;

    #[derive(Clone)]
    struct CurrencyTestInstrument {
        id: String,
        attributes: Attributes,
        value_currency: Currency,
        value_fails: bool,
        valuation_calls: Arc<AtomicUsize>,
    }

    impl CurrencyTestInstrument {
        fn successful(value_currency: Currency) -> Self {
            Self {
                id: "currency-test".to_string(),
                attributes: Attributes::default(),
                value_currency,
                value_fails: false,
                valuation_calls: Arc::new(AtomicUsize::new(0)),
            }
        }

        fn failing() -> Self {
            Self {
                value_fails: true,
                ..Self::successful(Currency::EUR)
            }
        }

        fn valuation_calls(&self) -> usize {
            self.valuation_calls.load(Ordering::SeqCst)
        }
    }

    impl CashflowScheduleSource for CurrencyTestInstrument {
        fn notional(&self) -> Option<Money> {
            Some(Money::new(1_000_000.0, Currency::USD))
        }

        fn raw_cashflow_schedule(
            &self,
            _market: &MarketContext,
            _as_of: Date,
        ) -> finstack_quant_core::Result<CashFlowSchedule> {
            Ok(schedule_from_classified_flows(
                Vec::new(),
                DayCount::Act365F,
                ScheduleBuildOpts {
                    notional_hint: self.notional(),
                    meta: CashFlowMeta {
                        representation: CashflowRepresentation::NoResidual,
                        ..Default::default()
                    },
                },
            ))
        }
    }

    impl Instrument for CurrencyTestInstrument {
        fn id(&self) -> &str {
            &self.id
        }

        fn key(&self) -> InstrumentType {
            InstrumentType::Bond
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }

        fn attributes(&self) -> &Attributes {
            &self.attributes
        }

        fn attributes_mut(&mut self) -> &mut Attributes {
            &mut self.attributes
        }

        fn clone_box(&self) -> Box<dyn Instrument> {
            Box::new(self.clone())
        }

        fn base_value(
            &self,
            _market: &MarketContext,
            _as_of: Date,
        ) -> finstack_quant_core::Result<Money> {
            self.valuation_calls.fetch_add(1, Ordering::SeqCst);
            if self.value_fails {
                Err(finstack_quant_core::Error::Validation(
                    "intentional test valuation failure".to_string(),
                ))
            } else {
                Ok(Money::new(100.0, self.value_currency))
            }
        }

        fn market_dependencies(&self) -> finstack_quant_core::Result<MarketDependencies> {
            Ok(MarketDependencies::new())
        }
    }

    fn spec_with_config(config: AttributionConfig) -> AttributionSpec {
        let bond = finstack_quant_valuations::instruments::Bond::example()
            .expect("test bond should build");
        let market = finstack_quant_core::market_data::context::MarketContextState::from(
            &MarketContext::new(),
        );
        AttributionSpec {
            instrument: InstrumentJson::Bond(bond),
            market_t0: market.clone(),
            market_t1: market,
            as_of_t0: date!(2025 - 01 - 01),
            as_of_t1: date!(2025 - 01 - 02),
            method: AttributionMethod::Parallel,
            model_params_t0: None,
            config: Some(config),
            credit_factor_model: None,
            credit_factor_detail_options: Default::default(),
            full_cross_attribution: false,
        }
    }

    fn config(rounding_scale: Option<u32>, target_currency: Option<Currency>) -> AttributionConfig {
        AttributionConfig {
            tolerance_abs: None,
            tolerance_pct: None,
            metrics: None,
            strict_validation: None,
            rounding_scale,
            rate_bump_bp: None,
            target_currency,
            execution_policy: None,
        }
    }

    #[test]
    fn rounding_probe_uses_value_currency_for_overrides() {
        let instrument = CurrencyTestInstrument::successful(Currency::EUR);
        let market = MarketContext::new();

        let probe = probe_rounding_currency(&instrument, &market, date!(2025 - 01 - 01), Some(6));
        let finstack_config = spec_with_config(config(Some(6), None))
            .build_finstack_config(probe.currency)
            .expect("rounding config should build");

        assert_eq!(
            probe.value.map(|value| value.currency()),
            Some(Currency::EUR)
        );
        assert_eq!(
            finstack_config
                .rounding
                .output_scale
                .overrides
                .get(&Currency::EUR),
            Some(&6)
        );
        assert!(!finstack_config
            .rounding
            .output_scale
            .overrides
            .contains_key(&Currency::USD));
        assert_eq!(probe.successful_valuations(), 1);
        assert_eq!(instrument.valuation_calls(), 1);
    }

    #[test]
    fn target_only_skips_probe_and_uses_attribution_currency() {
        let instrument = CurrencyTestInstrument::successful(Currency::EUR);
        let market = MarketContext::new();

        let probe = probe_rounding_currency(&instrument, &market, date!(2025 - 01 - 01), None);
        let attribution = crate::PnlAttribution::new(
            Money::new(10.0, Currency::EUR),
            instrument.id(),
            date!(2025 - 01 - 01),
            date!(2025 - 01 - 02),
            AttributionMethod::Parallel,
        );

        assert_eq!(instrument.valuation_calls(), 0);
        assert_eq!(probe.currency, None);
        assert_eq!(probe.value, None);
        assert_eq!(
            target_currency_requiring_translation(Some(Currency::USD), &attribution),
            Some(Currency::USD)
        );
        assert_eq!(
            target_currency_requiring_translation(Some(Currency::EUR), &attribution),
            None
        );
    }

    #[test]
    fn reusable_probe_avoids_duplicate_but_model_override_reprices() {
        let instrument = CurrencyTestInstrument::successful(Currency::EUR);
        let instrument_arc: Arc<dyn Instrument> = Arc::new(instrument.clone());
        let market = MarketContext::new();
        let probe = Money::new(100.0, Currency::EUR);
        let mut notes = Vec::new();
        let mut num_repricings = 1;

        let reused = translation_t0_value(
            &instrument_arc,
            &market,
            date!(2025 - 01 - 01),
            None,
            Some(probe),
            &mut notes,
            &mut num_repricings,
        )
        .expect("probe should be reusable");

        assert_eq!(reused, probe);
        assert_eq!(instrument.valuation_calls(), 0);
        assert_eq!(num_repricings, 1);

        let repriced = translation_t0_value(
            &instrument_arc,
            &market,
            date!(2025 - 01 - 01),
            Some(&ModelParamsSnapshot::None),
            Some(probe),
            &mut notes,
            &mut num_repricings,
        )
        .expect("model-parameter path should reprice");

        assert_eq!(repriced.currency(), Currency::EUR);
        assert_eq!(instrument.valuation_calls(), 1);
        assert_eq!(num_repricings, 2);
    }

    #[test]
    fn failed_rounding_probe_falls_back_to_notional_without_counting() {
        let instrument = CurrencyTestInstrument::failing();
        let market = MarketContext::new();

        let probe = probe_rounding_currency(&instrument, &market, date!(2025 - 01 - 01), Some(6));

        assert_eq!(probe.currency, Some(Currency::USD));
        assert_eq!(probe.value, None);
        assert_eq!(probe.successful_valuations(), 0);
        assert_eq!(instrument.valuation_calls(), 1);
    }
}
