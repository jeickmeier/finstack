//! CDS metrics module.
//!
//! Provides metric calculators specific to `CreditDefaultSwap`, split into
//! focused files. The calculators compose with the shared metrics framework
//! and are registered via `register_cds_metrics`.
//!
//! Exposed metrics:
//! - Par spread (bp)
//! - Risky PV01
//! - Risky annuity
//! - CS01
//! - Protection leg PV
//! - Premium leg PV
//! - Expected loss
//! - Jump to default (includes accrued premium)
//! - Jump to default LGD only (excludes accrued premium)

mod cs_gamma;
mod dv01;
mod expected_loss;
mod jump_to_default;
mod par_spread;
mod pv_premium;
mod pv_protection;
mod recovery01;
mod risky_annuity;
mod risky_pv01;

use crate::market::quotes::cds::CdsQuote;
use crate::market::quotes::ids::Pillar;
use crate::metrics::MetricRegistry;
use finstack_quant_core::dates::DayCountContext;
use finstack_quant_core::market_data::term_structures::{HazardCalibrationRecipe, HazardCurve};
use finstack_quant_core::HashMap;

pub(crate) fn market_doc_clause(
    cds: &crate::instruments::credit_derivatives::cds::CreditDefaultSwap,
) -> crate::market::conventions::ids::CdsDocClause {
    use crate::instruments::credit_derivatives::cds::CdsDocClause as InstrumentClause;
    use crate::market::conventions::ids::CdsDocClause as MarketClause;

    match cds.doc_clause_effective() {
        InstrumentClause::Cr14 | InstrumentClause::Mr14 | InstrumentClause::Xr14 => {
            MarketClause::IsdaNa
        }
        InstrumentClause::Mm14 | InstrumentClause::IsdaEu => MarketClause::IsdaEu,
        InstrumentClause::IsdaNa => MarketClause::IsdaNa,
        InstrumentClause::IsdaAs | InstrumentClause::IsdaAu | InstrumentClause::IsdaNz => {
            MarketClause::IsdaAs
        }
        InstrumentClause::Custom => MarketClause::Custom,
    }
}

pub(crate) fn hazard_with_deal_quote(
    cds: &crate::instruments::credit_derivatives::cds::CreditDefaultSwap,
    hazard: &HazardCurve,
) -> finstack_quant_core::Result<Option<HazardCurve>> {
    let Some(quote_bp) = cds.instrument_pricing_overrides.market_quotes.cds_quote_bp else {
        return Ok(None);
    };
    if !cds.uses_clean_price() {
        return Ok(None);
    }

    let Some(source_recipe) = hazard.hazard_calibration() else {
        return Ok(None);
    };
    let mut risk_inputs = source_recipe.spread_risk_inputs.clone();
    let template_input =
        risk_inputs
            .first()
            .ok_or_else(|| finstack_quant_core::Error::Calibration {
                message: format!(
                    "CDS quote override for '{}' requires at least one spread-risk replay input",
                    hazard.id()
                ),
                category: "cs01_rebootstrap".to_string(),
            })?;
    let template_quote: CdsQuote =
        serde_json::from_value(template_input.quote.clone()).map_err(|error| {
            finstack_quant_core::Error::Validation(format!(
                "CDS quote override for '{}' found an invalid spread-risk quote: {error}",
                hazard.id()
            ))
        })?;
    let deal_quote = match template_quote {
        CdsQuote::CdsParSpread {
            id,
            entity,
            convention,
            recovery_rate,
            ..
        } => CdsQuote::CdsParSpread {
            id,
            entity,
            convention,
            pillar: Pillar::Date(cds.premium.end),
            spread_bp: quote_bp,
            recovery_rate,
        },
        CdsQuote::CdsUpfront { .. } => {
            return Err(finstack_quant_core::Error::Calibration {
                message: format!(
                    "CDS quote override for '{}' found an upfront quote in par-spread risk inputs",
                    hazard.id()
                ),
                category: "cs01_rebootstrap".to_string(),
            });
        }
    };
    let build_ctx = crate::market::BuildCtx::new(hazard.base_date(), 1.0, HashMap::default());
    let contractual_pillar =
        crate::market::build::cds::resolve_cds_quote_dates(&deal_quote, &build_ctx)?.maturity;
    let contractual_time = hazard.day_count().year_fraction(
        hazard.base_date(),
        contractual_pillar,
        DayCountContext::default(),
    )?;
    let target_index = risk_inputs
        .iter()
        .enumerate()
        .find(|(_, input)| {
            let time_scale = input.pillar_time.abs().max(contractual_time.abs()).max(1.0);
            input.pillar_date == contractual_pillar
                && (input.pillar_time - contractual_time).abs() <= 1e-12 * time_scale
        })
        .map(|(index, _)| index)
        .ok_or_else(|| finstack_quant_core::Error::Calibration {
            message: format!(
                "CDS quote override for '{}' has no exact spread-risk replay pillar for \
                 contractual date {} (time {})",
                hazard.id(),
                contractual_pillar,
                contractual_time
            ),
            category: "cs01_rebootstrap".to_string(),
        })?;
    let source_quote: CdsQuote = serde_json::from_value(risk_inputs[target_index].quote.clone())
        .map_err(|error| {
            finstack_quant_core::Error::Validation(format!(
                "CDS quote override for '{}' found an invalid spread-risk quote: {error}",
                hazard.id()
            ))
        })?;
    let overridden_quote = match source_quote {
        CdsQuote::CdsParSpread {
            id,
            entity,
            convention,
            pillar,
            recovery_rate,
            ..
        } => CdsQuote::CdsParSpread {
            id,
            entity,
            convention,
            pillar,
            spread_bp: quote_bp,
            recovery_rate,
        },
        CdsQuote::CdsUpfront { .. } => {
            return Err(finstack_quant_core::Error::Calibration {
                message: format!(
                    "CDS quote override for '{}' found an upfront quote in par-spread risk inputs",
                    hazard.id()
                ),
                category: "cs01_rebootstrap".to_string(),
            });
        }
    };
    risk_inputs[target_index].quote = serde_json::to_value(overridden_quote).map_err(|error| {
        finstack_quant_core::Error::Validation(format!(
            "failed to persist CDS quote override for '{}': {error}",
            hazard.id()
        ))
    })?;
    let derived_recipe = HazardCalibrationRecipe::new(
        source_recipe.hazard_params.clone(),
        source_recipe.calibration_inputs.clone(),
        risk_inputs,
        source_recipe.calibration_config.clone(),
    )?;

    Ok(Some(
        hazard
            .to_builder_with_id(hazard.id().clone())
            .hazard_calibration(derived_recipe)
            .build()?,
    ))
}

