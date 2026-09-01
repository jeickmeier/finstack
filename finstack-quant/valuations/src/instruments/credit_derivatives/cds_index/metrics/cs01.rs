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
//! - [`CdsIndexBucketedCs01Calculator`]: quote-bucketed par-spread CS01 — the
//!   bucketed counterpart of [`Cs01Calculator`]. Applies one exact atomic
//!   `spread_risk_inputs` shock at a time to each mode-aware credit curve,
//!   reprices end-to-end, and stores per-curve series whose sum reconciles to
//!   parallel `Cs01`.
//!
//! Sign convention (per canonical reference):
//! - Long index protection (sell protection) → CS01 negative.
//! - Short index protection (buy protection) → CS01 positive.
//!
//! [canonical]: crate::metrics::sensitivities::cs01
//! [`CDSIndex::cs01`]: crate::instruments::credit_derivatives::cds_index::CDSIndex::cs01

use crate::instruments::credit_derivatives::cds_index::{CDSIndex, IndexPricing};
use crate::metrics::sensitivities::config as sens_config;
use crate::metrics::sensitivities::cs01::{
    compute_key_rate_cs01_series_with_context_raw, cs01_reval, sensitivity_central_diff,
    KeyRateCs01Request,
};
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::math::NeumaierAccumulator;
use finstack_quant_core::types::CurveId;
use finstack_quant_core::Result;
use std::borrow::Cow;
use std::sync::Arc;

/// Parallel CS01 calculator for CDS Index (per-name finite difference).
pub(crate) struct Cs01Calculator;

impl MetricCalculator for Cs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let provider = context.recalibration_provider("cs01")?;
        let idx: &CDSIndex = context.instrument_as()?;
        idx.cs01(&context.curves, context.as_of, provider.as_ref())
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

fn index_credit_curve_ids(index: &CDSIndex) -> Result<Vec<CurveId>> {
    match index.pricing {
        IndexPricing::SingleCurve => Ok(vec![index.protection.credit_curve_id.clone()]),
        IndexPricing::Constituents => {
            let mut curve_ids = Vec::new();
            for constituent in index
                .constituents
                .iter()
                .filter(|constituent| !constituent.defaulted)
            {
                let curve_id = &constituent.credit.credit_curve_id;
                if !curve_ids.contains(curve_id) {
                    curve_ids.push(curve_id.clone());
                }
            }
            Ok(curve_ids)
        }
    }
}

