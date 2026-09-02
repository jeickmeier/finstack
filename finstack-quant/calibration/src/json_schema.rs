//! Checked-in JSON schemas owned by the calibration crate.

use finstack_quant_core::schema::{SchemaArtifact, SchemaKind};
use serde_json::Value;
use std::sync::OnceLock;

fn calibration_examples() -> finstack_quant_core::Result<Vec<Value>> {
    let plan = crate::api::schema::CalibrationPlan {
        id: "usd_curves".to_string(),
        description: Some("Bootstrap the USD OIS discount curve.".to_string()),
        quote_sets: Default::default(),
        steps: Vec::new(),
        settings: Default::default(),
    };
    let envelope = crate::api::schema::CalibrationEnvelope::new(plan, Vec::new(), Vec::new());
    serde_json::to_value(envelope)
        .map(|value| vec![value])
        .map_err(|error| {
            finstack_quant_core::Error::Internal(format!(
                "serialize calibration schema example: {error}"
            ))
        })
}

fn market_quote_examples() -> finstack_quant_core::Result<Vec<Value>> {
    let quote =
        crate::quotes::market_quote::MarketQuote::Rates(crate::quotes::rates::RateQuote::Deposit {
            id: crate::quotes::ids::QuoteId::new("USD-SOFR-DEP-1M"),
            index: finstack_quant_core::types::IndexId::new("USD-SOFR"),
            pillar: crate::quotes::ids::Pillar::Tenor("1M".parse()?),
            rate: 0.0525,
        });
    serde_json::to_value(quote)
        .map(|value| vec![value])
        .map_err(|error| {
            finstack_quant_core::Error::Internal(format!(
                "serialize market quote schema example: {error}"
            ))
        })
}

/// Return the calibration schema registry as a shared, lazily built slice.
#[must_use]
pub fn artifacts() -> &'static [SchemaArtifact] {
    static CACHE: OnceLock<Vec<SchemaArtifact>> = OnceLock::new();
    CACHE.get_or_init(build_artifacts)
}

fn build_artifacts() -> Vec<SchemaArtifact> {
    vec![
        SchemaArtifact::new::<crate::api::schema::CalibrationEnvelope>(
            "schemas/calibration/1/calibration.schema.json",
            "https://finstack_quant.dev/schemas/calibration/1/calibration.schema.json",
            "Calibration",
            "Canonical typed calibration request and result envelope.",
        )
        .with_packager(finstack_quant_valuations::schema::package_valuations_schema)
        .with_kind(SchemaKind::Input)
        .with_summary(
            "Build a market from quotes: a calibration plan, flat market data, and any pre-built curves or surfaces.",
        )
        .with_examples(calibration_examples),
        SchemaArtifact::new::<crate::quotes::market_quote::MarketQuote>(
            "schemas/market/1/market_quote.schema.json",
            "https://finstack_quant.dev/schemas/market/1/market_quote.schema.json",
            "Market Quote",
            "Canonical tagged market quote.",
        )
        .with_packager(finstack_quant_valuations::schema::package_valuations_schema)
        .with_summary("One market observation, tagged by asset class.")
        .with_examples(market_quote_examples),
    ]
}
