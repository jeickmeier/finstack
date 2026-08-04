//! Operation dispatch, effect processing, and market-bump batching.

use super::instrument_shocks::{apply_correlation_effect, apply_instrument_shock, CorrelationKind};
use super::{ExecutionContext, ScenarioChangeManifest, ScenarioMarketTarget};
use crate::adapters;
use crate::adapters::traits::ScenarioEffect;
use crate::error::Result;
use crate::spec::OperationSpec;
use crate::warning::Warning;
use finstack_quant_core::market_data::bumps::MarketBump;
use finstack_quant_core::types::CurveId;

/// Dispatch a single operation to the appropriate adapter and produce its effects.
///
/// Centralised match — the engine relies on Rust's exhaustiveness checker to
/// catch any newly added [`OperationSpec`] variant at compile time. Hierarchy-
/// targeted variants and `TimeRollForward` are handled separately and are
/// unreachable here (hierarchy variants are expanded upstream and time-roll is
/// processed in Phase 0 before this function is invoked).
fn generate_effects(op: &OperationSpec, ctx: &ExecutionContext) -> Result<Vec<ScenarioEffect>> {
    match op {
        OperationSpec::MarketFxPct { base, quote, pct } => {
            adapters::fx::fx_pct_effects(*base, *quote, *pct, ctx)
        }
        OperationSpec::EquityPricePct { ids, pct } => {
            adapters::equity::equity_pct_effects(ids, *pct, ctx)
        }
        OperationSpec::CurveParallelBp {
            curve_kind,
            curve_id,
            discount_curve_id,
            bp,
        } => adapters::curves::curve_parallel_effects(
            *curve_kind,
            curve_id,
            discount_curve_id.as_ref(),
            *bp,
            ctx,
        ),
        OperationSpec::CurveNodeBp {
            curve_kind,
            curve_id,
            discount_curve_id,
            nodes,
            match_mode,
        } => adapters::curves::curve_node_effects(
            *curve_kind,
            curve_id,
            discount_curve_id.as_ref(),
            nodes,
            *match_mode,
            ctx,
        ),
        OperationSpec::VolIndexParallelPts { curve_id, points } => {
            adapters::curves::vol_index_parallel_effects(curve_id, *points, ctx)
        }
        OperationSpec::VolIndexNodePts {
            curve_id,
            nodes,
            match_mode,
        } => adapters::curves::vol_index_node_effects(curve_id, nodes, *match_mode, ctx),
        OperationSpec::BaseCorrParallelPts { surface_id, points } => Ok(
            adapters::basecorr::base_corr_parallel_effects(surface_id, *points, ctx),
        ),
        OperationSpec::BaseCorrBucketPts {
            surface_id,
            detachment_bp,
            points,
        } => Ok(adapters::basecorr::base_corr_bucket_effects(
            surface_id,
            detachment_bp.as_deref(),
            *points,
            ctx,
        )),
        OperationSpec::VolSurfaceParallelPct {
            vol_surface_id,
            pct,
            ..
        } => adapters::vol::vol_parallel_effects(vol_surface_id, *pct, ctx),
        OperationSpec::VolSurfaceBucketPct {
            vol_surface_id,
            tenors,
            strikes,
            pct,
            ..
        } => adapters::vol::vol_bucket_effects(
            vol_surface_id,
            tenors.as_deref(),
            strikes.as_deref(),
            *pct,
            ctx,
        ),
        OperationSpec::StmtForecastPercent { node_id, pct } => Ok(
            adapters::statements::stmt_forecast_percent_effects(node_id, *pct),
        ),
        OperationSpec::StmtForecastAssign { node_id, value } => Ok(
            adapters::statements::stmt_forecast_assign_effects(node_id, *value),
        ),
        OperationSpec::RateBinding { binding } => {
            Ok(adapters::statements::rate_binding_effects(binding))
        }
        OperationSpec::InstrumentPricePctByType {
            instrument_types,
            pct,
        } => Ok(adapters::instruments::instrument_price_by_type_effects(
            instrument_types,
            *pct,
        )),
        OperationSpec::InstrumentPricePctByAttr { attrs, pct } => Ok(
            adapters::instruments::instrument_price_by_attr_effects(attrs, *pct),
        ),
        OperationSpec::InstrumentSpreadBpByType {
            instrument_types,
            bp,
        } => Ok(adapters::instruments::instrument_spread_by_type_effects(
            instrument_types,
            *bp,
        )),
        OperationSpec::InstrumentSpreadBpByAttr { attrs, bp } => Ok(
            adapters::instruments::instrument_spread_by_attr_effects(attrs, *bp),
        ),
        OperationSpec::AssetCorrelationPts { delta_pts } => {
            Ok(adapters::asset_corr::asset_corr_effects(*delta_pts))
        }
        OperationSpec::PrepayDefaultCorrelationPts { delta_pts } => Ok(
            adapters::asset_corr::prepay_default_corr_effects(*delta_pts),
        ),
        OperationSpec::TimeRollForward { .. }
        | OperationSpec::HierarchyCurveParallelBp { .. }
        | OperationSpec::HierarchyVolSurfaceParallelPct { .. }
        | OperationSpec::HierarchyEquityPricePct { .. }
        | OperationSpec::HierarchyBaseCorrParallelPts { .. } => {
            // These variants should never reach the centralized dispatch:
            // `TimeRollForward` is processed in Phase 0 and `Hierarchy*` ops
            // are expanded upstream by `expand_hierarchy_operations`. Returning
            // a typed error rather than panicking preserves the
            // `#![deny(clippy::panic)]` discipline and lets the caller surface
            // the bug through the normal error path instead of crashing the
            // process.
            Err(crate::error::Error::Internal(format!(
                "scenario engine reached centralized dispatch for an op that should have been \
                 handled upstream (Phase 0 or hierarchy expansion); this indicates a bug in the \
                 dispatch pipeline. Operation: {op:?}"
            )))
        }
    }
}