impl MetricCalculator for Cs01HazardCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let index: &CDSIndex = context.instrument_as()?;

        let bump_bp = sens_config::from_context_or_default(
            context.get_config(),
            context.get_metric_overrides(),
        )?
        .credit_spread_bump_bp;

        let credit_ids = index_credit_curve_ids(index)?;

        if credit_ids.is_empty() {
            return Ok(0.0);
        }

        let bump_all = |ctx: &MarketContext, bp: f64| -> Result<MarketContext> {
            let mut out = ctx.clone();
            for id in &credit_ids {
                let hazard = ctx.get_hazard(id.as_str())?;
                let bumped = hazard.with_parallel_hazard_rate_bump_bp(bp)?;
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

/// Key-rate direct hazard-shift CS01 for CDS Index.
///
/// Uses the same mode-aware credit-curve resolution and end-to-end repricing as
/// [`Cs01HazardCalculator`]. In constituent mode each curve gets its own
/// `bucketed_cs01_hazard::{curve}` series.
pub(crate) struct CdsIndexBucketedCs01HazardCalculator;

impl MetricCalculator for CdsIndexBucketedCs01HazardCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let index: CDSIndex = context.instrument_as::<CDSIndex>()?.clone();
        if context.as_of >= index.premium.end {
            return Ok(0.0);
        }

        let defaults = sens_config::from_context_or_default(
            context.get_config(),
            context.get_metric_overrides(),
        )?;
        let bump_bp = defaults.credit_spread_bump_bp;
        let credit_ids = index_credit_curve_ids(&index)?;
        if credit_ids.is_empty() {
            return Ok(0.0);
        }

        let curves = Arc::clone(&context.curves);
        let base_ctx = curves.as_ref();
        let as_of = context.as_of;
        let mut total = NeumaierAccumulator::new();

        for curve_id in credit_ids {
            let hazard = base_ctx.get_hazard(curve_id.as_str())?;
            let node_times =
                crate::metrics::sensitivities::cs01::effective_hazard_node_times(hazard.as_ref());
            let single_node = node_times.len() == 1;
            let mut series: Vec<(Cow<'static, str>, f64)> = Vec::with_capacity(node_times.len());
            for tenor in node_times {
                let bumped_up = if single_node {
                    hazard.with_parallel_hazard_rate_bump_bp(bump_bp)?
                } else {
                    hazard.with_tenor_hazard_rate_bumps_bp(&[(tenor, bump_bp)])?
                };
                let bumped_down = if single_node {
                    hazard.with_parallel_hazard_rate_bump_bp(-bump_bp)?
                } else {
                    hazard.with_tenor_hazard_rate_bumps_bp(&[(tenor, -bump_bp)])?
                };
                let (pv_up, pv_down) = context.with_market_scratch(|ctx, scratch| {
                    scratch.insert_mut(bumped_up);
                    let pv_up = ctx.reprice_raw(scratch, as_of)?;
                    scratch.insert_mut(Arc::clone(&hazard));

                    scratch.insert_mut(bumped_down);
                    let pv_down = ctx.reprice_raw(scratch, as_of)?;
                    scratch.insert_mut(Arc::clone(&hazard));

                    Ok((pv_up, pv_down))
                })?;
                let cs01 = sensitivity_central_diff(pv_up, pv_down, bump_bp);
                series.push((
                    crate::metrics::sensitivities::cs01::format_hazard_node_label(tenor),
                    cs01,
                ));
                total.add(cs01);
            }
            context.store_bucketed_series(
                MetricId::custom(format!("bucketed_cs01_hazard::{}", curve_id.as_str())),
                series,
            );
        }

        Ok(total.total())
    }
}

/// Quote-bucketed par-spread CS01 calculator for CDS Index.
///
/// Uses the shared exact replay-binding decomposition for each relevant curve:
/// `SingleCurve` processes the synthetic index curve, while `Constituents`
/// processes each distinct surviving constituent curve. Every series is stored
/// under `bucketed_cs01::{curve_id}` with collision-safe quote labels.
pub(crate) struct CdsIndexBucketedCs01Calculator;

impl MetricCalculator for CdsIndexBucketedCs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let index: CDSIndex = context.instrument_as::<CDSIndex>()?.clone();

        // Expired → zero, no series (mirrors the parallel aggregation path).
        if context.as_of >= index.premium.end {
            return Ok(0.0);
        }

        let defaults = sens_config::from_context_or_default(
            context.get_config(),
            context.get_metric_overrides(),
        )?;
        let bump_bp = defaults.credit_spread_bump_bp;

        let credit_ids = index_credit_curve_ids(&index)?;
        if credit_ids.is_empty() {
            return Ok(0.0);
        }
        let discount_id = index.premium.discount_curve_id;
        let mut total = NeumaierAccumulator::new();
        for credit_id in credit_ids {
            let series_id = MetricId::custom(format!("bucketed_cs01::{}", credit_id.as_str()));
            let reval = cs01_reval(context);
            total.add(compute_key_rate_cs01_series_with_context_raw(
                context,
                &credit_id,
                KeyRateCs01Request {
                    series_id,
                    bump_bp,
                    discount_curve_id: discount_id.clone(),
                    doc_clause: None,
                    cds_valuation_convention: None,
                    deal_quote_override: None,
                },
                reval,
            )?);
        }
        Ok(total.total())
    }
}
