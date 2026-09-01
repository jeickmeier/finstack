//! Shared pricing runner helpers for instrument-level golden fixtures.

use crate::golden::schema::{GoldenFixture, Market};
use finstack_quant_calibration::api::engine;
use finstack_quant_calibration::api::schema::CalibrationEnvelope;
use finstack_quant_calibration::recalibration::CachedRecalibrationProvider;
use finstack_quant_core::contract::LoadLimits;
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_valuations::instruments::PricingOptions;
use finstack_quant_valuations::pricer::{
    price_instrument_from_json as canonical_price_instrument_from_json, JsonPricingRequest,
};
use std::collections::BTreeMap;
use std::sync::Arc;

fn price_instrument_from_json(
    instrument_json: &str,
    market: &MarketContext,
    as_of: &str,
    model: &str,
    metrics: &[String],
    instrument_pricing_overrides_json: Option<&str>,
    market_history_json: Option<&str>,
) -> finstack_quant_core::Result<finstack_quant_valuations::results::ValuationResult> {
    canonical_price_instrument_from_json(JsonPricingRequest {
        instrument_json,
        market,
        as_of,
        model,
        metrics,
        instrument_pricing_overrides_json,
        market_history_json,
        pricing_options: PricingOptions::default()
            .with_recalibration_provider(Arc::new(CachedRecalibrationProvider::new())),
    })
}

fn metric_base(metric: &str) -> &str {
    metric.split_once("::").map_or(metric, |(base, _)| base)
}

/// Metrics to request from the pricer, derived from the expected-output keys.
///
/// `npv` is always produced by the pricer and is therefore never requested.
pub(crate) fn requested_metrics(fixture: &GoldenFixture) -> Vec<String> {
    let mut metrics = Vec::new();
    for key in fixture.expected.keys() {
        let base = metric_base(key);
        if base != "npv" && !metrics.iter().any(|m| m == base) {
            metrics.push(base.to_string());
        }
    }
    metrics
}

fn resolve_market(market: &Market) -> Result<MarketContext, String> {
    match market {
        Market::Snapshot { data } => serde_json::from_value::<MarketContext>(data.clone())
            .map_err(|err| format!("parse market snapshot: {err}")),
        Market::Envelope { envelope } => {
            let bytes = serde_json::to_vec(envelope)
                .map_err(|error| format!("encode market envelope: {error}"))?;
            let (env, _load_report) =
                CalibrationEnvelope::from_slice_strict(&bytes, &LoadLimits::default())
                    .map_err(|error| format!("strictly load market envelope: {error}"))?;
            let result = engine::execute(&env).map_err(|error| {
                let plan_id = &env.plan.id;
                let details = error.details();
                format!(
                    "calibrate market envelope for plan '{plan_id}' failed \
                     (stage={}, category={}, step={:?}): {}",
                    details.stage.as_str(),
                    details.category,
                    details.step_id,
                    details.cause,
                )
            })?;
            let plan_id = env.plan.id;
            MarketContext::try_from(result.result.final_market)
                .map_err(|err| format!("rehydrate calibrated market for plan '{plan_id}': {err}"))
        }
    }
}

