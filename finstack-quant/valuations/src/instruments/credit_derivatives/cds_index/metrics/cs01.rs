//! CDS Index CS01 metric calculators.
//!
//! Both calculators report CS01 against the [canonical convention][canonical]:
//! a parallel 1 bp shock to credit spreads with a symmetric (central) finite
//! difference `(PV(s + 1bp) − PV(s − 1bp)) / 2`. They differ only in *which*
//! spread is shocked and how the index aggregates per-name sensitivity:
//!
//! - [`Cs01Calculator`]: parallel CS01 derived from per-name finite differences
//!   summed over surviving constituents (or computed on the synthetic CDS in
//!   `SingleCurve` mode). Routed through [`CDSIndex::cs01`]; treats each
//!   constituent's bump as a parallel par-spread shock.
//! - [`Cs01HazardCalculator`]: parallel hazard-shift CS01 that bumps **every**
//!   credit curve declared as a dependency by the index (one synthetic curve
//!   in `SingleCurve` mode, N constituent curves in `Constituents` mode) and
//!   reprices end-to-end. Replaces the generic `GenericParallelCs01Hazard`,
//!   which would only bump the (unused) index-level curve in `Constituents`
//!   mode.
//! - [`CdsIndexBucketedCs01Calculator`]: key-rate (per-tenor) par-spread CS01 —
//!   the bucketed counterpart of [`Cs01Calculator`]. Applies the par-spread
//!   shock one standard tenor at a time to the same mode-aware credit-curve
//!   set as [`Cs01HazardCalculator`], reprices end-to-end, and stores a
//!   per-tenor series whose sum reconciles to the parallel `Cs01`.
//!
//! Sign convention (per canonical reference):
//! - Long index protection (sell protection) → CS01 negative.
//! - Short index protection (buy protection) → CS01 positive.
//!
//! [canonical]: crate::metrics::sensitivities::cs01
//! [`CDSIndex::cs01`]: crate::instruments::credit_derivatives::cds_index::CDSIndex::cs01

use crate::calibration::bumps::hazard::bump_hazard_shift;
use crate::calibration::bumps::BumpRequest;
use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::credit_derivatives::cds_index::{CDSIndex, IndexPricing};
use crate::metrics::sensitivities::config as sens_config;
use crate::metrics::sensitivities::cs01::sensitivity_central_diff;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::market_data::term_structures::HazardCurve;
use finstack_quant_core::types::CurveId;
use finstack_quant_core::{Error, Result};
use std::borrow::Cow;
use std::sync::Arc;

/// Parallel CS01 calculator for CDS Index (per-name finite difference).
pub(crate) struct Cs01Calculator;

impl MetricCalculator for Cs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let idx: &CDSIndex = context.instrument_as()?;
        idx.cs01(&context.curves, context.as_of)
    }
}

/// Parallel hazard-shift CS01 for CDS Index.
///
/// Bumps every credit curve declared as a dependency by the instrument
/// (in `Constituents` mode this is N hazard curves, one per surviving name),
/// reprices, and computes a central difference. This is correct for
/// `IndexPricing::Constituents` where the generic single-curve form would
/// only bump the unused index-level curve.
pub(crate) struct Cs01HazardCalculator;

impl MetricCalculator for Cs01HazardCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let index: &CDSIndex = context.instrument_as()?;

        let bump_bp =
            sens_config::from_context_or_default(context.config(), context.get_metric_overrides())?
                .credit_spread_bump_bp;

        // Determine which credit curves to bump. In SingleCurve mode this is
        // just the index-level curve; in Constituents mode it's the union
        // of surviving constituent curves.
        let credit_ids: Vec<_> = match index.pricing {
            IndexPricing::SingleCurve => {
                vec![index.protection.credit_curve_id.clone()]
            }
            IndexPricing::Constituents => {
                // Pull from canonical dependencies but skip the index-level curve
                // because it is informational only in Constituents mode.
                let curves = index.market_dependencies()?.curves;
                curves
                    .credit_curves
                    .into_iter()
                    .filter(|id| id != &index.protection.credit_curve_id)
                    .collect()
            }
        };

        if credit_ids.is_empty() {
            return Ok(0.0);
        }

        let bump_all = |ctx: &MarketContext, bp: f64| -> Result<MarketContext> {
            let mut out = ctx.clone();
            for id in &credit_ids {
                let hazard = ctx.get_hazard(id.as_str())?;
                let bumped = bump_hazard_shift(hazard.as_ref(), &BumpRequest::Parallel(bp))?;
                out = out.insert(bumped);
            }
            Ok(out)
        };

        let base_ctx = context.curves.as_ref();
        let ctx_up = bump_all(base_ctx, bump_bp)?;
        let ctx_down = bump_all(base_ctx, -bump_bp)?;

        let as_of = context.as_of;
        let pv_up = context.reprice_raw(&ctx_up, as_of)?;
        let pv_down = context.reprice_raw(&ctx_down, as_of)?;

        Ok((pv_up - pv_down) / (2.0 * bump_bp))
    }
}