fn market_target_for_id(op: &OperationSpec, id: &CurveId) -> Option<ScenarioMarketTarget> {
    match op {
        OperationSpec::EquityPricePct { .. } => Some(ScenarioMarketTarget::EquityPrice {
            price_id: id.clone(),
        }),
        OperationSpec::CurveParallelBp { curve_kind, .. }
        | OperationSpec::CurveNodeBp { curve_kind, .. } => Some(ScenarioMarketTarget::Curve {
            curve_kind: *curve_kind,
            curve_id: id.clone(),
        }),
        OperationSpec::VolIndexParallelPts { .. } | OperationSpec::VolIndexNodePts { .. } => {
            Some(ScenarioMarketTarget::VolatilityIndex {
                curve_id: id.clone(),
            })
        }
        OperationSpec::BaseCorrParallelPts { .. } | OperationSpec::BaseCorrBucketPts { .. } => {
            Some(ScenarioMarketTarget::BaseCorrelation {
                surface_id: id.clone(),
            })
        }
        OperationSpec::VolSurfaceParallelPct { .. } | OperationSpec::VolSurfaceBucketPct { .. } => {
            Some(ScenarioMarketTarget::VolSurface {
                vol_surface_id: id.clone(),
            })
        }
        OperationSpec::MarketFxPct { .. }
        | OperationSpec::StmtForecastPercent { .. }
        | OperationSpec::StmtForecastAssign { .. }
        | OperationSpec::RateBinding { .. }
        | OperationSpec::InstrumentPricePctByType { .. }
        | OperationSpec::InstrumentPricePctByAttr { .. }
        | OperationSpec::InstrumentSpreadBpByType { .. }
        | OperationSpec::InstrumentSpreadBpByAttr { .. }
        | OperationSpec::AssetCorrelationPts { .. }
        | OperationSpec::PrepayDefaultCorrelationPts { .. }
        | OperationSpec::HierarchyCurveParallelBp { .. }
        | OperationSpec::HierarchyVolSurfaceParallelPct { .. }
        | OperationSpec::HierarchyEquityPricePct { .. }
        | OperationSpec::HierarchyBaseCorrParallelPts { .. }
        | OperationSpec::TimeRollForward { .. } => None,
    }
}