/// Per-deal CS01 conventions for `CreditDefaultSwap`.
///
/// Drives the generic credit CS01 calculators
/// ([`crate::metrics::sensitivities::cs01::CreditParallelCs01`] /
/// [`CreditBucketedCs01`](crate::metrics::sensitivities::cs01::CreditBucketedCs01))
/// so the hazard curve is re-bootstrapped under this CDS's doc clause and
/// valuation convention. The optional curve override reproduces the legacy
/// deal-quote hazard substitution.
impl crate::metrics::sensitivities::cs01::CdsCs01Conventions
    for crate::instruments::credit_derivatives::cds::CreditDefaultSwap
{
    fn cs01_bootstrap_convention(
        &self,
        _as_of: finstack_quant_core::dates::Date,
    ) -> finstack_quant_core::Result<(
        crate::market::conventions::ids::CdsDocClause,
        crate::instruments::credit_derivatives::cds::CdsValuationConvention,
    )> {
        Ok((market_doc_clause(self), self.valuation_convention))
    }

    fn cs01_curve_override(
        &self,
        curves: &finstack_quant_core::market_data::context::MarketContext,
        hazard_id: &finstack_quant_core::types::CurveId,
        _as_of: finstack_quant_core::dates::Date,
    ) -> finstack_quant_core::Result<Option<finstack_quant_core::market_data::context::MarketContext>>
    {
        let hazard = curves.get_hazard(hazard_id.as_str())?;
        Ok(hazard_with_deal_quote(self, hazard.as_ref())?
            .map(|quote_hazard| curves.clone().insert(quote_hazard)))
    }
}