/// Bump one index credit curve by `bp` at a single tenor `t`.
///
/// Re-bootstrap a hazard curve after a tenor-local par-spread shock.
///
/// Curves without par-spread points and failed re-bootstrap attempts are
/// errors. A direct hazard-rate shift has different units and is available
/// separately through `BucketedCs01Hazard`.
fn bump_index_credit_curve_at_tenor(
    context: &MetricContext,
    hazard: &HazardCurve,
    base_ctx: &MarketContext,
    discount_id: &CurveId,
    t: f64,
    bp: f64,
) -> Result<Arc<HazardCurve>> {
    let req = BumpRequest::Tenors(vec![(t, bp)]);
    if hazard.hazard_calibration().is_none() {
        return crate::calibration::bumps::hazard::bump_hazard_shift(hazard, &req).map(Arc::new);
    }
    context
        .bump_hazard_spreads_cached(hazard, base_ctx, &req, Some(discount_id), None, None)
        .map_err(|error| Error::Calibration {
            message: format!(
                "CDS index bucketed CS01 failed to re-bootstrap hazard curve '{}' \
                 at tenor {t}: {error}",
                hazard.id()
            ),
            category: "cs01_rebootstrap".to_string(),
        })
}

/// Key-rate (bucketed) par-spread CS01 calculator for CDS Index.
///
/// The bucketed counterpart of [`Cs01Calculator`]: applies a par-spread shock
/// one standard tenor at a time to the same mode-aware credit-curve set as
/// [`Cs01HazardCalculator`] (`SingleCurve` → the synthetic index curve;
/// `Constituents` → the surviving constituent curves), reprices the index
/// end-to-end, central-differences, and stores the per-tenor series under
/// `bucketed_cs01::{index_credit_curve}`. Because each curve's par point is
/// bumped by exactly the bucket within 0.1y of it, the per-bucket CS01s sum to
/// the parallel `Cs01`.
pub(crate) struct CdsIndexBucketedCs01Calculator;

impl MetricCalculator for CdsIndexBucketedCs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let index: CDSIndex = context.instrument_as::<CDSIndex>()?.clone();

        // Expired → zero, no series (mirrors the parallel aggregation path).
        if context.as_of >= index.premium.end {
            return Ok(0.0);
        }

        let defaults =
            sens_config::from_context_or_default(context.config(), context.get_metric_overrides())?;
        let bump_bp = defaults.credit_spread_bump_bp;
        let buckets = defaults.cs01_buckets_years;

        // Credit-curve set — identical resolution to `Cs01HazardCalculator`.
        let credit_ids: Vec<CurveId> = match index.pricing {
            IndexPricing::SingleCurve => vec![index.protection.credit_curve_id.clone()],
            IndexPricing::Constituents => index
                .market_dependencies()?
                .curves
                .credit_curves
                .into_iter()
                .filter(|id| id != &index.protection.credit_curve_id)
                .collect(),
        };
        if credit_ids.is_empty() {
            return Ok(0.0);
        }
        let discount_id = index.premium.discount_curve_id.clone();

        // Clone the Arc so building bumped contexts holds no borrow of
        // `context` across the reprice / `store_bucketed_series` calls.
        let curves = Arc::clone(&context.curves);
        let base_ctx = curves.as_ref();
        let as_of = context.as_of;

        let build_bumped_ctx = |t: f64, bp: f64| -> Result<MarketContext> {
            let mut out = base_ctx.clone();
            for id in &credit_ids {
                let hazard = base_ctx.get_hazard(id.as_str())?;
                out.insert_mut(bump_index_credit_curve_at_tenor(
                    context,
                    hazard.as_ref(),
                    base_ctx,
                    &discount_id,
                    t,
                    bp,
                )?);
            }
            Ok(out)
        };

        let mut series: Vec<(Cow<'static, str>, f64)> = Vec::new();
        let mut total = 0.0;
        for t in buckets {
            let ctx_up = build_bumped_ctx(t, bump_bp)?;
            let ctx_down = build_bumped_ctx(t, -bump_bp)?;
            let pv_up = context.reprice_raw(&ctx_up, as_of)?;
            let pv_down = context.reprice_raw(&ctx_down, as_of)?;
            let cs01 = sensitivity_central_diff(pv_up, pv_down, bump_bp);
            series.push((sens_config::format_bucket_label_cow(t), cs01));
            total += cs01;
        }

        context.store_bucketed_series(
            MetricId::custom(format!(
                "bucketed_cs01::{}",
                index.protection.credit_curve_id.as_str()
            )),
            series,
        );
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_quant_core::market_data::term_structures::HazardCurve;
    use finstack_quant_core::money::Money;

    #[test]
    fn bucketed_cs01_uses_model_hazard_shift_without_recipe() {
        let index = CDSIndex::example();
        let as_of = index.premium.start;
        let discount_id = index.premium.discount_curve_id.clone();
        let currency = index.notional.currency();
        let context = MetricContext::new(
            Arc::new(index),
            Arc::new(MarketContext::new()),
            as_of,
            Money::new(0.0, currency),
            MetricContext::default_config(),
        );
        let hazard = HazardCurve::builder("DIRECT-HAZARD")
            .base_date(as_of)
            .recovery_rate(0.40)
            .knots([(1.0, 0.02), (5.0, 0.03)])
            .build()
            .expect("hazard curve");

        let bumped = bump_index_credit_curve_at_tenor(
            &context,
            &hazard,
            &MarketContext::new(),
            &discount_id,
            5.0,
            1.0,
        )
        .expect("model hazard shift");
        assert!(bumped.hazard_calibration().is_none());
        assert!(bumped.hazard_rate(5.0) > hazard.hazard_rate(5.0));
    }
}