fn market_target_for_bump(op: &OperationSpec, bump: &MarketBump) -> Option<ScenarioMarketTarget> {
    match (op, bump) {
        (OperationSpec::MarketFxPct { .. }, MarketBump::FxPct { base, quote, .. }) => {
            Some(ScenarioMarketTarget::Fx {
                base: *base,
                quote: *quote,
            })
        }
        (_, MarketBump::Curve { id, .. }) => market_target_for_id(op, id),
        (_, MarketBump::VolBucketPct { vol_surface_id, .. }) => {
            market_target_for_id(op, vol_surface_id)
        }
        (_, MarketBump::BaseCorrBucketPts { surface_id, .. }) => {
            market_target_for_id(op, surface_id)
        }
        _ => None,
    }
}

fn market_target_for_curve_update(
    op: &OperationSpec,
    storage: &finstack_quant_core::market_data::context::CurveStorage,
) -> Option<ScenarioMarketTarget> {
    market_target_for_id(op, storage.id())
}

/// Process a single op's effects, threading them through `pending_bumps`,
/// `deferred_stmts`, and the running counters. Extracted from `apply` to keep
/// the main pipeline readable; the dispatch is otherwise identical to the
/// inline match.
pub(super) fn process_effects(
    op: &OperationSpec,
    ctx: &mut ExecutionContext,
    pending_bumps: &mut Vec<MarketBump>,
    deferred_stmts: &mut Vec<ScenarioEffect>,
    warnings: &mut Vec<Warning>,
    applied: &mut usize,
    changes: &mut ScenarioChangeManifest,
) -> Result<()> {
    let effects = generate_effects(op, ctx)?;
    for effect in effects {
        match effect {
            ScenarioEffect::MarketBump(b) => {
                // Within a single op's effects, two bumps targeting the same
                // curve/surface/FX pair must compose sequentially rather than
                // collapse into one batch entry; flush before queueing if so.
                if would_conflict_with_pending(pending_bumps, &b) {
                    flush_pending_bumps(pending_bumps, ctx.market)?;
                }
                match market_target_for_bump(op, &b) {
                    Some(target) => changes.record_market_target(target),
                    None => changes.all_dirty = true,
                }
                pending_bumps.push(b);
                *applied += 1;
            }
            ScenarioEffect::Warning(w) => warnings.push(w),
            ScenarioEffect::UpdateCurve(storage) => {
                // Flush any pending bumps so the curve replacement observes
                // the bumped market state in the same order as the original
                // per-effect application.
                flush_pending_bumps(pending_bumps, ctx.market)?;
                match market_target_for_curve_update(op, &storage) {
                    Some(target) => changes.record_market_target(target),
                    None => changes.all_dirty = true,
                }
                *ctx.market = std::mem::take(ctx.market).insert(storage);
                *applied += 1;
            }
            ScenarioEffect::InstrumentPriceShock { types, attrs, pct } => {
                flush_pending_bumps(pending_bumps, ctx.market)?;
                let outcome = apply_instrument_shock(
                    types.as_deref(),
                    attrs.as_ref(),
                    pct,
                    "price",
                    &mut ctx.instruments,
                    adapters::instruments::apply_instrument_type_price_shock,
                    adapters::instruments::apply_instrument_attr_price_shock,
                );
                *applied += outcome.count;
                changes.record_instrument_indices(outcome.changed_indices);
                warnings.extend(outcome.warnings);
            }
            ScenarioEffect::InstrumentSpreadShock { types, attrs, bp } => {
                flush_pending_bumps(pending_bumps, ctx.market)?;
                let outcome = apply_instrument_shock(
                    types.as_deref(),
                    attrs.as_ref(),
                    bp,
                    "spread",
                    &mut ctx.instruments,
                    adapters::instruments::apply_instrument_type_spread_shock,
                    adapters::instruments::apply_instrument_attr_spread_shock,
                );
                *applied += outcome.count;
                changes.record_instrument_indices(outcome.changed_indices);
                warnings.extend(outcome.warnings);
            }
            ScenarioEffect::AssetCorrelationShock { delta_pts } => {
                flush_pending_bumps(pending_bumps, ctx.market)?;
                let (count, indices, ws) =
                    apply_correlation_effect(CorrelationKind::Asset, delta_pts, ctx);
                *applied += count;
                changes.record_instrument_indices(indices);
                warnings.extend(ws);
            }
            ScenarioEffect::PrepayDefaultCorrelationShock { delta_pts } => {
                flush_pending_bumps(pending_bumps, ctx.market)?;
                let (count, indices, ws) =
                    apply_correlation_effect(CorrelationKind::PrepayDefault, delta_pts, ctx);
                *applied += count;
                changes.record_instrument_indices(indices);
                warnings.extend(ws);
            }
            stmt @ (ScenarioEffect::StmtForecastPercent { .. }
            | ScenarioEffect::StmtForecastAssign { .. }
            | ScenarioEffect::RateBinding { .. }) => {
                deferred_stmts.push(stmt);
            }
        }
    }
    Ok(())
}

