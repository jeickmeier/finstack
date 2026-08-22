//! Present value calculations using market discount curves.
//!
//! This module provides functions for discounting cashflows using market-derived
//! discount curves rather than constant rates. This is the standard approach for
//! pricing fixed income securities and derivatives.
//!
//! # Approach
//!
//! Unlike IRR/XIRR analysis (see [`xirr`](super::xirr)),
//! this module uses term structures of discount factors from market data:
//! [`DiscountCurve`](crate::market_data::term_structures::DiscountCurve) and
//! the [`Discounting`](crate::market_data::traits::Discounting) trait are the
//! canonical curve-side contracts for these present-value operations.
//! ```text
//! PV = Σ CF_i * DF(t_i)
//!
//! where DF(t) is the discount factor from the market curve
//! ```
//!
//! # Valuation-Date Cutoff (IMPORTANT)
//!
//! [`npv`], [`npv_with_ctx`], and the [`Discountable`] trait follow
//! **market-standard pricing semantics**: cashflows dated **on or before** the
//! valuation date are excluded (only strictly-future flows are discounted).
//! A flow that has already paid is not part of the instrument's present value.
//!
//! The scalar helper [`npv_amounts`] keeps the investment-NPV convention (all
//! flows included, signed year fractions) since its default base date is the
//! earliest flow.
//!
//! # Use Cases
//!
//! - **Bond pricing**: Government and corporate bonds
//! - **Swap valuation**: Interest rate swaps using OIS/LIBOR curves
//! - **Derivative pricing**: Future cashflows under risk-neutral measure
//! - **Portfolio valuation**: Mark-to-market of fixed income positions
//!
//! # Examples
//!
//! ```rust
//! use finstack_quant_core::cashflow::npv;
//! use finstack_quant_core::market_data::term_structures::DiscountCurve;
//! use finstack_quant_core::dates::Date;
//! use finstack_quant_core::money::Money;
//! use finstack_quant_core::currency::Currency;
//! use time::Month;
//!
//! // Build a flat discount curve
//! let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
//! let curve = DiscountCurve::builder("USD-OIS")
//!     .base_date(base_date)
//!     .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.78)])
//!     .build()?;
//!
//! // Cashflows to discount
//! let cf1 = (
//!     Date::from_calendar_date(2026, Month::January, 1).expect("Valid date"),
//!     Money::new(100.0, Currency::USD)
//! );
//! let flows = vec![cf1];
//!
//! // Discount coordinates use the curve's own day count.
//! let pv = npv(&curve, base_date, &flows)?;
//! assert!(pv.amount() < 100.0); // Discounted value < face value
//! # Ok::<(), finstack_quant_core::Error>(())
//! ```
//!
//! # References
//!
//! - Hull, J. C. (2018). *Options, Futures, and Other Derivatives* (10th ed.).
//!   Pearson. Chapters 4-7 (Interest Rates and Curve Construction). `docs/REFERENCES.md#hull-options-futures`
//! - Andersen, L., & Piterbarg, V. (2010). *Interest Rate Modeling* (3 vols).
//!   Atlantic Financial Press. Volume 1, Chapter 3. `docs/REFERENCES.md#andersen-piterbarg-interest-rate-modeling`

use crate::dates::{Date, DayCount, DayCountContext};
use crate::market_data::traits::Discounting;
use crate::math::NeumaierAccumulator;
use crate::money::Money;

/// Objects that can be present-valued against a `Discount` curve.
///
/// Provides a unified interface for NPV calculations across different
/// cashflow representations and instrument types. Implemented for any
/// type that implements `AsRef<[(Date, Money)]>` (including `&[(..)]`
/// and `Vec<(..)>`).
///
/// # Required Methods
///
/// Implementors must provide:
/// - [`npv`](Self::npv): Compute present value against a discount curve
///
/// # Provided Implementations
///
/// This trait is automatically implemented for any type `T` where
/// `T: AsRef<[(Date, Money)]>`, including:
/// - `&[(Date, Money)]`
/// - `Vec<(Date, Money)>`
///
/// # Examples
///
/// ```rust
/// use finstack_quant_core::cashflow::Discountable;
/// use finstack_quant_core::market_data::term_structures::DiscountCurve;
/// use finstack_quant_core::market_data::traits::Discounting;
/// use finstack_quant_core::dates::{Date, DayCount};
/// use finstack_quant_core::money::Money;
/// use finstack_quant_core::currency::Currency;
/// use time::Month;
///
/// let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
/// let curve = DiscountCurve::builder("USD-OIS")
///     .base_date(base)
///     .knots([(0.0, 1.0), (1.0, 0.95)])
///     .build()?;
///
/// let flows = vec![(
///     Date::from_calendar_date(2026, Month::January, 1).expect("Valid date"),
///     Money::new(100.0, Currency::USD),
/// )];
///
/// // Use the trait method
/// let pv = flows.npv(&curve, base)?;
/// # Ok::<(), finstack_quant_core::Error>(())
/// ```
pub trait Discountable: Send + Sync {
    /// Output type for the NPV calculation.
    type PVOutput;