/// Price an instrument fixture that follows the common pricing input contract.
pub(crate) fn run_pricing_fixture(
    fixture: &GoldenFixture,
) -> Result<BTreeMap<String, f64>, String> {
    let pricing = fixture
        .pricing()
        .ok_or("pricing runner requires a 'pricing' fixture body")?;
    let market = resolve_market(&pricing.market)?;
    let instrument_json = serde_json::to_string(&pricing.instrument)
        .map_err(|err| format!("serialize instrument: {err}"))?;
    let metrics = requested_metrics(fixture);

    let result = price_instrument_from_json(
        &instrument_json,
        &market,
        &fixture.metadata.valuation_date,
        &pricing.model,
        &metrics,
        None,
        None,
    )
    .map_err(|err| format!("price instrument JSON: {err}"))?;

    let mut actuals = BTreeMap::new();
    for metric in fixture.expected.keys() {
        let value = if metric == "npv" {
            result.value.amount()
        } else {
            *result
                .measures
                .get(metric.as_str())
                .ok_or_else(|| format!("result missing metric '{metric}'"))?
        };
        actuals.insert(metric.clone(), value);
    }
    Ok(actuals)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::golden::schema::SCHEMA;
    use finstack_quant_core::market_data::bumps::{BumpSpec, MarketBump};
    use finstack_quant_core::types::CurveId;
    use finstack_quant_valuations::instruments::{Instrument, InstrumentEnvelope, InstrumentJson};

    fn pricing_fixture(market: serde_json::Value) -> GoldenFixture {
        let json = serde_json::json!({
            "schema": SCHEMA,
            "metadata": {
                "name": "market_test",
                "domain": "rates.deposit",
                "description": "market resolution test",
                "valuation_date": "2026-04-30",
                "source": "formula",
                "source_detail": "unit test",
                "captured_by": "test",
                "captured_on": "2026-04-30",
                "last_reviewed_by": "test",
                "last_reviewed_on": "2026-04-30",
                "review_interval_months": 6,
                "regen_command": ""
            },
            "kind": "pricing",
            "model": "discounting",
            "market": market,
            "instrument": {},
            "expected": {"npv": 0.0},
            "tolerances": {"npv": {"abs": 0.0}}
        });
        serde_json::from_value(json).expect("parse fixture")
    }

    fn minimal_market() -> serde_json::Value {
        serde_json::json!({
            "schema_version": 1,
            "curves": [],
            "fx": null,
            "surfaces": [],
            "prices": {},
            "series": [],
            "inflation_indices": [],
            "dividends": [],
            "credit_indices": [],
            "fx_delta_vol_surfaces": [],
            "vol_cubes": [],
            "collateral": {},
            "hierarchy": null
        })
    }

    fn minimal_envelope() -> serde_json::Value {
        serde_json::json!({
            "schema": "finstack_quant.calibration/1",
            "plan": {"id": "test_envelope", "quote_sets": {}, "steps": [], "settings": {}}
        })
    }

    fn structured_credit_fixture() -> GoldenFixture {
        serde_json::from_str(include_str!(
            "data/pricing/regression_goldens/structured_credit/abs_credit_card_senior.json"
        ))
        .expect("parse structured-credit golden fixture")
    }

    fn price_fixture_npv(
        fixture: &GoldenFixture,
        market: &MarketContext,
        instrument_json: &str,
    ) -> f64 {
        let pricing = fixture.pricing().expect("pricing body");
        let result = price_instrument_from_json(
            instrument_json,
            market,
            &fixture.metadata.valuation_date,
            &pricing.model,
            &[],
            None,
            None,
        )
        .expect("structured-credit fixture should price");
        result.value.amount()
    }

    fn direct_parallel_dv01(
        fixture: &GoldenFixture,
        market: &MarketContext,
        instrument_json: &str,
        curve_ids: &[CurveId],
    ) -> f64 {
        let bumped_market = |direction| {
            market
                .bump(curve_ids.iter().cloned().map(|id| MarketBump::Curve {
                    id,
                    spec: BumpSpec::parallel_bp(direction),
                }))
                .expect("declared curve should support a parallel bump")
        };
        let up = price_fixture_npv(fixture, &bumped_market(1.0), instrument_json);
        let down = price_fixture_npv(fixture, &bumped_market(-1.0), instrument_json);
        (up - down) / 2.0
    }

    fn assert_close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() < tolerance,
            "expected {expected:.15}, got {actual:.15}"
        );
    }

    #[test]
    fn requested_metrics_derives_from_expected_and_excludes_npv() {
        let json = serde_json::json!({
            "schema": SCHEMA,
            "metadata": {
                "name": "m", "domain": "rates.irs", "description": "d",
                "valuation_date": "2026-04-30", "source": "formula",
                "source_detail": "u", "captured_by": "t", "captured_on": "2026-04-30",
                "last_reviewed_by": "t", "last_reviewed_on": "2026-04-30",
                "review_interval_months": 6, "regen_command": ""
            },
            "kind": "pricing",
            "model": "discounting",
            "market": {"kind": "envelope", "envelope": minimal_envelope()},
            "instrument": {},
            "expected": {"npv": 1.0, "dv01": 1.0, "bucketed_dv01::USD-OIS::1y": 1.0},
            "tolerances": {
                "npv": {"abs": 1.0}, "dv01": {"abs": 1.0},
                "bucketed_dv01::USD-OIS::1y": {"abs": 1.0}
            }
        });
        let fixture: GoldenFixture = serde_json::from_value(json).expect("parse");
        let metrics = requested_metrics(&fixture);
        assert_eq!(
            metrics,
            vec!["bucketed_dv01".to_string(), "dv01".to_string()]
        );
    }

    #[test]
    fn resolve_market_snapshot_only() {
        let fixture =
            pricing_fixture(serde_json::json!({"kind": "snapshot", "data": minimal_market()}));
        let pricing = fixture.pricing().expect("pricing body");
        resolve_market(&pricing.market).expect("snapshot resolves");
    }

    #[test]
    fn resolve_market_envelope_only() {
        let fixture = pricing_fixture(
            serde_json::json!({"kind": "envelope", "envelope": minimal_envelope()}),
        );
        let pricing = fixture.pricing().expect("pricing body");
        resolve_market(&pricing.market).expect("envelope resolves through engine::execute");
    }

    #[test]
    fn structured_credit_dependencies_preserve_curve_roles_and_fixing_ids() {
        let fixture = structured_credit_fixture();
        let pricing = fixture.pricing().expect("pricing body");
        let envelope: InstrumentEnvelope = serde_json::from_value(pricing.instrument.clone())
            .expect("parse structured-credit instrument envelope");
        let instrument = envelope.instrument;
        let InstrumentJson::StructuredCredit(instrument) = instrument else {
            panic!("fixture should contain structured credit");
        };

        let dependencies = instrument
            .market_dependencies()
            .expect("collect structured-credit dependencies");
        let discount_curves: Vec<_> = dependencies
            .curves
            .discount_curves
            .iter()
            .map(|id| id.as_str())
            .collect();
        let forward_curves: Vec<_> = dependencies
            .curves
            .forward_curves
            .iter()
            .map(|id| id.as_str())
            .collect();

        assert_eq!(discount_curves, ["USD-SOFR-DISC"]);
        assert_eq!(forward_curves, ["SOFR-3M"]);
        assert!(dependencies.curves.credit_curves.is_empty());
        assert!(dependencies.curves.inflation_curves.is_empty());
        assert_eq!(dependencies.series_ids, ["FIXING:SOFR-3M"]);
        assert!(dependencies.market_scalar_ids.is_empty());
        assert!(dependencies.volatility_dependencies.is_empty());
        assert!(dependencies.fx_pairs.is_empty());
    }

    #[test]
    #[ignore = "slow: covered by mise goldens-test or mise rust-test-slow"]
    fn structured_credit_dv01_matches_declared_curve_repricing() {
        let fixture = structured_credit_fixture();
        let pricing = fixture.pricing().expect("pricing body");
        let market = resolve_market(&pricing.market).expect("resolve fixture market");
        let instrument_json =
            serde_json::to_string(&pricing.instrument).expect("serialize instrument");

        let discount = direct_parallel_dv01(
            &fixture,
            &market,
            &instrument_json,
            &[CurveId::new("USD-SOFR-DISC")],
        );
        let sofr_3m = direct_parallel_dv01(
            &fixture,
            &market,
            &instrument_json,
            &[CurveId::new("SOFR-3M")],
        );
        let combined = direct_parallel_dv01(
            &fixture,
            &market,
            &instrument_json,
            &[CurveId::new("USD-SOFR-DISC"), CurveId::new("SOFR-3M")],
        );

        let registry_result = price_instrument_from_json(
            &instrument_json,
            &market,
            &fixture.metadata.valuation_date,
            &pricing.model,
            &["dv01".to_string()],
            None,
            None,
        )
        .expect("registry DV01 should price");
        let registry_dv01 = registry_result.measures["dv01"];

        assert_close(discount, -3_051.583_130_820_654, 1e-6);
        assert_close(sofr_3m, 2_893.358_724_945_225, 1e-6);
        // Take the combined target from the fixture rather than repeating the
        // literal here, so a re-blessed fixture cannot leave this test stale.
        assert_close(combined, fixture.expected["dv01"], 1e-6);
        assert_close(combined, registry_dv01, 1e-8);
        // Bumping both curves together is not exactly the sum of the two
        // single-curve bumps: the OC/IC triggers divert cash discontinuously in
        // rates, so the legs carry a small cross-term. Bound it relative to the
        // leg size instead of in absolute currency.
        let cross_term = (combined - (discount + sofr_3m)).abs();
        assert!(
            cross_term < 1e-5 * discount.abs(),
            "curve-leg cross-term {cross_term:.15} exceeds 1e-5 of the discount leg"
        );
        assert!((combined - discount).abs() > 2_000.0);
    }
}