/// Flush any accumulated [`MarketBump`]s through `MarketContext::bump` in a
/// single batched call. No-op when the buffer is empty.
pub(super) fn flush_pending_bumps(
    pending: &mut Vec<MarketBump>,
    market: &mut finstack_quant_core::market_data::context::MarketContext,
) -> Result<()> {
    if pending.is_empty() {
        return Ok(());
    }
    let drained: Vec<MarketBump> = std::mem::take(pending);
    *market = market.bump(drained)?;
    Ok(())
}

/// Returns `true` when applying `incoming` would collide with a pending bump.
///
/// `MarketContext::bump_observed` keys [`MarketBump::Curve`] effects in a
/// `HashMap<CurveId, BumpSpec>`, so two bumps targeting the same curve in a
/// single batch would overwrite each other instead of composing
/// `pre * (1+a) * (1+b)`. To preserve the established sequential semantics,
/// we flush the pending batch whenever a new bump would land on the same
/// target as an already-queued one.
fn would_conflict_with_pending(pending: &[MarketBump], incoming: &MarketBump) -> bool {
    pending.iter().any(|p| match (p, incoming) {
        (
            MarketBump::FxPct {
                base: ba,
                quote: qa,
                ..
            },
            MarketBump::FxPct {
                base: bb,
                quote: qb,
                ..
            },
        ) => ba == bb && qa == qb,
        (MarketBump::Curve { id: a, .. }, MarketBump::Curve { id: b, .. })
        | (
            MarketBump::VolBucketPct {
                vol_surface_id: a,
                ..
            },
            MarketBump::VolBucketPct {
                vol_surface_id: b,
                ..
            },
        )
        | (
            MarketBump::BaseCorrBucketPts { surface_id: a, .. },
            MarketBump::BaseCorrBucketPts { surface_id: b, .. },
        ) => a == b,
        // A `Curve` bump on the same id as a `VolBucketPct` is also a logical
        // conflict (both target a vol surface) — flush to be safe.
        (
            MarketBump::Curve { id: a, .. },
            MarketBump::VolBucketPct {
                vol_surface_id: b,
                ..
            },
        )
        | (
            MarketBump::VolBucketPct {
                vol_surface_id: a,
                ..
            },
            MarketBump::Curve { id: b, .. },
        )
        | (MarketBump::Curve { id: a, .. }, MarketBump::BaseCorrBucketPts { surface_id: b, .. })
        | (MarketBump::BaseCorrBucketPts { surface_id: a, .. }, MarketBump::Curve { id: b, .. }) => {
            a == b
        }
        _ => false,
    })
}
