use super::annuity::periods_per_year;
use super::types::YieldCompounding;
use super::ExitCandidate;
use crate::cashflow::accrual::AccrualIndex;
use crate::cashflow::builder::CashFlowSchedule;
use crate::cashflow::primitives::CFKind;
use crate::instruments::fixed_income::bond::pricing::ytm_solver::{solve_ytm, YtmPricingSpec};
use crate::instruments::fixed_income::bond::Bond;
use finstack_quant_core::dates::{Date, DayCountContext};
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::math::summation::NeumaierAccumulator;
use finstack_quant_core::money::Money;
use rust_decimal::prelude::ToPrimitive;

/// Discount factor from yield.
///
/// Computes the discount factor for a given yield, time, and compounding convention.
///
/// # Arguments
///
/// * `ytm` - Yield to maturity as decimal (e.g., 0.05 for 5%)
/// * `t` - Time in years from valuation date to cashflow date
/// * `comp` - Compounding convention (see [`YieldCompounding`])
/// * `bond_frequency` - Bond's coupon frequency (used for `Street` and `TreasuryActual`)
///
/// # Compounding Formulas
///
/// | Convention | Formula |
/// |------------|---------|
/// | Simple | `1 / (1 + y * t)` |
/// | Annual | `(1 + y)^(-t)` |
/// | Periodic(m) | `(1 + y/m)^(-m*t)` |
/// | Continuous | `exp(-y*t)` |
/// | Street | `(1 + y/f)^(-f*t)` where f = frequency |
/// | TreasuryActual | Simple for t < 1/f, then periodic |
///
/// # Errors
///
/// Returns `Err` if the bond frequency is invalid (zero periods).
///
/// # Negative Yields
///
/// Negative yields are supported for all compounding conventions. However:
/// - **Extreme negative yields** (< -50%) will log a warning as they often indicate
///   data or input errors.
/// - For periodic/annual compounding, yields more negative than `-m` (where `m` is
///   compounding frequency) would make `(1 + y/m)` negative, leading to `NaN` from
///   `powf`. Such cases return `Err`.
/// - Discount factors > 1.0 are mathematically valid for negative rates but unusual
///   in practice.
#[inline]
pub fn df_from_yield(
    ytm: f64,
    t: f64,
    comp: YieldCompounding,
    bond_frequency: finstack_quant_core::dates::Tenor,
) -> finstack_quant_core::Result<f64> {
    if t <= 0.0 {
        return Ok(1.0);
    }

    // Warn on extreme negative yields which often indicate data errors
    if ytm < -0.5 {
        tracing::warn!(
            ytm = ytm,
            "Extreme negative yield detected (< -50%). This may indicate a data error."
        );
    }

    Ok(match comp {
        YieldCompounding::Simple => {
            let denom = 1.0 + ytm * t;
            // Check for non-positive denominator which would give invalid discount factor
            if denom <= 0.0 {
                return Err(finstack_quant_core::Error::Validation(format!(
                    "Simple interest denominator (1 + y*t) = {} is non-positive for ytm={}, t={}",
                    denom, ytm, t
                )));
            }
            1.0 / denom
        }
        YieldCompounding::Annual => {
            let base = 1.0 + ytm;
            if base <= 0.0 {
                return Err(finstack_quant_core::Error::Validation(format!(
                    "Annual compounding base (1 + y) = {} is non-positive for ytm={}",
                    base, ytm
                )));
            }
            base.powf(-t)
        }
        YieldCompounding::Periodic(m) => {
            let m = m as f64;
            let base = 1.0 + ytm / m;
            if base <= 0.0 {
                return Err(finstack_quant_core::Error::Validation(format!(
                    "Periodic compounding base (1 + y/m) = {} is non-positive for ytm={}, m={}",
                    base, ytm, m
                )));
            }
            base.powf(-m * t)
        }
        YieldCompounding::Continuous => (-ytm * t).exp(),
        YieldCompounding::Street => {
            let m = periods_per_year(bond_frequency)?.max(1.0);
            let base = 1.0 + ytm / m;
            if base <= 0.0 {
                return Err(finstack_quant_core::Error::Validation(format!(
                    "Street compounding base (1 + y/m) = {} is non-positive for ytm={}, m={}",
                    base, ytm, m
                )));
            }
            base.powf(-m * t)
        }
        YieldCompounding::TreasuryActual => {
            // ISDA/Treasury actual convention:
            // - Use simple interest for the first (potentially irregular) period
            // - Use periodic compounding for subsequent full periods
            //
            // LIMITATION: Stub period detection is TIME-BASED, not SCHEDULE-AWARE.
            // We identify the first period as t < 1/frequency (i.e., less than
            // one full coupon period). This is a reasonable approximation that
            // captures the essence of the convention for standard bonds.
            //
            // For bonds with irregular first coupons that don't align with the
            // standard frequency (e.g., a long-first stub spanning 8 months on
            // a semi-annual bond), this heuristic may misclassify the stub.
            // For exact ISDA compliance with non-standard structures, consider
            // passing actual stub information from the cashflow schedule.
            let m = periods_per_year(bond_frequency)?.max(1.0);
            let period_length = 1.0 / m;

            // Validate periodic compounding base for extreme negative yields
            let periodic_base = 1.0 + ytm / m;
            if periodic_base <= 0.0 {
                return Err(finstack_quant_core::Error::Validation(format!(
                    "TreasuryActual periodic base (1 + y/m) = {} is non-positive for ytm={}, m={}",
                    periodic_base, ytm, m
                )));
            }

            if t <= period_length {
                // First (potentially stub) period: simple interest
                let denom = 1.0 + ytm * t;
                if denom <= 0.0 {
                    return Err(finstack_quant_core::Error::Validation(format!(
                        "TreasuryActual simple interest denom (1 + y*t) = {} is non-positive for ytm={}, t={}",
                        denom, ytm, t
                    )));
                }
                1.0 / denom
            } else {
                // For subsequent periods, we need to compound:
                // - Simple interest for the first period portion
                // - Periodic compounding for the remaining full periods
                //
                // Total time t = stub_time + n_full_periods / m
                // where stub_time <= period_length
                //
                // DF = DF_stub * DF_periodic
                //    = 1/(1 + y*stub) * (1 + y/m)^(-n_full_periods)
                let n_full_periods = (t * m).floor();
                let stub_time = t - n_full_periods / m;

                if stub_time > 1e-10 {
                    // Has a stub period
                    let stub_denom = 1.0 + ytm * stub_time;
                    if stub_denom <= 0.0 {
                        return Err(finstack_quant_core::Error::Validation(format!(
                            "TreasuryActual stub denom (1 + y*stub) = {} is non-positive for ytm={}, stub_time={}",
                            stub_denom, ytm, stub_time
                        )));
                    }
                    let df_stub = 1.0 / stub_denom;
                    let df_periodic = periodic_base.powf(-n_full_periods);
                    df_stub * df_periodic
                } else {
                    // No stub, pure periodic
                    periodic_base.powf(-m * t)
                }
            }
        }
    })
}

