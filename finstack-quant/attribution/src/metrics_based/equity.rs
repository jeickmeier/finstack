use super::super::helpers::*;
use super::super::types::*;
use super::context::AttributionInputs;
use super::shifts::{average_over, inflation_source_abs_shift_bp, twist_diagnostic_note};
use finstack_quant_core::market_data::diff::{
    measure_inflation_source_shift, measure_scalar_absolute_shift,
};
use finstack_quant_valuations::metrics::MetricId;

pub(super) fn apply_spot(
    inputs: &AttributionInputs<'_>,
    attribution: &mut PnlAttribution,
    non_finite_detected: &mut bool,
) {
    // 6. Market scalars: spot price Delta/Gamma attribution
    //
    // METRIC DEFINITION (see MetricId::Delta / MetricId::Gamma):
    // - Delta: dPV/dS — currency per UNIT of underlying move
    // - Gamma: d²PV/dS² — currency per (unit underlying)²
    // - Formula: PnL = Delta × ΔS + ½ × Gamma × (ΔS)², with ΔS the ABSOLUTE
    //   spot move. Multiplying by a percentage shift would mis-scale the P&L
    //   by 100/S₀ (resp. (100/S₀)²), exact only when S₀ = 100.
    //
    // Uses market_scalar_ids from MarketDependencies to identify underlying spot prices.
    let market_scalar_ids = &inputs.market_deps.market_scalar_ids;
    let delta_opt = inputs.val_t0.measures.get(MetricId::Delta.as_str());
    let gamma_opt = inputs.val_t0.measures.get(MetricId::Gamma.as_str());

    if let Some(&delta) = delta_opt {
        // Guard against a non-finite Delta silently corrupting attributed
        // P&L. `MetricId::Delta` is contractually dPV/dS (currency per unit
        // underlying move) — see the unit note above.
        debug_assert!(
            delta.is_finite(),
            "Delta metric must be finite for P&L attribution, got {delta}"
        );

        // Note: `Delta` / `Gamma` are sensitivities to the
        // instrument's PRIMARY spot driver, not a per-spot vector. The old
        // code multiplied the single Delta by EVERY spot's move and summed
        // (~N× overstatement for multi-spot instruments) while applying
        // Gamma once to the average. Both orders are now applied once, to
        // the measurable spot with the LARGEST |ΔS| (deterministic
        // tie-break: first declared wins) — binding to the first measurable
        // spot even when its move was 0.0 locked out the real driver.
        // Additional spot moves are unattributed (they flow to the
        // residual) and noted.
        let measured: Vec<(&String, f64)> = market_scalar_ids
            .iter()
            .filter_map(|spot_id| {
                measure_scalar_absolute_shift(spot_id, inputs.market_t0, inputs.market_t1)
                    .ok()
                    .map(|shift| (spot_id, shift))
            })
            .collect();
        let mut primary: Option<(usize, f64)> = None;
        for (idx, &(_, shift)) in measured.iter().enumerate() {
            match primary {
                Some((_, best)) if shift.abs() <= best.abs() => {}
                _ => primary = Some((idx, shift)),
            }
        }
        let primary_shift: Option<f64> = primary.map(|(_, shift)| shift);
        let extra_spots: Vec<&String> = primary
            .map(|(primary_idx, _)| {
                measured
                    .iter()
                    .enumerate()
                    .filter(|&(idx, _)| idx != primary_idx)
                    .map(|(_, &(spot_id, _))| spot_id)
                    .collect()
            })
            .unwrap_or_default();

        if let Some(spot_shift) = primary_shift {
            let mut total_spot_pnl = delta * spot_shift;

            // Second-order: Gamma applied to the same primary spot move.
            if let Some(&gamma) = gamma_opt {
                debug_assert!(
                    gamma.is_finite(),
                    "Gamma metric must be finite for P&L attribution, got {gamma}"
                );
                total_spot_pnl += 0.5 * gamma * spot_shift * spot_shift;
            }

            attribution.market_scalars_pnl = factor_money_or_invalid(
                total_spot_pnl,
                inputs.val_t1.value.currency(),
                "market scalars (delta/gamma) P&L",
                &mut attribution.meta.notes,
                non_finite_detected,
            );
        }
        if !extra_spots.is_empty() {
            attribution.meta.notes.push(format!(
                "Spot Delta/Gamma attribution applied to the primary spot driver only; \
                         moves on additional spot ids ({}) are unattributed and flow to the \
                         residual — provide per-spot sensitivities for multi-underlying books",
                extra_spots
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
}

pub(super) fn apply_dividend(
    inputs: &AttributionInputs<'_>,
    attribution: &mut PnlAttribution,
    non_finite_detected: &mut bool,
) {
    // 7. Dividend attribution (accumulates into market_scalars_pnl alongside spot Delta/Gamma)
    if let Some(dividend01) = inputs.val_t0.measures.get(MetricId::Dividend01.as_str()) {
        if let Some(scalar_id) = inputs.instrument.dividend_schedule_id() {
            // Note: the `Dividend01` producers emit **$ per
            // 1bp** of absolute dividend-yield move (the central difference is
            // rescaled by `DIVIDEND_BUMP_BP`, see equity_option/convertible
            // `dividend_risk.rs`). `measure_scalar_absolute_shift` returns the
            // DECIMAL Δq, so the move must be converted to bp before
            // multiplying — the former per-unit pairing understated dividend
            // P&L by 10,000×.
            if let Ok(div_abs_shift) = measure_scalar_absolute_shift(
                scalar_id.as_str(),
                inputs.market_t0,
                inputs.market_t1,
            ) {
                let div_shift_bp = div_abs_shift * 10_000.0;
                let div_amount = dividend01 * div_shift_bp;
                attribution.market_scalars_pnl = factor_money_or_invalid(
                    attribution.market_scalars_pnl.amount() + div_amount,
                    inputs.val_t1.value.currency(),
                    "dividend P&L",
                    &mut attribution.meta.notes,
                    non_finite_detected,
                );
            }
        }
    }
}

pub(super) fn apply_inflation(
    inputs: &AttributionInputs<'_>,
    attribution: &mut PnlAttribution,
    non_finite_detected: &mut bool,
) {
    // 9. Inflation sensitivity
    if let Some(inflation01) = inputs.val_t0.measures.get(MetricId::Inflation01.as_str()) {
        // Restrict the shift to the instrument's declared inflation sources.
        // This keeps unrelated curves in a shared market context from
        // contaminating the instrument-level attribution.
        let curve_ids = &inputs.market_deps.curves.inflation_curves;

        let mut total_shift = 0.0;
        let mut curve_count = 0;
        let mut measurement_failed = false;

        for curve_id in curve_ids {
            match measure_inflation_source_shift(
                curve_id.as_str(),
                inputs.market_t0,
                inputs.market_t1,
            ) {
                Ok(shift_bp) => {
                    total_shift += shift_bp;
                    curve_count += 1;
                }
                Err(err) => {
                    measurement_failed = true;
                    attribution.meta.notes.push(format!(
                        "Inflation attribution could not measure declared source '{}': {err}",
                        curve_id.as_str()
                    ));
                }
            }
        }

        if measurement_failed || curve_ids.is_empty() {
            *non_finite_detected = true;
            if curve_ids.is_empty() {
                attribution.meta.notes.push(
                    "Inflation01 was supplied but the instrument declared no inflation source"
                        .to_string(),
                );
            }
        }

        let avg_shift = if curve_count > 0 {
            total_shift / curve_count as f64
        } else {
            0.0
        };
        // First-order: Inflation01 × Δi (Δi in basis points)
        let inflation_amount = inflation01 * avg_shift;
        attribution.inflation_curves_pnl = factor_money_or_invalid(
            inflation_amount,
            inputs.val_t1.value.currency(),
            "inflation P&L",
            &mut attribution.meta.notes,
            non_finite_detected,
        );

        // Second-order: Inflation convexity (if available).
        //
        // UNIT CONTRACT: `InflationConvexity` is ∂²V/∂i² in $ per decimal² of
        // inflation rate (the `CsGamma`-style convention, NOT the dimensionless
        // `Convexity` convention). The debug assertion guards against a
        // non-finite metric silently corrupting the attributed P&L (it cannot
        // enforce units — see the unit-contract table at the top of this file).
        if let Some(inflation_convexity) = inputs
            .val_t0
            .measures
            .get(MetricId::InflationConvexity.as_str())
        {
            debug_assert!(
                    inflation_convexity.is_finite(),
                    "InflationConvexity metric must be finite for P&L attribution, got {inflation_convexity}"
                );
            let shift_decimal = avg_shift / 10_000.0;
            let convexity_pnl = 0.5 * inflation_convexity * shift_decimal * shift_decimal;
            attribution.inflation_curves_pnl = factor_money_or_invalid(
                attribution.inflation_curves_pnl.amount() + convexity_pnl,
                inputs.val_t1.value.currency(),
                "inflation convexity P&L",
                &mut attribution.meta.notes,
                non_finite_detected,
            );

            // TWIST GUARD: emit a diagnostic note when the inflation curve is
            // twisted (signed mean shift collapses toward 0 but L1 mean is
            // non-trivial). Same shape as the rates / credit twist guards.
            let abs_avg = average_over(curve_ids, |curve_id| {
                let v = inflation_source_abs_shift_bp(
                    curve_id.as_str(),
                    inputs.market_t0,
                    inputs.market_t1,
                );
                (v > 0.0).then_some(v)
            })
            .0;
            if let Some(abs_avg) = abs_avg {
                if let Some(note) = twist_diagnostic_note("Inflation convexity", avg_shift, abs_avg)
                {
                    attribution.meta.notes.push(note);
                    attribution
                        .meta
                        .notes
                        .push("Inflation convexity: unreliable / bounds-exceeded".to_string());
                }
            }
        }
    }
}
