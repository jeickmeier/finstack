//! Node-coupon descriptor construction for stochastic-rate floating resets.
//!
//! The rates-credit lattice values a future floating coupon's node
//! dependence as an **increment** over its deterministic projection (see
//! [`NodeCoupon`]). This module scans an instrument's canonical
//! [`CashFlowSchedule`] and turns every future-reset floating coupon into a
//! descriptor, using only metadata the emission already stamps on each
//! flow: the reset (observation) date, the accrual period, and the
//! projected pre-composition index rate. No second coupon specification is
//! introduced.
//!
//! Shared by the callable-bond and callable-term-loan tree engines; each
//! passes its own slice-snapping convention so a coupon's reset/payment
//! slices land on the same steps that engine snaps exercise dates to.
//!
//! # What stays deterministic
//!
//! - Coupons whose observation date is on or before the valuation origin
//!   (known fixings, including the reset-lag sliver where the fixing has
//!   published but accrual has not started).
//! - Coupons the emission itself projected deterministically via a
//!   fallback policy (`SpreadOnly` / `FixedRate`), identified by a missing
//!   `projected_index_rate`.
//! - Every fixed-rate, fee, and principal flow.

use crate::cashflow::builder::rate_helpers::FloatingRateParams;
use crate::cashflow::builder::schedule::CashFlowSchedule;
use crate::cashflow::builder::specs::{FloatingRateSpec, OvernightIndexConstraintApplication};
use crate::cashflow::primitives::CFKind;
use crate::models::trees::two_factor_rates_credit::NodeCoupon;
use finstack_quant_core::dates::{Date, DayCount, DayCountContext};
use finstack_quant_core::market_data::traits::Discounting;
use finstack_quant_core::{Error, Result};
use rust_decimal::prelude::ToPrimitive;

/// Convert a [`FloatingRateSpec`] into the f64 composition parameters the
/// increment shares with the emission (same mapping the term-loan
/// discounting pricer applies for realized fixings).
pub(crate) fn params_from_spec(spec: &FloatingRateSpec) -> FloatingRateParams {
    FloatingRateParams {
        spread_bp: spec.spread_bp.to_f64().unwrap_or_default(),
        gearing: spec.gearing.to_f64().unwrap_or(1.0),
        gearing_includes_spread: spec.gearing_includes_spread,
        index_floor_bp: spec.index_floor_bp.and_then(|value| value.to_f64()),
        index_cap_bp: spec.index_cap_bp.and_then(|value| value.to_f64()),
        all_in_floor_bp: spec.all_in_floor_bp.and_then(|value| value.to_f64()),
        all_in_cap_bp: spec.all_in_cap_bp.and_then(|value| value.to_f64()),
    }
}

/// Whether period-level composition must drop index-level floor/cap
/// because the leg applies them daily inside an overnight-compounded
/// window (mirrors the emission's `period_rate_params_for_overnight`).
pub(crate) fn strips_index_constraints(spec: &FloatingRateSpec) -> bool {
    spec.overnight_compounding.is_some()
        && spec.overnight_index_constraints == OvernightIndexConstraintApplication::Daily
}

/// How an engine maps an event time onto its uniform tree grid.
///
/// Descriptors must use the same convention the engine uses for exercise
/// dates, so a call on a reset or payment date lands on the same step and
/// is recognised as boundary-aligned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SliceSnap {
    /// Nearest grid step (bond convention).
    Nearest,
    /// First grid step at or after the event time (term-loan convention).
    Ceil,
}

impl SliceSnap {
    fn apply(self, time_steps: &[f64], t: f64) -> usize {
        let n = time_steps.len() - 1;
        if t <= time_steps[0] {
            return 0;
        }
        if t >= time_steps[n] {
            return n;
        }
        match self {
            Self::Nearest => {
                let upper = time_steps.partition_point(|&g| g <= t);
                let lower = upper - 1;
                if (t - time_steps[lower]) <= (time_steps[upper] - t) {
                    lower
                } else {
                    upper
                }
            }
            Self::Ceil => time_steps.partition_point(|&g| g < t),
        }
    }
}