/// `TreasuryActual` discount factor with a schedule-flagged first-period length.
///
/// Unlike [`df_from_yield`], which infers the first (stub) period purely from time
/// (`t <= 1/m`), this variant takes the **actual** first-coupon period length
/// `first_period_len` derived from the bond's cashflow schedule. Simple interest
/// is applied over the whole first period — long, short, or regular — and periodic
/// compounding over the remaining full periods. This avoids the 1-2bp
/// misclassification on new issues with irregular (notably long) first coupons.
pub(super) fn df_treasury_actual_with_first_period(
    ytm: f64,
    t: f64,
    m: f64,
    first_period_len: f64,
) -> finstack_quant_core::Result<f64> {
    // First-period simple-interest leg over `min(t, first_period_len)`.
    let stub_t = t.min(first_period_len);
    let stub_denom = 1.0 + ytm * stub_t;
    if stub_denom <= 0.0 {
        return Err(finstack_quant_core::Error::Validation(format!(
            "TreasuryActual simple interest denom (1 + y*t) = {stub_denom} is non-positive for ytm={ytm}, t={stub_t}"
        )));
    }
    let df_stub = 1.0 / stub_denom;

    if t <= first_period_len {
        return Ok(df_stub);
    }

    // Periodic compounding over the remaining time after the first period.
    let periodic_base = 1.0 + ytm / m;
    if periodic_base <= 0.0 {
        return Err(finstack_quant_core::Error::Validation(format!(
            "TreasuryActual periodic base (1 + y/m) = {periodic_base} is non-positive for ytm={ytm}, m={m}"
        )));
    }
    let remaining = t - first_period_len;
    Ok(df_stub * periodic_base.powf(-m * remaining))
}