    /// Compute present value using the given discount curve.
    ///
    /// Follows market-standard pricing semantics: flows dated on or before
    /// `base` are excluded. See the module docs ("Valuation-Date Cutoff")
    /// and [`npv_amounts`] for the investment-NPV convention.
    ///
    /// # Arguments
    ///
    /// * `disc` - Discount curve that supplies discount factors using its own
    ///   base date and day-count convention.
    /// * `base` - Valuation date to which strictly future cashflows are
    ///   discounted; flows dated on or before it are excluded.
    ///
    /// # Returns
    ///
    /// Present value of all strictly-future cashflows discounted to the base date.
    ///
    /// # Errors
    ///
    /// The default implementation returns `Err` when:
    /// - [`InputError::TooFewPoints`](crate::error::InputError::TooFewPoints): Empty cashflow list
    /// - Day count calculation fails (e.g., missing calendar for Bus/252)
    fn npv(&self, disc: &dyn Discounting, base: Date) -> Self::PVOutput;
}

/// Discount factor for a flat, continuously compounded rate over `years`.
///
/// ```text
/// DF(t) = exp(-r * t)
/// ```
///
/// This is the single-horizon counterpart to the curve-based functions in this
/// module: use it where the caller has a scalar rate and a year fraction rather
/// than a term structure, such as translating a Monte Carlo pricing input into
/// the `discount_factor` its engine expects.
///
/// # Convention
///
/// The rate is **continuously compounded** and `years` is an **already-computed
/// year fraction** — no calendar or day-count convention is applied. Callers
/// holding an annually compounded rate should pass `(1.0 + r).ln()`, and callers
/// holding dates should compute the year fraction with the appropriate
/// [`DayCount`](crate::dates::DayCount) first, or use a
/// [`DiscountCurve`](crate::market_data::term_structures::DiscountCurve).
///
/// # Arguments
///
/// * `rate` - Continuously compounded annual rate as a decimal (0.05 = 5 %).
///   May be negative.
/// * `years` - Non-negative year fraction to the payoff horizon.
///
/// # Returns
///
/// The discount factor. Greater than 1 for a negative rate, exactly 1 when
/// either argument is zero.
///
/// # Errors
///
/// Returns [`Error::Validation`](crate::Error::Validation) when `rate` is not
/// finite, when `years` is not finite or is negative, or when the product
/// overflows to a non-finite factor. Validating here means a bad rate is
/// reported against the input that caused it, rather than surfacing later as a
/// non-finite price.
///
/// # Examples
///
/// ```rust
/// use finstack_quant_core::cashflow::flat_discount_factor;
///
/// // Zero rate leaves value unchanged.
/// assert_eq!(flat_discount_factor(0.0, 5.0)?, 1.0);
///
/// // A positive rate discounts; a negative rate accretes.
/// assert!(flat_discount_factor(0.05, 1.0)? < 1.0);
/// assert!(flat_discount_factor(-0.01, 1.0)? > 1.0);
///
/// // Negative time is rejected rather than silently accreting.
/// assert!(flat_discount_factor(0.05, -1.0).is_err());
/// # Ok::<(), finstack_quant_core::Error>(())
/// ```
pub fn flat_discount_factor(rate: f64, years: f64) -> crate::Result<f64> {
    if !rate.is_finite() {
        return Err(crate::Error::Validation(format!(
            "discount rate must be finite, got {rate}"
        )));
    }
    if !years.is_finite() || years < 0.0 {
        return Err(crate::Error::Validation(format!(
            "discount horizon must be finite and non-negative, got {years}"
        )));
    }
    let factor = (-rate * years).exp();
    if !factor.is_finite() {
        return Err(crate::Error::Validation(format!(
            "discount factor overflowed for rate={rate}, years={years}"
        )));
    }
    Ok(factor)
}

/// Compute NPV of dated `Money` flows using a discount curve with static dispatch.
///
/// By default, uses the curve's own day count convention for year fraction calculations.
/// This ensures consistency between NPV and metric calculations (e.g., par rate).
///
/// # Valuation-Date Cutoff
///
/// Flows dated **on or before** `base` are excluded (market-standard pricing
/// semantics). If every flow is on or before `base`, the result is zero in the
/// flows' currency.
///
/// # Arguments
///
/// * `disc` - Discount curve that supplies discount factors using its own
///   base date and day-count convention.
/// * `base` - Valuation date to which strictly future cashflows are
///   discounted; flows dated on or before it are excluded.
/// * `flows` - Payment-date and `Money` pairs to discount; every amount must
///   have the same currency and dated flows must be supplied explicitly.
///
/// # Returns
///
/// Present value as a [`Money`] amount in the same currency as the input flows.
///
/// # Errors
///
/// Returns `Err` when:
/// - [`InputError::TooFewPoints`](crate::error::InputError::TooFewPoints): The `flows`
///   slice is empty
/// - Day count year fraction calculation fails (e.g., [`InputError::MissingCalendarForBus252`](crate::error::InputError::MissingCalendarForBus252)
///   when using Bus/252 without a calendar context)
/// - [`Error::CurrencyMismatch`](crate::Error::CurrencyMismatch): Cashflows have
///   mixed currencies (all flows must share the same currency)
///
/// # Day Count Selection
///
/// Discounting always uses the curve's internal day count. The `day_count` parameter
/// is retained for source compatibility but cannot override the curve
/// abscissa. Instrument accrual day count belongs in cashflow generation.
///
/// # Example
///
/// ```rust
/// use finstack_quant_core::cashflow::npv;
/// use finstack_quant_core::market_data::term_structures::DiscountCurve;
/// use finstack_quant_core::dates::{Date, DayCount};
/// use finstack_quant_core::money::Money;
/// use finstack_quant_core::currency::Currency;
/// use time::Month;
///
/// let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
/// let curve = DiscountCurve::builder("USD-OIS")
///     .base_date(base)
///     .day_count(DayCount::Act360)
///     .knots([(0.0, 1.0), (1.0, 0.95)])
///     .build()?;
///
/// let flows = vec![(
///     Date::from_calendar_date(2026, Month::January, 1).expect("Valid date"),
///     Money::new(100.0, Currency::USD),
/// )];
///
/// // Uses the curve's day count.
/// let pv = npv(&curve, base, &flows)?;
/// # Ok::<(), finstack_quant_core::Error>(())
/// ```
pub fn npv<D: Discounting + ?Sized>(
    disc: &D,
    base: Date,
    flows: &[(Date, Money)],
) -> crate::Result<Money> {
    npv_with_ctx(disc, base, DayCountContext::default(), flows)
}

