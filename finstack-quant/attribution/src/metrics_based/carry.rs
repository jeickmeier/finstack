use super::super::helpers::*;
use super::super::types::*;
use super::context::AttributionInputs;
use finstack_quant_core::money::Money;
use finstack_quant_valuations::metrics::MetricId;

pub(super) fn apply(
    inputs: &AttributionInputs<'_>,
    attribution: &mut PnlAttribution,
    non_finite_detected: &mut bool,
) {
    let time_period_days = inputs.time_period_days;

    // 1. Carry attribution (Theta / Carry decomposition)
    //
    // METRIC DEFINITION:
    // - Theta: Dollar P&L per day ($ / day)
    // - Formula: Theta × Δt (where Δt is time period in days)
    // - Carry decomposition metrics, when present, are scaled over the same horizon.
    // Theta / CarryTotal / CouponIncome / PullToPar / RollDown / FundingCost
    // are PERIOD TOTALS over the producer's `theta_period` (default 1D),
    // capped at expiry. When the producer stamped the realized horizon
    // (`theta_period_days`), normalize by it before rescaling to the
    // attribution window; otherwise assume the 1D default (
    // multiplying a 1M carry total by the window's day count double-scales).
    let theta_horizon_days = inputs
        .val_t0
        .measures
        .get(MetricId::ThetaPeriodDays.as_str())
        .copied()
        .filter(|d| d.is_finite() && *d > 0.0);
    let carry_scale = match theta_horizon_days {
        Some(horizon) => time_period_days / horizon,
        None => time_period_days,
    };
    if let Some(horizon) = theta_horizon_days {
        if (horizon - 1.0).abs() > 1e-9 {
            attribution.meta.notes.push(format!(
                "Carry metrics normalized from a {horizon}-day producer horizon \
                     (theta_period override) to the {time_period_days}-day attribution window"
            ));
        }
    } else if time_period_days > 1.0
        && (inputs
            .val_t0
            .measures
            .get(MetricId::CarryTotal.as_str())
            .is_some()
            || inputs
                .val_t0
                .measures
                .get(MetricId::Theta.as_str())
                .is_some())
    {
        // Audit fix: without a ThetaPeriodDays stamp the carry metrics are
        // linearly extrapolated across the window from an assumed 1-day
        // producer horizon — including any discrete coupon component. The
        // operator must be able to distinguish "true period carry" from
        // "1-day carry × N" (the residual silently absorbs the difference).
        attribution.meta.notes.push(format!(
            "Carry metrics scaled linearly ×{time_period_days} from an \
                 assumed 1-day producer horizon (no theta_period_days stamp); \
                 discrete cashflows and carry convexity inside the window are \
                 extrapolated, not observed"
        ));
    }

    if let Some(carry_total) = inputs.val_t0.measures.get(MetricId::CarryTotal.as_str()) {
        attribution.carry = factor_money_or_invalid(
            carry_total * carry_scale,
            inputs.ccy,
            "carry total",
            &mut attribution.meta.notes,
            non_finite_detected,
        );

        let get_scaled = |id: MetricId,
                          notes: &mut Vec<String>,
                          flag: &mut bool|
         -> Option<Money> {
            inputs.val_t0.measures.get(id.as_str()).map(|value| {
                factor_money_or_invalid(value * carry_scale, inputs.ccy, id.as_str(), notes, flag)
            })
        };

        attribution.carry_detail = Some(CarryDetail {
            total: attribution.carry,
            coupon_income: get_scaled(
                MetricId::CouponIncome,
                &mut attribution.meta.notes,
                non_finite_detected,
            )
            .map(SourceLine::scalar),
            pull_to_par: get_scaled(
                MetricId::PullToPar,
                &mut attribution.meta.notes,
                non_finite_detected,
            ),
            roll_down: get_scaled(
                MetricId::RollDown,
                &mut attribution.meta.notes,
                non_finite_detected,
            )
            .map(SourceLine::scalar),
            funding_cost: get_scaled(
                MetricId::FundingCost,
                &mut attribution.meta.notes,
                non_finite_detected,
            ),
        });
    } else if let Some(theta) = inputs.val_t0.measures.get(MetricId::Theta.as_str()) {
        let carry_amount = theta * carry_scale;
        attribution.carry = factor_money_or_invalid(
            carry_amount,
            inputs.ccy,
            "carry/theta",
            &mut attribution.meta.notes,
            non_finite_detected,
        );
        attribution.carry_detail = Some(CarryDetail {
            total: attribution.carry,
            coupon_income: None,
            pull_to_par: None,
            roll_down: Some(SourceLine::scalar(attribution.carry)),
            funding_cost: None,
        });
    } else {
        note_warning(
                attribution,
                "Metrics-based carry attribution skipped: neither CarryTotal nor Theta metric was present; carry P&L set to zero",
                inputs.instrument.id(),
                "carry",
            );
    }
}