/// Price from yield using explicit day count and frequency (no `Bond` borrow required).
///
/// For the [`YieldCompounding::TreasuryActual`] convention the first (potentially
/// irregular) coupon period is flagged from the **cashflow schedule** — the
/// year-fraction to the first post-`as_of` flow — rather than inferred from time.
/// This keeps the YTM↔price conversion correct for new issues with long first
/// coupons, where the time-based `t <= 1/m` heuristic in [`df_from_yield`] would
/// misapply simple interest to the wrong horizon.
///
/// # Arguments
///
/// * `day_count` - Bond coupon day-count convention used to measure settlement-
///   to-cashflow time.
/// * `frequency` - Contractual coupon frequency, including ACT/ACT reference-period
///   context and periodic compounding frequency.
/// * `flows` - Dated signed bond cashflows in payment-date order; flows on or
///   before `as_of` are excluded.
/// * `as_of` - Yield settlement/valuation date from which cashflows discount.
/// * `ytm` - Annual yield to maturity as a decimal under `comp`.
/// * `comp` - Yield compounding convention used to turn `ytm` into discount
///   factors.
#[inline]
pub fn price_from_ytm_compounded_params(
    day_count: finstack_quant_core::dates::DayCount,
    frequency: finstack_quant_core::dates::Tenor,
    flows: &[(Date, Money)],
    as_of: Date,
    ytm: f64,
    comp: YieldCompounding,
) -> finstack_quant_core::Result<f64> {
    // ACT/ACT (ICMA) requires the coupon frequency in the day-count context;
    // the default context hard-errors for that convention.
    let dc_ctx = DayCountContext {
        frequency: Some(frequency),
        ..DayCountContext::default()
    };

    // Schedule-aware first-period length for the TreasuryActual stub: the
    // year-fraction from `as_of` to the first cashflow strictly after `as_of`.
    let treasury_first_period = if matches!(comp, YieldCompounding::TreasuryActual) {
        let mut first: Option<f64> = None;
        for &(date, _) in flows {
            if date <= as_of {
                continue;
            }
            let yf = day_count.year_fraction(as_of, date, dc_ctx)?;
            if yf > 0.0 {
                first = Some(yf);
                break;
            }
        }
        first
    } else {
        None
    };

    let mut pv = NeumaierAccumulator::new();
    for &(date, amount) in flows {
        if date <= as_of {
            continue;
        }
        let t = day_count.year_fraction(as_of, date, dc_ctx)?;
        if t > 0.0 {
            let df = match (comp, treasury_first_period) {
                (YieldCompounding::TreasuryActual, Some(first_period_len)) => {
                    let m = periods_per_year(frequency)?.max(1.0);
                    df_treasury_actual_with_first_period(ytm, t, m, first_period_len)?
                }
                _ => df_from_yield(ytm, t, comp, frequency)?,
            };
            pv.add(amount.amount() * df);
        }
    }
    Ok(pv.total())
}