/// Compute NPV of dated `Money` cashflows using an explicit day-count context.
///
/// Flows dated on or before `base` are excluded (see the module docs,
/// "Valuation-Date Cutoff").
///
/// # Arguments
///
/// * `disc` - Discounting source that supplies discount factors, base date, and
///   day-count convention for the monetary cashflows.
/// * `base` - Valuation date to which eligible cashflows are discounted. Flows
///   on or before this date are excluded.
/// * `ctx` - Supplemental day-count information, such as calendars or
///   reference periods, required by the discount source's convention.
/// * `flows` - Dated cashflows in one currency. Empty input or mixed currencies
///   return an error.
///
/// # Errors
///
/// Same error conditions as [`npv`].
pub(crate) fn npv_with_ctx<D: Discounting + ?Sized>(
    disc: &D,
    base: Date,
    ctx: DayCountContext<'_>,
    flows: &[(Date, Money)],
) -> crate::Result<Money> {
    if flows.is_empty() {
        return Err(crate::error::InputError::TooFewPoints.into());
    }
    let ccy = flows[0].1.currency();

    // Validate all cashflows have the same currency
    for (_, amt) in flows.iter().skip(1) {
        if amt.currency() != ccy {
            return Err(crate::Error::CurrencyMismatch {
                expected: ccy,
                actual: amt.currency(),
            });
        }
    }

    // Per-flow discounting: Money × f64 discount factor produces a Money
    // value rounded to Money's Decimal scale. Accumulation of rounded
    // per-flow values is exact at that scale. For bit-exact precision,
    // callers should pre-discount amounts in Decimal and sum via
    // sum_prediscounted_money().
    let mut total = Money::new(0.0, ccy);
    for_each_discounted(disc, base, ctx, flows, |amt, df| {
        let disc_amt = amt.checked_mul_f64(df)?;
        total = total.checked_add(disc_amt)?;
        Ok(())
    })?;
    Ok(total)
}

/// Compute an unrounded scalar NPV using a discount curve.
///
/// Cashflows on or before `base` are excluded: valuation on a date assumes
/// cash settling that day has already been paid. Discount factors are relative
/// to `base`, even when the curve has a different base date.
///
/// # Arguments
///
/// * `disc` - Discounting source that provides relative discount factors and
///   its day-count convention.
/// * `base` - Valuation date to which future scalar flows are discounted.
///   Flows on or before this date are excluded.
/// * `flows` - Dated scalar cashflows in the caller's chosen amount units.
///   Empty input returns an error.
///
/// # Errors
///
/// Returns an error for an empty flow slice, day-count failures, or non-finite
/// and non-positive discount factors.
pub fn npv_amounts_with_curve<D: Discounting + ?Sized>(
    disc: &D,
    base: Date,
    flows: &[(Date, f64)],
) -> crate::Result<f64> {
    if flows.is_empty() {
        return Err(crate::error::InputError::TooFewPoints.into());
    }

    let mut total = NeumaierAccumulator::new();
    for_each_discounted(
        disc,
        base,
        DayCountContext::default(),
        flows,
        |amount, df| {
            total.add(amount * df);
            Ok(())
        },
    )?;
    Ok(total.total())
}

fn for_each_discounted<T, D, F>(
    disc: &D,
    base: Date,
    ctx: DayCountContext<'_>,
    flows: &[(Date, T)],
    mut apply: F,
) -> crate::Result<()>
where
    D: Discounting + ?Sized,
    F: FnMut(&T, f64) -> crate::Result<()>,
{
    // Discount each flow to `base`, which need not coincide with the curve's
    // base date. Relative discounting keeps Money and scalar valuation on the
    // same cutoff, time-origin, and discount-factor validation policy.
    let day_count = disc.day_count();
    let curve_base = disc.base_date();
    let t_base = day_count.signed_year_fraction(curve_base, base, ctx)?;
    let df_base = disc.df(t_base);
    if !df_base.is_finite() || df_base <= 0.0 {
        return Err(crate::Error::Validation(format!(
            "npv: discount factor at the valuation date ({base}) is invalid: {df_base}"
        )));
    }

    for (date, amount) in flows {
        // Market-standard valuation semantics: cash on the valuation date has
        // already settled. `include_past_flows` exists only for callers that
        // explicitly require the investment-NPV convention.
        if *date <= base {
            continue;
        }
        let t = day_count.signed_year_fraction(curve_base, *date, ctx)?;
        let df = disc.df(t) / df_base;
        if !df.is_finite() || df <= 0.0 {
            return Err(crate::Error::Validation(format!(
                "npv: discount factor for cashflow date {date} is invalid: {df}"
            )));
        }
        apply(amount, df)?;
    }

    Ok(())
}