/// Inputs for [`build_node_coupons`].
pub(crate) struct NodeCouponBuildInputs<'a> {
    /// Canonical emitted schedule (already projected off today's curves).
    pub schedule: &'a CashFlowSchedule,
    /// Instrument-level floating-rate composition parameters.
    pub params: FloatingRateParams,
    /// Valuation origin — the tree's `t = 0`.
    pub grid_origin: Date,
    /// Uniform tree time grid (`time_steps[0] == 0.0`).
    pub time_steps: &'a [f64],
    /// Day count of the tree's time axis (the discount curve's).
    pub day_count: DayCount,
    /// The tree's discount curve (slice forwards and timing corrections).
    pub discount: &'a dyn Discounting,
    /// The engine's exercise-date snapping convention.
    pub snap: SliceSnap,
    /// Whether to drop index-level floor/cap from the composition because
    /// the leg applies them **daily** inside an overnight-compounded
    /// observation window (`OvernightIndexConstraintApplication::Daily`);
    /// the emission strips them from period-level composition in that
    /// case, and the increment must compose the same way.
    pub strip_index_constraints: bool,
}

/// Scan the schedule for future-reset floating coupons and build their
/// node-coupon descriptors.
///
/// `notional_at_step` supplies the outstanding principal the coupon
/// accrues on, indexed by the snapped reset step — the same step-indexed
/// outstanding the engine uses for recovery and redemption.
///
/// # Errors
///
/// Returns [`Error::Validation`] when a future floating coupon's reset and
/// payment dates snap to the same tree slice (grid too coarse) or a
/// year-fraction computation fails.
pub(crate) fn build_node_coupons(
    inputs: &NodeCouponBuildInputs<'_>,
    notional_at_step: impl Fn(usize) -> f64,
) -> Result<Vec<NodeCoupon>> {
    let steps = inputs.time_steps.len() - 1;
    let mut coupons = Vec::new();

    let mut params = inputs.params.clone();
    if inputs.strip_index_constraints {
        params.index_floor_bp = None;
        params.index_cap_bp = None;
    }

    for cf in inputs.schedule.get_flows() {
        if cf.kind != CFKind::FloatReset {
            continue;
        }
        // Future observation only: a published fixing (reset on or before
        // the origin) is deterministic and must not move with rate vol.
        let future_reset = match cf.reset_date {
            Some(reset) => reset > inputs.grid_origin,
            None => false,
        };
        if !future_reset {
            continue;
        }
        let Some(accrual) = cf.accrual else {
            continue;
        };
        // Fallback-projected coupons (SpreadOnly / FixedRate) carry no
        // projected index rate; the emission treated them as deterministic
        // and the lattice must agree.
        let Some(base_index_rate) = accrual.projected_index_rate else {
            continue;
        };
        let tau = cf.accrual_factor;
        if tau <= 0.0 || !tau.is_finite() {
            continue;
        }

        let ctx = DayCountContext::default();
        let t_start = inputs
            .day_count
            .year_fraction(inputs.grid_origin, accrual.start, ctx)?;
        let t_pay = inputs
            .day_count
            .year_fraction(inputs.grid_origin, cf.date, ctx)?;
        let reset_step = inputs.snap.apply(inputs.time_steps, t_start.max(0.0));
        let payment_step = inputs.snap.apply(inputs.time_steps, t_pay);
        if reset_step >= payment_step {
            return Err(Error::Validation(format!(
                "floating coupon accruing {} to {} (paid {}) snaps to a single \
                 tree slice (step {reset_step}) under stochastic rates; the tree \
                 grid is too coarse to distinguish its reset from its payment. \
                 Increase tree_steps (currently {steps}).",
                accrual.start, accrual.end, cf.date
            )));
        }

        let t_slice_reset = inputs.time_steps[reset_step];
        let t_slice_pay = inputs.time_steps[payment_step];
        let df_reset = inputs.discount.df(t_slice_reset);
        let df_pay = inputs.discount.df(t_slice_pay);
        if df_pay <= f64::EPSILON || df_reset <= f64::EPSILON {
            return Err(Error::Validation(format!(
                "degenerate discount factors over floating period slices \
                 [{t_slice_reset}, {t_slice_pay}]"
            )));
        }
        let base_discount_forward = (df_reset / df_pay - 1.0) / tau;
        // Same DF timing correction the deterministic booking applies via
        // `value_at_step_time`: value at the true payment date, expressed
        // at the snapped payment slice.
        let timing_scale = inputs.discount.df(t_pay) / df_pay;

        coupons.push(NodeCoupon {
            reset_step,
            payment_step,
            accrual: tau,
            notional: notional_at_step(reset_step),
            base_index_rate,
            base_discount_forward,
            timing_scale,
            params: params.clone(),
        });
    }

    Ok(coupons)
}