/// Price from ytm compounded.
///
/// # Arguments
///
/// * `bond` - Bond supplying coupon day count and frequency conventions.
/// * `flows` - Dated signed bond cashflows to discount; flows on or before
///   `as_of` are excluded.
/// * `as_of` - Yield settlement/valuation date from which cashflows discount.
/// * `ytm` - Annual yield to maturity as a decimal under `comp`.
/// * `comp` - Yield compounding convention used to turn `ytm` into discount
///   factors.
pub fn price_from_ytm_compounded(
    bond: &Bond,
    flows: &[(Date, Money)],
    as_of: Date,
    ytm: f64,
    comp: YieldCompounding,
) -> finstack_quant_core::Result<f64> {
    price_from_ytm_compounded_params(
        bond.cashflow_spec.day_count(),
        bond.cashflow_spec.frequency(),
        flows,
        as_of,
        ytm,
        comp,
    )
}

/// Price from ytm (using Street convention).
///
/// # Arguments
///
/// * `bond` - Bond supplying coupon day count and frequency conventions.
/// * `flows` - Dated signed bond cashflows to discount; flows on or before
///   `as_of` are excluded.
/// * `as_of` - Yield settlement/valuation date from which cashflows discount.
/// * `ytm` - Annual Street-compounded yield to maturity as a decimal.
pub fn price_from_ytm(
    bond: &Bond,
    flows: &[(Date, Money)],
    as_of: Date,
    ytm: f64,
) -> finstack_quant_core::Result<f64> {
    price_from_ytm_compounded(bond, flows, as_of, ytm, YieldCompounding::Street)
}

/// Compute outstanding principal at a given date from the cashflow schedule.
///
/// This is used by YTW and other yield calculations to determine the
/// redemption amount for amortizing callable/putable bonds.
pub(crate) fn outstanding_principal_at_date(schedule: &CashFlowSchedule, target_date: Date) -> f64 {
    let initial = schedule.get_notional().initial.amount();
    let mut outstanding = initial;

    // Sum all amortization and principal payments up to (and including) target_date
    for cf in schedule.get_flows() {
        if cf.date > target_date {
            break;
        }
        if matches!(cf.kind, CFKind::Amortization | CFKind::Notional) && cf.amount.amount() > 0.0 {
            outstanding -= cf.amount.amount();
        }
    }

    outstanding.max(0.0)
}