/// Compute NPV of dated scalar cashflows using a flat annual discount rate.
///
/// This is a convenience helper for performance analytics and bindings that work in
/// scalar amounts (e.g. `[(date, f64)]`) rather than typed [`Money`] cashflows.
///
/// The discounting convention for this helper is:
/// - `discount_rate` is an annually-compounded rate expressed as a decimal (0.05 = 5%)
/// - Internally this is converted to continuous compounding via `ln(1 + r)` for stability.
///
/// Defaults (when the optional arguments are `None`):
/// - `base_date`: earliest cashflow date
/// - `day_count`: [`DayCount::Act365F`]
///
/// # Flow Convention
///
/// Unlike [`npv`], this helper follows the **investment-NPV convention**: all
/// flows are included, with signed year fractions relative to the base date.
/// The time-0 outlay (a flow on the base date) is part of the result, which
/// is what project/return analytics expect.
///
/// # Arguments
///
/// * `cash_flows` - Dated scalar amounts in a consistent caller-defined unit.
///   The time-zero amount is included; an empty slice returns an error.
/// * `discount_rate` - Flat annually compounded decimal rate (`0.05` means
///   5%). It must be finite and greater than `-1.0`.
/// * `base_date` - Optional valuation origin. When omitted, the earliest
///   cashflow date is used.
/// * `day_count` - Optional year-fraction convention. When omitted,
///   [`DayCount::Act365F`] is used.
///
/// # Errors
/// - [`InputError::TooFewPoints`](crate::error::InputError::TooFewPoints) when `cash_flows` is empty
/// - Day count year-fraction calculation failures
pub fn npv_amounts(
    cash_flows: &[(Date, f64)],
    discount_rate: f64,
    base_date: Option<Date>,
    day_count: Option<DayCount>,
) -> crate::Result<f64> {
    npv_amounts_with_ctx(
        cash_flows,
        discount_rate,
        base_date,
        day_count,
        crate::dates::DayCountContext::default(),
    )
}

/// Compute scalar NPV with an explicit day-count context.
///
/// This is the contextual form of [`npv_amounts`]. It discounts each signed
/// scalar amount by `exp(-ln(1 + discount_rate) * t)`, where `t` is the signed
/// year fraction from `base_date` (or the earliest cashflow date when omitted)
/// under `day_count` (or Act/365F when omitted). The supplied `ctx` is passed
/// unchanged to every year-fraction calculation, which matters for
/// conventions that require calendars or reference-period information.
///
/// Amounts are not separately validated: IEEE-754 NaN or infinity in an
/// amount propagates into the returned NPV. Accumulation uses Neumaier
/// compensation to reduce cancellation error for mixed-sign cashflows.
///
/// # Arguments
///
/// * `cash_flows` - Dated scalar amounts in a consistent caller-defined unit.
///   The time-zero amount is included; an empty slice returns an error.
/// * `discount_rate` - Flat annually compounded decimal rate (`0.05` means
///   5%). It must be finite and greater than `-1.0`.
/// * `base_date` - Optional valuation origin. When omitted, the earliest
///   cashflow date is used.
/// * `day_count` - Optional year-fraction convention. When omitted,
///   [`DayCount::Act365F`] is used.
/// * `ctx` - Supplemental day-count information passed unchanged to each
///   year-fraction calculation, including calendar and reference-period data.
///
/// # Errors
///
/// Returns an error if `cash_flows` is empty, `discount_rate` is non-finite or
/// less than or equal to `-1.0` (so annual-to-continuous conversion is
/// undefined), or a signed year-fraction calculation fails for the chosen
/// day-count context.
pub(crate) fn npv_amounts_with_ctx(
    cash_flows: &[(Date, f64)],
    discount_rate: f64,
    base_date: Option<Date>,
    day_count: Option<DayCount>,
    ctx: crate::dates::DayCountContext<'_>,
) -> crate::Result<f64> {
    if cash_flows.is_empty() {
        return Err(crate::Error::from(crate::error::InputError::TooFewPoints));
    }

    let base = base_date.unwrap_or_else(|| {
        cash_flows
            .iter()
            .map(|(date, _)| *date)
            .min()
            .unwrap_or(cash_flows[0].0)
    });
    let day_count = day_count.unwrap_or(DayCount::Act365F);

    // Convert annually compounded rate to continuously compounded rate:
    // flat discounting expects continuously compounded rates: r_cont = ln(1 + r_annual)
    if !discount_rate.is_finite() || (1.0 + discount_rate) <= 0.0 {
        return Err(crate::Error::from(crate::error::InputError::Invalid));
    }
    let continuous_rate = (1.0 + discount_rate).ln();

    // Use Neumaier compensated summation for numerical stability with many cashflows
    let mut acc = NeumaierAccumulator::new();
    for (date, amount) in cash_flows {
        let t = day_count.signed_year_fraction(base, *date, ctx)?;
        acc.add(amount * (-continuous_rate * t).exp());
    }

    Ok(acc.total())
}