/// Reject exercise steps strictly inside a node-dependent coupon period.
///
/// Between a future reset and its payment the coupon amount is known only
/// at the reset node, so an exercise decision there would need
/// path-dependent state the recombining lattice does not carry. Exercise
/// **at** the reset slice (coupon forfeited by redemption) and **at** the
/// payment slice (coupon paid regardless) both remain exact.
///
/// # Errors
///
/// Returns [`Error::Validation`] naming the offending step and period.
pub(crate) fn validate_exercise_alignment(
    coupons: &[NodeCoupon],
    exercise_steps: impl Iterator<Item = usize>,
    instrument_label: &str,
) -> Result<()> {
    for step in exercise_steps {
        for coupon in coupons {
            if step > coupon.reset_step && step < coupon.payment_step {
                return Err(Error::Validation(format!(
                    "{instrument_label}: exercise step {step} falls strictly inside \
                     the floating coupon period spanning tree slices \
                     [{}, {}]. Under stochastic rates the coupon amount there \
                     depends on the rate node at the reset slice, which the \
                     recombining lattice cannot carry through a mid-period \
                     exercise. Align call/put dates with the coupon's reset or \
                     payment dates, or price with deterministic rates \
                     (hw1f_sigma unset).",
                    coupon.reset_step, coupon.payment_step
                )));
            }
        }
    }
    Ok(())
}

/// Whether the schedule carries any capitalizing (PIK) flow after the
/// valuation origin.
///
/// A future floating PIK coupon capitalizes a node-dependent amount into
/// principal, making outstanding balance, recovery, and redemption
/// path-dependent. The engines reject that combination in stochastic-rate
/// mode rather than misstate principal.
pub(crate) fn has_future_pik(schedule: &CashFlowSchedule, grid_origin: Date) -> bool {
    schedule
        .get_flows()
        .iter()
        .any(|cf| cf.kind == CFKind::Pik && cf.date > grid_origin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slice_snap_conventions() {
        let grid = [0.0, 0.5, 1.0, 1.5, 2.0];
        assert_eq!(SliceSnap::Nearest.apply(&grid, 0.70), 1);
        assert_eq!(SliceSnap::Nearest.apply(&grid, 0.80), 2);
        assert_eq!(SliceSnap::Ceil.apply(&grid, 0.70), 2);
        assert_eq!(SliceSnap::Ceil.apply(&grid, 0.50), 1);
        assert_eq!(SliceSnap::Nearest.apply(&grid, -0.1), 0);
        assert_eq!(SliceSnap::Ceil.apply(&grid, 9.0), 4);
    }

    #[test]
    fn alignment_rejects_interior_exercise_only() {
        let coupon = NodeCoupon {
            reset_step: 4,
            payment_step: 8,
            accrual: 0.5,
            notional: 100.0,
            base_index_rate: 0.03,
            base_discount_forward: 0.03,
            timing_scale: 1.0,
            params: FloatingRateParams::default(),
        };
        let coupons = [coupon];
        // Boundary steps are fine.
        validate_exercise_alignment(&coupons, [4usize, 8].into_iter(), "TEST")
            .expect("boundary exercise is aligned");
        // Steps outside the period are fine.
        validate_exercise_alignment(&coupons, [2usize, 9].into_iter(), "TEST")
            .expect("outside exercise is aligned");
        // Interior step is rejected with the period named.
        let err = validate_exercise_alignment(&coupons, [6usize].into_iter(), "TEST")
            .expect_err("interior exercise must be rejected");
        assert!(err.to_string().contains("[4, 8]"), "unexpected: {err}");
    }
}