/// Enumerate call/put exit candidates for yield-to-worst analysis.
///
/// For each call or put window `[start_date, end_date]` in `bond.call_put`,
/// this function produces one `ExitCandidate` per admissible exercise date:
///
/// 1. Seed with `start_date` and `end_date`.
/// 2. Extend with any flow dates that fall within `[start_date, end_date]`.
/// 3. Sort and de-duplicate the resulting dates.
/// 4. Retain only dates in `[as_of, bond.maturity]`.
///
/// Returns an empty `Vec` when the bond has no `call_put` schedule.
///
/// # Arguments
///
/// * `bond`  – The bond whose `call_put` schedule is enumerated.
/// * `flows` – Holder-view cashflows used to align candidates to payment dates.
/// * `as_of` – Earliest admissible exercise date (valuation/quote date).
pub(crate) fn enumerate_exit_paths(
    bond: &Bond,
    flows: &[(Date, Money)],
    as_of: Date,
) -> Vec<ExitCandidate> {
    let Some(cp) = &bond.call_put else {
        return Vec::new();
    };

    let mut call_candidates: Vec<ExitCandidate> = Vec::new();
    let mut put_candidates: Vec<ExitCandidate> = Vec::new();

    let push_period_candidates = |candidates: &mut Vec<ExitCandidate>,
                                  start_date: Date,
                                  end_date: Date,
                                  price_pct_of_par: f64| {
        let align_to_flow_date = |boundary: Date| {
            flows
                .iter()
                .map(|(date, _)| *date)
                .filter_map(|date| {
                    let distance = (date - boundary).whole_days().unsigned_abs();
                    (distance <= 7).then_some((distance, date))
                })
                .min()
                .map_or(boundary, |(_, date)| date)
        };
        let aligned_start = align_to_flow_date(start_date);
        let aligned_end = align_to_flow_date(end_date);
        let mut exercise_dates = vec![aligned_start, aligned_end];
        exercise_dates.extend(
            flows
                .iter()
                .map(|(date, _)| *date)
                .filter(|date| *date >= aligned_start && *date <= aligned_end),
        );
        exercise_dates.sort_unstable();
        exercise_dates.dedup();

        for exercise_date in exercise_dates {
            if exercise_date >= as_of && exercise_date <= bond.maturity {
                candidates.push(ExitCandidate {
                    date: exercise_date,
                    price_pct_of_par,
                });
            }
        }
    };

    for c in &cp.calls {
        push_period_candidates(
            &mut call_candidates,
            c.start_date,
            c.end_date,
            c.price_pct_of_par,
        );
    }
    for p in &cp.puts {
        push_period_candidates(
            &mut put_candidates,
            p.start_date,
            p.end_date,
            p.price_pct_of_par,
        );
    }

    // Adjacent step-down windows share boundary dates. At such a boundary the
    // issuer exercises the cheapest call, while the holder exercises the most
    // valuable put. Retaining both stale and current strikes creates
    // economically impossible YTW paths.
    call_candidates.sort_by(|left, right| {
        left.date
            .cmp(&right.date)
            .then_with(|| left.price_pct_of_par.total_cmp(&right.price_pct_of_par))
    });
    call_candidates.dedup_by_key(|candidate| candidate.date);
    put_candidates.sort_by(|left, right| {
        left.date
            .cmp(&right.date)
            .then_with(|| right.price_pct_of_par.total_cmp(&left.price_pct_of_par))
    });
    put_candidates.dedup_by_key(|candidate| candidate.date);
    call_candidates.extend(put_candidates);

    call_candidates
}