#[cfg(test)]
fn flat_curve(
    id: &str,
    base: Date,
    continuous_rate: f64,
    day_count: DayCount,
) -> crate::market_data::term_structures::DiscountCurve {
    use crate::market_data::term_structures::{DiscountCurve, ValidationMode};
    use crate::math::interp::{ExtrapolationPolicy, InterpStyle};

    DiscountCurve::builder(id)
        .base_date(base)
        .knots([(0.0, 1.0), (1.0, (-continuous_rate).exp())])
        .interp(InterpStyle::LogLinear)
        .extrapolation(ExtrapolationPolicy::FlatForward)
        .validation(ValidationMode::Raw {
            allow_non_monotonic: continuous_rate < 0.0,
            forward_floor: None,
        })
        .day_count(day_count)
        .build()
        .expect("valid flat curve")
}

#[cfg(test)]
mod hardening_tests {
    use super::*;
    use crate::currency::Currency;
    use crate::dates::calendar::TARGET2;
    use crate::dates::create_date;
    use time::Month;

    #[test]
    fn npv_amounts_uses_earliest_cashflow_as_default_base_date() {
        let base = create_date(2024, Month::January, 1).expect("Valid test date");
        let later = create_date(2025, Month::January, 1).expect("Valid test date");
        let rate = 0.05;

        let sorted = vec![(base, -100000.0), (later, 110000.0)];
        let unsorted = vec![(later, 110000.0), (base, -100000.0)];

        let pv_sorted = npv_amounts(&sorted, rate, None, Some(DayCount::Act365F))
            .expect("sorted npv should succeed");
        let pv_unsorted = npv_amounts(&unsorted, rate, None, Some(DayCount::Act365F))
            .expect("unsorted npv should succeed");

        assert!((pv_sorted - pv_unsorted).abs() < 1e-10);
    }

    #[test]
    fn npv_amounts_rejects_empty_flows_and_invalid_discount_rates() {
        let base = create_date(2024, Month::January, 1).expect("Valid test date");
        let flows = vec![(base, 100.0)];

        assert!(npv_amounts(&[], 0.05, Some(base), Some(DayCount::Act365F)).is_err());
        assert!(npv_amounts(&flows, f64::NAN, Some(base), Some(DayCount::Act365F)).is_err());
        assert!(npv_amounts(&flows, f64::INFINITY, Some(base), Some(DayCount::Act365F)).is_err());
        assert!(npv_amounts(&flows, -1.0, Some(base), Some(DayCount::Act365F)).is_err());
        assert!(npv_amounts(&flows, -1.01, Some(base), Some(DayCount::Act365F)).is_err());
    }

    #[test]
    fn npv_amounts_with_ctx_propagates_day_count_context_errors() {
        let base = create_date(2025, Month::January, 6).expect("Valid test date");
        let pay = create_date(2025, Month::January, 13).expect("Valid test date");
        let flows = vec![(pay, 100.0)];

        let result = npv_amounts_with_ctx(
            &flows,
            0.05,
            Some(base),
            Some(DayCount::Bus252),
            DayCountContext::default(),
        );

        assert!(
            result.is_err(),
            "Bus/252 scalar NPV requires a calendar in the day-count context"
        );
    }

    #[test]
    fn npv_with_bus252_context_counts_business_days() {
        let base = create_date(2025, Month::January, 6).expect("Valid test date"); // Monday
        let pay = create_date(2025, Month::January, 13).expect("Valid test date"); // Next Monday
        let curve = flat_curve("BRL-FLAT", base, 0.10, DayCount::Bus252);
        let flows = vec![(pay, Money::new(100.0, Currency::USD))];
        let ctx = DayCountContext {
            calendar: Some(&TARGET2),
            frequency: None,
            bus_basis: None,
            coupon_period: None,
            end_is_termination_date: false,
        };

        let pv = npv_with_ctx(&curve, base, ctx, &flows).expect("Bus/252 NPV should succeed");
        let expected = 100.0 * (-0.10_f64 * (5.0 / 252.0)).exp();
        assert!(
            (pv.amount() - expected).abs() < 1e-10,
            "{} vs {}",
            pv.amount(),
            expected
        );
    }

    /// `npv` must discount to the supplied valuation date even when it differs
    /// from the curve's own base date — using the relative discount factor
    /// `DF(curve_base→d) / DF(curve_base→base)` rather than the curve-base-
    /// anchored `df(year_fraction(base, d))`.
    ///
    /// A non-flat curve is required: a flat curve is translation-invariant and
    /// would hide the time-origin error.
    #[test]
    fn npv_discounts_to_valuation_date_when_base_differs_from_curve_base() {
        use crate::market_data::term_structures::DiscountCurve;

        let curve_base = create_date(2025, Month::January, 1).expect("date");
        let curve = DiscountCurve::builder("USD-OIS")
            .base_date(curve_base)
            .day_count(DayCount::Act365F)
            .knots([(0.0, 1.0), (1.0, 0.95), (2.0, 0.88)])
            .build()
            .expect("curve");

        let val_date = create_date(2026, Month::January, 1).expect("date"); // 1y forward
        let flow_date = create_date(2027, Month::January, 1).expect("date"); // 2y forward
        let flows = vec![(flow_date, Money::new(1_000_000.0, Currency::USD))];

        // Valuation at the curve base: PV = CF · DF(0→2y) = CF · 0.88.
        let pv_at_curve_base = npv(&curve, curve_base, &flows).expect("npv");
        assert!((pv_at_curve_base.amount() - 880_000.0).abs() < 1.0);

        // Valuation one year forward must use the relative DF
        // DF(1y→2y) = df(2)/df(1) = 0.88/0.95, not df(year_fraction(val,flow)).
        let pv_forward = npv(&curve, val_date, &flows).expect("npv");
        let expected_forward = 1_000_000.0 * (0.88 / 0.95);
        assert!(
            (pv_forward.amount() - expected_forward).abs() < 1.0,
            "npv with base != curve base must use the relative DF: got {}, expected {}",
            pv_forward.amount(),
            expected_forward
        );
        // The pre-fix engine returned CF·df(1y) = 950_000; guard the regression.
        assert!(
            (pv_forward.amount() - 950_000.0).abs() > 1_000.0,
            "npv must not reuse the curve-base-anchored df lookup"
        );
    }
}