/// Register all CDS metrics with the registry
pub(crate) fn register_cds_metrics(
    registry: &mut MetricRegistry,
) -> std::result::Result<(), crate::metrics::MetricRegistryError> {
    use crate::metrics::MetricId;
    use crate::pricer::InstrumentType;
    use std::sync::Arc;

    registry.replace_metric(
        MetricId::RiskyPv01,
        Arc::new(risky_pv01::RiskyPv01Calculator),
        &[InstrumentType::Cds],
    )?;

    // Recovery01 (custom metric - recovery rate sensitivity)
    registry.replace_metric(
        MetricId::Recovery01,
        Arc::new(recovery01::Recovery01Calculator),
        &[InstrumentType::Cds],
    )?;

    // JumpToDefaultLgdOnly (custom metric - LGD only, excludes accrued)
    registry.replace_metric(
        MetricId::custom("jump_to_default_lgd_only"),
        Arc::new(jump_to_default::JumpToDefaultLgdOnlyCalculator),
        &[InstrumentType::Cds],
    )?;

    // Standard metrics using macro
    crate::register_metrics! {
        registry: registry,
        instrument: InstrumentType::Cds,
        metrics: [
            (ParSpread, par_spread::ParSpreadCalculator),
            (RiskyAnnuity, risky_annuity::RiskyAnnuityCalculator),
            (Cs01, crate::metrics::sensitivities::cs01::CreditParallelCs01::<
                crate::instruments::credit_derivatives::cds::CreditDefaultSwap,
            >::default()),
            (BucketedCs01, crate::metrics::sensitivities::cs01::CreditBucketedCs01::<
                crate::instruments::credit_derivatives::cds::CreditDefaultSwap,
            >::default()),
            (CsGamma, cs_gamma::CsGammaCalculator),
            (ProtectionLegPv, pv_protection::ProtectionLegPvCalculator),
            (PremiumLegPv, pv_premium::PremiumLegPvCalculator),
            (ExpectedLoss, expected_loss::ExpectedLossCalculator),
            (JumpToDefault, jump_to_default::JumpToDefaultCalculator),
            (DefaultExposure, jump_to_default::DefaultExposureCalculator),
            (Dv01, dv01::CdsDv01Calculator),
            (BucketedDv01, crate::metrics::UnifiedDv01Calculator::<
                crate::instruments::CreditDefaultSwap,
            >::new(crate::metrics::Dv01CalculatorConfig::triangular_key_rate())),
        ]
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::credit_derivatives::cds::CreditDefaultSwap;
    use crate::market::conventions::ids::{CdsConventionKey, CdsDocClause};
    use crate::market::quotes::cds::CdsQuote;
    use crate::market::quotes::ids::{Pillar, QuoteId};
    use finstack_quant_core::market_data::term_structures::{
        HazardCalibrationInput, HazardCalibrationRecipe,
    };

    #[test]
    fn cds_quote_override_updates_replay_risk_quote() {
        let mut cds = CreditDefaultSwap::example();
        cds.instrument_pricing_overrides.market_quotes.cds_quote_bp = Some(321.0);
        let quote = CdsQuote::CdsParSpread {
            id: QuoteId::new("ACME-5Y"),
            entity: "ACME".to_string(),
            convention: CdsConventionKey {
                currency: cds.notional.currency(),
                doc_clause: CdsDocClause::IsdaNa,
            },
            pillar: Pillar::Date(cds.premium.end),
            spread_bp: 150.0,
            recovery_rate: cds.protection.recovery_rate,
        };
        let input = HazardCalibrationInput {
            quote: serde_json::to_value(quote).expect("serialize quote"),
            pillar_date: cds.premium.end,
            pillar_time: finstack_quant_core::dates::DayCount::Act365F
                .year_fraction(
                    cds.premium.start,
                    cds.premium.end,
                    DayCountContext::default(),
                )
                .expect("valid pillar time"),
        };
        let recipe = HazardCalibrationRecipe::new(
            serde_json::json!({}),
            vec![input.clone()],
            vec![input],
            serde_json::json!({}),
        )
        .expect("valid replay recipe");
        let hazard = HazardCurve::builder(cds.protection.credit_curve_id.clone())
            .base_date(cds.premium.start)
            .recovery_rate(cds.protection.recovery_rate)
            .knots([(1.0, 0.01), (5.0, 0.02)])
            .hazard_calibration(recipe)
            .build()
            .expect("recipe-backed hazard");

        let derived = hazard_with_deal_quote(&cds, &hazard)
            .expect("derive deal replay")
            .expect("deal quote override");
        let risk_quote: CdsQuote = serde_json::from_value(
            derived
                .hazard_calibration()
                .expect("derived curve must remain replayable")
                .spread_risk_inputs[0]
                .quote
                .clone(),
        )
        .expect("deserialize risk quote");
        assert!(matches!(
            risk_quote,
            CdsQuote::CdsParSpread { spread_bp, .. } if spread_bp == 321.0
        ));
    }
}