/// Solve yield-to-worst over all call/put/maturity candidates for a given flow set.
///
/// Returns the worst (minimum) yield and the corresponding truncated cashflow path.
///
/// # Call/Put Redemption Convention
///
/// Call/put redemption prices are dirty street redemption amounts:
/// `outstanding_principal × (price_pct_of_par / 100) + accrued_interest(exercise_date)`,
/// where `outstanding_principal` is the remaining principal at the exercise date after
/// any amortization. This correctly handles amortizing callable bonds and is consistent
/// with the tree-based OAS pricing.
///
/// # Arguments
///
/// * `bond` - The bond to calculate YTW for
/// * `flows` - Holder-view cashflows (coupons + principal)
/// * `as_of` - Valuation/quote date
/// * `dirty_price_target` - Target dirty price to match
/// * `schedule` - Optional full cashflow schedule for accurate outstanding principal
///   computation on amortizing bonds. When `None`, falls back to original notional.
pub(crate) fn solve_ytw_from_flows(
    bond: &Bond,
    flows: &[(Date, Money)],
    as_of: Date,
    dirty_price_target: Money,
    schedule: Option<&CashFlowSchedule>,
) -> finstack_quant_core::Result<(f64, Vec<(Date, Money)>)> {
    // Generate call/put candidates + maturity.
    // Call/put paths come from enumerate_exit_paths; maturity is appended separately.
    let exit_paths = enumerate_exit_paths(bond, flows, as_of);
    let mut candidates: Vec<(Date, Money)> = exit_paths
        .into_iter()
        .map(|ec| {
            (
                ec.date,
                Money::new(ec.price_pct_of_par, bond.notional.currency()),
            )
        })
        .collect();

    // At maturity, principal redemption is already present in the cashflow schedule,
    // so use a zero additional redemption here to avoid double-counting.
    //
    // The redemption Notional flow is dated on the BDC-adjusted maturity, which can
    // roll past the unadjusted `bond.maturity` (e.g. maturity falling on a holiday),
    // so truncate the maturity candidate at the final projected flow date instead of
    // dropping the redemption.
    let maturity_candidate = flows
        .iter()
        .map(|(d, _)| *d)
        .max()
        .map_or(bond.maturity, |last| last.max(bond.maturity));
    candidates.push((
        maturity_candidate,
        Money::new(0.0, bond.notional.currency()),
    ));

    let mut best_yield = f64::INFINITY;
    let mut best_flows: Vec<(Date, Money)> = Vec::new();

    let accrual_cfg = bond.accrual_config();
    let accrual_index = match schedule {
        Some(sched) => Some(AccrualIndex::build(sched, &accrual_cfg)?),
        None => None,
    };

    for (exercise_date, pct_or_zero) in candidates {
        // Truncate flows to exercise and add redemption
        let mut ex_flows: Vec<(Date, Money)> = Vec::with_capacity(flows.len());
        for &(d, a) in flows {
            if d > as_of && d <= exercise_date {
                ex_flows.push((d, a));
            }
        }

        // Compute redemption amount:
        // - For maturity: pct is 0, so redemption is 0 (already in flows)
        // - For call/put: use dirty street redemption at exercise date
        let redemption = if pct_or_zero.amount() > 0.0 {
            // This is a call/put candidate, pct_or_zero holds the price_pct_of_par
            let pct = pct_or_zero.amount();
            // Use full schedule for accurate outstanding principal when available;
            // otherwise fall back to original notional (valid for bullet bonds).
            let outstanding = if let Some(sched) = schedule {
                outstanding_principal_at_date(sched, exercise_date)
            } else {
                bond.notional.amount()
            };
            let accrued = match accrual_index.as_ref() {
                Some(index) => index.accrued_at(exercise_date)?,
                None => 0.0,
            };
            Money::new(
                outstanding * (pct / 100.0) + accrued,
                bond.notional.currency(),
            )
        } else {
            Money::new(0.0, bond.notional.currency())
        };
        ex_flows.push((exercise_date, redemption));

        // Solve yield that matches target dirty price
        let coupon_rate = match &bond.cashflow_spec {
            crate::instruments::fixed_income::bond::CashflowSpec::Fixed(spec) => {
                spec.rate.to_f64().unwrap_or(0.0)
            }
            _ => 0.0,
        };
        let y = solve_ytm(
            &ex_flows,
            as_of,
            dirty_price_target,
            YtmPricingSpec {
                day_count: bond.cashflow_spec.day_count(),
                notional: bond.notional,
                coupon_rate,
                compounding: YieldCompounding::Street,
                frequency: bond.cashflow_spec.frequency(),
            },
        )?;
        if y < best_yield {
            best_yield = y;
            best_flows = ex_flows;
        }
    }

    Ok((best_yield, best_flows))
}

/// Price from Yield-To-Worst by scanning call/put candidates and selecting the lowest yield path.
///
/// # Arguments
///
/// * `bond` - Callable or puttable bond whose cashflow schedule and exercise
///   candidates define the yield-to-worst paths.
/// * `curves` - Market context supplying curve and schedule dependencies.
/// * `as_of` - Valuation date from which candidate cashflows are generated.
/// * `dirty_price_target` - Target dirty price in the bond notional currency
///   used to solve each candidate Street yield.
pub fn price_from_ytw(
    bond: &Bond,
    curves: &MarketContext,
    as_of: Date,
    dirty_price_target: Money,
) -> finstack_quant_core::Result<f64> {
    // Build signed canonical schedule flows and full schedule for accurate amortizing bond handling
    let flows = bond.pricing_dated_cashflows(curves, as_of)?;
    let schedule = bond.full_cashflow_schedule(curves)?;
    let (best_yield, best_flows) =
        solve_ytw_from_flows(bond, &flows, as_of, dirty_price_target, Some(&schedule))?;

    // Re-price along the worst-yield path for a consistent price result
    let best_price = price_from_ytm_compounded(
        bond,
        &best_flows,
        as_of,
        best_yield,
        YieldCompounding::Street,
    )?;

    Ok(best_price)
}