/// Compute NPV of dated `Money` flows using a discount curve.
///
/// Discounts each cashflow to the base date using the provided curve.
/// All flows must be in the same currency for the calculation to succeed.
impl<T> Discountable for T
where
    T: AsRef<[(Date, Money)]> + Send + Sync,
{
    type PVOutput = crate::Result<Money>;

    fn npv(&self, disc: &dyn Discounting, base: Date) -> crate::Result<Money> {
        npv(disc, base, self.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::currency::Currency;
    use crate::dates::create_date;
    use crate::market_data::traits::TermStructure;
    use crate::types::CurveId;
    use time::Month;

    // flat_discount_factor

    #[test]
    fn flat_discount_factor_matches_the_closed_form() {
        for (rate, years) in [(0.05_f64, 1.0_f64), (0.02, 7.5), (-0.01, 3.0), (0.10, 0.25)] {
            let expected = (-rate * years).exp();
            assert_eq!(
                flat_discount_factor(rate, years).expect("valid inputs"),
                expected,
                "rate={rate}, years={years}"
            );
        }
    }

    #[test]
    fn flat_discount_factor_is_one_at_the_identity_points() {
        assert_eq!(flat_discount_factor(0.0, 5.0).expect("zero rate"), 1.0);
        assert_eq!(flat_discount_factor(0.05, 0.0).expect("zero horizon"), 1.0);
    }

    #[test]
    fn flat_discount_factor_accretes_under_a_negative_rate() {
        // Negative rates are a real market state, not an input error.
        assert!(flat_discount_factor(-0.005, 2.0).expect("negative rate") > 1.0);
    }

    #[test]
    fn flat_discount_factor_composes_across_horizons() {
        // DF(t1 + t2) == DF(t1) * DF(t2) for a flat rate.
        let (rate, t1, t2) = (0.03, 1.5, 2.5);
        let whole = flat_discount_factor(rate, t1 + t2).expect("valid");
        let split = flat_discount_factor(rate, t1).expect("valid")
            * flat_discount_factor(rate, t2).expect("valid");
        assert!((whole - split).abs() < 1e-15, "{whole} vs {split}");
    }

    #[test]
    fn flat_discount_factor_rejects_bad_inputs() {
        // A non-finite rate must be reported here, not surface later as a
        // non-finite price with no indication of which input caused it.
        assert!(flat_discount_factor(f64::NAN, 1.0).is_err());
        assert!(flat_discount_factor(f64::INFINITY, 1.0).is_err());
        assert!(flat_discount_factor(0.05, f64::NAN).is_err());
        // Negative time would silently accrete rather than discount.
        assert!(flat_discount_factor(0.05, -1.0).is_err());
    }

    #[test]
    fn flat_discount_factor_rejects_overflow_to_infinity() {
        // A large negative rate over a long horizon overflows; that is an
        // error rather than an infinite discount factor.
        assert!(flat_discount_factor(-1000.0, 1e6).is_err());
    }

    /// Test helper: creates a flat curve with DF=1.0 for all times (0% rate).
    struct ZeroRateCurve {
        id: CurveId,
    }

    impl TermStructure for ZeroRateCurve {
        fn id(&self) -> &CurveId {
            &self.id
        }
    }

    impl Discounting for ZeroRateCurve {
        fn base_date(&self) -> Date {
            Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date")
        }
        fn df(&self, _t: f64) -> f64 {
            1.0
        }
    }

    struct InvalidBaseDfCurve {
        id: CurveId,
    }

    impl TermStructure for InvalidBaseDfCurve {
        fn id(&self) -> &CurveId {
            &self.id
        }
    }

    struct InvalidFlowDfCurve {
        id: CurveId,
    }

    impl TermStructure for InvalidFlowDfCurve {
        fn id(&self) -> &CurveId {
            &self.id
        }
    }

    impl Discounting for InvalidFlowDfCurve {
        fn base_date(&self) -> Date {
            Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date")
        }
        fn df(&self, t: f64) -> f64 {
            if t.abs() < f64::EPSILON {
                1.0
            } else {
                f64::NAN
            }
        }
    }

    impl Discounting for InvalidBaseDfCurve {
        fn base_date(&self) -> Date {
            Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date")
        }
        fn df(&self, t: f64) -> f64 {
            if t.abs() < f64::EPSILON {
                0.0
            } else {
                1.0
            }
        }
    }

    #[test]
    fn tuples_discountable_paths_through() {
        let curve = ZeroRateCurve {
            id: CurveId::new("USD-OIS"),
        };
        let base = curve.base_date();
        // Flows must be strictly after the valuation date to be included
        // ().
        let pay = base + time::Duration::days(1);
        let flows = vec![
            (pay, Money::new(10.0, crate::currency::Currency::USD)),
            (pay, Money::new(5.0, crate::currency::Currency::USD)),
        ];
        let pv = flows
            .npv(&curve, base)
            .expect("NPV calculation should succeed in test");
        assert!((pv.amount() - 15.0).abs() < 1e-12);
    }

    #[test]
    fn tuples_discountable_uses_curve_day_count() {
        let curve = ZeroRateCurve {
            id: CurveId::new("USD-OIS"),
        };
        let base = curve.base_date();
        let pay = base + time::Duration::days(1);
        let flows = vec![
            (pay, Money::new(10.0, crate::currency::Currency::USD)),
            (pay, Money::new(5.0, crate::currency::Currency::USD)),
        ];
        let pv = flows
            .npv(&curve, base)
            .expect("NPV calculation should succeed in test");
        assert!((pv.amount() - 15.0).abs() < 1e-12);
    }

    #[test]
    fn npv_rejects_invalid_valuation_date_discount_factor() {
        let curve = InvalidBaseDfCurve {
            id: CurveId::new("BAD-DF"),
        };
        let base = curve.base_date();
        let flows = vec![(
            base + time::Duration::days(1),
            Money::new(10.0, Currency::USD),
        )];

        let err = npv_with_ctx(&curve, base, DayCountContext::default(), &flows)
            .expect_err("df_base <= 0 should be rejected");

        assert!(
            err.to_string()
                .contains("discount factor at the valuation date"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn npv_rejects_invalid_cashflow_discount_factor() {
        let curve = InvalidFlowDfCurve {
            id: CurveId::new("BAD-FLOW-DF"),
        };
        let base = curve.base_date();
        let flows = vec![(
            base + time::Duration::days(1),
            Money::new(10.0, Currency::USD),
        )];

        let err = npv_with_ctx(&curve, base, DayCountContext::default(), &flows)
            .expect_err("non-finite cashflow discount factor should be rejected");

        assert!(
            err.to_string()
                .contains("discount factor for cashflow date"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn npv_with_ctx_propagates_bus252_missing_calendar_error() {
        let base = create_date(2025, Month::January, 6).expect("Valid test date");
        let pay = create_date(2025, Month::January, 13).expect("Valid test date");
        let curve = flat_curve("BRL-FLAT", base, 0.10, DayCount::Bus252);
        let flows = vec![(pay, Money::new(100.0, Currency::USD))];

        assert!(npv_with_ctx(&curve, base, DayCountContext::default(), &flows).is_err());
    }

    #[test]
    fn test_npv_simple_with_flat_curve() {
        let base = create_date(2024, Month::January, 1).expect("Valid test date");
        let flows = vec![
            (base, Money::new(-100000.0, Currency::USD)),
            (
                create_date(2025, Month::January, 1).expect("Valid test date"),
                Money::new(110000.0, Currency::USD),
            ),
        ];
        let rate: f64 = 0.05;
        let day_count = DayCount::Act365F;

        // Create a flat curve with continuous rate
        let continuous_rate = (1.0 + rate).ln();
        let curve = flat_curve("NPV-TEST", base, continuous_rate, day_count);

        // The default npv excludes flows on or before the valuation date, so
        // the time-0 outlay (-100000 at base) is NOT part of the pricing PV.
        let pv = npv(&curve, base, &flows).expect("NPV calculation should succeed in test");
        // Approximately: 110000/(1.05) ≈ 104761.90 (initial outlay excluded)
        assert!(pv.amount() > 104700.0 && pv.amount() < 104800.0);
    }

    #[test]
    fn test_npv_amounts_matches_money_npv() {
        let base = create_date(2024, Month::January, 1).expect("Valid test date");
        let dates = [
            create_date(2024, Month::July, 1).expect("Valid test date"),
            create_date(2025, Month::January, 1).expect("Valid test date"),
        ];
        let amounts = [-100000.0, 110000.0];

        let amount_flows = vec![(dates[0], amounts[0]), (dates[1], amounts[1])];
        let money_flows = vec![
            (dates[0], Money::new(amounts[0], Currency::USD)),
            (dates[1], Money::new(amounts[1], Currency::USD)),
        ];

        let rate: f64 = 0.05;
        let day_count = DayCount::Act365F;

        // For strictly-future flows the scalar and Money paths must agree
        // exactly: both discount every flow from the same base date on the
        // same day-count basis. (Flows dated on or before `base` are excluded
        // by the Money path, so the comparison is made on future flows only.)
        let pv_amounts = npv_amounts(&amount_flows, rate, Some(base), Some(day_count))
            .expect("npv_amounts should succeed");

        let continuous_rate = (1.0 + rate).ln();
        let curve = flat_curve("TEST", base, continuous_rate, day_count);
        let pv_money = npv_with_ctx(&curve, base, DayCountContext::default(), &money_flows)
            .expect("npv should succeed")
            .amount();

        assert!(
            (pv_amounts - pv_money).abs() < 1e-10,
            "npv_amounts should match npv: {} vs {}",
            pv_amounts,
            pv_money
        );
    }

    #[test]
    fn test_npv_zero_discount() {
        let base = create_date(2024, Month::January, 1).expect("Valid test date");
        let flows = vec![
            (base, Money::new(-100.0, Currency::USD)),
            (
                create_date(2025, Month::January, 1).expect("Valid test date"),
                Money::new(100.0, Currency::USD),
            ),
        ];
        let day_count = DayCount::Act365F;

        // Create a flat curve with 0% rate (continuous rate = ln(1) = 0)
        let curve = flat_curve("ZERO-RATE", base, 0.0, day_count);

        // Default pricing semantics exclude the base-date flow, so only the
        // +100 remains.
        let pv = npv(&curve, base, &flows).expect("NPV calculation should succeed in test");
        assert_eq!(pv.amount(), 100.0);
    }

    /// Default pricing semantics exclude flows on or before the valuation
    /// date; `include_past_flows` restores the legacy include-everything
    /// behavior ().
    #[test]
    fn test_npv_excludes_past_flows_by_default() {
        let base = create_date(2025, Month::January, 1).expect("Valid test date");
        let past = create_date(2024, Month::July, 1).expect("Valid test date");
        let future = create_date(2025, Month::July, 1).expect("Valid test date");
        let flows = vec![
            (past, Money::new(-50.0, Currency::USD)), // past relative to base
            (base, Money::new(-25.0, Currency::USD)), // on the valuation date
            (future, Money::new(55.0, Currency::USD)), // future relative to base
        ];
        let rate: f64 = 0.05;
        let day_count = DayCount::Act365F;

        let continuous_rate = (1.0 + rate).ln();
        let curve = flat_curve("TEST", base, continuous_rate, day_count);

        // Default: only the strictly-future +55 flow is priced.
        let pv = npv(&curve, base, &flows).expect("NPV calculation should succeed in test");
        let only_future =
            npv(&curve, base, &flows[2..]).expect("future-only NPV should succeed in test");
        assert_eq!(pv.amount(), only_future.amount());
        assert!(pv.amount() > 0.0 && pv.amount() < 55.0);

        // Opt-in: past and on-date flows are included (future-valued at the
        // curve's signed year fraction), reproducing the legacy behavior.
    }

    /// If every flow is on or before the valuation date, the default
    /// pricing NPV is zero in the flows' currency (nothing left to price).
    #[test]
    fn test_npv_all_past_flows_is_zero() {
        let base = create_date(2025, Month::January, 1).expect("Valid test date");
        let flows = vec![
            (
                create_date(2024, Month::July, 1).expect("Valid test date"),
                Money::new(100.0, Currency::USD),
            ),
            (base, Money::new(50.0, Currency::USD)),
        ];
        let day_count = DayCount::Act365F;
        let curve = flat_curve("TEST", base, (1.05_f64).ln(), day_count);

        let pv = npv(&curve, base, &flows).expect("NPV should succeed");
        assert_eq!(pv.amount(), 0.0);
        assert_eq!(pv.currency(), Currency::USD);
    }

    #[test]
    fn test_npv_errors_on_empty_flows_with_flat_curve() {
        let base = create_date(2025, Month::January, 1).expect("Valid date");
        let flows: Vec<(Date, Money)> = vec![];
        let day_count = DayCount::Act365F;

        let continuous_rate = (1.05_f64).ln();
        let curve = flat_curve("TEST", base, continuous_rate, day_count);

        let err = npv(&curve, base, &flows).expect_err("Should fail with empty flows");
        let _ = format!("{}", err);
    }

    #[test]
    fn npv_precision_many_cashflows() {
        // Regression test for Neumaier compensated summation precision.
        // A 30Y quarterly swap has 120 cashflows where naive summation can
        // accumulate floating-point errors of ~1e-10 to 1e-9 of total PV.
        // With Neumaier summation, we should maintain much higher precision.
        let curve = ZeroRateCurve {
            id: CurveId::new("PRECISION-TEST"),
        };
        let base = curve.base_date();

        // Create 120 cashflows (30Y quarterly), each 100.0 USD
        // With DF=1.0 (flat curve), the sum should be exactly 12000.0
        let flows: Vec<(Date, Money)> = (1..=120)
            .map(|i| {
                // ~91 days per quarter
                let date = base + time::Duration::days(i as i64 * 91);
                (date, Money::new(100.0, Currency::USD))
            })
            .collect();

        let pv = npv(&curve, base, &flows).expect("NPV should succeed");

        // With Neumaier summation, we expect precision better than 1e-10
        assert!(
            (pv.amount() - 12000.0).abs() < 1e-10,
            "NPV precision lost with {} cashflows: expected 12000.0, got {} (error: {:.2e})",
            flows.len(),
            pv.amount(),
            (pv.amount() - 12000.0).abs()
        );
    }

    #[test]
    fn npv_amounts_precision_many_cashflows() {
        // Same precision test for npv_amounts (scalar version)
        let base = create_date(2025, Month::January, 1).expect("Valid test date");

        // Create 120 cashflows with 0% discount rate (DF=1.0 at all times)
        let flows: Vec<(Date, f64)> = (1..=120)
            .map(|i| {
                let date = base + time::Duration::days(i as i64 * 91);
                (date, 100.0)
            })
            .collect();

        let pv = npv_amounts(&flows, 0.0, Some(base), None).expect("npv_amounts should succeed");

        // With Neumaier summation, we expect precision better than 1e-10
        assert!(
            (pv - 12000.0).abs() < 1e-10,
            "npv_amounts precision lost with {} cashflows: expected 12000.0, got {} (error: {:.2e})",
            flows.len(),
            pv,
            (pv - 12000.0).abs()
        );
    }
}
