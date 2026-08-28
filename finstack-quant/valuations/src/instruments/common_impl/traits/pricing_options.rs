//! Pricing options implementation used by the traits subsystem.
//!
// Pricing Options

use crate::metrics::risk::MarketHistory;
use crate::metrics::MetricRegistry;
use crate::pricer::{shared_standard_registry, ModelKey, PricerRegistry};
use finstack_quant_core::config::FinstackConfig;
use std::sync::Arc;

/// Optional overrides for a pricing-and-metrics request.
///
/// This struct consolidates optional parameters for `Instrument::price_with_metrics`,
/// replacing the proliferation of `_with_config`, `_with_market_history` variants.
///
/// Pass [`PricingOptions::default`] when no overrides are needed, or chain
/// [`PricingOptions::with_config`] and [`PricingOptions::with_market_history`]
/// to supply a metric configuration or the history required by historical VaR.
#[derive(Clone, Default)]
pub struct PricingOptions {
    /// Optional configuration for metric computation (bump sizes, tolerances, etc.)
    pub config: Option<Arc<FinstackConfig>>,
    /// Optional market history for Historical VaR / Expected Shortfall metrics
    pub market_history: Option<Arc<MarketHistory>>,
    /// Optional explicit pricing model override.
    ///
    /// When `None`, [`super::Instrument::price_with_metrics`] uses
    /// [`super::Instrument::default_model`]. Set this to select a different registered
    /// pricing path, such as hazard-rate or tree/OAS pricing, without dropping
    /// down to [`crate::pricer::PricerRegistry`] directly.
    pub model: Option<ModelKey>,
    /// Optional explicit pricer registry override.
    pub registry: Option<Arc<PricerRegistry>>,
    /// Quote-recalibration service shared by one immutable pricing batch.
    pub recalibration_provider: Option<Arc<dyn crate::recalibration::RecalibrationProvider>>,
}

impl PricingOptions {
    /// Create new pricing options with no extras.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the configuration for metric computation.
    ///
    /// The config controls sensitivity bump sizes and other calculation parameters.
    ///
    /// # Arguments
    ///
    /// * `cfg` - Metric configuration to clone into this request. Supplies the
    ///   finite-difference bump sizes (rate bumps in decimal, credit-spread bumps
    ///   in basis points, relative spot/vol bumps), solver tolerances, and
    ///   iteration caps used by sensitivity calculations. Cloned into an `Arc`, so
    ///   the caller retains ownership.
    pub fn with_config(mut self, cfg: &FinstackConfig) -> Self {
        self.config = Some(Arc::new(cfg.clone()));
        self
    }

    /// Set the market history for Historical VaR / Expected Shortfall.
    ///
    /// Required for computing `MetricId::HVar` and `MetricId::ExpectedShortfall`.
    ///
    /// # Arguments
    ///
    /// * `history` - Shared historical market snapshots used to build the return
    ///   distribution for historical VaR and expected shortfall. The observation
    ///   dates must cover the lookback window requested by the metric
    ///   configuration; when the history is absent or too short, those metrics
    ///   report an error rather than falling back to a parametric estimate.
    pub fn with_market_history(mut self, history: Arc<MarketHistory>) -> Self {
        self.market_history = Some(history);
        self
    }

    /// Set the pricing model for this pricing request.
    ///
    /// Most callers can stay on [`super::Instrument::price_with_metrics`] and use this
    /// override only when they need a non-default registered model.
    ///
    /// # Arguments
    ///
    /// * `model` - Model supplied by the caller for this operation
    pub fn with_model(mut self, model: ModelKey) -> Self {
        self.model = Some(model);
        self
    }

    /// Set an explicit pricer registry override for this pricing request.
    ///
    /// # Arguments
    ///
    /// * `registry` - Registry supplied by the caller for this operation
    pub fn with_registry(mut self, registry: Arc<PricerRegistry>) -> Self {
        self.registry = Some(registry);
        self
    }

    /// Attach the quote-recalibration service for this immutable pricing batch.
    ///
    /// # Arguments
    ///
    /// * `provider` - Shared service that owns quote decoding, replay, and
    ///   batch-local rate and credit calibration caches.
    pub fn with_recalibration_provider(
        mut self,
        provider: Arc<dyn crate::recalibration::RecalibrationProvider>,
    ) -> Self {
        self.recalibration_provider = Some(provider);
        self
    }

    /// Set an explicit metric registry for this pricing request.
    ///
    /// The metric registry is attached to the selected pricer registry so the
    /// existing [`Self::registry`] field remains the single dispatch bundle.
    /// Call [`Self::with_registry`] first when overriding both registries; a
    /// later `with_registry` call replaces the complete registry selection.
    pub fn with_metric_registry(mut self, metric_registry: Arc<MetricRegistry>) -> Self {
        let registry = self
            .registry
            .as_deref()
            .cloned()
            .unwrap_or_else(|| shared_standard_registry().as_ref().clone())
            .with_metric_registry(metric_registry);
        self.registry = Some(Arc::new(registry));
        self
    }
}
